//! Form bodies (`application/x-www-form-urlencoded`) and query strings,
//! parsed the way Rack's `QueryParser` builds nested params.
//!
//! Rack (as run by the Rails kiosk) rejects a key nested deeper than
//! [`MAX_DEPTH`] levels or a string with more than [`MAX_PARAMS`] pairs, and
//! Rails answers 400. Both caps are checked while scanning, before any value
//! is built, and the tree is built with a loop, so no input can recurse.

use super::{upsert, Param};
use crate::http::percent_decode;

/// Rack's `param_depth_limit`: the levels of one key, counting the name
/// itself (`items[a]` is 2). A final `[]` appends and adds no level.
/// Captured from Rails 8.0.2: 31 nested `[a]` → 422 (accepted), 32 → 400.
pub const MAX_DEPTH: usize = 32;

/// Rack's `params_limit`: 4096 `&`-separated pairs are accepted, 4097 → 400.
pub const MAX_PARAMS: usize = 4096;

/// A form or query string Rack refuses to parse (Rails answers 400).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormLimit {
    /// More than [`MAX_PARAMS`] pairs.
    TooManyParams,
    /// A key nested deeper than [`MAX_DEPTH`] levels.
    TooDeep,
}

/// Parse a form body or query string into an object.
///
/// Supports the Rack shapes a kiosk client would send: `key=v`, `key[]=v`
/// (array of strings) and `key[a][b]=v` (nested objects). Rack's
/// array-of-hashes form (`key[][a]=v`) is not supported; such keys are skipped.
///
/// # Errors
/// [`FormLimit`] when the input passes Rack's pair or depth limit.
pub fn parse_form(input: &str) -> Result<Param, FormLimit> {
    if input.matches('&').count() >= MAX_PARAMS {
        return Err(FormLimit::TooManyParams);
    }
    let mut root: Vec<(String, Param)> = Vec::new();
    for pair in input.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let key = percent_decode(k, true);
        let (head, rest) = match key.find('[') {
            Some(at) => (&key[..at], &key[at..]),
            None => (key.as_str(), ""),
        };
        let Some(path) = bracket_path(rest)? else { continue };
        if head.is_empty() {
            continue;
        }
        let value = Param::Str(percent_decode(v, true));
        insert_path(&mut root, head.to_string(), &path, value);
    }
    Ok(Param::Obj(root))
}

/// Split `[a][b][]` into `["a", "b", ""]`; `Ok(None)` when malformed.
///
/// # Errors
/// [`FormLimit::TooDeep`] as soon as the key passes [`MAX_DEPTH`] levels, so
/// a hostile key is never fully split.
fn bracket_path(mut rest: &str) -> Result<Option<Vec<String>>, FormLimit> {
    let mut path: Vec<String> = Vec::new();
    while !rest.is_empty() {
        // `[]` is only understood as the final segment.
        if path.last().is_some_and(String::is_empty) {
            return Ok(None);
        }
        let Some(inner) = rest.strip_prefix('[') else { return Ok(None) };
        let Some(close) = inner.find(']') else { return Ok(None) };
        let seg = &inner[..close];
        // The name is level 1; each named segment before this one added one.
        if !seg.is_empty() && path.len() + 2 > MAX_DEPTH {
            return Err(FormLimit::TooDeep);
        }
        path.push(seg.to_string());
        rest = &inner[close + 1..];
    }
    Ok(Some(path))
}

/// Set `value` at `key` + `path` under `root`, creating objects on the way.
/// Iterative: each step moves one object level down.
fn insert_path(root: &mut Vec<(String, Param)>, key: String, path: &[String], value: Param) {
    let mut pairs = root;
    let mut key = key;
    for seg in path {
        if seg.is_empty() {
            match pairs.iter_mut().find(|(k, _)| *k == key) {
                Some((_, Param::Arr(items))) => items.push(value),
                _ => upsert(pairs, key, Param::Arr(vec![value])),
            }
            return;
        }
        let idx = match pairs.iter().position(|(k, v)| *k == key && matches!(v, Param::Obj(_))) {
            Some(i) => i,
            None => {
                upsert(pairs, key.clone(), Param::Obj(Vec::new()));
                pairs.iter().position(|(k, _)| *k == key).unwrap_or(0)
            }
        };
        let Some((_, Param::Obj(inner))) = pairs.get_mut(idx) else { return };
        pairs = inner;
        key.clone_from(seg);
    }
    upsert(pairs, key, value);
}

