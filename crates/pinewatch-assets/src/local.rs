//! `Local` (`Files/Local.cs`): an asset's translation, with the English file as fallback when the player's language
//! isn't English.
//!
//! The `format` overloads with arguments go through `string.Format` with the player's culture, which comes with
//! composite formatting; nothing in the asset loading ported so far calls them.

use pinewatch_dat::DictRef;
use pinewatch_mono::NetString;

/// `Local`: the translation data and its fallback, either of which can be missing.
#[derive(Clone, Copy)]
pub struct Local<'t> {
    data: Option<DictRef<'t>>,
    fallback_data: Option<DictRef<'t>>,
}

impl<'t> Local<'t> {
    /// `new Local(data, fallbackData)`; `new Local()` is `Local::new(None, None)`.
    pub fn new(data: Option<DictRef<'t>>, fallback_data: Option<DictRef<'t>>) -> Self {
        Self { data, fallback_data }
    }

    /// `TryReadString`: the language's text if it exists and isn't null or empty, else the fallback's on the same
    /// terms.
    fn try_read_string(&self, key: &str) -> Option<&'t NetString> {
        let usable = |dictionary: &Option<DictRef<'t>>| -> Option<&'t NetString> {
            dictionary.as_ref()?.try_get_string(key)?.filter(|text| !text.is_empty())
        };
        usable(&self.data).or_else(|| usable(&self.fallback_data))
    }

    /// `read(key)`: the language data's `GetString(key)`, which is null for a flag or a missing key. The fallback
    /// isn't read.
    pub fn read(&self, key: &str) -> Option<NetString> {
        self.data.as_ref()?.get_string(key, None).cloned()
    }

    /// `format(key)`: the text, or the key itself.
    pub fn format(&self, key: &str) -> NetString {
        self.try_read_string(key).cloned().unwrap_or_else(|| NetString::from(key))
    }

    /// `FormatOrNull(key)`.
    pub fn format_or_null(&self, key: &str) -> Option<NetString> {
        self.try_read_string(key).cloned()
    }

    /// `FormatOrEmpty(key)`.
    pub fn format_or_empty(&self, key: &str) -> NetString {
        self.try_read_string(key).cloned().unwrap_or_else(|| NetString::from(""))
    }

    /// `FormatEnglishOrEmpty(key)`: the English text, from the language data when the language is English
    /// (`Provider.languageIsEnglish`) and from the fallback otherwise. A flag's null is returned as it is; a
    /// missing key gives "".
    pub fn format_english_or_empty(&self, key: &str, language_is_english: bool) -> Option<NetString> {
        let english = if language_is_english { self.data } else { self.fallback_data };
        match english.and_then(|dictionary| dictionary.try_get_string(key)) {
            Some(text) => text.cloned(),
            None => Some(NetString::from("")),
        }
    }

    /// `has(key)`: whether the language data has the key, whatever it holds.
    pub fn has(&self, key: &str) -> bool {
        self.data.as_ref().is_some_and(|dictionary| dictionary.contains_key(key))
    }
}
