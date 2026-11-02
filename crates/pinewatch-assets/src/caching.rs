//! `CachingAssetRef` and `CachingBcAssetRef` (`Bundles/CachingAssetRef.cs`, `Bundles/CachingBcAssetRef.cs`): asset
//! references by GUID (the second also by legacy type and ID) that keep the asset they last found, with
//! `UnturnedDatEx`'s readers for them. Assets are indexes into [`AssetLoader::headers`].

use std::hash::{Hash, Hasher};

use pinewatch_dat::{DatParseable, DictRef, NodeRef};
use pinewatch_io::LogSink;
use pinewatch_mono::{
    Guid, MonoException, NetEnum, NetString, NumberFormat, NumberStyles, try_parse_enum, try_parse_u16,
};

use crate::load::AssetLoader;
use crate::types::AssetCategory;

/// `EAssetType.NONE`.
const NONE: AssetCategory = AssetCategory(0);

/// `CachingAssetRef`: an asset GUID and the asset it last found. Equality and hashing use the GUID only.
#[derive(Debug, Clone, Copy, Default)]
pub struct CachingAssetRef {
    guid: Guid,
    cached: Option<usize>,
}

impl PartialEq for CachingAssetRef {
    fn eq(&self, other: &Self) -> bool {
        self.guid == other.guid
    }
}

impl Eq for CachingAssetRef {}

impl Hash for CachingAssetRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.guid.hash(state);
    }
}

impl CachingAssetRef {
    /// `Empty`.
    pub const EMPTY: Self = Self { guid: Guid::EMPTY, cached: None };

    /// `new CachingAssetRef(guid)`.
    pub fn new(guid: Guid) -> Self {
        Self { guid, cached: None }
    }

    /// `IsEmpty`: the GUID is empty.
    pub fn is_empty(&self) -> bool {
        self.guid == Guid::EMPTY
    }

    /// `IsAssigned`.
    pub fn is_assigned(&self) -> bool {
        !self.is_empty()
    }

    /// `Guid`: the GUID it was given, not necessarily the found asset's.
    pub fn guid(&self) -> Guid {
        self.guid
    }

    /// The asset `get` last found.
    pub fn cached(&self) -> Option<usize> {
        self.cached
    }

    /// `Get()`: the cached asset, looked up again with `Assets.find` when there is none or it has been replaced.
    pub fn get(&mut self, loader: &AssetLoader, log: &mut dyn LogSink) -> Option<usize> {
        if self.cached.is_none_or(|index| loader.headers[index].has_been_replaced) {
            self.cached = loader.find(self.guid, log);
        }
        self.cached
    }

    /// `IsReferenceTo(asset)`: the cached asset is compared when there is one and `asset` hasn't been replaced, the
    /// GUID otherwise; no asset matches an empty reference.
    pub fn is_reference_to(&self, loader: &AssetLoader, asset: Option<usize>) -> bool {
        let Some(asset) = asset else {
            return self.guid == Guid::EMPTY;
        };
        if let Some(cached) = self.cached
            && !loader.headers[asset].has_been_replaced
        {
            return cached == asset;
        }
        self.guid == loader.headers[asset].guid
    }

    /// `Clear()`.
    pub fn clear(&mut self) {
        *self = Self::EMPTY;
    }

    /// `ToString()`, which looks the asset up for its friendly name.
    pub fn format(&mut self, loader: &AssetLoader, log: &mut dyn LogSink) -> String {
        let name = asset_name(loader, self.get(loader, log));
        format!("(ID: {}, Asset: {name})", self.guid.to_string_n())
    }

    /// `TryParse(string, out result)`: `None` for null, empty and `0`, and for text `Guid.TryParse` refuses.
    pub fn try_parse_str(input: Option<&NetString>) -> Option<Self> {
        let input = input.filter(|input| !input.is_empty() && input.as_units() != [u16::from(b'0')])?;
        Guid::try_parse(Some(input)).map(Self::new)
    }

    /// `Parse(string)`: `Empty` where `try_parse_str` fails.
    pub fn parse_str(input: Option<&NetString>) -> Self {
        Self::try_parse_str(input).unwrap_or(Self::EMPTY)
    }

