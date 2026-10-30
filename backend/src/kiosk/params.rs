//! Request parameters for `POST /orders`, read the way Rails builds `params`.
//!
//! Rails merges the query string over the parsed body (JSON or form-encoded),
//! keeps JSON numbers as Integer or Float, and later re-encodes the `items`
//! value through ActiveSupport. [`Param`] keeps the original number literal so
//! that re-encoding can reproduce Ruby's Integer/Float split exactly.

use super::encode;
use crate::http::percent_decode;

/// A parameter value: a JSON-like tree whose numbers keep their source text.
#[derive(Debug, Clone, PartialEq)]
pub enum Param {
    /// JSON `null`.
    Null,
    /// JSON `true` / `false`.
    Bool(bool),
    /// A JSON number, as written in the request.
    Num(String),
    /// A string (every form-encoded value is one).
    Str(String),
    /// An array.
    Arr(Vec<Param>),
    /// An object, in first-insertion order; a repeated key keeps its first
    /// position and takes the last value, like a Ruby Hash.
    Obj(Vec<(String, Param)>),
}

impl Param {
    /// Value at `key` when this is an object.
    pub fn get(&self, key: &str) -> Option<&Param> {
        match self {
            Param::Obj(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
}

/// Insert or replace `key` in an object's pairs, keeping first-seen order.
fn upsert(pairs: &mut Vec<(String, Param)>, key: String, value: Param) {
    match pairs.iter_mut().find(|(k, _)| *k == key) {
        Some(slot) => slot.1 = value,
        None => pairs.push((key, value)),
    }
}

// ==== JSON ====

/// A request body that is not valid JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidJson;

/// Parse a JSON document into a [`Param`] tree.
///
/// # Errors
/// [`InvalidJson`] on any syntax error; the caller answers 400 like Rails'
/// `ActionDispatch::Http::Parameters::ParseError`.
pub fn parse_json(input: &[u8]) -> Result<Param, InvalidJson> {
    let text = std::str::from_utf8(input).map_err(|_| InvalidJson)?;
    let mut p = JsonParser { s: text.as_bytes(), i: 0, depth: 0 };
    p.ws();
    let v = p.value().map_err(|()| InvalidJson)?;
    p.ws();
    if p.i != p.s.len() {
        return Err(InvalidJson);
    }
    Ok(v)
}

/// Nesting cap so a hostile body cannot overflow the stack.
const MAX_DEPTH: usize = 100;

struct JsonParser<'a> {
    s: &'a [u8],
    i: usize,
    depth: usize,
}

impl JsonParser<'_> {
    fn ws(&mut self) {
        while matches!(self.s.get(self.i), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    fn lit(&mut self, word: &str, v: Param) -> Result<Param, ()> {
        if self.s[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            Ok(v)
        } else {
            Err(())
        }
    }

    fn value(&mut self) -> Result<Param, ()> {
        match self.s.get(self.i).ok_or(())? {
            b'{' => self.nested(Self::object),
            b'[' => self.nested(Self::array),
            b'"' => self.string().map(Param::Str),
            b't' => self.lit("true", Param::Bool(true)),
            b'f' => self.lit("false", Param::Bool(false)),
            b'n' => self.lit("null", Param::Null),
            _ => self.number(),
        }
    }

    fn nested(&mut self, f: fn(&mut Self) -> Result<Param, ()>) -> Result<Param, ()> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(());
        }
        let v = f(self);
        self.depth -= 1;
        v
    }

    fn object(&mut self) -> Result<Param, ()> {
        self.i += 1;
        let mut pairs = Vec::new();
        self.ws();
        if self.s.get(self.i) == Some(&b'}') {
            self.i += 1;
            return Ok(Param::Obj(pairs));
        }
        loop {
            self.ws();
            if self.s.get(self.i) != Some(&b'"') {
                return Err(());
            }
            let key = self.string()?;
            self.ws();
            if self.s.get(self.i) != Some(&b':') {
                return Err(());
            }
            self.i += 1;
            self.ws();
            let v = self.value()?;
            upsert(&mut pairs, key, v);
            self.ws();
            match self.s.get(self.i) {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Param::Obj(pairs));
                }
                _ => return Err(()),
            }
        }
    }

    fn array(&mut self) -> Result<Param, ()> {
        self.i += 1;
        let mut items = Vec::new();
        self.ws();
        if self.s.get(self.i) == Some(&b']') {
            self.i += 1;
            return Ok(Param::Arr(items));
        }
        loop {
            self.ws();
            items.push(self.value()?);
            self.ws();
            match self.s.get(self.i) {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    return Ok(Param::Arr(items));
                }
                _ => return Err(()),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, ()> {
        let h = self.s.get(self.i..self.i + 4).ok_or(())?;
        let h = std::str::from_utf8(h).map_err(|_| ())?;
        self.i += 4;
        u32::from_str_radix(h, 16).map_err(|_| ())
    }

    fn string(&mut self) -> Result<String, ()> {
        self.i += 1;
        let mut out = String::new();
        loop {
            let start = self.i;
            while let Some(&b) = self.s.get(self.i) {
                if b == b'"' || b == b'\\' || b < 0x20 {
                    break;
                }
                self.i += 1;
            }
            out.push_str(std::str::from_utf8(&self.s[start..self.i]).map_err(|_| ())?);
            match self.s.get(self.i).ok_or(())? {
                b'"' => {
                    self.i += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.i += 1;
                    let esc = *self.s.get(self.i).ok_or(())?;
                    self.i += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => out.push(self.unicode_escape()?),
                        _ => return Err(()),
                    }
                }
                _ => return Err(()),
            }
        }
    }

    /// Decode `XXXX` after `\u`, joining a surrogate pair when one follows.
    fn unicode_escape(&mut self) -> Result<char, ()> {
        let hi = self.hex4()?;
        if (0xD800..0xDC00).contains(&hi) {
            if self.s.get(self.i..self.i + 2) != Some(b"\\u") {
                return Err(());
            }
            self.i += 2;
            let lo = self.hex4()?;
            if !(0xDC00..0xE000).contains(&lo) {
                return Err(());
            }
            return char::from_u32(0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)).ok_or(());
        }
        char::from_u32(hi).ok_or(())
    }

    fn number(&mut self) -> Result<Param, ()> {
        let start = self.i;
        let at = |p: &Self, i: usize| p.s.get(i).copied();
        if at(self, self.i) == Some(b'-') {
            self.i += 1;
        }
        match at(self, self.i) {
            Some(b'0') => self.i += 1,
            Some(b'1'..=b'9') => {
                while matches!(at(self, self.i), Some(b'0'..=b'9')) {
                    self.i += 1;
                }
            }
            _ => return Err(()),
        }
        if at(self, self.i) == Some(b'.') {
            self.i += 1;
            let digits = self.i;
            while matches!(at(self, self.i), Some(b'0'..=b'9')) {
                self.i += 1;
            }
            if self.i == digits {
                return Err(());
            }
        }
        if matches!(at(self, self.i), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(at(self, self.i), Some(b'+' | b'-')) {
                self.i += 1;
            }
            let digits = self.i;
            while matches!(at(self, self.i), Some(b'0'..=b'9')) {
                self.i += 1;
            }
            if self.i == digits {
                return Err(());
            }
        }
        let text = std::str::from_utf8(&self.s[start..self.i]).map_err(|_| ())?;
        Ok(Param::Num(text.to_string()))
    }
}

