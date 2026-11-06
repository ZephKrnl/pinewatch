//! `Block` (`Files/Block.cs`): a byte array read from a moving `step`, as `ReadWrite.readBlock` returns it. Every
//! read checks that its bytes are there and otherwise gives zero, false or empty without moving, except the ones
//! that can throw (`readBooleanArray`, `readByteArray` with `longBinaryData`, and `readGUID`). Numbers are
//! little-endian (`BitConverter` on x86).
//!
//! Not ported yet: `readSingleQuaternion` (Unity's `Quaternion.Euler`, L6) and the `read(Type…)` dispatch, which
//! come with the layouts that need them; the writers.

use pinewatch_mono::{Guid, MonoException, NetString, utf8_get_string};
use pinewatch_unity::{Color, Vector3};
use sha1::{Digest, Sha1};

/// `Regions.REGION_SIZE`: `8192 / Regions.CONST_WORLD_SIZE` (64).
const REGION_SIZE: f64 = 128.0;

/// `Block` with its contents.
#[derive(Debug, Clone)]
pub struct Block {
    /// `step`: the next byte to read.
    pub step: usize,
    /// `longBinaryData`: byte arrays have an `int` count instead of a `byte` one.
    pub long_binary_data: bool,
    block: Vec<u8>,
}

impl Block {
    /// `new Block(prefix, contents)`: reading starts at `prefix`.
    pub fn new(prefix: usize, contents: Vec<u8>) -> Self {
        Self { step: prefix, long_binary_data: false, block: contents }
    }

    /// Whether `count` more bytes are there (`step <= block.Length - count`).
    fn has(&self, count: usize) -> bool {
        self.block.len().checked_sub(count).is_some_and(|last| self.step <= last)
    }

