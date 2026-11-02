//! The main thread's side of asset loading (`LoadAssetsFromWorkerThread`, `TryLoadFile` and `LoadFile` in
//! `Bundles/Assets.cs`), up to creating the asset: its header.
//!
//! Master bundles count as loaded as soon as they arrive (V25). Definitions wait until the core master bundle is
//! there, then load in arrival order, as in the original.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};

use pinewatch_dat::{DatTree, DictRef};
use pinewatch_io::mono_path;
use pinewatch_io::{LogLevel, LogSink, log_exception, log_exception_only};
use pinewatch_mono::{ExceptionKind, Guid, MonoException, NetString, chars, sr};
use sha1::{Digest, Sha1};

use crate::local::Local;
use crate::master_bundle::{MasterBundleConfig, asset_bundle_version};
use crate::origin::{OriginId, Origins};
use crate::paths::{MonoPath, STYLE};
use crate::reader::{AssetDefinition, ResultItem};
use crate::types::{AssetType, AssetTypeResolver, legacy_asset_type};

/// What `LoadFile` knows about an asset when it creates it: the asset's fields before its own `PopulateAsset`.
#[derive(Debug)]
pub struct AssetHeader {
    pub path: MonoPath,
    /// The file name without extension, or the folder name for `Asset.dat`.
    pub internal_name: String,
    /// `Asset.name` after the base `PopulateAsset`: the bundle's name.
    pub name: String,
    pub guid: Guid,
    pub asset_type: AssetType,
    /// The legacy 16-bit ID (`ID`).
    pub id: u16,
    /// The definition's hash, combined with its bundle path's for a master bundle asset.
    pub hash: [u8; 20],
    /// The master bundle the asset loads from, as an index into [`AssetLoader::master_bundles`].
    pub master_bundle: Option<usize>,
    /// The path inside the master bundle, with `/` separators.
    pub bundle_relative_path: Option<String>,
    /// The legacy per-asset bundle (`.unity3d`) path, outside master bundles.
    pub legacy_bundle_path: Option<String>,
    pub convert_shaders_to_standard: bool,
    pub consolidate_shaders: bool,
    /// The parser reported errors for the definition.
    pub has_errors: bool,
    pub origin: OriginId,
    pub keep_localization_loaded: bool,
    pub ignore_npot: bool,
    pub ignore_texture_readable: bool,
    /// The asset's data is the `Asset` dictionary rather than the whole file.
    pub uses_asset_dictionary: bool,
    /// The parsed definition and its translations, for the asset's own `PopulateAsset`.
    pub definition: AssetDefinition,
}

impl AssetHeader {
    /// The dictionary `LoadFile` calls `data`.
    pub fn data(&self) -> DictRef<'_> {
        data_dictionary(&self.definition.asset_data)
    }

    /// `new Local(file.translationData, file.fallbackTranslationData)`.
    pub fn localization(&self) -> Local<'_> {
        Local::new(
            self.definition.translation_data.as_ref().map(DatTree::root),
            self.definition.fallback_translation_data.as_ref().map(DatTree::root),
        )
    }
}

/// `rootData.TryGetDictionary("Asset", out assetData) ? assetData : rootData`.
fn data_dictionary(tree: &DatTree) -> DictRef<'_> {
    tree.root().try_get_dictionary("Asset").unwrap_or_else(|| tree.root())
}

/// The asset loading state of `Assets` that this part of the port keeps.
#[derive(Debug)]
pub struct AssetLoader {
    origins: Origins,
    /// `allMasterBundles`, in arrival order.
    pub master_bundles: Vec<MasterBundleConfig>,
    core_master_bundle: Option<usize>,
    pending: VecDeque<AssetDefinition>,
    /// `Assets.errors`: every `reportError` text.
    pub errors: Vec<String>,
    /// The headers of the definitions `LoadFile` got as far as creating an asset for, in loading order.
    pub headers: Vec<AssetHeader>,
}

impl AssetLoader {
    pub fn new(origins: Origins) -> Self {
        Self {
            origins,
            master_bundles: Vec::new(),
            core_master_bundle: None,
            pending: VecDeque::new(),
            errors: Vec::new(),
            headers: Vec::new(),
        }
    }

    pub fn origins(&self) -> &Origins {
        &self.origins
    }

    /// `coreMasterBundle`, as an index into `master_bundles`.
    pub fn core_master_bundle(&self) -> Option<usize> {
        self.core_master_bundle
    }

    /// Definitions waiting for the core master bundle. If it never comes, the original keeps waiting forever.
    pub fn pending(&self) -> usize {
        self.pending.len()
    }

