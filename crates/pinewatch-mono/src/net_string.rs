use std::fmt;

/// A .NET `string`: UTF-16 code units, which may include unpaired surrogates.
///
/// Lengths, indexes and comparisons all work on code units, exactly like C#.
#[derive(Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NetString(Vec<u16>);

impl NetString {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        Self(s.encode_utf16().collect())
    }

    pub fn from_units(units: Vec<u16>) -> Self {
        Self(units)
    }

    pub fn as_units(&self) -> &[u16] {
        &self.0
    }

    pub fn into_units(self) -> Vec<u16> {
        self.0
    }

    /// Length in UTF-16 code units (`string.Length`).
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Converts to a Rust string, replacing unpaired surrogates with U+FFFD. For display only.
    pub fn to_string_lossy(&self) -> String {
        String::from_utf16_lossy(&self.0)
    }
}

impl From<&str> for NetString {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl PartialEq<str> for NetString {
    fn eq(&self, other: &str) -> bool {
        self.0.iter().copied().eq(other.encode_utf16())
    }
}

impl PartialEq<&str> for NetString {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

impl fmt::Debug for NetString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.to_string_lossy(), f)
    }
}

impl fmt::Display for NetString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_lossy())
    }
}
