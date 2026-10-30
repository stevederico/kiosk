//! Rails-compatible value encoding.
//!
//! The Rails kiosk rendered JSON through ActiveSupport (json gem 2.13) and
//! stored timestamps and decimals through ActiveRecord's SQLite adapter. These
//! helpers reproduce those byte formats so responses and stored rows match the
//! Rails app exactly. Every rule here was checked against Rails 8.0.2 running
//! from a copy of the original app.

/// Append `s` as a JSON string literal, escaped the way ActiveSupport does.
///
/// Beyond the JSON minimum, ActiveSupport escapes `<`, `>`, `&`, U+2028 and
/// U+2029 so the output is safe to inline in HTML (`"Hotdog \u0026 Soda"`).
/// Other control characters use lowercase `\u00xx`; non-ASCII stays raw UTF-8.
pub fn push_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Format a float the way the json gem 2.13 generator does (`Float#to_json`).
///
/// Shortest round-trip digits, then fpconv's layout rules: a plain integer
/// (with `.0`) while the exponent stays under `digits + 7`, a plain decimal
/// with under 7 fraction digits or an exponent under 10, scientific (`1e+14`,
/// `1.5e-10`) otherwise. Non-finite values encode as `null`, as ActiveSupport does.
pub fn float(x: f64) -> String {
    if !x.is_finite() {
        return "null".to_string();
    }
    let neg = x.is_sign_negative();
    let sci = format!("{:e}", x.abs());
    let (mantissa, exp10) = sci.split_once('e').unwrap_or((sci.as_str(), "0"));
    let exp10: i32 = exp10.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let n = digits.len() as i32;
    // value = digits × 10^k
    let k = exp10 - (n - 1);
    let exp = (k + n - 1).abs();
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    if k >= 0 && exp < n + 7 {
        out.push_str(&digits);
        out.push_str(&"0".repeat(k as usize));
        out.push_str(".0");
    } else if k < 0 && (k > -7 || exp < 10) {
        let offset = n + k;
        if offset <= 0 {
            out.push_str("0.");
            out.push_str(&"0".repeat((-offset) as usize));
            out.push_str(&digits);
        } else {
            let (int_part, frac_part) = digits.split_at(offset as usize);
            out.push_str(int_part);
            out.push('.');
            out.push_str(frac_part);
        }
    } else {
        let (first, rest) = digits.split_at(1);
        out.push_str(first);
        if !rest.is_empty() {
            out.push('.');
            out.push_str(rest);
        }
        out.push('e');
        out.push(if exp10 < 0 { '-' } else { '+' });
        out.push_str(&exp10.abs().to_string());
    }
    out
}

/// Render a stored decimal the way Rails serializes a `BigDecimal` column.
///
/// SQLite hands the `decimal` column back as REAL or INTEGER; Rails turns it
/// into a `BigDecimal` whose JSON form is a plain-notation string: `1.5`,
/// `0.79`, `2.0`. Shortest round-trip digits match what Rails printed for
/// every seeded price.
pub fn decimal_string(x: f64) -> String {
    let s = format!("{x}");
    if s.contains('.') || s.contains("inf") || s.contains("NaN") {
        s
    } else {
        format!("{s}.0")
    }
}

/// Civil date from days since the Unix epoch (Howard Hinnant's algorithm).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Format epoch microseconds as Rails stores a `datetime(6)` in SQLite:
/// `2026-10-30 15:24:43.540984` (UTC).
pub fn db_timestamp(micros: i64) -> String {
    let secs = micros.div_euclid(1_000_000);
    let frac = micros.rem_euclid(1_000_000);
    let (days, sod) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}.{frac:06}",
        sod / 3600,
        (sod % 3600) / 60,
        sod % 60
    )
}

/// Current time in epoch microseconds. A clock before the epoch clamps to 0.
pub fn now_micros() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros() as i64)
        .unwrap_or(0)
}

