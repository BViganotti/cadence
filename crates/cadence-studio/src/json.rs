//! Small bounded JSON parser and serializer used by the Studio API.
//!
//! The parser enforces byte, depth, node and string limits and never recurses
//! deeper than 64 levels. The serializer is deterministic and preserves object
//! insertion order.

use cadence_core::{Error, Result};

pub const MAX_JSON_BYTES: usize = 1024 * 1024;
pub const MAX_JSON_DEPTH: usize = 64;
pub const MAX_JSON_NODES: usize = 200_000;
pub const MAX_JSON_STRING: usize = 512 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(entries) => entries
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.as_f64().filter(|value| *value >= 0.0 && value.fract() == 0.0).map(|value| value as u64)
    }

    pub fn as_i64(&self) -> Option<i64> {
        self.as_f64().filter(|value| value.fract() == 0.0).map(|value| value as i64)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(values) => Some(values),
            _ => None,
        }
    }

    pub fn stringify(&self) -> String {
        let mut out = String::new();
        write_value(&mut out, self, 0);
        out
    }
}

pub fn parse(input: &str) -> Result<Value> {
    if input.len() > MAX_JSON_BYTES {
        return Err(Error::Limit("JSON bytes"));
    }
    let mut parser = JsonParser {
        bytes: input.as_bytes(),
        pos: 0,
        depth: 0,
        nodes: 0,
    };
    parser.skip_ws();
    let value = parser.parse_value()?;
    parser.skip_ws();
    if parser.pos != parser.bytes.len() {
        return Err(Error::msg("trailing JSON content"));
    }
    Ok(value)
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
    depth: usize,
    nodes: usize,
}

impl<'a> JsonParser<'a> {
    fn parse_value(&mut self) -> Result<Value> {
        self.nodes += 1;
        if self.nodes > MAX_JSON_NODES {
            return Err(Error::Limit("JSON nodes"));
        }
        self.skip_ws();
        let byte = *self.bytes.get(self.pos).ok_or(Error::UnexpectedEof)?;
        match byte {
            b'n' => {
                self.expect(b"null")?;
                Ok(Value::Null)
            }
            b't' => {
                self.expect(b"true")?;
                Ok(Value::Bool(true))
            }
            b'f' => {
                self.expect(b"false")?;
                Ok(Value::Bool(false))
            }
            b'"' => self.parse_string().map(Value::String),
            b'-' | b'0'..=b'9' => self.parse_number(),
            b'[' => self.parse_array(),
            b'{' => self.parse_object(),
            _ => Err(Error::msg("invalid JSON value")),
        }
    }