    /// `TryParse(IDatNode, out result)`: a value's text, or a dictionary's `GUID`.
    pub fn try_parse_node(node: NodeRef<'_>) -> Option<Self> {
        match node {
            NodeRef::Value(value) => Self::try_parse_str(value.value()),
            NodeRef::Dictionary(dictionary) => dictionary.try_parse_guid("GUID").map(Self::new),
            NodeRef::List(_) => None,
        }
    }
}

impl DatParseable for CachingAssetRef {
    type Context = ();

    /// `IDatParseable.TryParse`: a value replaces the whole reference; a dictionary sets only the GUID (`Guid.Empty`
    /// when it doesn't parse).
    fn try_parse(&mut self, node: NodeRef<'_>, _: &mut ()) -> Result<bool, MonoException> {
        Ok(match node {
            NodeRef::Value(value) => {
                let parsed = Self::try_parse_str(value.value());
                *self = parsed.unwrap_or(Self::EMPTY);
                parsed.is_some()
            }
            NodeRef::Dictionary(dictionary) => {
                let parsed = dictionary.try_parse_guid("GUID");
                self.guid = parsed.unwrap_or(Guid::EMPTY);
                parsed.is_some()
            }
            NodeRef::List(_) => false,
        })
    }
}

/// `CachingBcAssetRef`: an asset GUID, or a legacy type and ID, and the asset it last found. Equality and hashing
/// use the GUID, ID and type.
#[derive(Debug, Clone, Copy, Default)]
pub struct CachingBcAssetRef {
    guid: Guid,
    legacy_id: u16,
    legacy_type: AssetCategory,
    cached: Option<usize>,
}

impl PartialEq for CachingBcAssetRef {
    fn eq(&self, other: &Self) -> bool {
        self.guid == other.guid && self.legacy_id == other.legacy_id && self.legacy_type == other.legacy_type
    }
}

impl Eq for CachingBcAssetRef {}

impl Hash for CachingBcAssetRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.guid, self.legacy_id, self.legacy_type).hash(state);
    }
}

impl From<CachingAssetRef> for CachingBcAssetRef {
    /// `new CachingBcAssetRef(assetRef)`: its GUID and cached asset.
    fn from(reference: CachingAssetRef) -> Self {
        Self { guid: reference.guid, legacy_id: 0, legacy_type: NONE, cached: reference.cached }
    }
}

impl CachingBcAssetRef {
    /// `Empty`.
    pub const EMPTY: Self = Self { guid: Guid::EMPTY, legacy_id: 0, legacy_type: NONE, cached: None };

    /// `new CachingBcAssetRef(guid)`.
    pub fn from_guid(guid: Guid) -> Self {
        Self { guid, ..Self::EMPTY }
    }

    /// `new CachingBcAssetRef(legacyType, legacyId)`: ID 0 keeps no type.
    pub fn from_legacy(legacy_type: AssetCategory, legacy_id: u16) -> Self {
        let legacy_type = if legacy_id > 0 { legacy_type } else { NONE };
        Self { legacy_id, legacy_type, ..Self::EMPTY }
    }

    /// `new CachingBcAssetRef(guid, legacyType, legacyId)`: a legacy ID wins over the GUID, and ID 0 keeps no type.
    pub fn new(guid: Guid, legacy_type: AssetCategory, legacy_id: u16) -> Self {
        Self { guid: if legacy_id > 0 { Guid::EMPTY } else { guid }, ..Self::from_legacy(legacy_type, legacy_id) }
    }

    /// `IsEmpty`: neither a GUID nor a legacy ID.
    pub fn is_empty(&self) -> bool {
        self.guid == Guid::EMPTY && self.legacy_id == 0
    }

    /// `IsAssigned`.
    pub fn is_assigned(&self) -> bool {
        !self.is_empty()
    }

    /// `Guid`, as given.
    pub fn guid(&self) -> Guid {
        self.guid
    }

    /// `LegacyId`, as given.
    pub fn legacy_id(&self) -> u16 {
        self.legacy_id
    }

    /// `LegacyType`, as given.
    pub fn legacy_type(&self) -> AssetCategory {
        self.legacy_type
    }

    /// The asset `get` last found.
    pub fn cached(&self) -> Option<usize> {
        self.cached
    }