/// Convert a stored `datetime(6)` to the JSON form Rails renders:
/// `2026-10-30 15:24:43.540984` → `2026-10-30T15:24:43.540Z`.
///
/// Fractional seconds are truncated (not rounded) to milliseconds, as
/// `ActiveSupport::TimeWithZone#xmlschema(3)` does.
pub fn json_timestamp(stored: &str) -> String {
    let (date_time, frac) = stored.split_once('.').unwrap_or((stored, ""));
    let millis: String = frac.chars().chain("000".chars()).take(3).collect();
    format!("{}.{millis}Z", date_time.replacen(' ', "T", 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc(s: &str) -> String {
        let mut out = String::new();
        push_str(&mut out, s);
        out
    }

    #[test]
    fn escapes_html_sensitive_chars_like_active_support() {
        assert_eq!(enc("Hotdog & Soda"), r#""Hotdog \u0026 Soda""#);
    }

    #[test]
    fn escapes_controls_quotes_and_separators() {
        let input = "\u{1}\u{1f}\u{7f}/\u{e9}\t\u{8}\u{c}\n\r\"\\<>&\u{2028}\u{2029}";
        let expected = "\"\\u0001\\u001f\u{7f}/\u{e9}\\t\\b\\f\\n\\r\\\"\\\\\\u003c\\u003e\\u0026\\u2028\\u2029\"";
        assert_eq!(enc(input), expected);
    }

    #[test]
    fn floats_match_json_gem_layout() {
        // Pairs captured from ActiveSupport::JSON.encode on Rails 8.0.2 / json 2.13.
        let cases: &[(f64, &str)] = &[
            (1.0, "1.0"),
            (100.0, "100.0"),
            (1e7, "10000000.0"),
            (1e8, "1e+8"),
            (1.5e8, "150000000.0"),
            (1e14, "1e+14"),
            (99_999_999_999_999.9, "99999999999999.9"),
            (1e15, "1e+15"),
            (1e15 + 1.0, "1000000000000001.0"),
            (123_456_789_012_345.0, "123456789012345.0"),
            (1.5e15, "1.5e+15"),
            (1e22, "1e+22"),
            (12_345_678_901_234_567_000.0, "12345678901234567000.0"),
            (0.1, "0.1"),
            (0.0001, "0.0001"),
            (0.00001, "0.00001"),
            (2.5e-7, "0.00000025"),
            (1e-9, "0.000000001"),
            (1.5e-9, "0.0000000015"),
            (1e-10, "1e-10"),
            (1.23e-10, "1.23e-10"),
            (123.456, "123.456"),
            (-1e15, "-1e+15"),
            (-0.00001, "-0.00001"),
            (-0.0, "-0.0"),
            (1.0 / 3.0, "0.3333333333333333"),
            (1e100, "1e+100"),
            (1.5e-100, "1.5e-100"),
            (1.797_693_134_862_315_7e308, "1.7976931348623157e+308"),
            (5e-324, "5e-324"),
        ];
        for (x, want) in cases {
            assert_eq!(float(*x), *want, "float({x:e})");
        }
    }

    #[test]
    fn non_finite_float_is_null() {
        assert_eq!(float(f64::INFINITY), "null");
    }

    #[test]
    fn decimal_strings_match_seeded_prices() {
        assert_eq!(decimal_string(1.5), "1.5");
        assert_eq!(decimal_string(1.99), "1.99");
        assert_eq!(decimal_string(0.79), "0.79");
        assert_eq!(decimal_string(9.95), "9.95");
    }

    #[test]
    fn integral_decimal_keeps_point_zero() {
        assert_eq!(decimal_string(2.0), "2.0");
    }

    #[test]
    fn db_timestamp_has_microseconds() {
        // 2026-10-30T15:24:43.540984Z
        assert_eq!(
            db_timestamp(1_793_373_883_540_984),
            "2026-10-30 15:24:43.540984"
        );
    }

    #[test]
    fn db_timestamp_epoch() {
        assert_eq!(db_timestamp(0), "1970-01-01 00:00:00.000000");
    }

    #[test]
    fn json_timestamp_truncates_to_millis() {
        assert_eq!(
            json_timestamp("2026-10-28 01:10:18.559999"),
            "2026-10-28T01:10:18.559Z"
        );
    }

    #[test]
    fn json_timestamp_without_fraction() {
        assert_eq!(
            json_timestamp("2026-10-28 01:10:18"),
            "2026-10-28T01:10:18.000Z"
        );
    }
}
