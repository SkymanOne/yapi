//! Lenient JSON parsing for model output: tool-call arguments and SSE payloads.

use serde_json::{Map, Number, Value};

const VALID_ESCAPES: &[char] = &['"', '\\', '/', 'b', 'f', 'n', 'r', 't', 'u'];

/// Escapes what models commonly emit raw inside JSON strings: control characters,
/// lone backslashes and invalid escapes. Valid JSON is returned unchanged.
pub fn repair_json(json: &str) -> String {
    let chars: Vec<char> = json.chars().collect();
    let mut repaired = String::with_capacity(json.len());
    let mut in_string = false;
    let mut index = 0;
    while index < chars.len() {
        let char = chars[index];
        if !in_string {
            repaired.push(char);
            in_string = char == '"';
        } else if char == '"' {
            repaired.push(char);
            in_string = false;
        } else if char == '\\' {
            match chars.get(index + 1) {
                None => repaired.push_str("\\\\"),
                Some('u')
                    if chars.len() >= index + 6
                        && chars[index + 2..index + 6]
                            .iter()
                            .all(char::is_ascii_hexdigit) =>
                {
                    repaired.extend(&chars[index..index + 6]);
                    index += 5;
                }
                Some(next) if VALID_ESCAPES.contains(next) => {
                    repaired.push('\\');
                    repaired.push(*next);
                    index += 1;
                }
                Some(_) => repaired.push_str("\\\\"),
            }
        } else if (char as u32) < 0x20 {
            match char {
                '\u{8}' => repaired.push_str("\\b"),
                '\u{c}' => repaired.push_str("\\f"),
                '\n' => repaired.push_str("\\n"),
                '\r' => repaired.push_str("\\r"),
                '\t' => repaired.push_str("\\t"),
                _ => repaired.push_str(&format!("\\u{:04x}", char as u32)),
            }
        } else {
            repaired.push(char);
        }
        index += 1;
    }
    repaired
}

/// Parses JSON, retrying once on the [`repair_json`] form.
pub fn parse_json_with_repair(json: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(json).or_else(|err| {
        let repaired = repair_json(json);
        if repaired == json {
            Err(err)
        } else {
            serde_json::from_str(&repaired)
        }
    })
}

/// The message of the error JavaScript's `JSON.parse` throws for `text`, as
/// V8 words it; `None` when `text` is valid JSON.
pub fn js_parse_error(text: &str) -> Option<String> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut scan = V8Scan {
        units: &units,
        at: 0,
    };
    let failure = match scan.value() {
        Ok(()) => {
            scan.skip_whitespace();
            if scan.at == units.len() {
                return None;
            }
            Some("Unexpected non-whitespace character after JSON")
        }
        Err(failure) => failure,
    };
    Some(scan.message(text, failure))
}

/// A cursor over UTF-16 units, as V8's JSON parser reads them. A failure is
/// at the cursor, with V8's message for it, or `None` to name the token there.
struct V8Scan<'a> {
    units: &'a [u16],
    at: usize,
}

type V8Failure = Option<&'static str>;