    /// `Get()`: the cached asset, looked up again when there is none or it has been replaced: by GUID without a
    /// legacy ID, else by legacy type and ID (which throws for a type the mapping doesn't hold).
    pub fn get(&mut self, loader: &AssetLoader, log: &mut dyn LogSink) -> Result<Option<usize>, MonoException> {
        if self.cached.is_none_or(|index| loader.headers[index].has_been_replaced) {
            self.cached = if self.legacy_id == 0 {
                loader.find(self.guid, log)
            } else {
                loader.find_legacy(self.legacy_type, self.legacy_id, log)?
            };
        }
        Ok(self.cached)
    }

    /// `IsReferenceTo(asset)`: the cached asset when there is one and `asset` hasn't been replaced; else the GUID if
    /// set, else the legacy type and ID (`assetCategory` and `id`) if set. No asset matches an empty reference.
    pub fn is_reference_to(&self, loader: &AssetLoader, asset: Option<usize>) -> bool {
        let Some(asset) = asset else {
            return self.is_empty();
        };
        let header = &loader.headers[asset];
        if let Some(cached) = self.cached
            && !header.has_been_replaced
        {
            return cached == asset;
        }
        if self.guid != Guid::EMPTY {
            self.guid == header.guid
        } else if self.legacy_id != 0 {
            self.legacy_type == header.category && self.legacy_id == header.id
        } else {
            false
        }
    }

    /// `Clear()`.
    pub fn clear(&mut self) {
        *self = Self::EMPTY;
    }

    /// `ToString()`, which looks the asset up for its friendly name.
    pub fn format(&mut self, loader: &AssetLoader, log: &mut dyn LogSink) -> Result<String, MonoException> {
        let name = asset_name(loader, self.get(loader, log)?);
        Ok(if self.legacy_id == 0 {
            format!("(GUID: {}, Asset: {name})", self.guid.to_string_n())
        } else {
            format!("(Legacy Type: {}, Legacy ID: {}, Asset: {name})", self.legacy_type, self.legacy_id)
        })
    }

    /// `TryParse(string, defaultLegacyType, out result)`. Null, empty and `0` fail. With a `:` after the first
    /// character, the part before it is an `EAssetType` (ignoring case) and the part after a `ushort`; a `NONE`
    /// type succeeds as `Empty`. Otherwise the text is a legacy ID of `default_legacy_type` when that isn't `NONE`
    /// and the text is a `ushort`, else a GUID. `ushort.TryParse` uses the current culture (V29).
    pub fn try_parse_str(input: Option<&NetString>, default_legacy_type: AssetCategory) -> (bool, Self) {
        let Some(input) = input.filter(|input| !input.is_empty() && input.as_units() != [u16::from(b'0')]) else {
            return (false, Self::EMPTY);
        };
        let units = input.as_units();
        match units.iter().position(|&unit| unit == u16::from(b':')) {
            Some(delimiter) if delimiter > 0 => {
                let type_text = NetString::from_units(units[..delimiter].to_vec());
                let Some(bits) = try_parse_enum(&AssetCategory::INFO, Some(&type_text), true) else {
                    return (false, Self::EMPTY);
                };
                let legacy_type = AssetCategory::from_bits(bits);
                if legacy_type == NONE {
                    return (true, Self::EMPTY);
                }
                match parse_ushort(&NetString::from_units(units[delimiter + 1..].to_vec())) {
                    // Even ID 0 parses.
                    Some(legacy_id) => (true, Self::from_legacy(legacy_type, legacy_id)),
                    None => (false, Self::EMPTY),
                }
            }
            _ => {
                if default_legacy_type != NONE
                    && let Some(legacy_id) = parse_ushort(input)
                {
                    return (true, Self::from_legacy(default_legacy_type, legacy_id));
                }
                match Guid::try_parse(Some(input)) {
                    Some(guid) => (true, Self::from_guid(guid)),
                    None => (false, Self::EMPTY),
                }
            }
        }
    }

    /// `Parse(string, defaultLegacyType)`: the `try_parse_str` result, `Empty` when it fails.
    pub fn parse_str(input: Option<&NetString>, default_legacy_type: AssetCategory) -> Self {
        Self::try_parse_str(input, default_legacy_type).1
    }

