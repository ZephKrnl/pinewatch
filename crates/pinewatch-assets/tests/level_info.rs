//! Level info (`Level.ReadLevelInfo`): `Level.dat`'s fields and the packed level version (M2f).

use pinewatch_assets::{ELevelSize, ELevelType, read_level_file, try_parse_packed_version};
use pinewatch_mono::NetString;
use sha1::{Digest, Sha1};

fn packed(text: Option<&str>) -> (bool, u32) {
    try_parse_packed_version(text.map(NetString::from).as_ref())
}

/// `Parser.TryGetUInt32FromIP`: four dot-separated `uint`s, each masked to a byte; a part that fails returns false
/// with the bits packed so far.
#[test]
fn packed_versions() {
    assert_eq!(packed(Some("3.0.0.0")), (true, 0x0300_0000));
    assert_eq!(packed(Some("1.2.3.300")), (true, 0x0102_032C));
    // 300 is 0x12C: the mask drops its 0x100, which "3" above happened to set anyway.
    assert_eq!(packed(Some("1.2.0.300")), (true, 0x0102_002C));
    assert_eq!(packed(Some("1.2.x.4")), (false, 0x0102_0000));
    for text in [Some("1.2.3"), Some("1.2.3.4.5"), Some(""), Some("   "), None] {
        assert_eq!(packed(text), (false, 0), "{text:?}");
    }
    assert_eq!(packed(Some("+1.0.0.0")), (true, 0x0100_0000));
    assert_eq!(packed(Some(" 1.0.0.0")), (true, 0x0100_0000));
    assert_eq!(packed(Some("1.0.0.-1")), (false, 0x0100_0000));
}

/// `ReadLevelInfo`'s reads from `Block(0, bytes)`: the version, the hash of the whole file, the owner's Steam ID,
/// the size, and the type only from version 2.
#[test]
fn level_file_fields() {
    let owner: u64 = 76561197960287930;
    let mut version_2 = vec![2];
    version_2.extend_from_slice(&owner.to_le_bytes());
    version_2.extend_from_slice(&[3, 2]);
    let file = read_level_file(version_2.clone());
    assert_eq!((file.version, file.owner, file.size, file.level_type), (2, owner, ELevelSize(3), ELevelType(2)));
    assert_eq!(file.size, ELevelSize::LARGE);
    let hash: [u8; 20] = Sha1::digest(&version_2).into();
    assert_eq!(file.hash, hash);

    let mut version_1 = vec![1];
    version_1.extend_from_slice(&owner.to_le_bytes());
    version_1.extend_from_slice(&[4, 9]);
    let file = read_level_file(version_1);
    assert_eq!((file.size, file.level_type), (ELevelSize::INSANE, ELevelType::SURVIVAL));

    let empty = read_level_file(Vec::new());
    let fields = (empty.version, empty.owner, empty.size, empty.level_type);
    assert_eq!(fields, (0, 0, ELevelSize::TINY, ELevelType::SURVIVAL));
    let hash: [u8; 20] = Sha1::digest([]).into();
    assert_eq!(empty.hash, hash);
}
