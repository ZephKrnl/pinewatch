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

/// Bits computed outside this crate. A separate big-integer model of corert's `NumberToDouble` (mono/corert
/// `11136ad5`, `Number.CoreRT.cs`), with the power-of-ten tables read from that upstream file, gave the first
/// column; the second is the correctly rounded double, for comparison. Rows marked as quirks are where corert's
/// 18-digit cut, lossy 64-bit products or denormal truncation give another result, and Pinewatch must match corert.
/// The upstream tables equal the port's, and 54 of their 72 mantissas are not the nearest 64-bit value.
#[test]
fn corert_bits_from_an_independent_model() {
    let cases: [(&str, u64, u64); 27] = [
        ("-1.79769313486231571982965e-323", 0x8000000000000003, 0x8000000000000004), // corert quirk
        ("2.4703282292062328e-324", 0x0000000000000001, 0x0000000000000001),
        ("2.4703282292062327e-324", 0x0000000000000001, 0x0000000000000000), // corert quirk
        ("4.9406564584124654e-324", 0x0000000000000001, 0x0000000000000001),
        ("1.7976931348623157e308", 0x7FEFFFFFFFFFFFFF, 0x7FEFFFFFFFFFFFFF),
        ("1.7976931348623158e308", 0x7FEFFFFFFFFFFFFF, 0x7FEFFFFFFFFFFFFF),
        ("2.2250738585072011e-308", 0x000FFFFFFFFFFFFF, 0x000FFFFFFFFFFFFF),
        ("2.2250738585072014e-308", 0x0010000000000000, 0x0010000000000000),
        ("1e22", 0x4480F0CF064DD592, 0x4480F0CF064DD592),
        ("1e23", 0x44B52D02C7E14AF6, 0x44B52D02C7E14AF6),
        ("1e-300", 0x01A56E1FC2F8F359, 0x01A56E1FC2F8F359),
        ("1.52198088343072251e-199", 0x16A74CCE2EA40D9D, 0x16A74CCE2EA40D9C), // corert quirk
        ("1.698080779995082121e-181", 0x1A668C41E4492A0E, 0x1A668C41E4492A0D), // corert quirk
        ("2.5694660560054571941e-192", 0x18277231D0588683, 0x18277231D0588682), // corert quirk
        ("3.79327207306758575e-64", 0x32C3F95215690711, 0x32C3F95215690710), // corert quirk
        ("3.334813557226493413068374e243", 0x727F41EF50CE086D, 0x727F41EF50CE086E), // corert quirk
        ("5.3313689557645818e-310", 0x0000622450812653, 0x0000622450812653),
        ("5.915331269190496061e-311", 0x00000AE3A0550169, 0x00000AE3A0550169),
        ("1.337976705174450561e-318", 0x00000000000421D9, 0x00000000000421DA), // corert quirk
        ("8.59587355437390701e-314", 0x000000040D0419E5, 0x000000040D0419E6), // corert quirk
        ("4.27183398762473289151e-314", 0x00000002035BE378, 0x00000002035BE379), // corert quirk
        ("3.662499891092985481e-313", 0x00000011427B555D, 0x00000011427B555E), // corert quirk
        ("7.71606887260498752319e-319", 0x000000000002620E, 0x000000000002620F), // corert quirk
        ("3.1389778496102898599e-317", 0x000000000060F1D1, 0x000000000060F1D2), // corert quirk
        ("9.8391108377283657937030595786506971726e-320", 0x0000000000004DCA, 0x0000000000004DCB), // corert quirk
        ("9.621603689739286979538e-264", 0x095363E8270AF601, 0x095363E8270AF601),
        ("8.517072558626137644044124247e194", 0x686755B21D8A74B9, 0x686755B21D8A74B9),
    ];
    for (text, corert, correctly_rounded) in cases {
        let parsed = f64_any(text).map(f64::to_bits);
        assert_eq!(parsed, Some(corert), "{text} (correctly rounded: {correctly_rounded:#018x})");
    }
}