    /// `TryParse(IDatNode, defaultLegacyType, out result)`: a value's text, or a dictionary's `GUID`, else its
    /// `Type` and `ID` (through the constructor, with no special case for `NONE`).
    pub fn try_parse_node(node: NodeRef<'_>, default_legacy_type: AssetCategory) -> (bool, Self) {
        match node {
            NodeRef::Value(value) => Self::try_parse_str(value.value(), default_legacy_type),
            NodeRef::Dictionary(dictionary) => {
                if let Some(guid) = dictionary.try_parse_guid("GUID") {
                    return (true, Self::from_guid(guid));
                }
                match (dictionary.try_parse_enum::<AssetCategory>("Type"), dictionary.try_parse_uint16("ID")) {
                    (Some(legacy_type), Some(legacy_id)) => (true, Self::from_legacy(legacy_type, legacy_id)),
                    _ => (false, Self::EMPTY),
                }
            }
            NodeRef::List(_) => (false, Self::EMPTY),
        }
    }
}

impl DatParseable for CachingBcAssetRef {
    type Context = ();

    /// `IDatParseable.TryParse`, with no default type. A value replaces the whole reference. A dictionary writes
    /// each field as it parses: the GUID (`Guid.Empty` when it doesn't parse), then the type and the ID as parsed,
    /// without the constructor's rules, so a failure can leave a parsed type behind.
    fn try_parse(&mut self, node: NodeRef<'_>, _: &mut ()) -> Result<bool, MonoException> {
        Ok(match node {
            NodeRef::Value(value) => {
                let (parsed, reference) = Self::try_parse_str(value.value(), NONE);
                *self = reference;
                parsed
            }
            NodeRef::Dictionary(dictionary) => {
                let guid = dictionary.try_parse_guid("GUID");
                self.guid = guid.unwrap_or(Guid::EMPTY);
                if guid.is_some() {
                    return Ok(true);
                }
                let legacy_type = dictionary.try_parse_enum::<AssetCategory>("Type");
                self.legacy_type = legacy_type.unwrap_or(NONE);
                // `&&` skips the ID when the type fails.
                if legacy_type.is_none() {
                    return Ok(false);
                }
                let legacy_id = dictionary.try_parse_uint16("ID");
                self.legacy_id = legacy_id.unwrap_or(0);
                legacy_id.is_some()
            }
            NodeRef::List(_) => false,
        })
    }
}

/// `ushort.TryParse(text, out value)`: `NumberStyles.Integer`, in the invariant culture for now (V29).
fn parse_ushort(text: &NetString) -> Option<u16> {
    // `NumberStyles.Integer` is a valid style, so the parse never returns `Err`.
    try_parse_u16(Some(text), NumberStyles::INTEGER, &NumberFormat::invariant()).ok().flatten()
}

/// `asset?.FriendlyName ?? "null"`.
fn asset_name(loader: &AssetLoader, asset: Option<usize>) -> String {
    asset.map_or_else(|| "null".to_string(), |index| loader.headers[index].friendly_name.clone())
}

/// `TryParseAssetRef(key, out assetRef)`: `CachingAssetRef.TryParse` on the node under `key`.
pub fn try_parse_asset_ref(dictionary: &DictRef<'_>, key: &str) -> Option<CachingAssetRef> {
    CachingAssetRef::try_parse_node(dictionary.try_get_node(key)?)
}

/// `ParseAssetRef(key)`: `Empty` where `try_parse_asset_ref` fails.
pub fn parse_asset_ref(dictionary: &DictRef<'_>, key: &str) -> CachingAssetRef {
    try_parse_asset_ref(dictionary, key).unwrap_or(CachingAssetRef::EMPTY)
}

/// `TryParseBcAssetRef(key, defaultLegacyType, out assetRef)`: `CachingBcAssetRef.TryParse` on the node under `key`.
pub fn try_parse_bc_asset_ref(
    dictionary: &DictRef<'_>,
    key: &str,
    default_legacy_type: AssetCategory,
) -> Option<CachingBcAssetRef> {
    match CachingBcAssetRef::try_parse_node(dictionary.try_get_node(key)?, default_legacy_type) {
        (true, reference) => Some(reference),
        (false, _) => None,
    }
}

/// `ParseGuidOrLegacyIdV2(key, defaultLegacyType)`: what `try_parse_bc_asset_ref` gives, `Empty` when it fails.
pub fn parse_guid_or_legacy_id_v2(
    dictionary: &DictRef<'_>,
    key: &str,
    default_legacy_type: AssetCategory,
) -> CachingBcAssetRef {
    try_parse_bc_asset_ref(dictionary, key, default_legacy_type).unwrap_or(CachingBcAssetRef::EMPTY)
}
