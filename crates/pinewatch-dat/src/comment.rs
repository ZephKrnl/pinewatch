//! `DatComment` (UnturnedDat/DatComment.cs): the comment lines right before a node.

use pinewatch_mono::NetString;

/// `DatComment`: a prefix comment, as the parser collects it in metadata mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatComment {
    /// `MessageLines`: one entry per comment line, without the `//`. An empty comment line is null.
    pub message_lines: Vec<Option<NetString>>,
    /// `StartingLineNumber`: the parser's line number at the first comment line.
    pub starting_line_number: i32,
    /// `EndingLineNumber`: the parser's line number at the last comment line.
    pub ending_line_number: i32,
}

impl DatComment {
    /// `JoinLines(separator)`: null without lines, the line itself when there is one, otherwise `string.Join`,
    /// which joins null lines as empty strings.
    pub fn join_lines(&self, separator: &str) -> Option<NetString> {
        match self.message_lines.as_slice() {
            [] => None,
            [line] => line.clone(),
            lines => {
                let separator: Vec<u16> = separator.encode_utf16().collect();
                let mut joined = Vec::new();
                for (index, line) in lines.iter().enumerate() {
                    if index > 0 {
                        joined.extend_from_slice(&separator);
                    }
                    if let Some(line) = line {
                        joined.extend_from_slice(line.as_units());
                    }
                }
                Some(NetString::from_units(joined))
            }
        }
    }

    /// `MessageWithLineBreaks`: the lines joined with `\n`.
    pub fn message_with_line_breaks(&self) -> Option<NetString> {
        self.join_lines("\n")
    }
}
