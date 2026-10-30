//! Request parameters for `POST /orders`, read the way Rails builds `params`.
//!
//! Rails merges the query string over the parsed body (JSON or form-encoded),
//! keeps JSON numbers as Integer or Float, and later re-encodes the `items`
//! value through ActiveSupport. [`Param`] keeps the original number literal so
//! that re-encoding can reproduce Ruby's Integer/Float split exactly.
//!
//! - [`json`]: JSON bodies (nesting capped at 100, like the json gem)
//! - [`form`]: form bodies and query strings (Rack's 32-level / 4096-param caps)
//! - [`cast`]: ActiveRecord's decimal cast for `total`

pub mod cast;
pub mod form;
pub mod json;

pub use cast::cast_decimal;
pub use form::{parse_form, FormLimit};
pub use json::{parse_json, InvalidJson};

use super::encode;

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

/// Encode a param tree the way `serialize :items, coder: JSON` stores it.
///
/// Ruby parsed JSON integers as Integer (printed verbatim, `-0` → `0`) and
/// anything with a fraction or exponent as Float (printed by the json gem).
/// Recursion is bounded by the parsers' nesting caps.
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
    fn query_overrides_body() {
        let merged = merge(parse_form("total=1&items=x").expect("form"), parse_form("total=2").expect("form"));
        assert_eq!(merged.get("total"), Some(&s("2")));
    }
}