    /// The next `N` bytes, moving past them, if they are all there.
    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        if !self.has(N) {
            return None;
        }
        let bytes = self.block[self.step..self.step + N].try_into().expect("N bytes");
        self.step += N;
        Some(bytes)
    }

    /// `readString`: a byte length, then that many bytes of UTF-8. A length past the end gives `""` and still moves
    /// the step past it.
    pub fn read_string(&mut self) -> NetString {
        if self.step >= self.block.len() {
            return NetString::new();
        }
        let length = usize::from(self.block[self.step]);
        self.step += 1;
        let value = if self.step + length <= self.block.len() {
            utf8_get_string(&self.block[self.step..self.step + length])
        } else {
            NetString::new()
        };
        self.step += length;
        value
    }

    /// `readStringArray`: a byte count, then the strings.
    pub fn read_string_array(&mut self) -> Vec<NetString> {
        if self.step >= self.block.len() {
            return Vec::new();
        }
        let count = self.read_byte();
        (0..count).map(|_| self.read_string()).collect()
    }

    /// `readBoolean`: `BitConverter.ToBoolean`, so any non-zero byte is true.
    pub fn read_boolean(&mut self) -> bool {
        self.take::<1>().is_some_and(|[byte]| byte != 0)
    }

    /// `readBooleanArray`: a `ushort` count, then the values eight to a byte, lowest bit first. The packed bytes
    /// aren't checked, so a count past the end throws `IndexOutOfRangeException`.
    pub fn read_boolean_array(&mut self) -> Result<Vec<bool>, MonoException> {
        if self.step >= self.block.len() {
            return Ok(Vec::new());
        }
        let count = usize::from(self.read_u16());
        // `(ushort) Mathf.CeilToInt(values.Length / 8f)`: exact for any `ushort` count.
        let chunks = count.div_ceil(8);
        let mut values = vec![false; count];
        for chunk in 0..chunks {
            for bit in 0..8 {
                let index = chunk * 8 + bit;
                if index >= count {
                    break;
                }
                let byte = *self.block.get(self.step + chunk).ok_or_else(MonoException::index_out_of_range)?;
                values[index] = byte & (1 << bit) != 0;
            }
        }
        self.step += chunks;
        Ok(values)
    }

    /// `readByte`.
    pub fn read_byte(&mut self) -> u8 {
        self.take::<1>().map_or(0, |[byte]| byte)
    }

    /// `readByteArray`: a `byte` count (an `int` one with `long_binary_data`, where 30000 or more gives an empty
    /// array), then the bytes. When they run past the end the copy is skipped, leaving zeros, and the step moves
    /// anyway. A negative `int` count passes the 30000 check, and `new byte[count]` throws `OverflowException`
    /// with an empty message, as the game's runtime does (V30).
    pub fn read_byte_array(&mut self) -> Result<Vec<u8>, MonoException> {
        if self.step >= self.block.len() {
            return Ok(Vec::new());
        }
        let count = if self.long_binary_data {
            let count = self.read_i32();
            if count >= 30000 {
                return Ok(Vec::new());
            }
            usize::try_from(count).map_err(|_| MonoException::negative_array_size())?
        } else {
            let count = usize::from(self.block[self.step]);
            self.step += 1;
            count
        };
        let mut values = vec![0; count];
        if self.step + count <= self.block.len() {
            values.copy_from_slice(&self.block[self.step..self.step + count]);
        }
        self.step += count;
        Ok(values)
    }

    /// `readInt16`.
    pub fn read_i16(&mut self) -> i16 {
        self.take().map_or(0, i16::from_le_bytes)
    }

    /// `readUInt16`.
    pub fn read_u16(&mut self) -> u16 {
        self.take().map_or(0, u16::from_le_bytes)
    }

    /// `readInt32`.
    pub fn read_i32(&mut self) -> i32 {
        self.take().map_or(0, i32::from_le_bytes)
    }

    /// `readInt32Array`: a `ushort` count, then the values (zero past the end).
    pub fn read_i32_array(&mut self) -> Vec<i32> {
        let count = self.read_u16();
        (0..count).map(|_| self.read_i32()).collect()
    }

    /// `readUInt32`.
    pub fn read_u32(&mut self) -> u32 {
        self.take().map_or(0, u32::from_le_bytes)
    }

    /// `readSingle`.
    pub fn read_f32(&mut self) -> f32 {
        self.take().map_or(0.0, f32::from_le_bytes)
    }

    /// `readInt64`.
    pub fn read_i64(&mut self) -> i64 {
        self.take().map_or(0, i64::from_le_bytes)
    }

    /// `readUInt64`.
    pub fn read_u64(&mut self) -> u64 {
        self.take().map_or(0, u64::from_le_bytes)
    }

    /// `readUInt64Array`: a `ushort` count, then the values (zero past the end).
    pub fn read_u64_array(&mut self) -> Vec<u64> {
        let count = self.read_u16();
        (0..count).map(|_| self.read_u64()).collect()
    }

    /// `readSteamID`: a `CSteamID` from a `ulong`.
    pub fn read_steam_id(&mut self) -> u64 {
        self.read_u64()
    }

    /// `readGUID`: `GuidBuffer.Read` on `readByteArray`'s bytes, which copies 16 bytes in the `Guid`'s own layout
    /// and throws `ArgumentException("Source buffer is too small!")` for fewer.
    pub fn read_guid(&mut self) -> Result<Guid, MonoException> {
        let bytes = self.read_byte_array()?;
        let bytes: [u8; 16] = bytes
            .get(..16)
            .and_then(|first| first.try_into().ok())
            .ok_or_else(|| MonoException::argument("Source buffer is too small!", None))?;
        Ok(Guid::from_bytes(bytes))
    }

    /// `readUInt16RVector3`: a region byte and a `ushort` fraction for x, a `ushort` for y, then z like x, computed
    /// in `double` and cast to `float`.
    pub fn read_u16_r_vector3(&mut self) -> Vector3 {
        let x_region = f64::from(self.read_byte());
        let x = f64::from(self.read_u16()) / f64::from(u16::MAX);
        let y = f64::from(self.read_u16()) / f64::from(u16::MAX);
        let z_region = f64::from(self.read_byte());
        let z = f64::from(self.read_u16()) / f64::from(u16::MAX);
        let x = x_region * REGION_SIZE + x * REGION_SIZE - 4096.0;
        let y = y * 2048.0 - 1024.0;
        let z = z_region * REGION_SIZE + z * REGION_SIZE - 4096.0;
        Vector3 { x: x as f32, y: y as f32, z: z as f32 }
    }

    /// `readSingleVector3`.
    pub fn read_single_vector3(&mut self) -> Vector3 {
        let (x, y, z) = (self.read_f32(), self.read_f32(), self.read_f32());
        Vector3 { x, y, z }
    }

    /// `readColor`: three bytes, each `/ 255f`, and an alpha of 1.
    pub fn read_color(&mut self) -> Color {
        let (r, g, b) = (self.read_byte(), self.read_byte(), self.read_byte());
        Color { r: f32::from(r) / 255.0, g: f32::from(g) / 255.0, b: f32::from(b) / 255.0, a: 1.0 }
    }

    /// `getHash`: the SHA-1 of the whole contents, wherever the step is.
    pub fn get_hash(&self) -> [u8; 20] {
        Sha1::digest(&self.block).into()
    }
}