// ==== FORM / QUERY ====

/// Parse `application/x-www-form-urlencoded` (or a query string) into an object.
///
/// Supports the Rack shapes a kiosk client would send: `key=v`, `key[]=v`
/// (array of strings) and `key[a][b]=v` (nested objects). Rack's
/// array-of-hashes form (`key[][a]=v`) is not supported; such keys are skipped.
pub fn parse_form(input: &str) -> Param {
    let mut root: Vec<(String, Param)> = Vec::new();
    for pair in input.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let key = percent_decode(k, true);
        let value = Param::Str(percent_decode(v, true));
        let (head, rest) = match key.find('[') {
            Some(at) => (key[..at].to_string(), &key[at..]),
            None => (key.clone(), ""),
        };
        if head.is_empty() {
            continue;
        }
        let Some(path) = bracket_path(rest) else { continue };
        insert_path(&mut root, head, &path, value);
    }
    Param::Obj(root)
}

/// Split `[a][b][]` into `["a", "b", ""]`. `None` when malformed.
fn bracket_path(mut rest: &str) -> Option<Vec<String>> {
    let mut path = Vec::new();
    while !rest.is_empty() {
        let inner = rest.strip_prefix('[')?;
        let close = inner.find(']')?;
        path.push(inner[..close].to_string());
        rest = &inner[close + 1..];
    }
    // `[]` is only understood as the final segment.
    if path.iter().rev().skip(1).any(String::is_empty) {
        return None;
    }
    Some(path)
}

fn insert_path(pairs: &mut Vec<(String, Param)>, key: String, path: &[String], value: Param) {
    match path.split_first() {
        None => upsert(pairs, key, value),
        Some((seg, _)) if seg.is_empty() => {
            match pairs.iter_mut().find(|(k, _)| *k == key) {
                Some((_, Param::Arr(items))) => items.push(value),
                _ => upsert(pairs, key, Param::Arr(vec![value])),
            }
        }
        Some((seg, tail)) => {
            let idx = match pairs.iter().position(|(k, v)| *k == key && matches!(v, Param::Obj(_))) {
                Some(i) => i,
                None => {
                    upsert(pairs, key.clone(), Param::Obj(Vec::new()));
                    pairs.iter().position(|(k, _)| *k == key).unwrap_or(0)
                }
            };
            if let Some((_, Param::Obj(inner))) = pairs.get_mut(idx) {
                insert_path(inner, seg.clone(), tail, value);
            }
        }
    }
}

