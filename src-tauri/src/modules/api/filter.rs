//! Twenty's REST query parameters (D-071), read into the engine's filter and sorts:
//! `filter=name[eq]:"Acme",employees[gte]:10` (all of them), `or(…)`, `and(…)`, `not(…)`, a
//! composite's part as `name.firstName`; `orderBy=name[AscNullsFirst],amount.amountMicros[DescNullsLast]`.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::error::AppError;
use crate::kernel::records::query::Sort;

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

struct Parser<'a> {
    text: &'a str,
    i: usize,
    /// How deep the groups and lists go (a request can't nest past MAX_DEPTH).
    depth: u8,
}

const MAX_DEPTH: u8 = 16;

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<char> {
        self.text[self.i..].chars().next()
    }

    fn skip_ws(&mut self) {
        // By the character's own length: a non-breaking space is two bytes.
        while let Some(c) = self.peek().filter(|c| c.is_whitespace()) {
            self.i += c.len_utf8();
        }
    }

    fn nest(&mut self) -> Result<()> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(invalid(format!("The filter nests more than {MAX_DEPTH} deep")));
        }
        Ok(())
    }

    fn expect(&mut self, c: char) -> Result<()> {
        self.skip_ws();
        if self.peek() == Some(c) {
            self.i += c.len_utf8();
            Ok(())
        } else {
            Err(invalid(format!("The filter doesn't read: “{c}” expected at {}", self.i)))
        }
    }

    /// A name: letters, digits, `_` and `.` (a composite's part).
    fn ident(&mut self) -> &'a str {
        self.skip_ws();
        let start = self.i;
        while self.peek().is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.') {
            self.i += 1;
        }
        &self.text[start..self.i]
    }

    fn list(&mut self) -> Result<Vec<Value>> {
        let mut items = vec![self.expr()?];
        loop {
            self.skip_ws();
            if self.peek() == Some(',') {
                self.i += 1;
                items.push(self.expr()?);
            } else {
                return Ok(items);
            }
        }
    }

    fn expr(&mut self) -> Result<Value> {
        let start = self.i;
        let word = self.ident();
        self.skip_ws();
        if matches!(word, "and" | "or" | "not") && self.peek() == Some('(') {
            self.i += 1;
            self.nest()?;
            let items = self.list()?;
            self.expect(')')?;
            self.depth -= 1;
            return Ok(match word {
                "and" => json!({ "and": items }),
                "or" => json!({ "or": items }),
                _ => json!({ "not": if items.len() == 1 { items[0].clone() } else { json!({ "and": items }) } }),
            });
        }
        self.i = start;
        let path = self.ident();
        if path.is_empty() {
            return Err(invalid(format!("The filter doesn't read: a field expected at {}", self.i)));
        }
        self.expect('[')?;
        let op = self.ident();
        self.expect(']')?;
        self.expect(':')?;
        let value = self.value()?;
        condition(path, op, value)
    }

    fn value(&mut self) -> Result<Value> {
        self.skip_ws();
        match self.peek() {
            Some('"') => {
                self.i += 1;
                let mut out = String::new();
                loop {
                    match self.peek() {
                        None => return Err(invalid("The filter doesn't read: a quote isn't closed")),
                        Some('"') => {
                            self.i += 1;
                            return Ok(Value::String(out));
                        }
                        Some('\\') => {
                            self.i += 1;
                            if let Some(c) = self.peek() {
                                out.push(c);
                                self.i += c.len_utf8();
                            }
                        }
                        Some(c) => {
                            out.push(c);
                            self.i += c.len_utf8();
                        }
                    }
                }
            }
            Some('[') => {
                self.i += 1;
                self.nest()?;
                let mut items = Vec::new();
                loop {
                    self.skip_ws();
                    match self.peek() {
                        None => return Err(invalid("The filter doesn't read: a “[” isn't closed")),
                        Some(']') => {
                            self.i += 1;
                            self.depth -= 1;
                            return Ok(Value::Array(items));
                        }
                        _ => {}
                    }
                    items.push(self.value()?);
                    self.skip_ws();
                    match self.peek() {
                        Some(',') => self.i += 1,
                        Some(']') => {}
                        _ => return Err(invalid(format!("The filter doesn't read: “,” or “]” expected at {}", self.i))),
                    }
                }
            }
            _ => {
                let start = self.i;
                while self.peek().is_some_and(|c| c != ',' && c != ')' && c != ']') {
                    self.i += self.peek().map_or(1, char::len_utf8);
                }
                let bare = self.text[start..self.i].trim();
                Ok(match bare {
                    "true" => json!(true),
                    "false" => json!(false),
                    _ => bare.parse::<f64>().map_or_else(|_| Value::String(bare.to_owned()), |n| json!(n)),
                })
            }
        }
    }
}