impl V8Scan<'_> {
    fn peek(&self) -> Option<u8> {
        let unit = *self.units.get(self.at)?;
        // Units past ASCII are ILLEGAL tokens, as any unexpected character.
        Some(u8::try_from(unit).unwrap_or(0xFF))
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    /// Skips whitespace and then `token`, or fails with `message`.
    fn expect(&mut self, token: u8, message: &'static str) -> Result<(), V8Failure> {
        self.skip_whitespace();
        if self.peek() != Some(token) {
            return Err(Some(message));
        }
        self.at += 1;
        Ok(())
    }

    fn digits(&mut self) -> usize {
        let start = self.at;
        while self.peek().is_some_and(|unit| unit.is_ascii_digit()) {
            self.at += 1;
        }
        self.at - start
    }

    fn value(&mut self) -> Result<(), V8Failure> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'"') => self.string(),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(b't') => self.literal(b"true"),
            Some(b'f') => self.literal(b"false"),
            Some(b'n') => self.literal(b"null"),
            Some(b'{') => {
                self.at += 1;
                self.skip_whitespace();
                if self.peek() == Some(b'}') {
                    self.at += 1;
                    return Ok(());
                }
                let mut name = "Expected property name or '}' in JSON";
                loop {
                    self.skip_whitespace();
                    if self.peek() != Some(b'"') {
                        return Err(Some(name));
                    }
                    self.string()?;
                    self.expect(b':', "Expected ':' after property name in JSON")?;
                    self.value()?;
                    self.skip_whitespace();
                    if self.peek() != Some(b',') {
                        return self
                            .expect(b'}', "Expected ',' or '}' after property value in JSON");
                    }
                    self.at += 1;
                    name = "Expected double-quoted property name in JSON";
                }
            }
            Some(b'[') => {
                self.at += 1;
                self.skip_whitespace();
                if self.peek() == Some(b']') {
                    self.at += 1;
                    return Ok(());
                }
                loop {
                    self.value()?;
                    self.skip_whitespace();
                    if self.peek() != Some(b',') {
                        return self
                            .expect(b']', "Expected ',' or ']' after array element in JSON");
                    }
                    self.at += 1;
                }
            }
            _ => Err(None),
        }
    }

    fn literal(&mut self, word: &[u8]) -> Result<(), V8Failure> {
        for &unit in word {
            if self.peek() != Some(unit) {
                return Err(None);
            }
            self.at += 1;
        }
        Ok(())
    }

    fn number(&mut self) -> Result<(), V8Failure> {
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        if self.peek() == Some(b'0') {
            self.at += 1;
            if self.peek().is_some_and(|unit| unit.is_ascii_digit()) {
                return Err(None);
            }
        } else if self.digits() == 0 {
            return Err(Some("No number after minus sign in JSON"));
        }
        if self.peek() == Some(b'.') {
            self.at += 1;
            if self.digits() == 0 {
                return Err(Some("Unterminated fractional number in JSON"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            if self.digits() == 0 {
                return Err(Some("Exponent part is missing a number in JSON"));
            }
        }
        Ok(())
    }

    fn string(&mut self) -> Result<(), V8Failure> {
        self.at += 1;
        loop {
            match self.peek() {
                None => return Err(Some("Unterminated string in JSON")),
                Some(b'"') => {
                    self.at += 1;
                    return Ok(());
                }
                Some(b'\\') => {
                    self.at += 1;
                    match self.peek() {
                        None => return Err(None),
                        Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => {
                            self.at += 1
                        }
                        Some(b'u') => {
                            self.at += 1;
                            for _ in 0..4 {
                                if !self.peek().is_some_and(|unit| unit.is_ascii_hexdigit()) {
                                    return Err(Some("Bad Unicode escape in JSON"));
                                }
                                self.at += 1;
                            }
                        }
                        Some(_) => return Err(Some("Bad escaped character in JSON")),
                    }
                }
                Some(0..0x20) => {
                    return Err(Some("Bad control character in string literal in JSON"));
                }
                Some(_) => self.at += 1,
            }
        }
    }

    /// V8's message for `failure` at the cursor of a scan of `text`.
    fn message(&self, text: &str, failure: V8Failure) -> String {
        let at = self.at;
        let failure = match (failure, self.peek()) {
            (Some(message), _) => message,
            (None, None) => return "Unexpected end of JSON input".into(),
            (None, Some(b'-' | b'0'..=b'9')) => "Unexpected number in JSON",
            (None, Some(b'"')) => "Unexpected string in JSON",
            (None, Some(_))
                if ["NaN", "Infinity", "undefined", "[object Object]"].contains(&text) =>
            {
                return format!("\"{text}\" is not valid JSON");
            }
            (None, Some(_)) => {
                const CONTEXT: usize = 10;
                let units = self.units;
                let slice = |from: usize, to: usize| String::from_utf16_lossy(&units[from..to]);
                let token = slice(at, at + 1);
                let source = if units.len() <= CONTEXT * 2 {
                    format!("\"{text}\"")
                } else if at < CONTEXT {
                    format!("\"{}\"...", slice(0, at + CONTEXT))
                } else if at < units.len() - CONTEXT {
                    format!("...\"{}\"...", slice(at - CONTEXT, at + CONTEXT))
                } else {
                    format!("...\"{}\"", slice(at - CONTEXT, units.len()))
                };
                return format!("Unexpected token '{token}', {source} is not valid JSON");
            }
        };
        // V8 counts `\r\n`, `\r` and `\n` as one line break each.
        let mut line = 1;
        let mut line_start = 0;
        let mut index = 0;
        while index < at {
            if self.units[index] == u16::from(b'\r')
                && self.units.get(index + 1) == Some(&u16::from(b'\n'))
                && index + 1 < at
            {
                index += 1;
            }
            if matches!(u8::try_from(self.units[index]), Ok(b'\r' | b'\n')) {
                line += 1;
                line_start = index + 1;
            }
            index += 1;
        }
        let column = at - line_start + 1;
        format!("{failure} at position {at} (line {line} column {column})")
    }
}

/// Parses possibly incomplete tool-call arguments into an object.
///
/// Complete JSON parses as-is (with repair); a prefix of an object yields the
/// fields seen so far, with a trailing string, number or literal kept in its partial
/// form. Anything that is not an object yields an empty object.
pub fn parse_streaming_json(partial: &str) -> Map<String, Value> {
    if partial.trim().is_empty() {
        return Map::new();
    }
    let value = parse_json_with_repair(partial)
        .ok()
        .or_else(|| parse_partial(partial))
        .or_else(|| parse_partial(&repair_json(partial)));
    match value {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

/// Parses a JSON prefix, closing whatever is still open.
fn parse_partial(text: &str) -> Option<Value> {
    let mut parser = Partial {
        chars: text.chars().collect(),
        index: 0,
    };
    parser.whitespace();
    parser.value().map(|(value, _)| value)
}

struct Partial {
    chars: Vec<char>,
    index: usize,
}

/// Whether a value ended before its closing token.
type Complete = bool;

impl Partial {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn whitespace(&mut self) {
        while self
            .peek()
            .is_some_and(|c| matches!(c, ' ' | '\t' | '\n' | '\r'))
        {
            self.index += 1;
        }
    }

    /// A value, or `None` when nothing usable precedes the end or an error.
    fn value(&mut self) -> Option<(Value, Complete)> {
        match self.peek()? {
            '{' => self.object(),
            '[' => self.array(),
            '"' => self
                .string()
                .map(|(text, complete)| (Value::String(text), complete)),
            't' => self.literal("true", Value::Bool(true)),
            'f' => self.literal("false", Value::Bool(false)),
            'n' => self.literal("null", Value::Null),
            '-' | '0'..='9' => self.number(),
            _ => None,
        }
    }

    fn object(&mut self) -> Option<(Value, Complete)> {
        self.index += 1;
        let mut map = Map::new();
        loop {
            self.whitespace();
            match self.peek() {
                None => return Some((Value::Object(map), false)),
                Some('}') => {
                    self.index += 1;
                    return Some((Value::Object(map), true));
                }
                Some(',') => {
                    self.index += 1;
                    continue;
                }
                Some('"') => {}
                Some(_) => return None,
            }
            let (key, complete) = self.string()?;
            if !complete {
                return Some((Value::Object(map), false));
            }
            self.whitespace();
            match self.peek() {
                None => return Some((Value::Object(map), false)),
                Some(':') => self.index += 1,
                Some(_) => return None,
            }
            self.whitespace();
            if self.peek().is_none() {
                return Some((Value::Object(map), false));
            }
            match self.value() {
                Some((value, true)) => {
                    map.insert(key, value);
                }
                Some((value, false)) => {
                    map.insert(key, value);
                    return Some((Value::Object(map), false));
                }
                None if self.peek().is_none() => return Some((Value::Object(map), false)),
                None => return None,
            }
        }
    }

    fn array(&mut self) -> Option<(Value, Complete)> {
        self.index += 1;
        let mut items = Vec::new();
        loop {
            self.whitespace();
            match self.peek() {
                None => return Some((Value::Array(items), false)),
                Some(']') => {
                    self.index += 1;
                    return Some((Value::Array(items), true));
                }
                Some(',') => {
                    self.index += 1;
                    continue;
                }
                Some(_) => {}
            }
            match self.value() {
                Some((value, true)) => items.push(value),
                Some((value, false)) => {
                    items.push(value);
                    return Some((Value::Array(items), false));
                }
                None if self.peek().is_none() => return Some((Value::Array(items), false)),
                None => return None,
            }
        }
    }

    fn string(&mut self) -> Option<(String, Complete)> {
        self.index += 1;
        let mut text = String::new();
        while let Some(char) = self.peek() {
            self.index += 1;
            match char {
                '"' => return Some((text, true)),
                '\\' => {
                    let Some(escape) = self.peek() else {
                        return Some((text, false));
                    };
                    self.index += 1;
                    match escape {
                        '"' | '\\' | '/' => text.push(escape),
                        'b' => text.push('\u{8}'),
                        'f' => text.push('\u{c}'),
                        'n' => text.push('\n'),
                        'r' => text.push('\r'),
                        't' => text.push('\t'),
                        'u' => {
                            let Some(unit) = self.hex4() else {
                                return Some((text, false));
                            };
                            let code = if (0xd800..0xdc00).contains(&unit)
                                && self.chars.get(self.index) == Some(&'\\')
                                && self.chars.get(self.index + 1) == Some(&'u')
                            {
                                self.index += 2;
                                match self.hex4() {
                                    Some(low) if (0xdc00..0xe000).contains(&low) => {
                                        0x10000 + ((unit - 0xd800) << 10) + (low - 0xdc00)
                                    }
                                    _ => 0xfffd,
                                }
                            } else {
                                unit
                            };
                            text.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                        }
                        _ => return None,
                    }
                }
                _ => text.push(char),
            }
        }
        Some((text, false))
    }

    fn hex4(&mut self) -> Option<u32> {
        let digits: String = self.chars.get(self.index..self.index + 4)?.iter().collect();
        let value = u32::from_str_radix(&digits, 16).ok()?;
        self.index += 4;
        Some(value)
    }

    fn literal(&mut self, word: &str, value: Value) -> Option<(Value, Complete)> {
        for expected in word.chars() {
            match self.peek() {
                None => return Some((value, false)),
                Some(char) if char == expected => self.index += 1,
                Some(_) => return None,
            }
        }
        Some((value, true))
    }

    fn number(&mut self) -> Option<(Value, Complete)> {
        let start = self.index;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_digit() || matches!(c, '-' | '+' | '.' | 'e' | 'E'))
        {
            self.index += 1;
        }
        let complete = self.peek().is_some();
        let mut text: String = self.chars[start..self.index].iter().collect();
        loop {
            if let Ok(number) = serde_json::from_str::<Number>(&text) {
                return Some((Value::Number(number), complete));
            }
            if complete || text.pop().is_none() || text.is_empty() {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Messages recorded from Node 26's `JSON.parse`.
    #[test]
    fn v8_parse_errors() {
        for (text, message) in [
            ("{\"type\":\"get_state\"}", None),
            (" [1, 2.5e-3, true, null, \"\\u00e9\"] ", None),
            ("", Some("Unexpected end of JSON input")),
            (
                "not json",
                Some("Unexpected token 'o', \"not json\" is not valid JSON"),
            ),
            (
                "{\"type\":\"x\"} extra",
                Some(
                    "Unexpected non-whitespace character after JSON at position 13 (line 1 column 14)",
                ),
            ),
            (
                "{",
                Some("Expected property name or '}' in JSON at position 1 (line 1 column 2)"),
            ),
            ("{\"type\":", Some("Unexpected end of JSON input")),
            (
                "{\"a\":1,}",
                Some(
                    "Expected double-quoted property name in JSON at position 7 (line 1 column 8)",
                ),
            ),
            (
                "{\"a\" 1}",
                Some("Expected ':' after property name in JSON at position 5 (line 1 column 6)"),
            ),
            (
                "\"abc",
                Some("Unterminated string in JSON at position 4 (line 1 column 5)"),
            ),
            (
                "[1 2]",
                Some(
                    "Expected ',' or ']' after array element in JSON at position 3 (line 1 column 4)",
                ),
            ),
            (
                "{\"a\":1 \"b\":2}",
                Some(
                    "Expected ',' or '}' after property value in JSON at position 7 (line 1 column 8)",
                ),
            ),
            (
                "-",
                Some("No number after minus sign in JSON at position 1 (line 1 column 2)"),
            ),
            (
                "1.",
                Some("Unterminated fractional number in JSON at position 2 (line 1 column 3)"),
            ),
            (
                "1e+",
                Some("Exponent part is missing a number in JSON at position 3 (line 1 column 4)"),
            ),
            (
                "\"\\x\"",
                Some("Bad escaped character in JSON at position 2 (line 1 column 3)"),
            ),
            (
                "\"\\u12g4\"",
                Some("Bad Unicode escape in JSON at position 5 (line 1 column 6)"),
            ),
            (
                "\"a\tb\"",
                Some(
                    "Bad control character in string literal in JSON at position 2 (line 1 column 3)",
                ),
            ),
            ("\"\\", Some("Unexpected end of JSON input")),
            ("tru", Some("Unexpected end of JSON input")),
            (
                "01",
                Some("Unexpected number in JSON at position 1 (line 1 column 2)"),
            ),
            (
                "[1,]",
                Some("Unexpected token ']', \"[1,]\" is not valid JSON"),
            ),
            ("NaN", Some("\"NaN\" is not valid JSON")),
            ("undefined", Some("\"undefined\" is not valid JSON")),
            (
                "x{\"type\":\"prompt\",\"message\":\"this is long\"}",
                Some("Unexpected token 'x', \"x{\"type\":\"\"... is not valid JSON"),
            ),
            (
                "{\"type\":\"prompt\",\"message\":\"long enough\"x",
                Some(
                    "Expected ',' or '}' after property value in JSON at position 40 (line 1 column 41)",
                ),
            ),
            (
                "{\"a\":[1,2,3,4,5,6,7,8,9]",
                Some(
                    "Expected ',' or '}' after property value in JSON at position 24 (line 1 column 25)",
                ),
            ),
            (
                "{\"x\":1}\r\n{",
                Some(
                    "Unexpected non-whitespace character after JSON at position 9 (line 2 column 1)",
                ),
            ),
            (
                "\"😀\" x",
                Some(
                    "Unexpected non-whitespace character after JSON at position 5 (line 1 column 6)",
                ),
            ),
            ("é", Some("Unexpected token 'é', \"é\" is not valid JSON")),
            (
                "{\"type\":\"prompt\",\"message\":x,\"more\":\"stuff\"}",
                Some("Unexpected token 'x', ...\"\"message\":x,\"more\":\"\"... is not valid JSON"),
            ),
            (
                "{\"type\":\"prompt\",\"message\":x}",
                Some("Unexpected token 'x', ...\"\"message\":x}\" is not valid JSON"),
            ),
        ] {
            assert_eq!(js_parse_error(text).as_deref(), message, "{text}");
        }
    }

    #[test]
    fn repairs_controls_and_escapes() {
        assert_eq!(repair_json("{\"a\":\"x\ny\"}"), "{\"a\":\"x\\ny\"}");
        assert_eq!(repair_json(r#"{"a":"C:\dir"}"#), r#"{"a":"C:\\dir"}"#);
        assert_eq!(repair_json(r#"{"a":"\u00e9\n"}"#), r#"{"a":"\u00e9\n"}"#);
        assert_eq!(
            parse_json_with_repair("{\"a\":\"tab\there\"}").unwrap(),
            json!({"a": "tab\there"})
        );
    }

    #[test]
    fn parses_prefixes() {
        let cases = [
            ("", json!({})),
            ("{", json!({})),
            (r#"{"pa"#, json!({})),
            (r#"{"path""#, json!({})),
            (r#"{"path":"#, json!({})),
            (r#"{"path":"/tm"#, json!({"path": "/tm"})),
            (r#"{"path":"/tmp","n":12"#, json!({"path": "/tmp", "n": 12})),
            (r#"{"n":1.5e"#, json!({"n": 1.5})),
            (r#"{"a":[1,{"b":tr"#, json!({"a": [1, {"b": true}]})),
            (r#"{"a":"\u00"#, json!({"a": ""})),
            (r#"{"a":"x\"#, json!({"a": "x"})),
            (r#"{"a":1}"#, json!({"a": 1})),
            ("[1,2]", json!({})),
            ("nonsense", json!({})),
        ];
        for (input, expected) in cases {
            assert_eq!(
                Value::Object(parse_streaming_json(input)),
                expected,
                "{input}"
            );
        }
    }
}
