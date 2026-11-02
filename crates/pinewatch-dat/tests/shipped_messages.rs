//! Every message text the `.dat` reader builds is found among the string literals of the shipped assembly that
//! builds it: `UnturnedDat.dll` for `DatTokenizer` and `DatParser`, `Assembly-CSharp.dll` for `Data.cs`. This
//! catches texts taken from the SDK source that differ from what the game ships, as `shipped_mscorlib` does for
//! `mscorlib.dll`.
//!
//! Each message is produced through the port, then split at its arguments into a template. The C# compiler stores
//! an interpolated string as one `string.Format` template (holes `{0}`, `{1}` …, braces doubled) when a hole isn't a
//! string, or as the literal pieces of a `string.Concat` when every hole is; either form is accepted. The message
//! half runs everywhere; the assembly half is skipped when no install is found. The install is only read.

use std::collections::HashSet;

use pinewatch_dat::{DatParser, LegacyData};
use pinewatch_io::find_install;
use pinewatch_mono::NetString;

/// A message as the port builds it, and the arguments it holds, in order. Each argument is found after the one
/// before it, so its value must not occur in the text before it.
struct Message {
    text: String,
    args: Vec<&'static str>,
}

impl Message {
    /// The literal text between the arguments.
    fn pieces(&self) -> Vec<String> {
        let mut pieces = Vec::new();
        let mut rest = self.text.as_str();
        for arg in &self.args {
            let at = rest.find(arg).unwrap_or_else(|| panic!("{arg:?} isn't in {:?}", self.text));
            pieces.push(rest[..at].to_string());
            rest = &rest[at + arg.len()..];
        }
        pieces.push(rest.to_string());
        pieces
    }

    /// The `string.Format` template the compiler would store.
    fn format_template(&self) -> String {
        let mut template = String::new();
        for (i, piece) in self.pieces().iter().enumerate() {
            if i > 0 {
                template.push_str(&format!("{{{}}}", i - 1));
            }
            template.push_str(&piece.replace('{', "{{").replace('}', "}}"));
        }
        template
    }
}

fn parser_messages(input: &str) -> Vec<String> {
    let mut parser = DatParser::new();
    parser.parse_str(input);
    parser.error_messages().iter().map(NetString::to_string_lossy).collect()
}

/// One message from `DatTokenizer` or `DatParser`, checked against the text the C# would build.
fn parser_message(input: &str, expected: &str, args: &[&'static str]) -> Message {
    let messages = parser_messages(input);
    assert!(messages.iter().any(|m| m == expected), "{input:?} gave {messages:?}, not {expected:?}");
    Message { text: expected.to_string(), args: args.to_vec() }
}

/// The `UnturnedDat.dll` messages: one case for each `PushErrorMessage` call, and the value descriptions the
/// duplicate-key message embeds.
fn unturned_dat_messages() -> Vec<Message> {
    vec![
        parser_message("k \"abc", "missing closing quotation mark (\") for string opened on line 1", &["1"]),
        parser_message(
            "k \"a\\Qb\"",
            "unrecognized escape sequence (\\Q) on line 1 \u{2014} if this is a file path please use forward slash (/)",
            &["Q", "1"],
        ),
        parser_message("k v\n}", "unexpected end of dictionary/object '}' on line 2", &["2"]),
        parser_message("k v\n\n]", "unexpected end of list ']' on line 3", &["3"]),
        parser_message("d\n{\nk v", "missing closing curly bracket '}' for dictionary opened on line 2", &["2"]),
        parser_message("l\n[\nv", "missing closing bracket ']' for list opened on line 2", &["2"]),
        parser_message(
            "Key \"Old\"\nKey \"New\"",
            "duplicate key \"Key\" on line 2 replacing existing value \"Old\" with \"New\"",
            &["Key", "2", "\"Old\"", "\"New\""],
        ),
        // The new value is stored once it is read, so the line is where the dictionary closes.
        parser_message(
            "Key\n[\nv\n]\nKey\n{\n}",
            concat!(
                "duplicate key \"Key\" on line 7 replacing existing value list with 1 item(s) with ",
                "dictionary with 0 item(s)"
            ),
            &["Key", "7", "list with 1 item(s)", "dictionary with 0 item(s)"],
        ),
        Message { text: "list with 1 item(s)".into(), args: vec!["1"] },
        Message { text: "dictionary with 0 item(s)".into(), args: vec!["0"] },
        // `$"\"{value.Value}\""`: one string hole, so `string.Concat` with the piece `"`.
        Message { text: "\"Old\"".into(), args: vec!["Old"] },
    ]
}

/// The `Assembly-CSharp.dll` message: `Data.cs`'s duplicate key.
fn assembly_csharp_messages() -> Vec<Message> {
    let data = LegacyData::new(&NetString::from("Kx Vold\nKx Vnew"));
    let errors: Vec<String> = data.errors().unwrap_or_default().iter().map(NetString::to_string_lossy).collect();
    let expected = "Duplicate key: \"Kx\" Old value: Vold New value: Vnew";
    assert_eq!(errors, [expected]);
    vec![Message { text: expected.into(), args: vec!["Kx", "Vold", "Vnew"] }]
}

#[test]
fn messages_split_into_the_expected_templates() {
    let messages = unturned_dat_messages();
    assert_eq!(messages[2].format_template(), "unexpected end of dictionary/object '}}' on line {0}");
    assert_eq!(
        messages[1].format_template(),
        "unrecognized escape sequence (\\{0}) on line {1} \u{2014} if this is a file path please use forward slash (/)"
    );
    assert_eq!(
        assembly_csharp_messages()[0].pieces(),
        ["Duplicate key: \"", "\" Old value: ", " New value: ", ""]
    );
}

fn u16_at(d: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([d[at], d[at + 1]]))
}

