//! `Local` (`Files/Local.cs`): reading an asset's translation with its English fallback (M2c task 8).

use pinewatch_assets::Local;
use pinewatch_dat::{DatParser, DatTree};
use pinewatch_mono::NetString;

fn tree(text: &str) -> DatTree {
    DatParser::new().parse_str(text)
}

fn text(value: Option<NetString>) -> Option<String> {
    value.map(|value| value.to_string_lossy())
}

const DATA: &str = "Name Gewehr\nEmpty \"\"\nFlag\nOnlyData Ja\nNested\n{\n}";
const FALLBACK: &str = "Name Rifle\nEmpty Blank\nFlag Fallback\nOnlyFallback Yes\nFallbackFlag";

#[test]
fn read_uses_only_the_language_data() {
    let (data, fallback) = (tree(DATA), tree(FALLBACK));
    let local = Local::new(Some(data.root()), Some(fallback.root()));
    assert_eq!(text(local.read("Name")).as_deref(), Some("Gewehr"));
    assert_eq!(text(local.read("Empty")).as_deref(), Some(""));
    // A flag's text is null, and so is a missing key: `GetString(key)` with a null default.
    assert_eq!(local.read("Flag"), None);
    assert_eq!(local.read("OnlyFallback"), None);
    assert_eq!(Local::new(None, Some(fallback.root())).read("Name"), None);
}

/// `TryReadString`: the language's text unless it's missing, null or empty, then the fallback's on the same terms.
#[test]
fn format_falls_back_past_empty_and_null_texts() {
    let (data, fallback) = (tree(DATA), tree(FALLBACK));
    let local = Local::new(Some(data.root()), Some(fallback.root()));
    assert_eq!(local.format("Name").to_string_lossy(), "Gewehr");
    assert_eq!(local.format("Empty").to_string_lossy(), "Blank");
    assert_eq!(local.format("Flag").to_string_lossy(), "Fallback");
    assert_eq!(local.format("OnlyData").to_string_lossy(), "Ja");
    assert_eq!(local.format("OnlyFallback").to_string_lossy(), "Yes");
    // Neither has a usable text: `format` returns the key, `FormatOrNull` null, `FormatOrEmpty` "".
    for key in ["Missing", "FallbackFlag", "Nested"] {
        assert_eq!(local.format(key).to_string_lossy(), key);
        assert_eq!(local.format_or_null(key), None);
        assert_eq!(local.format_or_empty(key).to_string_lossy(), "");
    }
    assert_eq!(text(local.format_or_null("Name")).as_deref(), Some("Gewehr"));
}

#[test]
fn without_data_everything_is_the_key() {
    let local = Local::new(None, None);
    assert_eq!(local.format("Name").to_string_lossy(), "Name");
    assert_eq!(local.read("Name"), None);
    assert!(!local.has("Name"));
}

/// `FormatEnglishOrEmpty` reads the language data when the language is English, else the fallback, and returns a
/// flag's null as it is.
#[test]
fn english_or_empty() {
    let (data, fallback) = (tree(DATA), tree(FALLBACK));
    let local = Local::new(Some(data.root()), Some(fallback.root()));
    assert_eq!(text(local.format_english_or_empty("Name", true)).as_deref(), Some("Gewehr"));
    assert_eq!(text(local.format_english_or_empty("Name", false)).as_deref(), Some("Rifle"));
    assert_eq!(local.format_english_or_empty("FallbackFlag", false), None);
    assert_eq!(text(local.format_english_or_empty("Missing", false)).as_deref(), Some(""));
    assert_eq!(text(Local::new(Some(data.root()), None).format_english_or_empty("Name", false)).as_deref(), Some(""));
}

/// `has` looks only at the language data, for any kind of node.
#[test]
fn has_checks_only_the_language_data() {
    let (data, fallback) = (tree(DATA), tree(FALLBACK));
    let local = Local::new(Some(data.root()), Some(fallback.root()));
    assert!(local.has("Name") && local.has("Flag") && local.has("Nested"));
    assert!(!local.has("OnlyFallback"));
}
