//! `DatTokenizer`, `DatToken` and `EDatTokenType` (UnturnedDat/DatTokenizer.cs).
//!
//! The line counting, the byte order mark check, the error texts and the token dump were checked against the IL of
//! the installed game's `UnturnedDat.dll`.

use pinewatch_mono::{NEW_LINE, NetString, chars};

const TAB: u16 = b'\t' as u16;
const LF: u16 = b'\n' as u16;
const CR: u16 = b'\r' as u16;
const SPACE: u16 = b' ' as u16;
const QUOTE: u16 = b'"' as u16;
const COMMA: u16 = b',' as u16;
const SLASH: u16 = b'/' as u16;
const BACKSLASH: u16 = b'\\' as u16;
const OPEN_BRACE: u16 = b'{' as u16;
const CLOSE_BRACE: u16 = b'}' as u16;
const OPEN_BRACKET: u16 = b'[' as u16;
const CLOSE_BRACKET: u16 = b']' as u16;
const LOWER_N: u16 = b'n' as u16;
const LOWER_T: u16 = b't' as u16;

/// `EDatTokenType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatTokenType {
    /// Not a valid token.
    Null,
    /// Dictionary key. Can span multiple lines.
    Key,
    /// String with `\n`, `\\` and `\t` escape sequences handled. Can span multiple lines.
    Value,
    /// Single line of comment text. Can be an inline comment following a quoted value.
    Comment,
    /// `\n`, `\r\n` or a lone `\r`.
    LineBreak,
    /// `{`
    OpenDictionary,
    /// `}`
    CloseDictionary,
    /// `[`
    OpenList,
    /// `]`
    CloseList,
}

impl DatTokenType {
    /// The member's name, which is what `ToString()` returns.
    pub fn name(self) -> &'static str {
        match self {
            Self::Null => "Null",
            Self::Key => "Key",
            Self::Value => "Value",
            Self::Comment => "Comment",
            Self::LineBreak => "LineBreak",
            Self::OpenDictionary => "OpenDictionary",
            Self::CloseDictionary => "CloseDictionary",
            Self::OpenList => "OpenList",
            Self::CloseList => "CloseList",
        }
    }
}

/// `DatToken`: a token's type and its text, if it has any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatToken {
    pub token_type: DatTokenType,
    pub value: Option<NetString>,
}

/// `EContext`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Context {
    Dictionary,
    List,
}

/// `DatTokenizer`.
///
/// Many modded "v2" assets had unnecessary commas that were read as dictionary keys or empty list entries, so one
/// comma right after `{`, `}`, `[`, `]` or a quoted string is skipped.
pub struct DatTokenizer {
    error_messages: Vec<NetString>,
    enable_comments: bool,
    /// How far `tokenize` has read its input: the state of the C#'s `inputReader`.
    input_position: usize,
    current_line_number: i32,
    current_char: u16,
    has_char: bool,
    tokens: Vec<DatToken>,
    context_stack: Vec<Context>,
    token_index: usize,
    string_builder: Vec<u16>,
}

impl Default for DatTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl DatTokenizer {
    /// `new DatTokenizer()`.
    pub fn new() -> Self {
        Self {
            error_messages: Vec::new(),
            enable_comments: false,
            input_position: 0,
            current_line_number: 0,
            current_char: 0,
            has_char: false,
            tokens: Vec::new(),
            context_stack: Vec::new(),
            token_index: 0,
            string_builder: Vec::new(),
        }
    }

    /// `HasError`.
    pub fn has_error(&self) -> bool {
        !self.error_messages.is_empty()
    }

    /// `EnableComments`: if true, comments are parsed into comment tokens. Otherwise, they are skipped.
    pub fn enable_comments(&self) -> bool {
        self.enable_comments
    }

    /// Sets `EnableComments`.
    pub fn set_enable_comments(&mut self, enable: bool) {
        self.enable_comments = enable;
    }

    /// `errorMessages`.
    pub fn error_messages(&self) -> &[NetString] {
        &self.error_messages
    }

