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
                Ok(parse_bundle_value(value.value(), current_master_bundle, &mut self.name, &mut self.path))
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

/// The value form `ContentReference` and `MasterBundleReference` share: null, empty and one-character text is
/// refused (a stray `}` from a common typo); otherwise the text splits at its first `:` into bundle name and path,
/// and without one it is the path, with the current master bundle's name if there is one (the name stays as it was
/// if not).
fn parse_bundle_value(
    text: Option<&NetString>,
    current_master_bundle: &Option<String>,
    name: &mut Option<NetString>,
    path: &mut Option<NetString>,
) -> bool {
    let Some(text) = text.filter(|text| text.as_units().len() >= 2) else {
        return false;
    };
    let units = text.as_units();
    match units.iter().position(|&unit| unit == u16::from(b':')) {
        None => {
            if let Some(bundle) = current_master_bundle {
                *name = Some(NetString::from(bundle.as_str()));
            }
            *path = Some(text.clone());
        }
        Some(delimiter) => {
            *name = Some(NetString::from_units(units[..delimiter].to_vec()));
            *path = Some(NetString::from_units(units[delimiter + 1..].to_vec()));
        }
    }
    true
}

/// `MasterBundleReference<T>`: a Unity asset in a master bundle, by bundle name and path. Loading it is Unity's
/// (M2d).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MasterBundleReference {
    pub name: Option<NetString>,
    pub path: Option<NetString>,
}

impl MasterBundleReference {
    /// `isNull`: the name or the path is null or empty.
    pub fn is_null(&self) -> bool {
        is_null_or_empty(&self.name) || is_null_or_empty(&self.path)
    }

    /// `isValid`: neither is.
    pub fn is_valid(&self) -> bool {
        !self.is_null()
    }
}

impl std::fmt::Display for MasterBundleReference {
    /// `ToString()`: `name:path`, with null as empty.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = |value: &Option<NetString>| value.as_ref().map(NetString::to_string_lossy).unwrap_or_default();
        write!(f, "{}:{}", text(&self.name), text(&self.path))
    }
}

impl DatParseable for MasterBundleReference {
    /// `Assets.currentMasterBundle`'s `assetBundleName`, when an asset from a master bundle is being loaded.
    type Context = Option<String>;

    /// A value as `ContentReference` reads one; a dictionary's keys are `MasterBundle` and `AssetPath`.
    fn try_parse(
        &mut self,
        node: NodeRef<'_>,
        current_master_bundle: &mut Option<String>,
    ) -> Result<bool, MonoException> {
        match node {
            NodeRef::Value(value) => {
                Ok(parse_bundle_value(value.value(), current_master_bundle, &mut self.name, &mut self.path))
            }
            NodeRef::Dictionary(dictionary) => {
                self.name = dictionary.get_string("MasterBundle", None).cloned();
                self.path = dictionary.get_string("AssetPath", None).cloned();
                Ok(true)
            }
            NodeRef::List(_) => Ok(false),
        }
    }
}

/// `AudioReference`: an audio clip in a master bundle, by bundle name and path. Loading it is Unity's (M2d).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioReference {
    pub asset_bundle_name: Option<NetString>,
    pub path: Option<NetString>,
}

impl AudioReference {
    /// `IsNullOrEmpty`: the bundle name or the path is null or empty.
    pub fn is_null_or_empty(&self) -> bool {
        is_null_or_empty(&self.asset_bundle_name) || is_null_or_empty(&self.path)
    }
}

fn is_null_or_empty(text: &Option<NetString>) -> bool {
    text.as_ref().is_none_or(|text| text.is_empty())
}

/// `readMasterBundleReference<T>(key, defaultMasterBundle)`: the key's text through `ParseMasterBundleReference`,
/// or the invalid reference (both null) when the key holds no value. `default_master_bundle` is the name of
/// `defaultMasterBundle ?? Assets.currentMasterBundle`. The problems found go to `reports`, in order, for the caller
/// to report as `ReportAssetError` does for the current asset (or `Assets.reportError` without one). A flag's null
/// text throws `NullReferenceException`.
pub fn read_master_bundle_reference(
    dictionary: &DictRef<'_>,
    key: &str,
    default_master_bundle: Option<&str>,
    reports: &mut Vec<String>,
) -> Result<MasterBundleReference, MonoException> {
    let Some(value) = dictionary.try_get_string(key) else {
        return Ok(MasterBundleReference::default());
    };
    let value = value.ok_or_else(MonoException::null_reference)?;
    let (name, path) = parse_master_bundle_reference(key, value, default_master_bundle, reports);
    Ok(MasterBundleReference { name: Some(name), path: Some(path) })
}

/// `ReadAudioReference(key, defaultMasterBundle)`: as `read_master_bundle_reference`, for an `AudioReference`
/// (`default`, both null, when the key holds no value).
pub fn read_audio_reference(
    dictionary: &DictRef<'_>,
    key: &str,
    default_master_bundle: Option<&str>,
    reports: &mut Vec<String>,
) -> Result<AudioReference, MonoException> {
    let reference = read_master_bundle_reference(dictionary, key, default_master_bundle, reports)?;
    Ok(AudioReference { asset_bundle_name: reference.name, path: reference.path })
}

/// `ParseMasterBundleReference`: split at the first `:` with no length check. Without one, the name is the default
/// master bundle's, or empty with a report when there is none; with one, an empty name and an empty path are each
/// reported.
fn parse_master_bundle_reference(
    key: &str,
    value: &NetString,
    default_master_bundle: Option<&str>,
    reports: &mut Vec<String>,
) -> (NetString, NetString) {
    let units = value.as_units();
    match units.iter().position(|&unit| unit == u16::from(b':')) {
        None => {
            let name = match default_master_bundle {
                Some(bundle) => NetString::from(bundle),
                None => {
                    reports.push(format!(
                        "MasterBundleRef \"{key}\" is not associated with a master bundle nor does it specify one"
                    ));
                    NetString::new()
                }
            };
            (name, value.clone())
        }
        Some(delimiter) => {
            let name = NetString::from_units(units[..delimiter].to_vec());
            let path = NetString::from_units(units[delimiter + 1..].to_vec());
            if name.is_empty() {
                reports.push(format!("MasterBundleRef \"{key}\" specified asset bundle name is empty"));
            }
            if path.is_empty() {
                reports.push(format!("MasterBundleRef \"{key}\" specified asset path is empty"));
            }
            (name, path)
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
