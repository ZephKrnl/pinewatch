// Ported from Json.NET 13.0.3 (commit 0a2e291c), `Src/Newtonsoft.Json/JsonTextReader.cs`, `JsonReader.cs`,
// `JsonPosition.cs`, `JsonReaderException.cs` and `Utilities/JavaScriptUtils.cs`. Copyright (c) 2007 James
// Newton-King. MIT license; see NOTICES-PORTED-CODE.md.

//! `JsonTextReader.Read` over a `StringReader`, with `JsonReader`'s defaults (`MaxDepth` 64,
//! `DateParseHandling.DateTime`, `DateTimeZoneHandling.RoundtripKind`, `FloatParseHandling.Double`, no
//! `SupportMultipleContent`), as `JsonConvert.DeserializeObject` creates it.
//!
//! The reader keeps the original's character buffer: 1024 characters, filled from the text in `StringReader.Read`
//! chunks, shifted and grown the same way. Positions in it are observable, because `TryReadOffset` compares one with
//! a string's length (`date.rs`).

use pinewatch_mono::{
    DOUBLE_NAN, MonoException, NetString, NumberFormat, NumberStyles, chars, convert_to_int64, try_parse_f64,
};

use crate::date::{JsonDate, try_parse_date_time};
use crate::number::{ParseResult, big_integer_text, int64_try_parse, try_hex_text_to_int};

/// A `JsonToken` with the reader's `Value`.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonToken {
    None,
    StartObject,
    StartArray,
    /// The constructor's name.
    StartConstructor(NetString),
    PropertyName(NetString),
    Comment(NetString),
    Integer(JsonInteger),
    Float(f64),
    String(NetString),
    Boolean(bool),
    Null,
    Undefined,
    EndObject,
    EndArray,
    EndConstructor,
    Date(JsonDate),
}

impl JsonToken {
    /// The `JsonToken` member's name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::StartObject => "StartObject",
            Self::StartArray => "StartArray",
            Self::StartConstructor(_) => "StartConstructor",
            Self::PropertyName(_) => "PropertyName",
            Self::Comment(_) => "Comment",
            Self::Integer(_) => "Integer",
            Self::Float(_) => "Float",
            Self::String(_) => "String",
            Self::Boolean(_) => "Boolean",
            Self::Null => "Null",
            Self::Undefined => "Undefined",
            Self::EndObject => "EndObject",
            Self::EndArray => "EndArray",
            Self::EndConstructor => "EndConstructor",
            Self::Date(_) => "Date",
        }
    }

    /// `JsonTokenUtils.IsStartToken`.
    fn is_start(&self) -> bool {
        matches!(self, Self::StartObject | Self::StartArray | Self::StartConstructor(_))
    }
}

/// An `Integer` token's value: a `long`, or the decimal digits of a `BigInteger` for integers that overflow one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonInteger {
    I64(i64),
    Big(String),
}

/// `JsonReader.State`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Start,
    Property,
    ObjectStart,
    Object,
    ArrayStart,
    Array,
    PostValue,
    ConstructorStart,
    Constructor,
    Finished,
}

/// `JsonContainerType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContainerType {
    None,
    Object,
    Array,
    Constructor,
}

impl ContainerType {
    fn name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Object => "Object",
            Self::Array => "Array",
            Self::Constructor => "Constructor",
        }
    }
}

/// `JsonPosition`.
#[derive(Debug, Clone)]
struct JsonPosition {
    kind: ContainerType,
    position: i32,
    property_name: Option<NetString>,
    has_index: bool,
}

impl JsonPosition {
    /// `new JsonPosition()`: the struct's default, position 0.
    fn none() -> Self {
        Self { kind: ContainerType::None, position: 0, property_name: None, has_index: false }
    }

    /// `new JsonPosition(type)`.
    fn new(kind: ContainerType) -> Self {
        let has_index = matches!(kind, ContainerType::Array | ContainerType::Constructor);
        Self { kind, position: -1, property_name: None, has_index }
    }

    /// `WriteTo`: `.name`, `['name']` for names with special characters, or `[index]`.
    fn write_to(&self, path: &mut Vec<u16>) {
        match self.kind {
            ContainerType::Object => {
                let name = self.property_name.as_ref().map(NetString::as_units).unwrap_or_default();
                if name.iter().any(|unit| PATH_SPECIAL_CHARACTERS.contains(unit)) {
                    path.extend("['".encode_utf16());
                    write_single_quote_escaped(path, name);
                    path.extend("']".encode_utf16());
                } else {
                    if !path.is_empty() {
                        path.push(u16::from(b'.'));
                    }
                    path.extend_from_slice(name);
                }
            }
            ContainerType::Array | ContainerType::Constructor => {
                path.extend(format!("[{}]", self.position).encode_utf16());
            }
            ContainerType::None => {}
        }
    }
}

/// `JsonPosition.SpecialCharacters`.
const PATH_SPECIAL_CHARACTERS: [u16; 18] = [
    0x2E, 0x20, 0x27, 0x2F, 0x22, 0x5B, 0x5D, 0x28, 0x29, 0x09, 0x0A, 0x0D, 0x0C, 0x08, 0x5C, 0x85, 0x2028, 0x2029,
];