fn u32_at(d: &[u8], at: usize) -> usize {
    u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]]) as usize
}

/// The string literals of a .NET assembly: the entries of its `#US` metadata heap (ECMA-335 II.24.2.4). The same
/// reader as `pinewatch-mono`'s `shipped_mscorlib` test.
fn user_strings(d: &[u8]) -> HashSet<String> {
    let pe = u32_at(d, 0x3C);
    let section_count = u16_at(d, pe + 6);
    let optional_size = u16_at(d, pe + 20);
    let optional = pe + 24;
    let directories = optional + if u16_at(d, optional) == 0x10B { 96 } else { 112 };
    let section_table = optional + optional_size;
    let offset = |rva: usize| -> usize {
        for i in 0..section_count {
            let s = section_table + i * 40;
            let (virtual_size, va, raw_size, raw) = (u32_at(d, s + 8), u32_at(d, s + 12), u32_at(d, s + 16), u32_at(d, s + 20));
            if va <= rva && rva < va + virtual_size.max(raw_size) {
                return rva - va + raw;
            }
        }
        panic!("RVA {rva:#x} is outside every section");
    };
    let cli = offset(u32_at(d, directories + 14 * 8));
    let metadata = offset(u32_at(d, cli + 8));
    let mut p = metadata + 16 + u32_at(d, metadata + 12);
    let stream_count = u16_at(d, p + 2);
    p += 4;
    for _ in 0..stream_count {
        let (start, size) = (u32_at(d, p), u32_at(d, p + 4));
        p += 8;
        let name_end = p + d[p..].iter().position(|&b| b == 0).expect("stream name");
        let name = &d[p..name_end];
        p = (name_end + 4) & !3;
        if name == b"#US" {
            return heap_entries(&d[metadata + start..metadata + start + size]);
        }
    }
    panic!("no #US heap");
}

fn heap_entries(heap: &[u8]) -> HashSet<String> {
    let mut strings = HashSet::new();
    let mut i = 1; // Entry 0 is the empty string.
    while i < heap.len() {
        let b = usize::from(heap[i]);
        let (length, header) = if b & 0x80 == 0 {
            (b, 1)
        } else if b & 0xC0 == 0x80 {
            (((b & 0x3F) << 8) | usize::from(heap[i + 1]), 2)
        } else {
            let rest = (usize::from(heap[i + 1]) << 16) | (usize::from(heap[i + 2]) << 8) | usize::from(heap[i + 3]);
            (((b & 0x1F) << 24) | rest, 4)
        };
        let start = i + header;
        if length > 0 {
            // UTF-16 code units, then one flag byte.
            let units: Vec<u16> =
                heap[start..start + length - 1].as_chunks::<2>().0.iter().map(|&c| u16::from_le_bytes(c)).collect();
            strings.insert(String::from_utf16_lossy(&units));
        }
        i = start + length;
    }
    strings
}

/// The messages whose text isn't in `literals`, either as a `string.Format` template or as `string.Concat`
/// pieces (empty pieces aren't stored).
fn missing(messages: &[Message], literals: &HashSet<String>) -> Vec<String> {
    messages
        .iter()
        .filter(|message| {
            let template = message.format_template();
            let pieces = message.pieces();
            let concat = pieces.iter().filter(|piece| !piece.is_empty()).all(|piece| literals.contains(piece));
            !literals.contains(&template) && !concat
        })
        .map(Message::format_template)
        .collect()
}

/// The matching on made-up literal sets: a message is found as its template or as all its pieces, not otherwise.
#[test]
fn messages_are_found_as_a_template_or_as_pieces() {
    let messages = unturned_dat_messages();
    let templates: HashSet<String> = messages.iter().map(Message::format_template).collect();
    assert!(missing(&messages, &templates).is_empty());
    let pieces: HashSet<String> = messages.iter().flat_map(Message::pieces).collect();
    assert!(missing(&messages, &pieces).is_empty());
    assert_eq!(missing(&messages, &HashSet::new()).len(), messages.len());

    let mut almost = templates.clone();
    almost.remove("missing closing bracket ']' for list opened on line {0}");
    almost.insert("missing closing bracket ']' for list opened at line {0}".into());
    assert_eq!(missing(&messages, &almost), ["missing closing bracket ']' for list opened on line {0}"]);
}

#[test]
fn every_message_text_is_a_literal_in_the_shipped_assemblies() {
    let Ok(install) = find_install(None) else {
        eprintln!("skipped: no Unturned install found");
        return;
    };
    for (dll, messages) in [
        ("Unturned_Data/Managed/UnturnedDat.dll", unturned_dat_messages()),
        ("Unturned_Data/Managed/Assembly-CSharp.dll", assembly_csharp_messages()),
    ] {
        let literals = user_strings(&install.read(dll).unwrap_or_else(|error| panic!("{dll}: {error}")));
        assert!(!literals.is_empty(), "{dll}: no string literals found; the heap reader is broken");
        let missing = missing(&messages, &literals);
        assert!(missing.is_empty(), "not string literals in {dll}: {missing:#?}");
    }
}
