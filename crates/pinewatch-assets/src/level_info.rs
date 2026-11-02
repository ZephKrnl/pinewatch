//! Level info (`Level.ReadLevelInfo` in `Level/Level.cs`): what level discovery reads from a level folder. So far
//! `Level.dat`'s fields and the packed level version; `Config.json` comes with `pinewatch-json` (M2f).

use pinewatch_binary::Block;
use pinewatch_mono::{NetString, NumberFormat, NumberStyles, chars, try_parse_u32};

pinewatch_mono::net_enum! {
    /// `ELevelSize`.
    pub struct ELevelSize(i32) {
        TINY = 0,
        SMALL = 1,
        MEDIUM = 2,
        LARGE = 3,
        INSANE = 4,
    }
}

pinewatch_mono::net_enum! {
    /// `ELevelType`.
    pub struct ELevelType(i32) {
        SURVIVAL = 0,
        HORDE = 1,
        ARENA = 2,
    }
}

/// What `ReadLevelInfo` reads from `Level.dat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelFile {
    pub version: u8,
    /// `block.getHash()`: the SHA-1 of the whole file.
    pub hash: [u8; 20],
    /// The Steam ID of the player who made the level, which makes it editable for them.
    pub owner: u64,
    pub size: ELevelSize,
    pub level_type: ELevelType,
}

/// `ReadLevelInfo`'s reads from `new Block(0, bytes)`: the version, the hash, the owner, the size, and the type only
/// from version 2 (`SURVIVAL` before). A short file reads zeros.
pub fn read_level_file(contents: Vec<u8>) -> LevelFile {
    let mut block = Block::new(0, contents);
    let version = block.read_byte();
    let hash = block.get_hash();
    let owner = block.read_steam_id();
    let size = ELevelSize(i32::from(block.read_byte()));
    let level_type = if version > 1 { ELevelType(i32::from(block.read_byte())) } else { ELevelType::SURVIVAL };
    LevelFile { version, hash, owner, size, level_type }
}

/// `Parser.TryGetUInt32FromIP(version, out packed)`: four `.`-separated parts, each `uint.TryParse`d and masked to a
/// byte, packed high byte first. A null or white-space text or a part count other than four gives `(false, 0)`; a
/// part that doesn't parse gives `false` with the bytes packed before it. `uint.TryParse` reads in the current
/// culture (V29).
pub fn try_parse_packed_version(version: Option<&NetString>) -> (bool, u32) {
    let Some(version) = version.filter(|text| !text.as_units().iter().all(|&unit| chars::is_white_space(unit))) else {
        return (false, 0);
    };
    let parts: Vec<&[u16]> = version.as_units().split(|&unit| unit == u16::from(b'.')).collect();
    if parts.len() != 4 {
        return (false, 0);
    }
    let mut value = 0u32;
    for (part, shift) in parts.into_iter().zip([24, 16, 8, 0]) {
        let part = NetString::from_units(part.to_vec());
        // `NumberStyles.Integer` is a valid style, so the parse never returns `Err`.
        match try_parse_u32(Some(&part), NumberStyles::INTEGER, &NumberFormat::invariant()).ok().flatten() {
            Some(byte) => value |= (byte & 0xFF) << shift,
            None => return (false, value),
        }
    }
    (true, value)
}