    /// `Tokenize(TextReader)`, given the characters the reader returns.
    pub fn tokenize(&mut self, input: &[u16]) {
        self.input_position = 0;
        self.has_char = false; // reset hasChar before ReadChar because it uses previous char value
        self.current_line_number = 1;
        self.tokens.clear();
        self.error_messages.clear();
        self.context_stack.clear();
        self.token_index = 0;

        self.read_char(input);
        self.skip_utf8_bom(input);

        while self.has_char {
            let current_char = self.current_char;
            if current_char == SLASH {
                if self.enable_comments {
                    self.read_comment(input);
                } else {
                    self.skip_to_end_of_line(input);
                }
            } else if current_char == CR {
                self.read_char(input);
                if self.current_char == LF {
                    self.read_char(input);
                }
                self.push_token(DatTokenType::LineBreak, None);
            } else if current_char == LF {
                self.read_char(input);
                self.push_token(DatTokenType::LineBreak, None);
            } else if current_char == OPEN_BRACE {
                self.push_token(DatTokenType::OpenDictionary, None);
                self.context_stack.push(Context::Dictionary);
                self.read_char(input);
                self.skip_comma(input);
            } else if current_char == CLOSE_BRACE {
                // ReadChar after so PopContext has appropriate line number for error
                self.pop_context(Context::Dictionary);
                self.push_token(DatTokenType::CloseDictionary, None);
                self.read_char(input);
                self.skip_comma(input);
            } else if current_char == OPEN_BRACKET {
                self.push_token(DatTokenType::OpenList, None);
                self.context_stack.push(Context::List);
                self.read_char(input);
                self.skip_comma(input);
            } else if current_char == CLOSE_BRACKET {
                // ReadChar after so PopContext has appropriate line number for error
                self.pop_context(Context::List);
                self.push_token(DatTokenType::CloseList, None);
                self.read_char(input);
                self.skip_comma(input);
            } else if chars::is_white_space(current_char) {
                self.read_char(input);
            } else if self.context() == Context::Dictionary {
                self.read_dictionary_key(input);
                self.skip_spaces_and_tabs(input);
                if self.has_char && !chars::is_white_space(self.current_char) {
                    self.read_string_value(input);
                }
            } else {
                self.read_string_value(input);
            }
        }
    }

    /// `ReadToken`: the next token, or `None` after the last one.
    pub fn read_token(&mut self) -> Option<DatToken> {
        let token = self.tokens.get(self.token_index)?.clone();
        self.token_index += 1;
        Some(token)
    }

    /// `DebugDumpTokensToString()`: for each token, its index, its type, and its value unless that's null or
    /// empty, then `Environment.NewLine`.
    pub fn debug_dump_tokens_to_string(&self) -> NetString {
        let mut output: Vec<u16> = Vec::new();
        for (index, token) in self.tokens.iter().enumerate() {
            output.extend(index.to_string().encode_utf16());
            output.push(SPACE);
            output.extend(token.token_type.name().encode_utf16());
            if let Some(value) = token.value.as_ref().filter(|value| !value.is_empty()) {
                output.push(SPACE);
                output.extend_from_slice(value.as_units());
            }
            output.extend(NEW_LINE.encode_utf16());
        }
        NetString::from_units(output)
    }

    /// `ReadChar`: `TextReader.Read()` gives the next character, or -1 at the end of the input.
    fn read_char(&mut self, input: &[u16]) {
        let was_previous_char_carriage_return = self.has_char && self.current_char == CR;

        let read_result = input.get(self.input_position).copied();
        if read_result.is_some() {
            self.input_position += 1;
        }
        self.has_char = read_result.is_some();
        self.current_char = read_result.unwrap_or(0);

        let line_break = self.current_char == CR || (self.current_char == LF && !was_previous_char_carriage_return);
        if self.has_char && line_break {
            // C# `+=` on an `int` wraps.
            self.current_line_number = self.current_line_number.wrapping_add(1);
        }
    }

    /// The `if (hasChar && currentChar == ',') ReadChar();` that follows brackets and quoted strings.
    fn skip_comma(&mut self, input: &[u16]) {
        if self.has_char && self.current_char == COMMA {
            self.read_char(input);
        }
    }

    /// `SkipUtf8Bom`: skips the UTF-8 byte order mark, if any. The input is already characters, so this compares
    /// them with the mark's bytes, and each one is read before the next is compared.
    fn skip_utf8_bom(&mut self, input: &[u16]) {
        if !self.has_char || self.current_char != 0xEF {
            return;
        }

        self.read_char(input);
        if !self.has_char || self.current_char != 0xBB {
            return;
        }

        self.read_char(input);
        if !self.has_char || self.current_char != 0xBF {
            return;
        }

        self.read_char(input);
    }

    fn skip_spaces_and_tabs(&mut self, input: &[u16]) {
        while self.has_char && (self.current_char == SPACE || self.current_char == TAB) {
            self.read_char(input);
        }
    }

    fn skip_to_end_of_line(&mut self, input: &[u16]) {
        while self.has_char && self.current_char != CR && self.current_char != LF {
            self.read_char(input);
        }
    }

    fn read_comment(&mut self, input: &[u16]) {
        // currentChar is '/'

        // Skip // leading into comment text.
        loop {
            self.read_char(input);
            if !(self.has_char && self.current_char == SLASH) {
                break;
            }
        }

        // Skip optional first space leading into comment text.
        if self.has_char && self.current_char == SPACE {
            self.read_char(input);
        }

        self.string_builder.clear();
        while self.has_char && self.current_char != CR && self.current_char != LF {
            self.string_builder.push(self.current_char);
            self.read_char(input);
        }

        // Push a comment token in either case to preserve spacer comment lines.
        let value = if self.string_builder.is_empty() { None } else { Some(self.built_string()) };
        self.push_token(DatTokenType::Comment, value);
    }