/// One condition, Twenty's operator as the engine's.
fn condition(path: &str, op: &str, value: Value) -> Result<Value> {
    let (field, sub) = match path.split_once('.') {
        Some((f, s)) => (f, Some(s)),
        None => (path, None),
    };
    let like = |v: &Value| Value::String(v.as_str().unwrap_or("").trim_matches('%').to_owned());
    let (op, value) = match op {
        "eq" => ("is", value),
        "neq" => ("isNot", value),
        "in" => ("in", value),
        "containsAny" => ("contains", value),
        "is" => match value.as_str() {
            Some(v) if v.eq_ignore_ascii_case("NULL") => ("isEmpty", Value::Null),
            Some(v) if v.eq_ignore_ascii_case("NOT_NULL") => ("isNotEmpty", Value::Null),
            _ => return Err(invalid("[is] takes NULL or NOT_NULL")),
        },
        "gt" | "gte" | "lt" | "lte" => (op, value),
        "startsWith" | "like" | "ilike" => ("contains", like(&value)),
        other => return Err(invalid(format!("No filter operator “{other}” — eq, neq, in, containsAny, is, gt, gte, lt, lte, startsWith, like, ilike"))),
    };
    Ok(json!({ "field": field, "sub": sub, "op": op, "value": value }))
}

/// `filter=` as the engine's filter (`{ and: [ … ] }`).
pub fn parse_filter(text: &str) -> Result<Value> {
    let mut p = Parser { text, i: 0, depth: 0 };
    let items = p.list()?;
    p.skip_ws();
    if p.i != text.len() {
        return Err(invalid(format!("The filter doesn't read after “{}”", &text[..p.i])));
    }
    Ok(json!({ "and": items }))
}

/// `orderBy=` as the engine's sorts (nulls go last, as the engine keeps them).
pub fn parse_order(text: &str) -> Result<Vec<Sort>> {
    text.split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|part| {
            let (path, dir) = match part.split_once('[') {
                Some((p, d)) => (p.trim(), d.trim_end_matches(']').trim()),
                None => (part, "AscNullsFirst"),
            };
            let desc = match dir {
                "AscNullsFirst" | "AscNullsLast" | "Asc" | "asc" => false,
                "DescNullsFirst" | "DescNullsLast" | "Desc" | "desc" => true,
                other => return Err(invalid(format!("No order “{other}” — AscNullsFirst, AscNullsLast, DescNullsFirst, DescNullsLast"))),
            };
            let (field, sub) = match path.split_once('.') {
                Some((f, s)) => (f.to_owned(), Some(s.to_owned())),
                None => (path.to_owned(), None),
            };
            Ok(Sort { field, sub, desc })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twenty_filters_read() {
        assert_eq!(parse_filter(r#"name[eq]:"Acme",employees[gte]:10"#).unwrap(), json!({ "and": [{ "field": "name", "sub": null, "op": "is", "value": "Acme" }, { "field": "employees", "sub": null, "op": "gte", "value": 10.0 }] }));
        let nested = parse_filter(r#"or(stage[eq]:"NEW",stage[eq]:"SCREENING"),not(name.firstName[ilike]:"%test%")"#).unwrap();
        assert_eq!(nested["and"][0]["or"][1]["value"], "SCREENING");
        assert_eq!(nested["and"][1]["not"], json!({ "field": "name", "sub": "firstName", "op": "contains", "value": "test" }));
        assert_eq!(parse_filter(r#"stage[in]:["NEW","MEETING"]"#).unwrap()["and"][0]["value"], json!(["NEW", "MEETING"]));
        assert_eq!(parse_filter("closeDate[is]:NULL").unwrap()["and"][0]["op"], "isEmpty");
        assert_eq!(parse_filter(r#"name[eq]:"Ha, \"Nam\" & co""#).unwrap()["and"][0]["value"], r#"Ha, "Nam" & co"#);
        assert!(parse_filter("name[between]:1").is_err());
        assert!(parse_filter(r#"name[eq]:"open"#).is_err());
        assert!(parse_filter("name[eq]:1)").is_err());
        let order = parse_order("amount.amountMicros[DescNullsLast],name[AscNullsFirst]").unwrap();
        assert_eq!((order[0].field.as_str(), order[0].sub.as_deref(), order[0].desc, order[1].desc), ("amount", Some("amountMicros"), true, false));
        assert!(parse_order("name[Sideways]").is_err());
        // Odd input is refused, never a crash or a spin.
        assert!(parse_filter(&"and(".repeat(100_000)).is_err());
        assert!(parse_filter(&format!("stage[in]:{}", "[".repeat(100_000))).is_err());
        assert!(parse_filter(r#"stage[in]:["NEW""#).is_err() && parse_filter("stage[in]:[)").is_err());
        assert!(parse_filter("name[eq]:1,\u{a0}jobTitle[eq]:2").is_ok(), "a non-breaking space between conditions");
    }
}
