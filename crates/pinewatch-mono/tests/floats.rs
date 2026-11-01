//! `float` and `double` parsing as Unity's Mono compiles it: corefx `Single.TryParse` and `Double.TryParse` over corert's
//! `TryParseSingle`/`TryParseDouble` and the portable `NumberToDouble` (`docs/VERIFICATION.md`, V16).

use pinewatch_mono::{ExceptionKind, NetString, NumberFormat, NumberStyles, sr, try_parse_f32, try_parse_f64};

fn f32_any(s: &str) -> Option<f32> {
    try_parse_f32(Some(&NetString::from_str(s)), NumberStyles::ANY, &NumberFormat::invariant()).expect("valid style")
}

fn f64_any(s: &str) -> Option<f64> {
    try_parse_f64(Some(&NetString::from_str(s)), NumberStyles::ANY, &NumberFormat::invariant()).expect("valid style")
}

#[test]
fn exact_values() {
    for (text, expected) in [("0.5", 0.5), ("1e3", 1000.0), ("1,000.25", 1000.25), ("(2)", -2.0), (" 3 ", 3.0)] {
        assert_eq!(f64_any(text), Some(expected), "{text}");
        assert_eq!(f32_any(text), Some(expected as f32), "{text}");
    }
}

#[test]
fn negative_zero_is_normalized() {
    for text in ["-0", "-0.0"] {
        assert_eq!(f64_any(text).map(f64::to_bits), Some(0), "{text}");
        assert_eq!(f32_any(text).map(f32::to_bits), Some(0), "{text}");
    }
}

#[test]
fn overflow_fails_and_symbols_parse() {
    assert_eq!(f64_any("1e400"), None);
    assert_eq!(f32_any("3.5e38"), None);
    assert_eq!(f64_any("Infinity"), Some(f64::INFINITY));
    assert_eq!(f32_any("Infinity"), Some(f32::INFINITY));
    assert_eq!(f64_any("-Infinity"), Some(f64::NEG_INFINITY));
    assert_eq!(f32_any("-Infinity"), Some(f32::NEG_INFINITY));
    for text in ["NaN", " NaN "] {
        assert!(f64_any(text).is_some_and(f64::is_nan), "{text}");
        assert!(f32_any(text).is_some_and(f32::is_nan), "{text}");
    }
}

#[test]
fn nan_has_the_bits_of_the_csharp_constants() {
    // `Double.NaN` and `Single.NaN` are 0.0 / 0.0 folded by the compiler: negative quiet NaNs (shipped IL).
    assert_eq!(f64_any("NaN").map(f64::to_bits), Some(0xFFF8_0000_0000_0000));
    assert_eq!(f32_any("NaN").map(f32::to_bits), Some(0xFFC0_0000));
}

#[test]
fn single_is_the_cast_of_double() {
    let inputs = [
        "0.1",
        "16777217",
        "3.4028235e38",
        "1.17549435e-38",
        "1e-46",
        "0.000000000000000000000000000000000000000000000700649232162408535461864791644958065640130970938257885878534141944895541342930300743319094181060791015625",
    ];
    for text in inputs {
        let single = f32_any(text).map(f32::to_bits);
        let cast = f64_any(text).map(|d| (d as f32).to_bits());
        assert_eq!(single, cast, "{text}");
    }
}

#[test]
fn hex_style_is_rejected() {
    let error = try_parse_f64(Some(&NetString::from_str("1")), NumberStyles::HEX_NUMBER, &NumberFormat::invariant())
        .unwrap_err();
    assert_eq!((error.kind, error.message.as_str()), (ExceptionKind::Argument, sr::ARG_HEX_STYLE_NOT_SUPPORTED));
    let error = try_parse_f32(None, NumberStyles::HEX_NUMBER, &NumberFormat::invariant()).unwrap_err();
    assert_eq!(error.kind, ExceptionKind::Argument);
}