/// `JavaScriptUtils.WriteEscapedJavaScriptString(writer, s, '\'', false, SingleQuoteCharEscapeFlags,
/// StringEscapeHandling.Default, ...)`.
fn write_single_quote_escaped(out: &mut Vec<u16>, s: &[u16]) {
    for &c in s {
        let escaped = match c {
            0x09 => Some("\\t".to_string()),
            0x0A => Some("\\n".to_string()),
            0x0D => Some("\\r".to_string()),
            0x0C => Some("\\f".to_string()),
            0x08 => Some("\\b".to_string()),
            0x5C => Some("\\\\".to_string()),
            0x85 => Some("\\u0085".to_string()),
            0x2028 => Some("\\u2028".to_string()),
            0x2029 => Some("\\u2029".to_string()),
            0x27 => Some("\\'".to_string()),
            // The other characters the single-quote flags escape: the controls, as lower-case `\u` escapes.
            0x00..=0x1F => Some(format!("\\u{c:04x}")),
            _ => None,
        };
        match escaped {
            Some(text) => out.extend(text.encode_utf16()),
            None => out.push(c),
        }
    }
}

/// Where `StringReference` points: into the character buffer, or into the string buffer (escaped strings).
#[derive(Debug, Clone, Copy)]
enum StringReference {
    Empty,
    Chars { start: usize, length: usize },
    Buffer { length: usize },
}

const LINE_FEED: u16 = 0x0A;
const CARRIAGE_RETURN: u16 = 0x0D;
const TAB: u16 = 0x09;
const UNICODE_REPLACEMENT_CHAR: u16 = 0xFFFD;
const MAXIMUM_JAVASCRIPT_INTEGER_CHARACTER_LENGTH: usize = 380;
const LARGE_BUFFER_LENGTH: usize = (i32::MAX / 2) as usize;

fn is_high_surrogate(c: u16) -> bool {
    (0xD800..=0xDBFF).contains(&c)
}

fn is_low_surrogate(c: u16) -> bool {
    (0xDC00..=0xDFFF).contains(&c)
}

/// A character as `string.Format` puts it in a message.
fn char_text(c: u16) -> String {
    String::from_utf16_lossy(&[c])
}

/// `JsonTextReader` reading a `StringReader` over `text`.
pub struct JsonTextReader {
    // The `StringReader`.
    text: Vec<u16>,
    text_pos: usize,
    // `JsonTextReader`.
    chars: Vec<u16>,
    chars_used: usize,
    char_pos: usize,
    line_start_pos: i64,
    line_number: i32,
    is_end_of_file: bool,
    string_buffer: Option<Vec<u16>>,
    string_reference: StringReference,
    // `JsonReader`.
    token: JsonToken,
    current_state: State,
    current_position: JsonPosition,
    stack: Vec<JsonPosition>,
    max_depth: i32,
    has_exceeded_max_depth: bool,
}

impl JsonTextReader {
    /// `new JsonTextReader(new StringReader(text))`.
    pub fn new(text: NetString) -> Self {
        Self {
            text: text.into_units(),
            text_pos: 0,
            chars: Vec::new(),
            chars_used: 0,
            char_pos: 0,
            line_start_pos: 0,
            line_number: 1,
            is_end_of_file: false,
            string_buffer: None,
            string_reference: StringReference::Empty,
            token: JsonToken::None,
            current_state: State::Start,
            current_position: JsonPosition::none(),
            stack: Vec::new(),
            max_depth: 64,
            has_exceeded_max_depth: false,
        }
    }

    /// The current token and its value.
    pub fn token(&self) -> &JsonToken {
        &self.token
    }

    /// `LineNumber`: 0 before the first token.
    pub fn line_number(&self) -> i32 {
        if self.current_state == State::Start
            && self.line_position() == 0
            && !matches!(self.token, JsonToken::Comment(_))
        {
            return 0;
        }
        self.line_number
    }

    /// `LinePosition`.
    pub fn line_position(&self) -> i32 {
        (self.char_pos as i64 - self.line_start_pos) as i32
    }

    /// `Depth`.
    pub fn depth(&self) -> i32 {
        let depth = self.stack.len() as i32;
        if self.token.is_start() || self.current_position.kind == ContainerType::None { depth } else { depth + 1 }
    }

    /// `Path`, as UTF-16.
    fn path_units(&self) -> Vec<u16> {
        let mut path = Vec::new();
        if self.current_position.kind == ContainerType::None {
            return path;
        }
        let inside_container =
            !matches!(self.current_state, State::ArrayStart | State::ConstructorStart | State::ObjectStart);
        for position in &self.stack {
            position.write_to(&mut path);
        }
        if inside_container {
            self.current_position.write_to(&mut path);
        }
        path
    }

    /// `Path`. A lone surrogate in a property name becomes U+FFFD here.
    pub fn path(&self) -> String {
        String::from_utf16_lossy(&self.path_units())
    }

    /// `JsonReaderException.Create(this, message)`: `JsonPosition.FormatMessage` adds the path and position.
    fn reader_error(&self, message: impl Into<String>) -> MonoException {
        let units: Vec<u16> = message.into().encode_utf16().collect();
        // `message.Trim()`, then a full stop if missing, then a space.
        let start = units.iter().position(|&c| !chars::is_white_space(c)).unwrap_or(units.len());
        let end = units.iter().rposition(|&c| !chars::is_white_space(c)).map_or(start, |i| i + 1);
        let mut message = String::from_utf16_lossy(&units[start..end.max(start)]);
        if !message.ends_with('.') {
            message.push('.');
        }
        message.push(' ');
        message.push_str(&format!(
            "Path '{}', line {}, position {}.",
            self.path(),
            self.line_number(),
            self.line_position()
        ));
        MonoException::new(pinewatch_mono::ExceptionKind::JsonReader, message)
    }

    fn unexpected_end(&self) -> MonoException {
        self.reader_error("Unexpected end when reading JSON.")
    }

    fn unexpected_character(&self, c: u16) -> MonoException {
        self.reader_error(format!("Unexpected character encountered while parsing value: {}.", char_text(c)))
    }

    // ---- JsonReader's state machine ----