#[cfg(test)]
mod tests {
    use super::super::encode_items;
    use super::*;

    fn s(v: &str) -> Param {
        Param::Str(v.to_string())
    }

    /// `items` plus `n` nested `[a]` segments, then `tail`.
    fn nested_key(n: usize, tail: &str) -> String {
        format!("items{}{tail}=1", "[a]".repeat(n))
    }

    #[test]
    fn form_array_items() {
        // Rails stored "[\"a\",\"b\"]" for items[]=a&items[]=b.
        let p = parse_form("items[]=a&items[]=b&total=3").expect("form");
        assert_eq!(encode_items(p.get("items").expect("items")), r#"["a","b"]"#);
    }

    #[test]
    fn form_nested_items() {
        // Rails stored {"0":{"id":"1","qty":"2"}} for items[0][id]=1&items[0][qty]=2.
        let p = parse_form("items[0][id]=1&items[0][qty]=2").expect("form");
        assert_eq!(encode_items(p.get("items").expect("items")), r#"{"0":{"id":"1","qty":"2"}}"#);
    }

    #[test]
    fn form_decodes_plus_and_percent() {
        let p = parse_form("items=Hotdog+%26+Soda").expect("form");
        assert_eq!(p.get("items"), Some(&s("Hotdog & Soda")));
    }

    #[test]
    fn form_skips_malformed_keys() {
        let p = parse_form("items[a=1&items[][b]=2&[x]=3&ok=4").expect("form");
        assert_eq!(encode_items(&p), r#"{"ok":"4"}"#);
    }

    #[test]
    fn form_accepts_depth_32() {
        // 31 nested segments + the name = 32 levels; Rails accepted it (422, not 400).
        let p = parse_form(&nested_key(MAX_DEPTH - 1, "")).expect("depth 32");
        let mut v = p.get("items").expect("items");
        for _ in 1..MAX_DEPTH - 1 {
            v = v.get("a").expect("level");
        }
        assert_eq!(v.get("a"), Some(&s("1")));
    }

    #[test]
    fn form_trailing_brackets_add_no_level() {
        assert!(parse_form(&nested_key(MAX_DEPTH - 1, "[]")).is_ok());
    }

    #[test]
    fn form_rejects_depth_33() {
        assert_eq!(parse_form(&nested_key(MAX_DEPTH, "")), Err(FormLimit::TooDeep));
    }

    #[test]
    fn form_rejects_depth_33_with_trailing_brackets() {
        assert_eq!(parse_form(&nested_key(MAX_DEPTH, "[]")), Err(FormLimit::TooDeep));
    }

    #[test]
    fn form_rejects_20k_nesting_without_overflow() {
        // QA's payload: ~60 KB, one key nested 20,000 levels.
        let body = format!("{}&total=1", nested_key(20_000, ""));
        assert_eq!(parse_form(&body), Err(FormLimit::TooDeep));
    }

    #[test]
    fn form_rejects_deep_percent_encoded_brackets() {
        let key = format!("items{}=1", "%5Ba%5D".repeat(MAX_DEPTH));
        assert_eq!(parse_form(&key), Err(FormLimit::TooDeep));
    }

    #[test]
    fn form_accepts_4096_params() {
        let body = (0..MAX_PARAMS).map(|i| format!("k{i}=1")).collect::<Vec<_>>().join("&");
        assert!(parse_form(&body).is_ok());
    }

    #[test]
    fn form_rejects_4097_params() {
        let body = (0..=MAX_PARAMS).map(|i| format!("k{i}=1")).collect::<Vec<_>>().join("&");
        assert_eq!(parse_form(&body), Err(FormLimit::TooManyParams));
    }
}
