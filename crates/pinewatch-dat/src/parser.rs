//! `DatParser` (UnturnedDat/DatParser.cs).

use pinewatch_mono::{NetString, decode_stream_reader_default};

use crate::tokenizer::{DatToken, DatTokenType, DatTokenizer};
use crate::tree::{DatTree, NodeId, NodeRef};

/// `DatParser`. One parser is reused for many files, so its fields carry over from one parse to the next exactly
/// as in the C#.
///
/// Parsing doesn't stop at an error. Lots of third-party assets have typos which technically work correctly if
/// ignored, and the old parser didn't log them.
pub struct DatParser {
    tokenizer: DatTokenizer,
    current_line_number: i32,
    current_token: DatToken,
    has_token: bool,
    error_messages: Vec<NetString>,
}

impl Default for DatParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DatParser {
    /// `new DatParser()`.
    pub fn new() -> Self {
        Self {
            tokenizer: DatTokenizer::new(),
            current_line_number: 0,
            current_token: DatToken { token_type: DatTokenType::Null, value: None },
            has_token: false,
            error_messages: Vec::new(),
        }
    }

    /// `Parse(TextReader)`, given the characters the reader returns.
    pub fn parse_text(&mut self, input: &NetString) -> DatTree {
        self.parse(input.as_units())
    }

    /// `Parse(string)`.
    pub fn parse_str(&mut self, input: &str) -> DatTree {
        let input: Vec<u16> = input.encode_utf16().collect();
        self.parse(&input)
    }

    /// `Parse(byte[])`: the bytes go through a `StreamReader` with its defaults (UTF-8, byte order mark detection).
    pub fn parse_bytes(&mut self, input: &[u8]) -> DatTree {
        let input = decode_stream_reader_default(input);
        self.parse(input.as_units())
    }

    /// `HasError`.
    pub fn has_error(&self) -> bool {
        !self.error_messages.is_empty()
    }

    /// `ErrorMessage`: the first error message.
    pub fn error_message(&self) -> Option<&NetString> {
        self.error_messages.first()
    }

    /// `ErrorMessages`.
    pub fn error_messages(&self) -> &[NetString] {
        &self.error_messages
    }

    fn parse(&mut self, input: &[u16]) -> DatTree {
        self.tokenizer.tokenize(input);
        self.error_messages.clear();
        self.has_token = false;
        self.current_line_number = 1;

        if self.tokenizer.has_error() {
            self.error_messages.extend_from_slice(self.tokenizer.error_messages());
        }

        self.read_token();

        let mut tree = DatTree::new();
        let root_dictionary = tree.root_id();

        while self.has_token {
            match self.current_token.token_type {
                DatTokenType::Key => {
                    let key = self.current_token.value.clone().unwrap_or_default();
                    let value = self.read_dictionary_value(&mut tree);
                    self.add_value_to_dictionary(&mut tree, root_dictionary, key, value);
                }
                DatTokenType::Comment => self.build_comment(),
                _ => self.read_token(),
            }
        }

        tree
    }

    /// `ReadToken`: after the last token, the current token is `new DatToken(EDatTokenType.Null)`.
    fn read_token(&mut self) {
        match self.tokenizer.read_token() {
            Some(token) => {
                self.has_token = true;
                self.current_token = token;
            }
            None => {
                self.has_token = false;
                self.current_token = DatToken { token_type: DatTokenType::Null, value: None };
            }
        }
        if self.current_token.token_type == DatTokenType::LineBreak {
            // C# `++` on an `int` wraps.
            self.current_line_number = self.current_line_number.wrapping_add(1);
        }
    }

    fn add_value_to_dictionary(&mut self, tree: &mut DatTree, dictionary: NodeId, key: NetString, value: NodeId) {
        if let Some(existing_value) = tree.dictionary_try_get_node(dictionary, key.as_units()) {
            let existing_string = node_string_for_error_message(tree.node(existing_value));
            let new_string = node_string_for_error_message(tree.node(value));
            let mut message = units("duplicate key \"");
            message.extend_from_slice(key.as_units());
            message.extend(units(&format!("\" on line {} replacing existing value ", self.current_line_number)));
            message.extend(existing_string);
            message.extend(units(" with "));
            message.extend(new_string);
            self.error_messages.push(NetString::from_units(message));
        }

        tree.dictionary_set(dictionary, key, value);
    }