    fn push(&mut self, kind: ContainerType) -> Result<(), MonoException> {
        self.update_scope_with_finished_value();
        if self.current_position.kind == ContainerType::None {
            self.current_position = JsonPosition::new(kind);
        } else {
            let previous = std::mem::replace(&mut self.current_position, JsonPosition::new(kind));
            self.stack.push(previous);
            // `Depth` counts the new container once its first value is read; testing here is the original's.
            if self.depth() + 1 > self.max_depth && !self.has_exceeded_max_depth {
                self.has_exceeded_max_depth = true;
                return Err(self.reader_error(format!(
                    "The reader's MaxDepth of {} has been exceeded.",
                    self.max_depth
                )));
            }
        }
        Ok(())
    }

    fn pop(&mut self) -> ContainerType {
        let old = match self.stack.pop() {
            Some(position) => std::mem::replace(&mut self.current_position, position),
            None => std::mem::replace(&mut self.current_position, JsonPosition::none()),
        };
        if self.depth() <= self.max_depth {
            self.has_exceeded_max_depth = false;
        }
        old.kind
    }

    fn peek(&self) -> ContainerType {
        self.current_position.kind
    }

    /// `SetToken(newToken, value, updateIndex)`.
    fn set_token(&mut self, token: JsonToken, update_index: bool) -> Result<(), MonoException> {
        self.token = token;
        match &self.token {
            JsonToken::StartObject => {
                self.current_state = State::ObjectStart;
                self.push(ContainerType::Object)?;
            }
            JsonToken::StartArray => {
                self.current_state = State::ArrayStart;
                self.push(ContainerType::Array)?;
            }
            JsonToken::StartConstructor(_) => {
                self.current_state = State::ConstructorStart;
                self.push(ContainerType::Constructor)?;
            }
            JsonToken::EndObject => self.validate_end("EndObject", ContainerType::Object)?,
            JsonToken::EndArray => self.validate_end("EndArray", ContainerType::Array)?,
            JsonToken::EndConstructor => self.validate_end("EndConstructor", ContainerType::Constructor)?,
            JsonToken::PropertyName(name) => {
                self.current_position.property_name = Some(name.clone());
                self.current_state = State::Property;
            }
            JsonToken::Undefined
            | JsonToken::Integer(_)
            | JsonToken::Float(_)
            | JsonToken::Boolean(_)
            | JsonToken::Null
            | JsonToken::Date(_)
            | JsonToken::String(_) => self.set_post_value_state(update_index),
            JsonToken::None | JsonToken::Comment(_) => {}
        }
        Ok(())
    }

    fn set_post_value_state(&mut self, update_index: bool) {
        if self.peek() != ContainerType::None {
            self.current_state = State::PostValue;
        } else {
            self.current_state = State::Finished;
        }
        if update_index {
            self.update_scope_with_finished_value();
        }
    }

    fn update_scope_with_finished_value(&mut self) {
        if self.current_position.has_index {
            self.current_position.position += 1;
        }
    }

    fn validate_end(&mut self, end_token: &str, closes: ContainerType) -> Result<(), MonoException> {
        let current = self.pop();
        if closes != current {
            return Err(self.reader_error(format!(
                "JsonToken {end_token} is not valid for closing JsonType {}.",
                current.name()
            )));
        }
        self.current_state = if self.peek() != ContainerType::None { State::PostValue } else { State::Finished };
        Ok(())
    }

    fn set_state_based_on_current(&mut self) {
        self.current_state = match self.peek() {
            ContainerType::Object => State::Object,
            ContainerType::Array => State::Array,
            ContainerType::Constructor => State::Constructor,
            ContainerType::None => State::Finished,
        };
    }

    // ---- The buffer ----

    fn ensure_buffer(&mut self) {
        if self.chars.is_empty() {
            self.chars = vec![0; 1024];
        }
    }

    fn block_copy_chars(&mut self, src_offset: usize, dst_offset: usize, count: usize) {
        self.chars.copy_within(src_offset..src_offset + count, dst_offset);
    }

    fn shift_buffer_if_needed(&mut self) {
        let length = self.chars.len();
        if (length - self.char_pos) as f64 <= length as f64 * 0.1 || length >= LARGE_BUFFER_LENGTH {
            let count = self.chars_used - self.char_pos;
            if count > 0 {
                self.block_copy_chars(self.char_pos, 0, count);
            }
            self.line_start_pos -= self.char_pos as i64;
            self.char_pos = 0;
            self.chars_used = count;
            self.chars[self.chars_used] = 0;
        }
    }

    fn prepare_buffer_for_read_data(&mut self, append: bool, chars_required: usize) {
        if self.chars_used + chars_required >= self.chars.len() - 1 {
            if append {
                let doubled = self.chars.len() * 2;
                let new_length = doubled.max(self.chars_used + chars_required + 1);
                let mut dst = vec![0; new_length];
                dst[..self.chars.len()].copy_from_slice(&self.chars);
                self.chars = dst;
            } else {
                let remaining = self.chars_used - self.char_pos;
                if remaining + chars_required + 1 >= self.chars.len() {
                    let mut dst = vec![0; remaining + chars_required + 1];
                    dst[..remaining].copy_from_slice(&self.chars[self.char_pos..self.char_pos + remaining]);
                    self.chars = dst;
                } else if remaining > 0 {
                    self.block_copy_chars(self.char_pos, 0, remaining);
                }
                self.line_start_pos -= self.char_pos as i64;
                self.char_pos = 0;
                self.chars_used = remaining;
            }
        }
    }

