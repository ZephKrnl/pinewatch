// Ported from corert `src/System.Private.CoreLib/src/System/Number.CoreRT.cs` (mono/corert 11136ad5).
// Copyright (c) .NET Foundation and Contributors. MIT license; see NOTICES-PORTED-CODE.md.

//! `Number.NumberToDouble`, the managed conversion Unity's Mono uses for every `float` and `double` it parses.
//!
//! The mantissa is a 64-bit integer with a separate exponent, so no float arithmetic is involved and the result is the
//! same on every platform. Only the first 18 significant digits count. The value is scaled by at most two table
//! entries (each product may be off by one bit), then rounded once to 53 bits. A denormal result is shifted without
//! a second rounding.

/// `DigitsToInt`: up to 9 decimal digits as a number.
fn digits_to_int(digits: &[u16]) -> u32 {
    let mut result = u32::from(digits[0]).wrapping_sub(u32::from(b'0'));
    for &digit in &digits[1..] {
        result = result.wrapping_mul(10).wrapping_add(u32::from(digit)).wrapping_sub(u32::from(b'0'));
    }
    result
}

/// `Mul32x32To64`.
fn mul_32x32_to_64(a: u32, b: u32) -> u64 {
    u64::from(a) * u64::from(b)
}

/// `Mul64Lossy`: the high 64 bits of the product, keeping the top bit set.
fn mul_64_lossy(a: u64, b: u64, exp: &mut i32) -> u64 {
    let mut val = mul_32x32_to_64((a >> 32) as u32, (b >> 32) as u32)
        .wrapping_add(mul_32x32_to_64((a >> 32) as u32, b as u32) >> 32)
        .wrapping_add(mul_32x32_to_64(a as u32, (b >> 32) as u32) >> 32);
    if val & 0x8000_0000_0000_0000 == 0 {
        val <<= 1;
        *exp -= 1;
    }
    val
}

/// `s_rgval64Power10`: normalized mantissas of 10^1 to 10^15, then of 0.1^1 to 0.1^15.
const VAL64_POWER10: [u64; 30] = [
    0xa000000000000000,
    0xc800000000000000,
    0xfa00000000000000,
    0x9c40000000000000,
    0xc350000000000000,
    0xf424000000000000,
    0x9896800000000000,
    0xbebc200000000000,
    0xee6b280000000000,
    0x9502f90000000000,
    0xba43b74000000000,
    0xe8d4a51000000000,
    0x9184e72a00000000,
    0xb5e620f480000000,
    0xe35fa931a0000000,
    0xcccccccccccccccd,
    0xa3d70a3d70a3d70b,
    0x83126e978d4fdf3c,
    0xd1b71758e219652e,
    0xa7c5ac471b478425,
    0x8637bd05af6c69b7,
    0xd6bf94d5e57a42be,
    0xabcc77118461ceff,
    0x89705f4136b4a599,
    0xdbe6fecebdedd5c2,
    0xafebff0bcb24ab02,
    0x8cbccc096f5088cf,
    0xe12e13424bb40e18,
    0xb424dc35095cd813,
    0x901d7cf73ab0acdc,
];

/// `s_rgexp64Power10`: binary exponents shared by both halves of `VAL64_POWER10`.
const EXP64_POWER10: [i8; 15] = [4, 7, 10, 14, 17, 20, 24, 27, 30, 34, 37, 40, 44, 47, 50];

/// `s_rgval64Power10By16`: normalized mantissas of 10^16 to 10^336 in steps of 10^16, then of their inverses.
const VAL64_POWER10_BY16: [u64; 42] = [
    0x8e1bc9bf04000000,
    0x9dc5ada82b70b59e,
    0xaf298d050e4395d6,
    0xc2781f49ffcfa6d4,
    0xd7e77a8f87daf7fa,
    0xefb3ab16c59b14a0,
    0x850fadc09923329c,
    0x93ba47c980e98cde,
    0xa402b9c5a8d3a6e6,
    0xb616a12b7fe617a8,
    0xca28a291859bbf90,
    0xe070f78d39275566,
    0xf92e0c3537826140,
    0x8a5296ffe33cc92c,
    0x9991a6f3d6bf1762,
    0xaa7eebfb9df9de8a,
    0xbd49d14aa79dbc7e,
    0xd226fc195c6a2f88,
    0xe950df20247c83f8,
    0x81842f29f2cce373,
    0x8fcac257558ee4e2,
    0xe69594bec44de160,
    0xcfb11ead453994c3,
    0xbb127c53b17ec165,
    0xa87fea27a539e9b3,
    0x97c560ba6b0919b5,
    0x88b402f7fd7553ab,
    0xf64335bcf065d3a0,
    0xddd0467c64bce4c4,
    0xc7caba6e7c5382ed,
    0xb3f4e093db73a0b7,
    0xa21727db38cb0053,
    0x91ff83775423cc29,
    0x8380dea93da4bc82,
    0xece53cec4a314f00,
    0xd5605fcdcf32e217,
    0xc0314325637a1978,
    0xad1c8eab5ee43ba2,
    0x9becce62836ac5b0,
    0x8c71dcd9ba0b495c,
    0xfd00b89747823938,
    0xe3e27a444d8d991a,
];

/// `s_rgexp64Power10By16`: binary exponents shared by both halves of `VAL64_POWER10_BY16`.
const EXP64_POWER10_BY16: [i16; 21] =
    [54, 107, 160, 213, 266, 319, 373, 426, 479, 532, 585, 638, 691, 745, 798, 851, 904, 957, 1010, 1064, 1117];

