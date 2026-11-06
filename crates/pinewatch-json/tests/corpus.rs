//! Every case of `tests/fixtures/reader-corpus.txt` against what the game's own `Newtonsoft.Json.dll` gives for it:
//! `tests/fixtures/reader-expected.txt`, written by `reference/checks/jsontokens.cs` on 2026-11-05 with copies of the
//! shipped `mscorlib.dll` and `Newtonsoft.Json.dll` (game 3.26.3.13). That program documents the format; a `Local`
//! date prints its UTC instant, which is what the port holds (docs/VERIFICATION.md, V31).

use std::fs;
use std::path::Path;

use pinewatch_json::{DateTimeKind, JsonInteger, JsonTextReader, JsonToken};
use pinewatch_mono::NetString;

/// The corpus line's text: `^n` LF, `^r` CR, `^t` tab, `^0` NUL, `^^` caret, `^uXXXX` any unit, `^E` the empty text.
fn decode(line: &str) -> Vec<u16> {
    if line == "^E" {
        return Vec::new();
    }
    let chars: Vec<char> = line.chars().collect();
    let mut units = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '^' {
            units.push(chars[i] as u16);
            i += 1;
            continue;
        }
        let code = chars[i + 1];
        i += 2;
        units.push(match code {
            'n' => 0x0A,
            'r' => 0x0D,
            't' => 0x09,
            '0' => 0x00,
            '^' => u16::from(b'^'),
            'u' => {
                let hex: String = chars[i..i + 4].iter().collect();
                i += 4;
                u16::from_str_radix(&hex, 16).unwrap()
            }
            other => panic!("bad code ^{other}"),
        });
    }
    units
}

fn escape(units: impl IntoIterator<Item = u16>) -> String {
    let mut out = String::new();
    for unit in units {
        match unit {
            0x5E => out.push_str("^^"),
            0x20..=0x7E => out.push(char::from(unit as u8)),
            _ => out.push_str(&format!("^u{unit:04X}")),
        }
    }
    out
}

fn text(value: &NetString) -> String {
    format!(" \"{}\"", escape(value.as_units().iter().copied()))
}

fn kind(kind: DateTimeKind) -> &'static str {
    if kind == DateTimeKind::Utc {
        "Utc"
    } else if kind == DateTimeKind::Local {
        "Local"
    } else {
        "Unspecified"
    }
}

fn value(token: &JsonToken) -> String {
    match token {
        JsonToken::PropertyName(v) | JsonToken::Comment(v) | JsonToken::String(v) | JsonToken::StartConstructor(v) => {
            text(v)
        }
        JsonToken::Integer(JsonInteger::I64(v)) => format!(" I64 {v}"),
        JsonToken::Integer(JsonInteger::Big(v)) => format!(" Big {v}"),
        JsonToken::Float(v) => format!(" F64 {:016x}", v.to_bits()),
        JsonToken::Boolean(v) => format!(" {v}"),
        JsonToken::Date(d) => format!(" Date {} {}", d.ticks, kind(d.kind)),
        _ => String::new(),
    }
}

fn run(units: Vec<u16>) -> String {
    let mut reader = JsonTextReader::new(NetString::from_units(units));
    let mut out = String::new();
    for i in 0.. {
        if i == 1000 {
            out.push_str("limit\n");
            break;
        }
        match reader.read() {
            Ok(more) => {
                let token = reader.token();
                out.push_str(&format!(
                    "{more} {}{} | depth {} | path \"{}\" | line {}:{}\n",
                    token.name(),
                    value(token),
                    reader.depth(),
                    escape(reader.path().encode_utf16()),
                    reader.line_number(),
                    reader.line_position()
                ));
                if !more {
                    break;
                }
            }
            Err(error) => {
                out.push_str(&format!(
                    "error {}: {}\n",
                    error.kind.type_name(),
                    escape(error.message.encode_utf16())
                ));
                break;
            }
        }
    }
    out
}

#[test]
fn every_case_matches_the_shipped_json_net() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let corpus = fs::read_to_string(fixtures.join("reader-corpus.txt")).unwrap();
    let expected = fs::read_to_string(fixtures.join("reader-expected.txt")).unwrap();

    let mut expected_cases: Vec<(String, String)> = Vec::new();
    for line in expected.split_inclusive('\n') {
        if line.starts_with("case ") {
            expected_cases.push((line.trim_end_matches('\n').to_string(), String::new()));
        } else {
            expected_cases.last_mut().unwrap().1.push_str(line);
        }
    }

    let mut failures = Vec::new();
    let mut count = 0;
    for raw in corpus.lines().filter(|line| !line.is_empty() && !line.starts_with('#')) {
        count += 1;
        let header = format!("case {count}: {raw}");
        let (expected_header, expected_body) = &expected_cases[count - 1];
        assert_eq!(expected_header, &header);
        let actual = run(decode(raw));
        if &actual != expected_body {
            failures.push(format!("{header}\n--- expected\n{expected_body}--- actual\n{actual}"));
        }
    }
    assert_eq!(count, expected_cases.len());
    let shown = failures[..failures.len().min(8)].join("\n");
    assert!(failures.is_empty(), "{} of {count} cases differ:\n{shown}", failures.len());
}
