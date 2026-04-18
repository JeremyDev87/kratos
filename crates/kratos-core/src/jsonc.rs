use crate::error::{KratosError, KratosResult};

#[derive(Clone, Debug, PartialEq)]
pub struct JsoncDocument {
    pub raw: String,
    pub sanitized: String,
    pub value: JsonValue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(JsonObject),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct JsonObject {
    entries: Vec<(String, JsonValue)>,
}

impl JsonObject {
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.entries
            .iter()
            .find(|(entry_key, _)| entry_key == key)
            .map(|(_, value)| value)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &JsonValue)> {
        self.entries.iter().map(|(key, value)| (key, value))
    }

    pub fn values(&self) -> impl Iterator<Item = &JsonValue> {
        self.entries.iter().map(|(_, value)| value)
    }

    fn insert(&mut self, key: String, value: JsonValue) {
        if let Some((_, existing)) = self
            .entries
            .iter_mut()
            .find(|(entry_key, _)| entry_key == &key)
        {
            *existing = value;
            return;
        }

        self.entries.push((key, value));
    }
}

impl JsonValue {
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            Self::Object(object) => object.get(key),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&JsonObject> {
        match self {
            Self::Object(object) => Some(object),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }
}

pub fn strip_jsonc_comments(source: &str) -> KratosResult<String> {
    Ok(strip_comments(source))
}

pub fn parse_jsonc_document(source: &str) -> KratosResult<JsoncDocument> {
    let without_comments = strip_jsonc_comments(source)?;
    let sanitized = strip_trailing_commas(&without_comments);
    let value = Parser::new(&sanitized).parse()?;

    Ok(JsoncDocument {
        raw: source.to_string(),
        sanitized,
        value,
    })
}

pub fn parse_loose_json(source: &str) -> KratosResult<JsonValue> {
    Ok(parse_jsonc_document(source)?.value)
}

fn strip_comments(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut output = String::with_capacity(input.len());
    let mut index = 0;
    let mut state = "code";

    while index < chars.len() {
        let current = chars[index];
        let next = chars.get(index + 1).copied();

        if state == "line-comment" {
            if current == '\n' {
                state = "code";
                output.push('\n');
            } else {
                output.push(' ');
            }

            index += 1;
            continue;
        }

        if state == "block-comment" {
            if current == '*' && next == Some('/') {
                output.push(' ');
                output.push(' ');
                index += 2;
                state = "code";
            } else {
                output.push(if current == '\n' { '\n' } else { ' ' });
                index += 1;
            }

            continue;
        }

        if state == "string" {
            output.push(current);

            if current == '\\' {
                if let Some(next_char) = next {
                    output.push(next_char);
                    index += 2;
                    continue;
                }
            } else if current == '"' {
                state = "code";
            }

            index += 1;
            continue;
        }

        if current == '/' && next == Some('/') {
            output.push(' ');
            output.push(' ');
            index += 2;
            state = "line-comment";
            continue;
        }

        if current == '/' && next == Some('*') {
            output.push(' ');
            output.push(' ');
            index += 2;
            state = "block-comment";
            continue;
        }

        if current == '"' {
            state = "string";
        }

        output.push(current);
        index += 1;
    }

    output
}

fn strip_trailing_commas(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut output = String::with_capacity(input.len());
    let mut index = 0;
    let mut state = "code";

    while index < chars.len() {
        let current = chars[index];

        if state == "string" {
            output.push(current);

            if current == '\\' {
                if let Some(next_char) = chars.get(index + 1).copied() {
                    output.push(next_char);
                    index += 2;
                    continue;
                }
            } else if current == '"' {
                state = "code";
            }

            index += 1;
            continue;
        }

        if current == '"' {
            state = "string";
            output.push(current);
            index += 1;
            continue;
        }

        if current == ',' {
            let mut lookahead = index + 1;

            while lookahead < chars.len() && chars[lookahead].is_whitespace() {
                lookahead += 1;
            }

            if matches!(chars.get(lookahead), Some(&'}') | Some(&']')) {
                index += 1;
                continue;
            }
        }

        output.push(current);
        index += 1;
    }

    output
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    fn parse(mut self) -> KratosResult<JsonValue> {
        self.skip_whitespace();
        let value = self.parse_value()?;
        self.skip_whitespace();

        if self.pos != self.chars.len() {
            return Err(self.error("Unexpected trailing content"));
        }

        Ok(value)
    }

    fn parse_value(&mut self) -> KratosResult<JsonValue> {
        self.skip_whitespace();

        match self.peek() {
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => self.parse_string().map(JsonValue::String),
            Some('t') => self.parse_keyword("true", JsonValue::Bool(true)),
            Some('f') => self.parse_keyword("false", JsonValue::Bool(false)),
            Some('n') => self.parse_keyword("null", JsonValue::Null),
            Some('-' | '0'..='9') => self.parse_number().map(JsonValue::Number),
            Some(_) => Err(self.error("Unexpected token")),
            None => Err(self.error("Unexpected end of input")),
        }
    }

    fn parse_object(&mut self) -> KratosResult<JsonValue> {
        self.expect('{')?;
        self.skip_whitespace();

        let mut values = JsonObject::default();

        if self.peek() == Some('}') {
            self.pos += 1;
            return Ok(JsonValue::Object(values));
        }

        loop {
            self.skip_whitespace();

            if self.peek() != Some('"') {
                return Err(self.error("Object keys must be strings"));
            }

            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect(':')?;
            self.skip_whitespace();

            let value = self.parse_value()?;
            values.insert(key, value);
            self.skip_whitespace();

            match self.peek() {
                Some(',') => {
                    self.pos += 1;
                }
                Some('}') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(self.error("Expected ',' or '}' in object")),
            }
        }

        Ok(JsonValue::Object(values))
    }

    fn parse_array(&mut self) -> KratosResult<JsonValue> {
        self.expect('[')?;
        self.skip_whitespace();

        let mut values = Vec::new();

        if self.peek() == Some(']') {
            self.pos += 1;
            return Ok(JsonValue::Array(values));
        }

        loop {
            values.push(self.parse_value()?);
            self.skip_whitespace();

            match self.peek() {
                Some(',') => {
                    self.pos += 1;
                }
                Some(']') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(self.error("Expected ',' or ']' in array")),
            }
        }

        Ok(JsonValue::Array(values))
    }

    fn parse_string(&mut self) -> KratosResult<String> {
        self.expect('"')?;
        let mut value = String::new();

        while let Some(current) = self.next() {
            match current {
                '"' => return Ok(value),
                '\\' => {
                    let escaped = self
                        .next()
                        .ok_or_else(|| self.error("Unterminated escape sequence"))?;

                    match escaped {
                        '"' => value.push('"'),
                        '\\' => value.push('\\'),
                        '/' => value.push('/'),
                        'b' => value.push('\u{0008}'),
                        'f' => value.push('\u{000C}'),
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        'u' => value.push_str(&self.parse_unicode_escape_fragment()?),
                        _ => return Err(self.error("Unsupported escape sequence")),
                    }
                }
                other if other <= '\u{001F}' => {
                    return Err(self.error("Unescaped control character in string"))
                }
                other => value.push(other),
            }
        }

        Err(self.error("Unterminated string"))
    }

    fn parse_unicode_escape_fragment(&mut self) -> KratosResult<String> {
        let (first, first_hex) = self.parse_unicode_code_unit()?;

        if (0xD800..=0xDBFF).contains(&first) {
            if let Some((second, _)) = self.peek_unicode_code_unit_after_escape()? {
                if (0xDC00..=0xDFFF).contains(&second) {
                    self.pos += 2;
                    let (second, _) = self.parse_unicode_code_unit()?;

                    let high = u32::from(first) - 0xD800;
                    let low = u32::from(second) - 0xDC00;
                    let codepoint = 0x10000 + ((high << 10) | low);

                    let decoded = char::from_u32(codepoint)
                        .ok_or_else(|| self.error("Invalid unicode codepoint"))?;
                    return Ok(decoded.to_string());
                }
            }

            return Ok(format!("\\u{first_hex}"));
        }

        if (0xDC00..=0xDFFF).contains(&first) {
            return Ok(format!("\\u{first_hex}"));
        }

        let decoded = char::from_u32(u32::from(first))
            .ok_or_else(|| self.error("Invalid unicode codepoint"))?;
        Ok(decoded.to_string())
    }

    fn parse_unicode_code_unit(&mut self) -> KratosResult<(u16, String)> {
        let mut hex = String::new();

        for _ in 0..4 {
            let ch = self
                .next()
                .ok_or_else(|| self.error("Incomplete unicode escape"))?;

            if !ch.is_ascii_hexdigit() {
                return Err(self.error("Invalid unicode escape"));
            }

            hex.push(ch);
        }

        let code_unit =
            u16::from_str_radix(&hex, 16).map_err(|_| self.error("Invalid unicode escape"))?;
        Ok((code_unit, hex))
    }

    fn peek_unicode_code_unit_after_escape(&self) -> KratosResult<Option<(u16, String)>> {
        if self.chars.get(self.pos) != Some(&'\\') || self.chars.get(self.pos + 1) != Some(&'u') {
            return Ok(None);
        }

        let mut hex = String::new();

        for offset in 2..6 {
            let Some(ch) = self.chars.get(self.pos + offset).copied() else {
                return Ok(None);
            };

            if !ch.is_ascii_hexdigit() {
                return Err(self.error("Invalid unicode escape"));
            }

            hex.push(ch);
        }

        let code_unit =
            u16::from_str_radix(&hex, 16).map_err(|_| self.error("Invalid unicode escape"))?;
        Ok(Some((code_unit, hex)))
    }

    fn parse_number(&mut self) -> KratosResult<String> {
        let start = self.pos;

        if self.peek() == Some('-') {
            self.pos += 1;
        }

        match self.peek() {
            Some('0') => {
                self.pos += 1;

                if matches!(self.peek(), Some('0'..='9')) {
                    return Err(self.error("Leading zeros are not allowed"));
                }
            }
            Some('1'..='9') => self.consume_digits()?,
            _ => return Err(self.error("Expected digit")),
        }

        if self.peek() == Some('.') {
            self.pos += 1;
            self.consume_digits()?;
        }

        if matches!(self.peek(), Some('e') | Some('E')) {
            self.pos += 1;

            if matches!(self.peek(), Some('+') | Some('-')) {
                self.pos += 1;
            }

            self.consume_digits()?;
        }

        Ok(self.chars[start..self.pos].iter().collect())
    }

    fn parse_keyword(&mut self, keyword: &str, value: JsonValue) -> KratosResult<JsonValue> {
        if self.starts_with(keyword) {
            self.pos += keyword.chars().count();
            Ok(value)
        } else {
            Err(self.error("Unexpected token"))
        }
    }

    fn consume_digits(&mut self) -> KratosResult<()> {
        let start = self.pos;

        while matches!(self.peek(), Some('0'..='9')) {
            self.pos += 1;
        }

        if self.pos == start {
            return Err(self.error("Expected digit"));
        }

        Ok(())
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(ch) if ch.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, expected: char) -> KratosResult<()> {
        match self.next() {
            Some(found) if found == expected => Ok(()),
            _ => Err(self.error(&format!("Expected '{expected}'"))),
        }
    }

    fn starts_with(&self, keyword: &str) -> bool {
        let needle: Vec<char> = keyword.chars().collect();
        self.chars[self.pos..].starts_with(&needle)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += 1;
        Some(ch)
    }

    fn error(&self, message: &str) -> KratosError {
        KratosError::Json(format!("{message} at character {}", self.pos))
    }
}
