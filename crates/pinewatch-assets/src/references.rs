//! References from one asset to others or to Unity content: `AssetReference<T>`, `ContentReference<T>`
//! (`Bundles/AssetReference.cs`, `Bundles/ContentReference.cs`), and `UnturnedDatEx`'s GUID-or-legacy-ID helpers
//! (`Files/UnturnedDatEx.cs`). The type parameters only matter when the reference is resolved, so they aren't kept.

use pinewatch_dat::{DatParseable, DictRef, NodeRef};
use pinewatch_mono::{Guid, MonoException, NetString, NumberFormat, NumberStyles, try_parse_u16};

/// `AssetReference<T>`: an asset's GUID.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct AssetReference {
    pub guid: Guid,
}

impl AssetReference {
    /// `isValid`: the GUID isn't empty.
    pub fn is_valid(&self) -> bool {
        self.guid != Guid::EMPTY
    }
}

impl DatParseable for AssetReference {
    type Context = ();

    /// A value holding the GUID, or a dictionary with `GUID`. A failed parse leaves `Guid.Empty`.
    fn try_parse(&mut self, node: NodeRef<'_>, _: &mut ()) -> Result<bool, MonoException> {
        let parsed = match node {
            NodeRef::Value(value) => value.try_parse_guid(),
            NodeRef::Dictionary(dictionary) => dictionary.try_parse_guid("GUID"),
            NodeRef::List(_) => return Ok(false),
        };
        self.guid = parsed.unwrap_or(Guid::EMPTY);
        Ok(parsed.is_some())
    }
}

/// `ContentReference<T>`: a Unity asset in an asset bundle, by bundle name and path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContentReference {
    pub name: Option<NetString>,
    pub path: Option<NetString>,
}

impl ContentReference {
    /// `isValid`: both the name and the path are set and not empty.
    pub fn is_valid(&self) -> bool {
        let set = |text: &Option<NetString>| text.as_ref().is_some_and(|text| !text.is_empty());
        set(&self.name) && set(&self.path)
    }
}

impl DatParseable for ContentReference {
    /// `Assets.currentMasterBundle`'s `assetBundleName`, when an asset from a master bundle is being loaded.
    type Context = Option<String>;

    fn try_parse(
        &mut self,
        node: NodeRef<'_>,
        current_master_bundle: &mut Option<String>,
    ) -> Result<bool, MonoException> {
        match node {
            NodeRef::Value(value) => {
                let Some(text) = value.value().filter(|text| !text.is_empty()) else {
                    return Ok(false);
                };
                let units = text.as_units();
                if units.len() < 2 {
                    return Ok(false);
                }
                match units.iter().position(|&unit| unit == u16::from(b':')) {
                    None => {
                        if let Some(name) = current_master_bundle {
                            self.name = Some(NetString::from(name.as_str()));
                        }
                        self.path = Some(text.clone());
                    }
                    Some(delimiter) => {
                        self.name = Some(NetString::from_units(units[..delimiter].to_vec()));
                        self.path = Some(NetString::from_units(units[delimiter + 1..].to_vec()));
                    }
                }
                Ok(true)
            }
            NodeRef::Dictionary(dictionary) => {
                self.name = dictionary.get_string("Name", None).cloned();
                self.path = dictionary.get_string("Path", None).cloned();
                Ok(true)
            }
            NodeRef::List(_) => Ok(false),
        }
    }
}

/// `ParseGuidOrLegacyId(key, out guid, out legacyId)`: a `ushort` first, since large GUID-like numbers fail as one,
/// then a GUID. A value that is null, empty or `0`, or that is neither, gives `Guid.Empty` and 0.
pub fn parse_guid_or_legacy_id(dictionary: &DictRef<'_>, key: &str) -> (Guid, u16) {
    let Some(Some(value)) = dictionary.try_get_string(key) else {
        return (Guid::EMPTY, 0);
    };
    if value.is_empty() || value.as_units() == [u16::from(b'0')] {
        return (Guid::EMPTY, 0);
    }
    // `NumberStyles.Any` is a valid style, so the parse never returns `Err`.
    if let Ok(Some(legacy_id)) = try_parse_u16(Some(value), NumberStyles::ANY, &NumberFormat::invariant()) {
        return (Guid::EMPTY, legacy_id);
    }
    match Guid::try_parse(Some(value)) {
        Some(guid) => (guid, 0),
        None => (Guid::EMPTY, 0),
    }
}

/// `readAssetReference<T>(key, defaultValue)`: `ParseGuid(key)` when the key exists, else `default`
/// (`AssetReference<T>.invalid` for the overload without one).
pub fn read_asset_reference(dictionary: &DictRef<'_>, key: &str, default: AssetReference) -> AssetReference {
    if dictionary.contains_key(key) {
        AssetReference { guid: dictionary.parse_guid(key, Guid::EMPTY) }
    } else {
        default
    }
}