    /// One location's results, in order, as `LoadAssetsFromWorkerThread` takes them.
    pub fn take_results(&mut self, results: Vec<ResultItem>, log: &mut dyn LogSink) {
        for item in results {
            match item {
                ResultItem::MasterBundle { config, .. } => {
                    // `FinishLoad` and the checks after it, with the bundle loaded (V25). `string.Equals(…,
                    // InvariantCulture)` for this ASCII name compares like `==`.
                    if config.origin == self.origins.core() && config.asset_bundle_name == "core.masterbundle" {
                        self.core_master_bundle = Some(self.master_bundles.len());
                    }
                    self.master_bundles.push(config);
                }
                ResultItem::Asset(definition) => self.pending.push_back(definition),
                ResultItem::Exception { message, exception } => log_exception(log, &exception, &message),
            }
            if self.core_master_bundle.is_some() {
                while let Some(definition) = self.pending.pop_front() {
                    self.try_load_file(definition, log);
                }
            }
        }
    }

    /// `Assets.reportError`.
    fn report_error(&mut self, error: String, log: &mut dyn LogSink) {
        log.log(LogLevel::Warn, &error);
        self.errors.push(error);
    }

    /// `TryLoadFile`.
    fn try_load_file(&mut self, file: AssetDefinition, log: &mut dyn LogSink) {
        let path = file.path.full.clone();
        if let Err(exception) = self.load_file(file, log) {
            log.log(LogLevel::Error, &format!("Exception loading file {path}:"));
            log_exception_only(log, &exception);
        }
    }

    /// `findMasterBundleByPath`: the master bundle whose folder holds `path` most closely. `path.StartsWith` is a
    /// culture-sensitive comparison in the C#, compared ordinally here (V26).
    fn find_master_bundle_by_path(&self, path: &[u16]) -> Option<usize> {
        let mut longest = 0;
        let mut best = None;
        for (index, master_bundle) in self.master_bundles.iter().enumerate() {
            let directory: Vec<u16> = master_bundle.directory_path.full.encode_utf16().collect();
            if directory.len() < longest || !path.starts_with(&directory) {
                continue;
            }
            if path.len() > directory.len() {
                let next = path[directory.len()];
                if next != u16::from(b'/') && next != u16::from(b'\\') {
                    continue;
                }
            }
            longest = directory.len();
            best = Some(index);
        }
        best
    }

    /// `findMasterBundleByName(name)`: the first bundle with this file name, compared with
    /// `InvariantCultureIgnoreCase` in the C#, ordinally ignoring case here (V26).
    fn find_master_bundle_by_name(&self, name: &NetString) -> Option<usize> {
        self.master_bundles.iter().position(|master_bundle| {
            let candidate: Vec<u16> = master_bundle.asset_bundle_name.encode_utf16().collect();
            chars::equals_ordinal_ignore_case(&candidate, name.as_units())
        })
    }