    /// `ReadData(append, charsRequired)`, reading from the `StringReader`.
    fn read_data(&mut self, append: bool, chars_required: usize) -> usize {
        if self.is_end_of_file {
            return 0;
        }
        self.prepare_buffer_for_read_data(append, chars_required);
        let attempt = self.chars.len() - self.chars_used - 1;
        let read = attempt.min(self.text.len() - self.text_pos);
        let source = &self.text[self.text_pos..self.text_pos + read];
        self.chars[self.chars_used..self.chars_used + read].copy_from_slice(source);
        self.text_pos += read;
        self.chars_used += read;
        if read == 0 {
            self.is_end_of_file = true;
        }
        self.chars[self.chars_used] = 0;
        read
    }

    fn ensure_chars(&mut self, relative_position: usize, append: bool) -> bool {
        if self.char_pos + relative_position >= self.chars_used {
            return self.read_chars(relative_position, append);
        }
        true
    }

    fn read_chars(&mut self, relative_position: usize, append: bool) -> bool {
        if self.is_end_of_file {
            return false;
        }
        let chars_required = self.char_pos + relative_position - self.chars_used + 1;
        let mut total = 0;
        while total < chars_required {
            let read = self.read_data(append, chars_required - total);
            if read == 0 {
                break;
            }
            total += read;
        }
        total >= chars_required
    }

    fn at(&self, offset: usize) -> u16 {
        self.chars[self.char_pos + offset]
    }

    fn reference_units(&self) -> &[u16] {
        match self.string_reference {
            StringReference::Empty => &[],
            StringReference::Chars { start, length } => &self.chars[start..start + length],
            StringReference::Buffer { length } => &self.string_buffer.as_ref().unwrap()[..length],
        }
    }

    fn reference_string(&self) -> NetString {
        NetString::from_units(self.reference_units().to_vec())
    }

    fn ensure_buffer_not_empty(&mut self) {
        self.string_buffer.get_or_insert_with(|| Vec::with_capacity(1024));
    }

    fn write_char_to_buffer(&mut self, write_char: u16, last_write_position: usize, write_to_position: usize) {
        let buffer = self.string_buffer.as_mut().unwrap();
        if write_to_position > last_write_position {
            buffer.extend_from_slice(&self.chars[last_write_position..write_to_position]);
        }
        buffer.push(write_char);
    }

    fn clear_recent_string(&mut self) {
        if let Some(buffer) = self.string_buffer.as_mut() {
            buffer.clear();
        }
        self.string_reference = StringReference::Empty;
    }

    // ---- Lines ----

    fn on_new_line(&mut self, pos: usize) {
        self.line_number += 1;
        self.line_start_pos = pos as i64;
    }

    fn set_new_line(&mut self, has_next_char: bool) {
        if has_next_char && self.chars[self.char_pos] == LINE_FEED {
            self.char_pos += 1;
        }
        self.on_new_line(self.char_pos);
    }

    fn process_line_feed(&mut self) {
        self.char_pos += 1;
        self.on_new_line(self.char_pos);
    }

    fn process_carriage_return(&mut self, append: bool) {
        self.char_pos += 1;
        let has_next = self.ensure_chars(1, append);
        self.set_new_line(has_next);
    }

    fn eat_whitespace(&mut self) {
        loop {
            let c = self.at(0);
            match c {
                0 => {
                    if self.chars_used == self.char_pos {
                        if self.read_data(false, 0) == 0 {
                            return;
                        }
                    } else {
                        self.char_pos += 1;
                    }
                }
                CARRIAGE_RETURN => self.process_carriage_return(false),
                LINE_FEED => self.process_line_feed(),
                _ if c == 0x20 || chars::is_white_space(c) => self.char_pos += 1,
                _ => return,
            }
        }
    }

    // ---- Read ----

    /// `Read()`: `true` with the next token, or `false` (token `None`) at the end.
    pub fn read(&mut self) -> Result<bool, MonoException> {
        self.ensure_buffer();
        loop {
            match self.current_state {
                State::Start
                | State::Property
                | State::Array
                | State::ArrayStart
                | State::Constructor
                | State::ConstructorStart => return self.parse_value(),
                State::Object | State::ObjectStart => return self.parse_object(),
                State::PostValue => {
                    if self.parse_post_value(false)? {
                        return Ok(true);
                    }
                }
                State::Finished => {
                    if self.ensure_chars(0, false) {
                        self.eat_whitespace();
                        if self.is_end_of_file {
                            self.set_token(JsonToken::None, true)?;
                            return Ok(false);
                        }
                        if self.at(0) == u16::from(b'/') {
                            self.parse_comment(true)?;
                            return Ok(true);
                        }
                        return Err(self.reader_error(format!(
                            "Additional text encountered after finished reading JSON content: {}.",
                            char_text(self.at(0))
                        )));
                    }
                    self.set_token(JsonToken::None, true)?;
                    return Ok(false);
                }
            }
        }
    }

    fn parse_post_value(&mut self, ignore_comments: bool) -> Result<bool, MonoException> {
        loop {
            let c = self.at(0);
            match c {
                0 => {
                    if self.chars_used == self.char_pos {
                        if self.read_data(false, 0) == 0 {
                            self.current_state = State::Finished;
                            return Ok(false);
                        }
                    } else {
                        self.char_pos += 1;
                    }
                }
                0x7D => {
                    self.char_pos += 1;
                    self.set_token(JsonToken::EndObject, true)?;
                    return Ok(true);
                }
                0x5D => {
                    self.char_pos += 1;
                    self.set_token(JsonToken::EndArray, true)?;
                    return Ok(true);
                }
                0x29 => {
                    self.char_pos += 1;
                    self.set_token(JsonToken::EndConstructor, true)?;
                    return Ok(true);
                }
                0x2F => {
                    self.parse_comment(!ignore_comments)?;
                    if !ignore_comments {
                        return Ok(true);
                    }
                }
                0x2C => {
                    self.char_pos += 1;
                    self.set_state_based_on_current();
                    return Ok(false);
                }
                0x20 | TAB => self.char_pos += 1,
                CARRIAGE_RETURN => self.process_carriage_return(false),
                LINE_FEED => self.process_line_feed(),
                _ if chars::is_white_space(c) => self.char_pos += 1,
                _ => {
                    return Err(self.reader_error(format!(
                        "After parsing a value an unexpected character was encountered: {}.",
                        char_text(c)
                    )));
                }
            }
        }
    }