/// `abs(int)`, which wraps for `int.MinValue` like the C#.
fn abs(value: i32) -> i32 {
    if value < 0 { value.wrapping_neg() } else { value }
}

/// `Number.NumberToDouble` for a number buffer with these digits (without the NUL terminator), scale and sign.
pub(crate) fn number_to_double(digits: &[u16], number_scale: i32, negative: bool) -> f64 {
    let total = digits.len() as i32;
    let mut remaining = total;
    let mut src = 0usize;

    while src < digits.len() && digits[src] == u16::from(b'0') {
        remaining -= 1;
        src += 1;
    }
    if remaining == 0 {
        return 0.0;
    }

    let mut count = remaining.min(9);
    remaining -= count;
    let mut val = u64::from(digits_to_int(&digits[src..src + count as usize]));
    if remaining > 0 {
        count = remaining.min(9);
        remaining -= count;
        // The power of ten for the second group, denormalized to fit 32 bits.
        let shift = 64 - u32::from(EXP64_POWER10[(count - 1) as usize].unsigned_abs());
        let mult = (VAL64_POWER10[(count - 1) as usize] >> shift) as u32;
        let tail = digits_to_int(&digits[src + 9..src + 9 + count as usize]);
        val = mul_32x32_to_64(val as u32, mult).wrapping_add(u64::from(tail));
    }

    let scale = number_scale - (total - remaining);
    let absscale = abs(scale);
    if absscale >= 22 * 16 {
        let mut result: u64 = if scale > 0 { 0x7FF0_0000_0000_0000 } else { 0 };
        if negative {
            result |= 0x8000_0000_0000_0000;
        }
        return f64::from_bits(result);
    }

    let mut exp: i32 = 64;
    if val & 0xFFFF_FFFF_0000_0000 == 0 {
        val <<= 32;
        exp -= 32;
    }
    if val & 0xFFFF_0000_0000_0000 == 0 {
        val <<= 16;
        exp -= 16;
    }
    if val & 0xFF00_0000_0000_0000 == 0 {
        val <<= 8;
        exp -= 8;
    }
    if val & 0xF000_0000_0000_0000 == 0 {
        val <<= 4;
        exp -= 4;
    }
    if val & 0xC000_0000_0000_0000 == 0 {
        val <<= 2;
        exp -= 2;
    }
    if val & 0x8000_0000_0000_0000 == 0 {
        val <<= 1;
        exp -= 1;
    }

    let mut index = absscale & 15;
    if index != 0 {
        let multexp = i32::from(EXP64_POWER10[(index - 1) as usize]);
        exp += if scale < 0 { -multexp + 1 } else { multexp };
        let multval = VAL64_POWER10[(index + if scale < 0 { 15 } else { 0 } - 1) as usize];
        val = mul_64_lossy(val, multval, &mut exp);
    }

    index = absscale >> 4;
    if index != 0 {
        let multexp = i32::from(EXP64_POWER10_BY16[(index - 1) as usize]);
        exp += if scale < 0 { -multexp + 1 } else { multexp };
        let multval = VAL64_POWER10_BY16[(index + if scale < 0 { 21 } else { 0 } - 1) as usize];
        val = mul_64_lossy(val, multval, &mut exp);
    }

    // Round half to even at bit 11, which becomes the last mantissa bit.
    if (val as i32) & (1 << 10) != 0 {
        let mut tmp = val.wrapping_add((1 << 10) - 1).wrapping_add((((val as i32) >> 11) & 1) as u64);
        if tmp < val {
            tmp = (tmp >> 1) | 0x8000_0000_0000_0000;
            exp += 1;
        }
        val = tmp;
    }

    exp += 0x3FE;
    if exp <= 0 {
        if exp == -52 && val >= 0x8000_0000_0000_0058 {
            // Just below the smallest denormal: rounds up to it instead of to zero.
            val = 0x0000_0000_0000_0001;
        } else if exp <= -52 {
            val = 0;
        } else {
            val >>= -exp + 11 + 1;
        }
    } else if exp >= 0x7FF {
        val = 0x7FF0_0000_0000_0000;
    } else {
        val = ((exp as u64) << 52).wrapping_add((val >> 11) & 0x000F_FFFF_FFFF_FFFF);
    }

    if negative {
        val |= 0x8000_0000_0000_0000;
    }
    f64::from_bits(val)
}

#[cfg(test)]
mod tests {
    use super::number_to_double;

    fn digits(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn small_exact_values() {
        assert_eq!(number_to_double(&digits("5"), 0, false), 0.5);
        assert_eq!(number_to_double(&digits("1"), 4, false), 1000.0);
        assert_eq!(number_to_double(&digits("25"), 1, true), -2.5);
        assert_eq!(number_to_double(&digits(""), 3, true).to_bits(), 0);
    }

    #[test]
    fn only_eighteen_digits_count() {
        let truncated = number_to_double(&digits("123456789012345678"), 22, false);
        assert_eq!(number_to_double(&digits("1234567890123456789999"), 22, false).to_bits(), truncated.to_bits());
    }

    #[test]
    fn scale_limits_go_to_infinity_and_signed_zero() {
        assert_eq!(number_to_double(&digits("1"), 353, false), f64::INFINITY);
        assert_eq!(number_to_double(&digits("1"), -351, true).to_bits(), 0x8000_0000_0000_0000);
    }
}