    /// `LoadFile`, up to `Activator.CreateInstance`. `Err` is an exception `TryLoadFile` catches.
    fn load_file(&mut self, file: AssetDefinition, log: &mut dyn LogSink) -> Result<(), MonoException> {
        let asset_path = file.path.full.clone();
        let asset_path_units: Vec<u16> = asset_path.encode_utf16().collect();

        if asset_path_units.len() > 260 {
            self.report_error(
                format!("Asset path exceeds 260 characters and might not load properly on Windows: \"{asset_path}\""),
                log,
            );
        }
        if let Some(errors) = &file.asset_errors {
            for error in errors.clone() {
                self.report_error(format!("Error parsing \"{asset_path}\": \"{}\"", error.to_string_lossy()), log);
            }
        }

        let asset_directory = mono_path::get_directory_name(STYLE, &asset_path)?.ok_or_else(MonoException::null_reference)?;
        let suffix: Vec<u16> = "Asset.dat".encode_utf16().collect();
        let ends_with_asset_dat = asset_path_units.len() >= suffix.len()
            && chars::equals_ordinal_ignore_case(&asset_path_units[asset_path_units.len() - suffix.len()..], &suffix);
        let asset_internal_name = if ends_with_asset_dat {
            mono_path::get_file_name(STYLE, &asset_directory)?
        } else {
            mono_path::get_file_name_without_extension(STYLE, &asset_path)?
        };

        let root = file.asset_data.root();
        let asset_guid: Guid;
        let mut asset_type = None;
        if let Some(metadata) = root.try_get_dictionary("Metadata") {
            let Some(guid) = metadata.try_parse_guid("GUID") else {
                self.report_error(format!("Unable to parse Metadata.GUID in \"{asset_path}\""), log);
                return Ok(());
            };
            asset_guid = guid;
            let Some(handle) = metadata.parse_type("Type", &AssetTypeResolver, None)? else {
                self.report_error(format!("Unable to parse Metadata.Type in \"{asset_path}\""), log);
                return Ok(());
            };
            asset_type = AssetType::from_handle(handle);
        } else if !root.contains_key("GUID") {
            // The original writes a new GUID into the file; Pinewatch never writes to the install (L3).
            asset_guid = new_guid();
        } else if let Some(guid) = root.try_parse_guid("GUID") {
            asset_guid = guid;
        } else {
            self.report_error(format!("Unable to parse GUID in \"{asset_path}\""), log);
            return Ok(());
        }

        if asset_guid == Guid::EMPTY {
            self.report_error(format!("Cannot use empty GUID in \"{asset_path}\""), log);
            return Ok(());
        }

        let uses_asset_dictionary = root.try_get_dictionary("Asset").is_some();
        let data = data_dictionary(&file.asset_data);

        let asset_type = match asset_type {
            Some(asset_type) => asset_type,
            None => {
                let legacy_type = data.get_string("Type", None).map(NetString::to_string_lossy).unwrap_or_default();
                if legacy_type.is_empty() {
                    self.report_error(format!("Missing asset Type in \"{asset_path}\""), log);
                    return Ok(());
                }
                match legacy_asset_type(&legacy_type) {
                    Some(asset_type) => asset_type,
                    None => match data.parse_type("Type", &AssetTypeResolver, None)?.and_then(AssetType::from_handle) {
                        Some(asset_type) => asset_type,
                        None => {
                            self.report_error(format!("Unhandled asset type \"{legacy_type}\" in \"{asset_path}\""), log);
                            return Ok(());
                        }
                    },
                }
            }
        };
        // Every type the resolver knows derives from `Asset`, so the `IsAssignableFrom` check always passes.

        let mut master_bundle = self.find_master_bundle_by_path(&asset_path_units);
        let override_name = data.get_string("Master_Bundle_Override", None).cloned();
        if let Some(override_name) = &override_name {
            master_bundle = self.find_master_bundle_by_name(override_name);
            if master_bundle.is_none() {
                log.log(
                    LogLevel::Warn,
                    &format!(
                        "Unable to find master bundle override '{}' for '{asset_path}'",
                        override_name.to_string_lossy()
                    ),
                );
            }
        } else if data.contains_key("Exclude_From_Master_Bundle") {
            master_bundle = None;
        }
        // A master bundle whose asset bundle failed to load is skipped with a warning; every bundle loads here (V25).

        let mut data_hash = file.hash;
        let mut master_bundle_version = -1;
        let mut bundle_relative_path = None;
        let mut legacy_bundle_path = None;
        let bundle_name;
        if let Some(index) = master_bundle {
            let config = &self.master_bundles[index];
            let relative = match data.try_get_string("Bundle_Override_Path") {
                // A flag: `TryGetString` succeeds with null.
                Some(value) => value.map(NetString::to_string_lossy),
                None => {
                    let directory_length = config.directory_path.full.encode_utf16().count();
                    let source = if !data.parse_bool("Bundle_Path_Include_Filename", false) {
                        asset_directory.clone()
                    } else {
                        mono_path::change_extension_to_none(STYLE, &asset_path)?
                    };
                    Some(substring(&source, directory_length)?.replace('\\', "/"))
                }
            };
            bundle_name = asset_internal_name.clone();
            // `bundleRelativePath.ToLowerInvariant()` throws for a null path.
            let relative_text = relative.clone().ok_or_else(MonoException::null_reference)?;
            let hashable_path = format!("{}/{}", to_lower_invariant(&relative_text), to_lower_invariant(&asset_internal_name));
            let path_hash: [u8; 20] = Sha1::digest(hashable_path.as_bytes()).into();
            let mut both = data_hash.to_vec();
            both.extend_from_slice(&path_hash);
            data_hash = Sha1::digest(&both).into();
            master_bundle_version = config.version;
            bundle_relative_path = relative;
        } else if data.contains_key("Bundle_Override_Path") {
            let override_path =
                data.get_string("Bundle_Override_Path", None).cloned().ok_or_else(MonoException::null_reference)?;
            let override_path = override_path.to_string_lossy();
            let file_name = match override_path.rfind('/') {
                Some(slash) => &override_path[slash + 1..],
                None => &override_path[..],
            };
            let path = format!("{override_path}/{file_name}.unity3d");
            legacy_bundle_resource(&path)?;
            bundle_name = asset_internal_name.clone();
            legacy_bundle_path = Some(path);
        } else {
            let path = format!("{asset_directory}/{asset_internal_name}.unity3d");
            bundle_name = mono_path::get_file_name_without_extension(STYLE, &path)?;
            legacy_bundle_resource(&path)?;
            legacy_bundle_path = Some(path);
        }

        let mut individual_version = data.parse_int32("Asset_Bundle_Version", asset_bundle_version::UNITY_5);
        if individual_version < asset_bundle_version::UNITY_5 {
            self.report_error(
                format!("{asset_internal_name} Lowest individual asset bundle version is 1 (default), associated with 5.5."),
                log,
            );
            individual_version = asset_bundle_version::UNITY_5;
        } else if individual_version > asset_bundle_version::NEWEST {
            self.report_error(
                format!("{asset_internal_name} Highest individual asset bundle version is 6, associated with 2022 LTS."),
                log,
            );
            individual_version = asset_bundle_version::NEWEST;
        }
        let effective_version = master_bundle_version.max(individual_version);
        let convert_shaders_to_standard = effective_version < asset_bundle_version::UNITY_2017_LTS;
        let consolidate_shaders = effective_version < asset_bundle_version::UNITY_2018_AND_2019_LTS
            || (data.contains_key("Enable_Shader_Consolidation") && !data.contains_key("Disable_Shader_Consolidation"));

        let legacy_id = data.parse_uint16("ID", 0);

        if asset_type.is_abstract() {
            let exception = abstract_class_exception(asset_type);
            self.report_error(
                format!(
                    "Caught exception while constructing {} in \"{asset_path}\": {}",
                    asset_type.full_name(),
                    exception_message(&exception)
                ),
                log,
            );
            log_exception_only(log, &exception);
            return Ok(());
        }

        let keep_localization_loaded = data.parse_bool("Keep_Localization_Loaded", false);
        let ignore_npot = data.contains_key("Ignore_NPOT");
        let ignore_texture_readable = data.contains_key("Ignore_TexRW");
        let has_errors = file.asset_errors.as_ref().is_some_and(|errors| !errors.is_empty());
        let origin = file.origin;
        self.headers.push(AssetHeader {
            path: file.path.clone(),
            internal_name: asset_internal_name,
            name: bundle_name,
            guid: asset_guid,
            asset_type,
            id: legacy_id,
            hash: data_hash,
            master_bundle,
            bundle_relative_path,
            legacy_bundle_path,
            convert_shaders_to_standard,
            consolidate_shaders,
            has_errors,
            origin,
            keep_localization_loaded,
            ignore_npot,
            ignore_texture_readable,
            uses_asset_dictionary,
            definition: file,
        });
        Ok(())
    }
}