    fn parse_object(&mut self) -> Result<bool, MonoException> {
        loop {
            let c = self.at(0);
            match c {
                0 => {
                    if self.chars_used == self.char_pos {
                        if self.read_data(false, 0) == 0 {
                            return Ok(false);
                        }
                    } else {
                        self.char_pos += 1;
                    }
                }
                0x7D => {
                    self.set_token(JsonToken::EndObject, true)?;
                    self.char_pos += 1;
                    return Ok(true);
                }
                0x2F => {
                    self.parse_comment(true)?;
                    return Ok(true);
                }
                CARRIAGE_RETURN => self.process_carriage_return(false),
                LINE_FEED => self.process_line_feed(),
                0x20 | TAB => self.char_pos += 1,
                _ if chars::is_white_space(c) => self.char_pos += 1,
                _ => return self.parse_property(),
            }
        }
    }

    fn valid_identifier_char(c: u16) -> bool {
        chars::is_letter_or_digit(c) || c == u16::from(b'_') || c == u16::from(b'$')
    }

    fn parse_property(&mut self) -> Result<bool, MonoException> {
        let first = self.at(0);
        if first == u16::from(b'"') || first == u16::from(b'\'') {
            self.char_pos += 1;
            self.shift_buffer_if_needed();
            self.read_string_into_buffer(first)?;
        } else if Self::valid_identifier_char(first) {
            self.shift_buffer_if_needed();
            self.parse_unquoted_property()?;
        } else {
            return Err(self.reader_error(format!("Invalid property identifier character: {}.", char_text(first))));
        }
        let name = self.reference_string();
        self.eat_whitespace();
        if self.at(0) != u16::from(b':') {
            return Err(self.reader_error(format!(
                "Invalid character after parsing property name. Expected ':' but got: {}.",
                char_text(self.at(0))
            )));
        }
        self.char_pos += 1;
        self.set_token(JsonToken::PropertyName(name), true)?;
        self.clear_recent_string();
        Ok(true)
    }

    fn parse_unquoted_property(&mut self) -> Result<(), MonoException> {
        let initial = self.char_pos;
        loop {
            let c = self.at(0);
            if c == 0 {
                if self.chars_used == self.char_pos {
                    if self.read_data(true, 0) == 0 {
                        return Err(self.reader_error("Unexpected end while parsing unquoted property name."));
                    }
                    continue;
                }
                self.string_reference = StringReference::Chars { start: initial, length: self.char_pos - initial };
                return Ok(());
            }
            if Self::valid_identifier_char(c) {
                self.char_pos += 1;
                continue;
            }
            if chars::is_white_space(c) || c == u16::from(b':') {
                self.string_reference = StringReference::Chars { start: initial, length: self.char_pos - initial };
                return Ok(());
            }
            let message = format!("Invalid JavaScript property identifier character: {}.", char_text(c));
            return Err(self.reader_error(message));
        }
    }

    fn parse_value(&mut self) -> Result<bool, MonoException> {
        loop {
            let c = self.at(0);
            match c {
                0 => {
                    if self.chars_used == self.char_pos {
                        if self.read_data(false, 0) == 0 {
                            return Ok(false);
                        }
                    } else {
                        self.char_pos += 1;
                    }
                }
                0x22 | 0x27 => {
                    self.parse_string(c)?;
                    return Ok(true);
                }
                0x74 => {
                    self.parse_literal("true", JsonToken::Boolean(true), "Error parsing boolean value.")?;
                    return Ok(true);
                }
                0x66 => {
                    self.parse_literal("false", JsonToken::Boolean(false), "Error parsing boolean value.")?;
                    return Ok(true);
                }
                0x6E => {
                    if self.ensure_chars(1, true) {
                        match self.at(1) {
                            0x75 => self.parse_literal("null", JsonToken::Null, "Error parsing null value.")?,
                            0x65 => self.parse_constructor()?,
                            _ => return Err(self.unexpected_character(self.at(0))),
                        }
                    } else {
                        self.char_pos += 1;
                        return Err(self.unexpected_end());
                    }
                    return Ok(true);
                }
                0x4E => {
                    self.parse_float_literal("NaN", DOUBLE_NAN)?;
                    return Ok(true);
                }
                0x49 => {
                    self.parse_float_literal("Infinity", f64::INFINITY)?;
                    return Ok(true);
                }
                0x2D => {
                    if self.ensure_chars(1, true) && self.at(1) == u16::from(b'I') {
                        self.parse_float_literal("-Infinity", f64::NEG_INFINITY)?;
                    } else {
                        self.parse_number()?;
                    }
                    return Ok(true);
                }
                0x2F => {
                    self.parse_comment(true)?;
                    return Ok(true);
                }
                0x75 => {
                    self.parse_literal("undefined", JsonToken::Undefined, "Error parsing undefined value.")?;
                    return Ok(true);
                }
                0x7B => {
                    self.char_pos += 1;
                    self.set_token(JsonToken::StartObject, true)?;
                    return Ok(true);
                }
                0x5B => {
                    self.char_pos += 1;
                    self.set_token(JsonToken::StartArray, true)?;
                    return Ok(true);
                }
                0x5D => {
                    self.char_pos += 1;
                    self.set_token(JsonToken::EndArray, true)?;
                    return Ok(true);
                }
                0x2C => {
                    // The comma stays for the next `Read`, so empty values between commas are `Undefined`.
                    self.set_token(JsonToken::Undefined, true)?;
                    return Ok(true);
                }
                0x29 => {
                    self.char_pos += 1;
                    self.set_token(JsonToken::EndConstructor, true)?;
                    return Ok(true);
                }
                CARRIAGE_RETURN => self.process_carriage_return(false),
                LINE_FEED => self.process_line_feed(),
                0x20 | TAB => self.char_pos += 1,
                _ if chars::is_white_space(c) => self.char_pos += 1,
                _ if chars::is_number(c) || c == u16::from(b'-') || c == u16::from(b'.') => {
                    self.parse_number()?;
                    return Ok(true);
                }
                _ => return Err(self.unexpected_character(c)),
            }
        }
    }

