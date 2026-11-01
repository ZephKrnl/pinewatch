// Ported from corefx `src/Common/src/CoreLib/System/Boolean.cs` (mono/corefx 5b4edb9b).
// Copyright (c) .NET Foundation and Contributors. MIT license; see NOTICES-PORTED-CODE.md.

//! `Boolean.TryParse`: `True` or `False` in any case, first as given, then with whitespace and NUL characters
//! trimmed from both ends.

use crate::{NetString, chars};

const TRUE_LITERAL: &str = "True";
const FALSE_LITERAL: &str = "False";

fn equals_ignore_case(literal: &str, value: &[u16]) -> bool {
    let literal: Vec<u16> = literal.encode_utf16().collect();
    chars::equals_ordinal_ignore_case(&literal, value)
}

/// `TrimWhiteSpaceAndNull`.
fn trim_white_space_and_null(value: &[u16]) -> &[u16] {
    let is_trimmed = |c: u16| chars::is_white_space(c) || c == 0;
    let start = value.iter().position(|&c| !is_trimmed(c)).unwrap_or(value.len());
    let end = value.iter().rposition(|&c| !is_trimmed(c)).map_or(start, |i| i + 1);
    &value[start..end.max(start)]
}

/// `bool.TryParse(string value, out bool result)`. A missing string (C# `null`) fails.
pub fn try_parse_bool(value: Option<&NetString>) -> Option<bool> {
    let value = value?.as_units();
    if equals_ignore_case(TRUE_LITERAL, value) {
        return Some(true);
    }
    if equals_ignore_case(FALSE_LITERAL, value) {
        return Some(false);
    }
    let value = trim_white_space_and_null(value);
    if equals_ignore_case(TRUE_LITERAL, value) {
        return Some(true);
    }
    if equals_ignore_case(FALSE_LITERAL, value) {
        return Some(false);
    }
    None
}