/// `string.Substring(startIndex)`, in UTF-16 code units.
fn substring(text: &str, start_index: usize) -> Result<String, MonoException> {
    let units: Vec<u16> = text.encode_utf16().collect();
    if start_index > units.len() {
        return Err(MonoException::argument_out_of_range_message(
            "startIndex",
            sr::ARGUMENT_OUT_OF_RANGE_START_INDEX_LARGER_THAN_LENGTH,
        ));
    }
    Ok(String::from_utf16_lossy(&units[start_index..]))
}

/// `string.ToLowerInvariant()`.
fn to_lower_invariant(text: &str) -> String {
    let units: Vec<u16> = text.encode_utf16().map(chars::to_lower_invariant).collect();
    String::from_utf16_lossy(&units)
}

/// The legacy `Bundle` constructor's `resource`, `ReadWrite.folderPath(path).Substring(1)`, kept only for the
/// exception it can throw. Loading the `.unity3d` file itself is Unity's (M2d).
fn legacy_bundle_resource(path: &str) -> Result<(), MonoException> {
    let folder = mono_path::get_directory_name(STYLE, path)?.ok_or_else(MonoException::null_reference)?;
    substring(&folder, 1).map(|_| ())
}

/// `getExceptionMessage`: the inner exception's message if there is one.
fn exception_message(exception: &MonoException) -> &str {
    exception.inner.as_deref().map_or(&exception.message, |inner| &inner.message)
}

/// What `Activator.CreateInstance` throws for an abstract class in Mono: `new MissingMethodException(
/// "Cannot create an abstract class '{0}'.", FullName)`, whose two-string constructor takes a class and a member
/// name, so the message is `Method '<that text>.<full name>' not found.`.
fn abstract_class_exception(asset_type: AssetType) -> MonoException {
    let member = format!("Cannot create an abstract class '{{0}}'..{}", asset_type.full_name());
    MonoException::new(ExceptionKind::MissingMethod, sr::MISSING_METHOD_NAME.replace("{0}", &member))
}

/// `Guid.NewGuid()`: a new version-4 GUID. Nothing compares it with the original's, which is random too.
fn new_guid() -> Guid {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut hasher = Sha1::new();
    let time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    hasher.update(time.as_nanos().to_le_bytes());
    hasher.update(std::process::id().to_le_bytes());
    hasher.update(COUNTER.fetch_add(1, Ordering::Relaxed).to_le_bytes());
    let digest: [u8; 20] = hasher.finalize().into();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    // Version 4 in the high nibble of the version field (byte 7 of the .NET layout), RFC 4122 variant in byte 8.
    bytes[7] = (bytes[7] & 0x0F) | 0x40;
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    Guid::from_bytes(bytes)
}