    fn parse_array(&mut self) -> Result<Value> {
        self.depth += 1;
        if self.depth > MAX_JSON_DEPTH {
            return Err(Error::Limit("JSON depth"));
        }
        self.pos += 1;
        let mut values = Vec::new();
        self.skip_ws();
        if self.bytes.get(self.pos) == Some(&b']') {
            self.pos += 1;
            self.depth -= 1;
            return Ok(Value::Array(values));
        }
        loop {
            values.push(self.parse_value()?);
            self.skip_ws();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    self.depth -= 1;
                    return Ok(Value::Array(values));
                }
                _ => return Err(Error::msg("expected ',' or ']' in JSON array")),
            }
        }
    }

    fn parse_object(&mut self) -> Result<Value> {
        self.depth += 1;
        if self.depth > MAX_JSON_DEPTH {
            return Err(Error::Limit("JSON depth"));
        }
        self.pos += 1;
        let mut entries = Vec::new();
        self.skip_ws();
        if self.bytes.get(self.pos) == Some(&b'}') {
            self.pos += 1;
            self.depth -= 1;
            return Ok(Value::Object(entries));
        }
        loop {
            self.skip_ws();
            if self.bytes.get(self.pos) != Some(&b'"') {
                return Err(Error::msg("expected JSON object key"));
            }
            let key = self.parse_string()?;
            self.skip_ws();
            if self.bytes.get(self.pos) != Some(&b':') {
                return Err(Error::msg("expected ':' in JSON object"));
            }
            self.pos += 1;
            let value = self.parse_value()?;
            if entries.iter().any(|(existing, _)| existing == &key) {
                return Err(Error::msg("duplicate JSON object key"));
            }
            entries.push((key, value));
            self.skip_ws();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    self.depth -= 1;
                    return Ok(Value::Object(entries));
                }
                _ => return Err(Error::msg("expected ',' or '}' in JSON object")),
            }
        }
    }

    fn parse_string(&mut self) -> Result<String> {
        self.pos += 1;
        let mut out = String::new();
        loop {
            let byte = *self.bytes.get(self.pos).ok_or(Error::UnexpectedEof)?;
            self.pos += 1;
            match byte {
                b'"' => return Ok(out),
                b'\\' => {
                    let escape = *self.bytes.get(self.pos).ok_or(Error::UnexpectedEof)?;
                    self.pos += 1;
                    match escape {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let first = self.parse_hex4()?;
                            let codepoint = if (0xD800..=0xDBFF).contains(&first) {
                                if self.bytes.get(self.pos) == Some(&b'\\')
                                    && self.bytes.get(self.pos + 1) == Some(&b'u')
                                {
                                    self.pos += 2;
                                    let second = self.parse_hex4()?;
                                    if !(0xDC00..=0xDFFF).contains(&second) {
                                        return Err(Error::msg("invalid surrogate pair"));
                                    }
                                    0x10000
                                        + ((first - 0xD800) << 10)
                                        + (second - 0xDC00)
                                } else {
                                    return Err(Error::msg("unpaired surrogate"));
                                }
                            } else {
                                first
                            };
                            let character = char::from_u32(codepoint)
                                .ok_or_else(|| Error::msg("invalid Unicode escape"))?;
                            out.push(character);
                        }
                        _ => return Err(Error::msg("invalid JSON escape")),
                    }
                }
                0..=0x1F => return Err(Error::msg("control character in JSON string")),
                _ => {
                    let start = self.pos - 1;
                    let remaining = std::str::from_utf8(&self.bytes[start..])
                        .map_err(|_| Error::msg("invalid UTF-8 in JSON string"))?;
                    let c = remaining.chars().next().unwrap_or('\0');
                    out.push(c);
                    self.pos = start + c.len_utf8();
                }
            }
            if out.len() > MAX_JSON_STRING {
                return Err(Error::Limit("JSON string bytes"));
            }
        }
    }

    fn parse_hex4(&mut self) -> Result<u32> {
        if self.pos + 4 > self.bytes.len() {
            return Err(Error::UnexpectedEof);
        }
        let text = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4])
            .map_err(|_| Error::msg("invalid Unicode escape"))?;
        let value = u32::from_str_radix(text, 16)
            .map_err(|_| Error::msg("invalid Unicode escape"))?;
        self.pos += 4;
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<Value> {
        let start = self.pos;
        if self.bytes.get(self.pos) == Some(&b'-') {
            self.pos += 1;
        }
        let digits_start = self.pos;
        while matches!(self.bytes.get(self.pos), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        if self.pos == digits_start {
            return Err(Error::msg("invalid JSON number"));
        }
        if self.bytes.get(self.pos) == Some(&b'.') {
            self.pos += 1;
            let fraction_start = self.pos;
            while matches!(self.bytes.get(self.pos), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
            if self.pos == fraction_start {
                return Err(Error::msg("invalid JSON number"));
            }
        }
        if matches!(self.bytes.get(self.pos), Some(b'e') | Some(b'E')) {
            self.pos += 1;
            if matches!(self.bytes.get(self.pos), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            let exponent_start = self.pos;
            while matches!(self.bytes.get(self.pos), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
            if self.pos == exponent_start {
                return Err(Error::msg("invalid JSON number"));
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| Error::msg("invalid JSON number"))?;
        let value: f64 = text
            .parse()
            .map_err(|_| Error::msg("invalid JSON number"))?;
        if !value.is_finite() {
            return Err(Error::out_of_range("non-finite JSON number"));
        }
        Ok(Value::Number(value))
    }

    fn expect(&mut self, text: &[u8]) -> Result<()> {
        if self.bytes[self.pos..].starts_with(text) {
            self.pos += text.len();
            Ok(())
        } else {
            Err(Error::msg("invalid JSON literal"))
        }
    }

    fn skip_ws(&mut self) {
        while matches!(
            self.bytes.get(self.pos),
            Some(b' ') | Some(b'\t') | Some(b'\r') | Some(b'\n')
        ) {
            self.pos += 1;
        }
    }
}

fn write_value(out: &mut String, value: &Value, indent: usize) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(number) => {
            if number.fract() == 0.0 && number.abs() < 9e15 {
                out.push_str(&format!("{}", *number as i64));
            } else {
                out.push_str(&format!("{number}"));
            }
        }
        Value::String(text) => write_string(out, text),
        Value::Array(values) => {
            out.push('[');
            for (index, item) in values.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                if indent > 0 {
                    out.push('\n');
                    for _ in 0..indent {
                        out.push(' ');
                    }
                }
                write_value(out, item, indent + 2);
            }
            out.push(']');
        }
        Value::Object(entries) => {
            out.push('{');
            for (index, (key, item)) in entries.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                if indent > 0 {
                    out.push('\n');
                    for _ in 0..indent {
                        out.push(' ');
                    }
                }
                write_string(out, key);
                out.push(':');
                if indent > 0 {
                    out.push(' ');
                }
                write_value(out, item, indent + 2);
            }
            if indent > 0 && !entries.is_empty() {
                out.push('\n');
                for _ in 0..indent.saturating_sub(2) {
                    out.push(' ');
                }
            }
            out.push('}');
        }
    }
}