    // ---- Strings ----

    fn parse_string(&mut self, quote: u16) -> Result<(), MonoException> {
        self.char_pos += 1;
        self.shift_buffer_if_needed();
        self.read_string_into_buffer(quote)?;
        // `ParseReadString` for `ReadType.Read`.
        self.set_post_value_state(true);
        let date = match self.string_reference {
            StringReference::Empty => None,
            StringReference::Chars { start, length } => try_parse_date_time(&self.chars, start, length)?,
            StringReference::Buffer { length } => try_parse_date_time(self.string_buffer.as_ref().unwrap(), 0, length)?,
        };
        let token = match date {
            Some(date) => JsonToken::Date(date),
            None => JsonToken::String(self.reference_string()),
        };
        self.set_token(token, false)
    }

    fn unterminated(&self, quote: u16) -> MonoException {
        self.reader_error(format!("Unterminated string. Expected delimiter: {}.", char_text(quote)))
    }

    fn read_string_into_buffer(&mut self, quote: u16) -> Result<(), MonoException> {
        let mut char_pos = self.char_pos;
        let initial_position = self.char_pos;
        let mut last_write_position = self.char_pos;
        if let Some(buffer) = self.string_buffer.as_mut() {
            buffer.clear();
        }
        loop {
            let c = self.chars[char_pos];
            char_pos += 1;
            match c {
                0 => {
                    if self.chars_used == char_pos - 1 {
                        char_pos -= 1;
                        if self.read_data(true, 0) == 0 {
                            self.char_pos = char_pos;
                            return Err(self.unterminated(quote));
                        }
                    }
                }
                0x5C => {
                    self.char_pos = char_pos;
                    if !self.ensure_chars(0, true) {
                        return Err(self.unterminated(quote));
                    }
                    let escape_start_pos = char_pos - 1;
                    let current = self.chars[char_pos];
                    char_pos += 1;
                    let write_char = match current {
                        0x62 => 0x08,
                        0x74 => 0x09,
                        0x6E => 0x0A,
                        0x66 => 0x0C,
                        0x72 => 0x0D,
                        0x5C => 0x5C,
                        0x22 | 0x27 | 0x2F => current,
                        0x75 => {
                            self.char_pos = char_pos;
                            let mut write_char = self.parse_unicode()?;
                            if is_low_surrogate(write_char) {
                                write_char = UNICODE_REPLACEMENT_CHAR;
                            } else if is_high_surrogate(write_char) {
                                loop {
                                    let mut another_high_surrogate = false;
                                    if self.ensure_chars(2, true)
                                        && self.at(0) == u16::from(b'\\')
                                        && self.at(1) == u16::from(b'u')
                                    {
                                        let mut high_surrogate = write_char;
                                        self.char_pos += 2;
                                        write_char = self.parse_unicode()?;
                                        if is_low_surrogate(write_char) {
                                            // A valid pair.
                                        } else if is_high_surrogate(write_char) {
                                            high_surrogate = UNICODE_REPLACEMENT_CHAR;
                                            another_high_surrogate = true;
                                        } else {
                                            high_surrogate = UNICODE_REPLACEMENT_CHAR;
                                        }
                                        self.ensure_buffer_not_empty();
                                        let (last, escape) = (last_write_position, escape_start_pos);
                                        self.write_char_to_buffer(high_surrogate, last, escape);
                                        last_write_position = self.char_pos;
                                    } else {
                                        write_char = UNICODE_REPLACEMENT_CHAR;
                                    }
                                    if !another_high_surrogate {
                                        break;
                                    }
                                }
                            }
                            char_pos = self.char_pos;
                            write_char
                        }
                        _ => {
                            self.char_pos = char_pos;
                            let message = format!("Bad JSON escape sequence: \\{}.", char_text(current));
                            return Err(self.reader_error(message));
                        }
                    };
                    self.ensure_buffer_not_empty();
                    self.write_char_to_buffer(write_char, last_write_position, escape_start_pos);
                    last_write_position = char_pos;
                }
                CARRIAGE_RETURN => {
                    self.char_pos = char_pos - 1;
                    self.process_carriage_return(true);
                    char_pos = self.char_pos;
                }
                LINE_FEED => {
                    self.char_pos = char_pos - 1;
                    self.process_line_feed();
                    char_pos = self.char_pos;
                }
                0x22 | 0x27 if self.chars[char_pos - 1] == quote => {
                    self.finish_read_string_into_buffer(char_pos - 1, initial_position, last_write_position);
                    return Ok(());
                }
                _ => {}
            }
        }
    }

