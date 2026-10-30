//! ActiveRecord's decimal cast, used for the order `total`.

use super::Param;

/// Cast a `total` param the way ActiveRecord's decimal type does, returning
/// the decimal text to bind (SQLite's NUMERIC affinity then stores it as
/// INTEGER or REAL, exactly as Rails' bound `BigDecimal` string is stored).
///
/// `None` means SQL NULL: a missing value, JSON null, or a blank string.
pub fn cast_decimal(v: Option<&Param>) -> Option<String> {
    match v? {
        Param::Null => None,
        Param::Bool(b) => Some(if *b { "1" } else { "0" }.to_string()),
        Param::Num(n) => Some(n.clone()),
        Param::Str(s) if s.trim().is_empty() => None,
        Param::Str(s) => Some(ruby_to_d(s)),
        Param::Arr(_) | Param::Obj(_) => Some("0".to_string()),
    }
}

/// Ruby `String#to_d`: skip leading whitespace, read the longest numeric
/// prefix (sign, digits with `_` separators, fraction, exponent), else zero.
fn ruby_to_d(s: &str) -> String {
    let b: Vec<char> = s.trim_start().chars().collect();
    let mut i = 0;
    let mut out = String::new();
    if matches!(b.first(), Some('+' | '-')) {
        if b[0] == '-' {
            out.push('-');
        }
        i += 1;
    }
    let digits = |out: &mut String, i: &mut usize| {
        let start = out.len();
        while *i < b.len() {
            match b[*i] {
                d @ '0'..='9' => out.push(d),
                '_' if out.len() > start && matches!(b.get(*i + 1), Some('0'..='9')) => {}
                _ => break,
            }
            *i += 1;
        }
        out.len() > start
    };
    let int_digits = digits(&mut out, &mut i);
    let mut frac_digits = false;
    if b.get(i) == Some(&'.') && matches!(b.get(i + 1), Some('0'..='9')) {
        i += 1;
        out.push('.');
        frac_digits = digits(&mut out, &mut i);
    }
    if !int_digits && !frac_digits {
        return "0".to_string();
    }
    if !int_digits {
        out.insert(usize::from(out.starts_with('-')), '0');
    }
    if matches!(b.get(i), Some('e' | 'E')) {
        let mut exp = String::from("e");
        let mut j = i + 1;
        if matches!(b.get(j), Some('+' | '-')) {
            exp.push(b[j]);
            j += 1;
        }
        if digits(&mut exp, &mut j) {
            out.push_str(&exp);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Param {
        Param::Str(v.to_string())
    }

    #[test]
    fn decimal_casts_match_rails() {
        // (input, bound text) pairs; SQLite affinity then yields the stored
        // values Rails produced (e.g. "12abc" → 12, "1e3" → 1000, "abc" → 0).
        let cases: &[(&str, Option<&str>)] = &[
            ("", None),
            ("  ", None),
            (" 1.5", Some("1.5")),
            ("12abc", Some("12")),
            ("1e3", Some("1e3")),
            ("abc", Some("0")),
            (".5", Some("0.5")),
            ("-2.50", Some("-2.50")),
            ("1_000", Some("1000")),
            ("1.", Some("1")),
            ("+3", Some("3")),
            ("0x10", Some("0")),
            ("1,5", Some("1")),
        ];
        for (input, want) in cases {
            assert_eq!(cast_decimal(Some(&s(input))).as_deref(), *want, "{input:?}");
        }
    }

    #[test]
    fn decimal_casts_non_strings() {
        assert_eq!(cast_decimal(None), None);
        assert_eq!(cast_decimal(Some(&Param::Null)), None);
        assert_eq!(cast_decimal(Some(&Param::Bool(true))).as_deref(), Some("1"));
        assert_eq!(
            cast_decimal(Some(&Param::Bool(false))).as_deref(),
            Some("0")
        );
        assert_eq!(
            cast_decimal(Some(&Param::Num("9.99".into()))).as_deref(),
            Some("9.99")
        );
        assert_eq!(
            cast_decimal(Some(&Param::Arr(vec![]))).as_deref(),
            Some("0")
        );
    }
}
