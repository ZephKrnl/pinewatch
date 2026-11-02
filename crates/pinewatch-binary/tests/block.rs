//! `Block` (`Files/Block.cs`): every read checks the bounds and gives zero, empty or false past the end, except
//! the two that throw.

use pinewatch_binary::Block;
use pinewatch_mono::{ExceptionKind, Guid, NetString};
use pinewatch_unity::Vector3;
use sha1::{Digest, Sha1};

fn block(bytes: &[u8]) -> Block {
    Block::new(0, bytes.to_vec())
}

fn text(value: &NetString) -> String {
    value.to_string_lossy()
}

#[test]
fn integers_are_little_endian() {
    let mut b = block(&[0x34, 0x12, 0x78, 0x56, 0x34, 0x12, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0x80, 0x3F]);
    assert_eq!(b.read_u16(), 0x1234);
    assert_eq!(b.read_i32(), 0x1234_5678);
    assert_eq!(b.read_i32(), -1);
    assert_eq!(b.read_f32(), 1.0);
    let mut wide = block(&[1, 0, 0, 0, 0, 0, 0, 0x80, 0xFE, 0xFF]);
    assert_eq!(wide.read_i64(), i64::MIN + 1);
    assert_eq!(wide.read_i16(), -2);
    let mut unsigned = block(&[0xFF; 12]);
    assert_eq!((unsigned.read_u32(), unsigned.read_u64()), (u32::MAX, u64::MAX));
}

#[test]
fn short_reads_give_zero_and_keep_the_step() {
    let mut b = block(&[1, 2, 3]);
    assert_eq!(b.read_i32(), 0);
    assert_eq!(b.step, 0);
    assert_eq!(b.read_u16(), 0x0201);
    assert_eq!(b.read_u16(), 0);
    assert_eq!(b.step, 2);
    assert_eq!(b.read_byte(), 3);
    assert_eq!(b.read_byte(), 0);
    assert!(!b.read_boolean());
    assert_eq!((b.read_u64(), b.read_f32(), b.step), (0, 0.0, 3));
}

#[test]
fn the_prefix_is_the_first_step() {
    assert_eq!(Block::new(2, vec![9, 9, 7]).read_byte(), 7);
}

/// A byte length, then that many bytes of UTF-8 (`Encoding.UTF8.GetString`); a length past the end gives `""` and
/// still moves the step.
#[test]
fn strings_have_a_one_byte_length() {
    let mut b = block(&[3, b'a', b'b', b'c', 5, b'x']);
    assert_eq!(text(&b.read_string()), "abc");
    assert_eq!(text(&b.read_string()), "");
    assert_eq!(b.step, 10);
    assert_eq!(text(&b.read_string()), "");
    assert_eq!(b.step, 10);
    assert_eq!(block(&[2, 0xC3, 0x28]).read_string().as_units(), "\u{FFFD}(".encode_utf16().collect::<Vec<_>>());
    // Ending exactly at the last byte still reads.
    assert_eq!(text(&block(&[2, b'h', b'i']).read_string()), "hi");
}

#[test]
fn string_arrays() {
    let strings: Vec<String> = block(&[2, 1, b'a', 0]).read_string_array().iter().map(text).collect();
    assert_eq!(strings, ["a", ""]);
    assert!(block(&[]).read_string_array().is_empty());
}

/// `BitConverter.ToBoolean`: any non-zero byte is true.
#[test]
fn booleans() {
    let mut b = block(&[0, 2]);
    assert_eq!((b.read_boolean(), b.read_boolean()), (false, true));
}

/// A `ushort` count, then the values eight to a byte, low bit first. The bytes aren't bounds-checked, so a count
/// past the end throws.
#[test]
fn boolean_arrays_are_bit_packed_low_bit_first() {
    let values = block(&[10, 0, 0b0000_0101, 0b0000_0010]).read_boolean_array().unwrap();
    let set: Vec<usize> = values.iter().enumerate().filter_map(|(index, value)| value.then_some(index)).collect();
    assert_eq!((values.len(), set), (10, vec![0, 2, 9]));
    let error = block(&[9, 0, 0xFF]).read_boolean_array().unwrap_err();
    assert_eq!(error.kind, ExceptionKind::IndexOutOfRange);
    assert_eq!(error.message, "Index was outside the bounds of the array.");
    assert!(block(&[]).read_boolean_array().unwrap().is_empty());
}

