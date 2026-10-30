//! JSON request bodies, parsed into a [`Param`] tree.
//!
//! Nesting is capped at [`MAX_DEPTH`] (the json gem's `max_nesting`), so a
//! hostile body cannot overflow the stack of the recursive descent.

use super::{upsert, Param};

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
    let mut p = JsonParser {
        s: text.as_bytes(),
        i: 0,
        depth: 0,
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_rejects_trailing_garbage() {
        assert!(parse_json(b"{\"a\":1} x").is_err());
    }

    #[test]
    fn json_rejects_leading_zero() {
        assert!(parse_json(b"[01]").is_err());
    }

    #[test]
    fn json_accepts_max_depth() {
        let ok = "[".repeat(MAX_DEPTH) + &"]".repeat(MAX_DEPTH);
        assert!(parse_json(ok.as_bytes()).is_ok());
    }

    #[test]
    fn json_rejects_deep_nesting() {
        let deep = "[".repeat(MAX_DEPTH + 1) + &"]".repeat(MAX_DEPTH + 1);
        assert!(parse_json(deep.as_bytes()).is_err());
    }

    #[test]
    fn json_rejects_very_deep_nesting_without_overflow() {
        let deep = "[".repeat(100_000);
        assert_eq!(parse_json(deep.as_bytes()), Err(InvalidJson));
    }

    #[test]
    fn json_decodes_surrogate_pair() {
        assert_eq!(
            parse_json(b"\"\\ud83c\\udf2d\""),
            Ok(Param::Str("\u{1f32d}".to_string()))
        );
    }

    #[test]
    fn json_duplicate_key_keeps_first_slot_last_value() {
        let p = parse_json(br#"{"a":1,"b":2,"a":3}"#).expect("parse");
        assert_eq!(super::super::encode_items(&p), r#"{"a":3,"b":2}"#);
    }
}