    fn finish_read_string_into_buffer(&mut self, char_pos: usize, initial_position: usize, last_write_position: usize) {
        if initial_position == last_write_position {
            let length = char_pos - initial_position;
            self.string_reference = StringReference::Chars { start: initial_position, length };
        } else {
            self.ensure_buffer_not_empty();
            if char_pos > last_write_position {
                let buffer = self.string_buffer.as_mut().unwrap();
                buffer.extend_from_slice(&self.chars[last_write_position..char_pos]);
            }
            let length = self.string_buffer.as_ref().unwrap().len();
            self.string_reference = StringReference::Buffer { length };
        }
        self.char_pos = char_pos + 1;
    }

    /// `ParseUnicode`: four hex digits, which need one more character after them to be available.
    fn parse_unicode(&mut self) -> Result<u16, MonoException> {
        if !self.ensure_chars(4, true) {
            return Err(self.reader_error("Unexpected end while parsing Unicode escape sequence."));
        }
        match try_hex_text_to_int(&self.chars, self.char_pos, self.char_pos + 4) {
            Some(value) => {
                self.char_pos += 4;
                Ok(value as u16)
            }
            None => {
                let text = String::from_utf16_lossy(&self.chars[self.char_pos..self.char_pos + 4]);
                Err(self.reader_error(format!("Invalid Unicode escape sequence: \\u{text}.")))
            }
        }
    }

    // ---- Literals ----

    /// `MatchValue(value)`.
    fn match_value(&mut self, value: &str) -> Result<bool, MonoException> {
        let units: Vec<u16> = value.encode_utf16().collect();
        if !self.ensure_chars(units.len() - 1, true) {
            self.char_pos = self.chars_used;
            return Err(self.unexpected_end());
        }
        for (i, &unit) in units.iter().enumerate() {
            if self.at(i) != unit {
                self.char_pos += i;
                return Ok(false);
            }
        }
        self.char_pos += units.len();
        Ok(true)
    }

    fn match_value_with_trailing_separator(&mut self, value: &str) -> Result<bool, MonoException> {
        if !self.match_value(value)? {
            return Ok(false);
        }
        if !self.ensure_chars(0, false) {
            return Ok(true);
        }
        Ok(self.is_separator(self.at(0)) || self.at(0) == 0)
    }

    fn is_separator(&mut self, c: u16) -> bool {
        match c {
            0x7D | 0x5D | 0x2C => true,
            0x2F => {
                if !self.ensure_chars(1, false) {
                    return false;
                }
                let next = self.at(1);
                next == u16::from(b'*') || next == u16::from(b'/')
            }
            0x29 => matches!(self.current_state, State::Constructor | State::ConstructorStart),
            0x20 | TAB | LINE_FEED | CARRIAGE_RETURN => true,
            _ => chars::is_white_space(c),
        }
    }

    /// `ParseTrue`, `ParseFalse`, `ParseNull`, `ParseUndefined`.
    fn parse_literal(&mut self, text: &str, token: JsonToken, error: &str) -> Result<(), MonoException> {
        if self.match_value_with_trailing_separator(text)? {
            self.set_token(token, true)
        } else {
            Err(self.reader_error(error))
        }
    }

    /// `ParseNumberNaN`, `ParseNumberPositiveInfinity`, `ParseNumberNegativeInfinity` for `ReadType.Read` with
    /// `FloatParseHandling.Double`.
    fn parse_float_literal(&mut self, text: &str, value: f64) -> Result<(), MonoException> {
        if self.match_value_with_trailing_separator(text)? {
            self.set_token(JsonToken::Float(value), true)
        } else {
            Err(self.reader_error(format!("Error parsing {text} value.")))
        }
    }

    // ---- Constructors ----

    fn parse_constructor(&mut self) -> Result<(), MonoException> {
        if !self.match_value_with_trailing_separator("new")? {
            return Err(self.reader_error("Unexpected content while parsing JSON."));
        }
        self.eat_whitespace();
        let initial = self.char_pos;
        let end;
        loop {
            let c = self.at(0);
            if c == 0 {
                if self.chars_used == self.char_pos {
                    if self.read_data(true, 0) == 0 {
                        return Err(self.reader_error("Unexpected end while parsing constructor."));
                    }
                } else {
                    end = self.char_pos;
                    self.char_pos += 1;
                    break;
                }
            } else if chars::is_letter_or_digit(c) {
                self.char_pos += 1;
            } else if c == CARRIAGE_RETURN {
                end = self.char_pos;
                self.process_carriage_return(true);
                break;
            } else if c == LINE_FEED {
                end = self.char_pos;
                self.process_line_feed();
                break;
            } else if chars::is_white_space(c) {
                end = self.char_pos;
                self.char_pos += 1;
                break;
            } else if c == u16::from(b'(') {
                end = self.char_pos;
                break;
            } else {
                return Err(self.reader_error(format!(
                    "Unexpected character while parsing constructor: {}.",
                    char_text(c)
                )));
            }
        }
        self.string_reference = StringReference::Chars { start: initial, length: end - initial };
        let name = self.reference_string();
        self.eat_whitespace();
        if self.at(0) != u16::from(b'(') {
            return Err(self.reader_error(format!(
                "Unexpected character while parsing constructor: {}.",
                char_text(self.at(0))
            )));
        }
        self.char_pos += 1;
        self.clear_recent_string();
        self.set_token(JsonToken::StartConstructor(name), true)
    }

    // ---- Comments ----