/// Merge `top` over `base` (Rails: `request_parameters.merge(query_parameters)`).
pub fn merge(base: Param, top: Param) -> Param {
    let (Param::Obj(mut pairs), Param::Obj(over)) = (base, top) else {
        return Param::Obj(Vec::new());
    };
    for (k, v) in over {
        upsert(&mut pairs, k, v);
    }
    Param::Obj(pairs)
}

// ==== CASTS ====

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

/// Encode a param tree the way `serialize :items, coder: JSON` stores it.
///
/// Ruby parsed JSON integers as Integer (printed verbatim, `-0` → `0`) and
/// anything with a fraction or exponent as Float (printed by the json gem).
pub fn encode_items(v: &Param) -> String {
    let mut out = String::new();
    push_param(&mut out, v);
    out
}

fn push_param(out: &mut String, v: &Param) {
    match v {
        Param::Null => out.push_str("null"),
        Param::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Param::Num(n) if n.contains(['.', 'e', 'E']) => {
            out.push_str(&encode::float(n.parse::<f64>().unwrap_or(f64::INFINITY)));
        }
        Param::Num(n) if n == "-0" => out.push('0'),
        Param::Num(n) => out.push_str(n),
        Param::Str(s) => encode::push_str(out, s),
        Param::Arr(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                push_param(out, item);
            }
            out.push(']');
        }
        Param::Obj(pairs) => {
            out.push('{');
            for (i, (k, item)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                encode::push_str(out, k);
                out.push(':');
                push_param(out, item);
            }
            out.push('}');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Param {
        Param::Str(v.to_string())
    }

    #[test]
    fn json_items_reencode_like_rails() {
        // Captured from Rails: Order.create!(items: params[:items]) then the stored text.
        let body = r#"{"items":[1, 1.0, 1.50, 1e2, -0, "x<y>&\u2028é", null, true, {"b":2,"a":{"c":[]}}]}"#;
        let p = parse_json(body.as_bytes()).expect("parse");
        assert_eq!(
            encode_items(p.get("items").expect("items")),
            r#"[1,1.0,1.5,100.0,0,"x\u003cy\u003e\u0026\u2028é",null,true,{"b":2,"a":{"c":[]}}]"#
        );
    }

    #[test]
    fn json_rejects_trailing_garbage() {
        assert!(parse_json(b"{\"a\":1} x").is_err());
    }

    #[test]
    fn json_rejects_leading_zero() {
        assert!(parse_json(b"[01]").is_err());
    }

    #[test]
    fn json_rejects_deep_nesting() {
        let deep = "[".repeat(MAX_DEPTH + 1) + &"]".repeat(MAX_DEPTH + 1);
        assert!(parse_json(deep.as_bytes()).is_err());
    }

    #[test]
    fn json_decodes_surrogate_pair() {
        assert_eq!(parse_json(b"\"\\ud83c\\udf2d\""), Ok(s("\u{1f32d}")));
    }

    #[test]
    fn json_duplicate_key_keeps_first_slot_last_value() {
        let p = parse_json(br#"{"a":1,"b":2,"a":3}"#).expect("parse");
        assert_eq!(encode_items(&p), r#"{"a":3,"b":2}"#);
    }

    #[test]
    fn form_array_items() {
        // Rails stored "[\"a\",\"b\"]" for items[]=a&items[]=b.
        let p = parse_form("items[]=a&items[]=b&total=3");
        assert_eq!(encode_items(p.get("items").expect("items")), r#"["a","b"]"#);
    }

    #[test]
    fn form_nested_items() {
        // Rails stored {"0":{"id":"1","qty":"2"}} for items[0][id]=1&items[0][qty]=2.
        let p = parse_form("items[0][id]=1&items[0][qty]=2");
        assert_eq!(encode_items(p.get("items").expect("items")), r#"{"0":{"id":"1","qty":"2"}}"#);
    }

    #[test]
    fn form_decodes_plus_and_percent() {
        let p = parse_form("items=Hotdog+%26+Soda");
        assert_eq!(p.get("items"), Some(&s("Hotdog & Soda")));
    }

    #[test]
    fn query_overrides_body() {
        let merged = merge(parse_form("total=1&items=x"), parse_form("total=2"));
        assert_eq!(merged.get("total"), Some(&s("2")));
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
        assert_eq!(cast_decimal(Some(&Param::Bool(false))).as_deref(), Some("0"));
        assert_eq!(cast_decimal(Some(&Param::Num("9.99".into()))).as_deref(), Some("9.99"));
        assert_eq!(cast_decimal(Some(&Param::Arr(vec![]))).as_deref(), Some("0"));
    }
}