/// A byte count (an `int` one with `longBinaryData`), then the bytes; the copy is skipped when they run past the
/// end, leaving zeros, and the step moves anyway.
#[test]
fn byte_arrays() {
    assert_eq!(block(&[2, 7, 8]).read_byte_array().unwrap(), [7, 8]);
    let mut short = block(&[5, 1]);
    assert_eq!(short.read_byte_array().unwrap(), [0; 5]);
    assert_eq!(short.step, 6);
    assert!(block(&[]).read_byte_array().unwrap().is_empty());

    let long = |bytes: &[u8]| {
        let mut b = block(bytes);
        b.long_binary_data = true;
        let values = b.read_byte_array();
        (values, b.step)
    };
    assert_eq!(long(&[2, 0, 0, 0, 7, 8]).0.unwrap(), [7, 8]);
    // 30000: an empty array, after the count.
    let (huge, step) = long(&[0x30, 0x75, 0, 0, 1]);
    assert!(huge.unwrap().is_empty());
    assert_eq!(step, 4);
    // Fewer than four bytes: `readInt32` gives 0 without moving.
    let (none, step) = long(&[1, 2]);
    assert!(none.unwrap().is_empty());
    assert_eq!(step, 0);
    // A negative count passes the check, and `new byte[-1]` throws.
    assert_eq!(long(&[0xFF, 0xFF, 0xFF, 0xFF, 1]).0.unwrap_err().kind, ExceptionKind::Overflow);
}

#[test]
fn integer_arrays_have_a_u16_count() {
    assert_eq!(block(&[2, 0, 1, 0, 0, 0, 2, 0, 0, 0]).read_i32_array(), [1, 2]);
    assert_eq!(block(&[3, 0, 1, 0, 0, 0]).read_i32_array(), [1, 0, 0]);
    assert_eq!(block(&[1, 0, 9, 0, 0, 0, 0, 0, 0, 0]).read_u64_array(), [9]);
    assert_eq!(block(&[5, 0, 0, 0, 0, 0, 0, 0]).read_steam_id(), 5);
}

/// `GuidBuffer.Read` on a byte array: 16 bytes in the `Guid`'s own layout, or an `ArgumentException`.
#[test]
fn guids_need_16_bytes() {
    let bytes: Vec<u8> = (1..=16).collect();
    let mut data = vec![16];
    data.extend_from_slice(&bytes);
    assert_eq!(block(&data).read_guid().unwrap(), Guid::from_bytes(bytes.try_into().unwrap()));
    let error = block(&[4, 1, 2, 3, 4]).read_guid().unwrap_err();
    assert_eq!(error.kind, ExceptionKind::Argument);
    assert_eq!(error.message, "Source buffer is too small!");
}

#[test]
fn vectors_and_colors() {
    let color = block(&[255, 0, 51]).read_color();
    assert_eq!((color.r, color.g, color.b, color.a), (1.0, 0.0, 51.0f32 / 255.0, 1.0));

    let mut floats = block(&[0, 0, 0x80, 0x3F, 0, 0, 0, 0x40, 0, 0, 0x40, 0x40]);
    assert_eq!(floats.read_single_vector3(), Vector3 { x: 1.0, y: 2.0, z: 3.0 });

    // `Regions.REGION_SIZE` is `8192 / 64`.
    let region = 128.0;
    let x = 1.0 * region + (32768.0 / 65535.0) * region - 4096.0;
    let y = (65535.0 / 65535.0) * 2048.0 - 1024.0;
    let z = 2.0 * region + 0.0 * region - 4096.0;
    let vector = block(&[1, 0x00, 0x80, 0xFF, 0xFF, 2, 0, 0]).read_u16_r_vector3();
    assert_eq!(vector, Vector3 { x: x as f32, y: y as f32, z: z as f32 });
}

#[test]
fn the_hash_is_sha1_of_the_contents() {
    let contents = [1u8, 2, 3, 4];
    let mut b = block(&contents);
    b.read_i16();
    let expected: [u8; 20] = Sha1::digest(contents).into();
    assert_eq!(b.get_hash(), expected);
}