    fn read_quoted_string(&mut self, input: &[u16], token_type: DatTokenType) {
        // currentChar is "

        let opening_line_number = self.current_line_number;
        self.read_char(input); // "

        let mut escape_next_char = false;
        let mut found_end = false;

        self.string_builder.clear();
        while self.has_char {
            if escape_next_char {
                if self.current_char == LOWER_N {
                    self.current_char = LF;
                } else if self.current_char == LOWER_T {
                    self.current_char = TAB;
                } else if self.current_char != BACKSLASH && self.current_char != QUOTE {
                    // Unrecognized escape sequence. Append the previously-ignored backslash. Since 3.23.7.0 (which
                    // broke mods using '\' in file paths) the invalid escape is kept as '\' and reported.
                    self.string_builder.push(BACKSLASH);
                    let message = unrecognized_escape_sequence(self.current_char, self.current_line_number);
                    self.error_messages.push(message);
                }
            } else if self.current_char == QUOTE {
                // end of quoted string
                self.read_char(input); // "
                found_end = true;
                break;
            } else if self.current_char == BACKSLASH {
                escape_next_char = true;
                self.read_char(input);
                continue;
            }

            escape_next_char = false;
            self.string_builder.push(self.current_char);
            self.read_char(input);
        }

        if !found_end {
            let message =
                format!("missing closing quotation mark (\") for string opened on line {opening_line_number}");
            self.error_messages.push(NetString::from(message.as_str()));
        }

        let value = self.built_string();
        self.push_token(token_type, Some(value));

        self.skip_comma(input);
    }

    fn read_dictionary_key(&mut self, input: &[u16]) {
        // currentChar is not whitespace

        if self.current_char == QUOTE {
            self.read_quoted_string(input, DatTokenType::Key);
        } else {
            self.string_builder.clear();
            loop {
                self.string_builder.push(self.current_char);
                self.read_char(input);
                if !(self.has_char && !chars::is_white_space(self.current_char)) {
                    break;
                }
            }
            let value = self.built_string();
            self.push_token(DatTokenType::Key, Some(value));
        }
    }

    fn read_string_value(&mut self, input: &[u16]) {
        // currentChar is not whitespace

        if self.current_char == QUOTE {
            self.read_quoted_string(input, DatTokenType::Value);
            return;
        }

        let mut escape_next_char = false;

        self.string_builder.clear();
        // `do { … } while (hasChar);`, where `continue` jumps to the condition.
        loop {
            if escape_next_char {
                if self.current_char == LOWER_N {
                    self.current_char = LF;
                } else if self.current_char == LOWER_T {
                    self.current_char = TAB;
                } else if self.current_char != BACKSLASH {
                    // Unrecognized escape sequence. Append the previously-ignored backslash. Since 3.23.7.0 (which
                    // broke mods using '\' in file paths) the invalid escape is kept as '\' and reported.
                    self.string_builder.push(BACKSLASH);
                    let message = unrecognized_escape_sequence(self.current_char, self.current_line_number);
                    self.error_messages.push(message);
                }
            } else if self.current_char == CR || self.current_char == LF {
                break;
            } else if self.current_char == BACKSLASH {
                escape_next_char = true;
                self.read_char(input);
                if self.has_char {
                    continue;
                }
                break;
            }

            escape_next_char = false;
            self.string_builder.push(self.current_char);
            self.read_char(input);
            if !self.has_char {
                break;
            }
        }

        let value = self.built_string();
        self.push_token(DatTokenType::Value, Some(value));
    }

    /// `GetContext`: the innermost open bracket, or a dictionary at the top level.
    fn context(&self) -> Context {
        self.context_stack.last().copied().unwrap_or(Context::Dictionary)
    }

    fn push_token(&mut self, token_type: DatTokenType, value: Option<NetString>) {
        self.tokens.push(DatToken { token_type, value });
    }

    fn pop_context(&mut self, expected_context: Context) {
        if self.context_stack.last() == Some(&expected_context) {
            self.context_stack.pop();
            return;
        }

        let line_number = self.current_line_number;
        let message = match expected_context {
            Context::Dictionary => format!("unexpected end of dictionary/object '}}' on line {line_number}"),
            Context::List => format!("unexpected end of list ']' on line {line_number}"),
        };
        self.error_messages.push(NetString::from(message.as_str()));
    }

    /// `stringBuilder.ToString()`.
    fn built_string(&self) -> NetString {
        NetString::from_units(self.string_builder.clone())
    }
}

/// `$"unrecognized escape sequence (\\{currentChar}) on line {currentLineNumber} — if this is a file path please use
/// forward slash (/)"`, with the character as it is, even a lone surrogate.
fn unrecognized_escape_sequence(escaped_char: u16, line_number: i32) -> NetString {
    let mut message: Vec<u16> = "unrecognized escape sequence (\\".encode_utf16().collect();
    message.push(escaped_char);
    message.extend(
        format!(") on line {line_number} \u{2014} if this is a file path please use forward slash (/)").encode_utf16(),
    );
    NetString::from_units(message)
}
