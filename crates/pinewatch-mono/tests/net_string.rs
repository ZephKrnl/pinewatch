use pinewatch_mono::NetString;

#[test]
fn surrogate_pair_counts_as_two_units() {
    let s = NetString::from_str("a\u{1F600}");
    assert_eq!(s.len(), 3);
    assert_eq!(s.as_units(), &[0x61, 0xD83D, 0xDE00]);
}

#[test]
fn lone_surrogate_survives_round_trip() {
    let s = NetString::from_units(vec![0x41, 0xD800, 0x42]);
    assert_eq!(s.as_units(), &[0x41, 0xD800, 0x42]);
    assert_eq!(s.to_string_lossy(), "A\u{FFFD}B");
}

#[test]
// `== ""` tests `PartialEq<&str>` with the empty string on purpose.
#[allow(clippy::comparison_to_empty)]
fn compares_exactly_with_str() {
    assert!(NetString::from_str("Coord") == "Coord");
    assert!(NetString::from_str("Coord") != "coord");
    assert!(NetString::from_str("") == "");
    assert!(NetString::from_str("").is_empty());
}
