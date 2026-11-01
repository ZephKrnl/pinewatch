//! `Boolean.TryParse` (corefx): "True" or "False" in any case, also after trimming whitespace and NUL characters.

use pinewatch_mono::{NetString, try_parse_bool};

fn parse(s: &str) -> Option<bool> {
    try_parse_bool(Some(&NetString::from_str(s)))
}

#[test]
fn bool_parsing() {
    for text in ["True", "true", "TRUE"] {
        assert_eq!(parse(text), Some(true), "{text:?}");
    }
    for text in [" false ", "false\0\0", "\0\tFALSE\r\n"] {
        assert_eq!(parse(text), Some(false), "{text:?}");
    }
    for text in ["t", "1", "", "yes", "tru e"] {
        assert_eq!(parse(text), None, "{text:?}");
    }
    assert_eq!(try_parse_bool(None), None);
}