    fn parse_comment(&mut self, set_token: bool) -> Result<(), MonoException> {
        self.char_pos += 1;
        if !self.ensure_chars(1, false) {
            return Err(self.reader_error("Unexpected end while parsing comment."));
        }
        let single_line = match self.at(0) {
            0x2A => false,
            0x2F => true,
            other => {
                return Err(self.reader_error(format!(
                    "Error parsing comment. Expected: *, got {}.",
                    char_text(other)
                )));
            }
        };
        self.char_pos += 1;
        let initial = self.char_pos;
        loop {
            match self.at(0) {
                0 => {
                    if self.chars_used == self.char_pos {
                        if self.read_data(true, 0) == 0 {
                            if !single_line {
                                return Err(self.reader_error("Unexpected end while parsing comment."));
                            }
                            return self.end_comment(set_token, initial, self.char_pos);
                        }
                    } else {
                        self.char_pos += 1;
                    }
                }
                0x2A => {
                    self.char_pos += 1;
                    if !single_line && self.ensure_chars(0, true) && self.at(0) == u16::from(b'/') {
                        self.end_comment(set_token, initial, self.char_pos - 1)?;
                        self.char_pos += 1;
                        return Ok(());
                    }
                }
                CARRIAGE_RETURN => {
                    if single_line {
                        return self.end_comment(set_token, initial, self.char_pos);
                    }
                    self.process_carriage_return(true);
                }
                LINE_FEED => {
                    if single_line {
                        return self.end_comment(set_token, initial, self.char_pos);
                    }
                    self.process_line_feed();
                }
                _ => self.char_pos += 1,
            }
        }
    }

    fn end_comment(&mut self, set_token: bool, initial: usize, end: usize) -> Result<(), MonoException> {
        if set_token {
            let text = NetString::from_units(self.chars[initial..end].to_vec());
            self.set_token(JsonToken::Comment(text), true)?;
        }
        Ok(())
    }

    // ---- Numbers ----

    fn parse_number(&mut self) -> Result<(), MonoException> {
        self.shift_buffer_if_needed();
        let first = self.at(0);
        let initial = self.char_pos;
        self.read_number_into_buffer()?;
        self.parse_read_number(first, initial)
    }

    fn read_number_into_buffer(&mut self) -> Result<(), MonoException> {
        let mut char_pos = self.char_pos;
        loop {
            let c = self.chars[char_pos];
            if c == 0 {
                self.char_pos = char_pos;
                if self.chars_used == char_pos {
                    if self.read_data(true, 0) == 0 {
                        return Ok(());
                    }
                } else {
                    return Ok(());
                }
            } else if self.read_number_char_into_buffer(c, char_pos)? {
                return Ok(());
            } else {
                char_pos += 1;
            }
        }
    }

    fn read_number_char_into_buffer(&mut self, c: u16, char_pos: usize) -> Result<bool, MonoException> {
        match c {
            0x2D | 0x2B | 0x2E | 0x30..=0x39 | 0x41..=0x46 | 0x61..=0x66 | 0x58 | 0x78 => Ok(false),
            _ => {
                self.char_pos = char_pos;
                if chars::is_white_space(c) || matches!(c, 0x2C | 0x7D | 0x5D | 0x29 | 0x2F) {
                    return Ok(true);
                }
                Err(self.reader_error(format!(
                    "Unexpected character encountered while parsing number: {}.",
                    char_text(c)
                )))
            }
        }
    }

    /// `ThrowReaderError`: the token becomes `Undefined` first.
    fn throw_reader_error(&mut self, message: String) -> MonoException {
        self.token = JsonToken::Undefined;
        self.set_post_value_state(false);
        self.reader_error(message)
    }

    fn parse_read_number(&mut self, first: u16, initial: usize) -> Result<(), MonoException> {
        // The state moves on first, so the reader can continue after a bad number.
        self.set_post_value_state(true);
        let length = self.char_pos - initial;
        self.string_reference = StringReference::Chars { start: initial, length };
        let number: Vec<u16> = self.chars[initial..initial + length].to_vec();
        let number_text = String::from_utf16_lossy(&number);
        let single_digit = chars::is_digit(first) && length == 1;
        let non_base_10 = first == u16::from(b'0')
            && length > 1
            && !matches!(number[1], 0x2E | 0x65 | 0x45);

        let token = if single_digit {
            JsonToken::Integer(JsonInteger::I64(i64::from(first) - 48))
        } else if non_base_10 {
            let hex = number.len() >= 2 && number[0] == u16::from(b'0') && (number[1] | 0x20) == u16::from(b'x');
            match convert_to_int64(&number, if hex { 16 } else { 8 }) {
                Ok(value) => JsonToken::Integer(JsonInteger::I64(value)),
                Err(inner) => {
                    let error =
                        self.throw_reader_error(format!("Input string '{number_text}' is not a valid number."));
                    return Err(error.with_inner(inner));
                }
            }
        } else {
            match int64_try_parse(&number, 0, length) {
                (ParseResult::Success, value) => JsonToken::Integer(JsonInteger::I64(value)),
                (ParseResult::Overflow, _) => {
                    if length > MAXIMUM_JAVASCRIPT_INTEGER_CHARACTER_LENGTH {
                        let message = format!("JSON integer {number_text} is too large to parse.");
                        return Err(self.throw_reader_error(message));
                    }
                    JsonToken::Integer(JsonInteger::Big(big_integer_text(&number)))
                }
                (ParseResult::Invalid, _) => {
                    let text = NetString::from_units(number.clone());
                    let parsed = try_parse_f64(Some(&text), NumberStyles::FLOAT, &NumberFormat::invariant())?;
                    match parsed {
                        Some(value) => JsonToken::Float(value),
                        None => {
                            return Err(
                                self.throw_reader_error(format!("Input string '{number_text}' is not a valid number."))
                            );
                        }
                    }
                }
            }
        };
        self.clear_recent_string();
        self.set_token(token, false)
    }
}
