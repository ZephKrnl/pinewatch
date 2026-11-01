//! `DebugDumpToString` (UnturnedDat/DatNodeEx.cs) and the `DebugDumpToStringBuilder` methods of `DatDictionary`,
//! `DatList` and `DatValue`. Game code puts these dumps in log messages.

use pinewatch_mono::{NEW_LINE, NetString};

use crate::tree::{DictRef, ListRef, NodeRef, ValueRef};

const TAB: u16 = b'\t' as u16;
const QUOTE: u16 = b'"' as u16;

impl NodeRef<'_> {
    /// `DebugDumpToString()`.
    pub fn debug_dump_to_string(&self) -> NetString {
        let mut output = Vec::new();
        self.debug_dump_to_builder(&mut output, 0);
        NetString::from_units(output)
    }

    /// `DebugDumpToStringBuilder(output, indentationLevel)`.
    fn debug_dump_to_builder(&self, output: &mut Vec<u16>, indentation_level: usize) {
        match self {
            Self::Value(value) => dump_value(*value, output),
            Self::Dictionary(dictionary) => dump_dictionary(*dictionary, output, indentation_level),
            Self::List(list) => dump_list(*list, output, indentation_level),
        }
    }
}

/// `DatDictionary.DebugDumpToStringBuilder`. A parsed tree never holds a null node, so the C#'s `"null"` line
/// for one can't occur.
fn dump_dictionary(dictionary: DictRef<'_>, output: &mut Vec<u16>, indentation_level: usize) {
    append(output, "{");
    append(output, NEW_LINE);
    for (key, node) in dictionary.iter() {
        tabs(output, indentation_level + 1);
        output.push(QUOTE);
        output.extend_from_slice(key.as_units());
        output.push(QUOTE);
        append(output, " = ");
        node.debug_dump_to_builder(output, indentation_level + 1);
    }
    tabs(output, indentation_level);
    append(output, "}");
    append(output, NEW_LINE);
}

/// `DatList.DebugDumpToStringBuilder`.
fn dump_list(list: ListRef<'_>, output: &mut Vec<u16>, indentation_level: usize) {
    append(output, "[");
    append(output, NEW_LINE);
    for (index, item) in list.iter().enumerate() {
        tabs(output, indentation_level + 1);
        append(output, &index.to_string());
        append(output, " = ");
        item.debug_dump_to_builder(output, indentation_level + 1);
    }
    tabs(output, indentation_level);
    append(output, "]");
    append(output, NEW_LINE);
}

/// `DatValue.DebugDumpToStringBuilder`.
fn dump_value(value: ValueRef<'_>, output: &mut Vec<u16>) {
    match value.value() {
        Some(text) => {
            output.push(QUOTE);
            output.extend_from_slice(text.as_units());
            output.push(QUOTE);
        }
        None => append(output, "value(null)"),
    }
    append(output, NEW_LINE);
}

fn append(output: &mut Vec<u16>, text: &str) {
    output.extend(text.encode_utf16());
}

fn tabs(output: &mut Vec<u16>, count: usize) {
    output.extend(std::iter::repeat_n(TAB, count));
}