fn write_string(out: &mut String, text: &str) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000c}' => out.push_str("\\f"),
            c if c < '\u{0020}' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

pub fn object(entries: Vec<(&str, Value)>) -> Value {
    Value::Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )
}

pub fn string(value: impl Into<String>) -> Value {
    Value::String(value.into())
}

pub fn number(value: impl Into<f64>) -> Value {
    Value::Number(value.into())
}

pub fn count(value: usize) -> Value {
    Value::Number(value as f64)
}

pub fn count_u64(value: u64) -> Value {
    Value::Number(value as f64)
}

pub fn bool(value: bool) -> Value {
    Value::Bool(value)
}

pub fn array(values: Vec<Value>) -> Value {
    Value::Array(values)
}

pub fn null() -> Value {
    Value::Null
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_json() {
        let value = parse(r#"{"a":[1,2.5,true,null,"x\n\u00e9"],"b":{"c":-3}}"#).unwrap();
        assert_eq!(value.get("a").unwrap().as_array().unwrap().len(), 5);
        assert_eq!(
            value
                .get("b")
                .unwrap()
                .get("c")
                .unwrap()
                .as_i64(),
            Some(-3)
        );
    }

    #[test]
    fn rejects_malformed_and_over_limit_json() {
        assert!(parse("{").is_err());
        assert!(parse("[1,]").is_err());
        assert!(parse("{\"a\":1} trailing").is_err());
        assert!(parse("\"\\uD800\"").is_err());
        assert!(parse("\"\\q\"").is_err());
        assert!(parse(&"[".repeat(MAX_JSON_DEPTH + 2)).is_err());
    }

    #[test]
    fn serializer_escapes_and_round_trips() {
        let original = object(vec![
            ("text", string("a\"b\\c\n")),
            ("number", number(3.0)),
            ("list", array(vec![bool(true), null()])),
        ]);
        let text = original.stringify();
        let parsed = parse(&text).unwrap();
        assert_eq!(parsed.get("text").and_then(Value::as_str), Some("a\"b\\c\n"));
        assert_eq!(parsed.get("number").and_then(Value::as_i64), Some(3));
    }

    #[test]
    fn duplicate_keys_are_rejected() {
        assert!(parse(r#"{"a":1,"a":2}"#).is_err());
    }

    #[test]
    fn numbers_are_finite() {
        assert!(parse("1e999").is_err());
        assert!(parse("NaN").is_err());
    }
}