    /// `BuildComment`.
    fn build_comment(&mut self) {
        self.read_token();
    }

    fn read_dictionary_value(&mut self, tree: &mut DatTree) -> NodeId {
        self.read_token(); // Advance past key

        // There is a weird case here: even if key is assigned a value, a list/dictionary start on the
        // next line takes precedence.
        let maybe_value_token = self.current_token.clone();
        if self.has_token && self.current_token.token_type == DatTokenType::Value {
            self.read_token(); // Advance past value to inline comment or line break.
        }

        if self.has_token && self.current_token.token_type == DatTokenType::Comment {
            self.read_token(); // Advance past inline comment to line break.
        }
        if self.has_token && self.current_token.token_type == DatTokenType::LineBreak {
            self.read_token(); // Advance past line break to next line's value.
        }

        if self.has_token {
            match self.current_token.token_type {
                DatTokenType::OpenDictionary => return self.read_dictionary(tree),
                DatTokenType::OpenList => return self.read_list(tree),
                _ => {}
            }
        }

        let value = if maybe_value_token.token_type == DatTokenType::Value { maybe_value_token.value } else { None };
        tree.add_value(value)
    }

    fn read_dictionary(&mut self, tree: &mut DatTree) -> NodeId {
        let opening_line_number = self.current_line_number;
        self.read_token(); // Advance past {

        let dictionary = tree.add_dictionary();

        let mut found_end = false;

        while self.has_token && !found_end {
            match self.current_token.token_type {
                DatTokenType::CloseDictionary => {
                    self.read_token(); // Advance past }
                    found_end = true;
                }
                DatTokenType::Key => {
                    let key = self.current_token.value.clone().unwrap_or_default();
                    let value = self.read_dictionary_value(tree);
                    self.add_value_to_dictionary(tree, dictionary, key, value);
                }
                DatTokenType::Comment => self.build_comment(),
                _ => self.read_token(),
            }
        }

        if !found_end {
            let message =
                format!("missing closing curly bracket '}}' for dictionary opened on line {opening_line_number}");
            self.error_messages.push(NetString::from(message.as_str()));
        }

        dictionary
    }

    fn read_list(&mut self, tree: &mut DatTree) -> NodeId {
        let opening_line_number = self.current_line_number;
        self.read_token(); // Advance past [

        let list = tree.add_list();

        let mut found_end = false;

        while self.has_token && !found_end {
            match self.current_token.token_type {
                DatTokenType::CloseList => {
                    self.read_token(); // Advance past ]
                    found_end = true;
                }
                DatTokenType::OpenDictionary => {
                    let dictionary = self.read_dictionary(tree);
                    tree.list_add(list, dictionary);
                }
                DatTokenType::OpenList => {
                    let child_list = self.read_list(tree);
                    tree.list_add(list, child_list);
                }
                DatTokenType::Value => {
                    let value = self.current_token.value.clone();
                    self.read_token();
                    let value_node = tree.add_value(value);
                    tree.list_add(list, value_node);
                }
                DatTokenType::Comment => self.build_comment(),
                _ => self.read_token(),
            }
        }

        if !found_end {
            let message = format!("missing closing bracket ']' for list opened on line {opening_line_number}");
            self.error_messages.push(NetString::from(message.as_str()));
        }

        list
    }
}

/// `GetNodeStringForErrorMessage`.
fn node_string_for_error_message(node: NodeRef<'_>) -> Vec<u16> {
    match node {
        NodeRef::List(list) => units(&format!("list with {} item(s)", list.len())),
        NodeRef::Dictionary(dictionary) => units(&format!("dictionary with {} item(s)", dictionary.len())),
        NodeRef::Value(value) => match value.value() {
            None => units("value(null)"),
            Some(text) => {
                let mut quoted = units("\"");
                quoted.extend_from_slice(text.as_units());
                quoted.extend(units("\""));
                quoted
            }
        },
    }
}

fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}
