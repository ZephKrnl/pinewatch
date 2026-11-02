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

use crate::landscape_material::LandscapeMaterialAsset;
use crate::local::Local;
use crate::master_bundle::{MasterBundleConfig, asset_bundle_version};
use crate::mapping::AssetMapping;
use crate::origin::{OriginId, Origins};
use crate::populate::populate;
use crate::paths::{MonoPath, STYLE};
use crate::reader::{AssetDefinition, ResultItem};
use crate::references::AssetReference;
use crate::types::{AssetCategory, AssetType, AssetTypeResolver, EAssetType, legacy_asset_type};

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
    /// `FriendlyName`, once the asset's `PopulateAsset` has run.
    pub friendly_name: String,
    /// `assetCategory`; `RedirectorAsset` reads it from its data.
    pub category: AssetCategory,
    /// `ItemAsset.isPro`.
    pub is_pro: bool,
    /// `hasBeenReplaced`: an asset with the same ID or GUID replaced this one in the mapping.
    pub has_been_replaced: bool,
    /// `RedirectorAsset.TargetGuid`, for redirectors.
    pub target_guid: Option<Guid>,
    /// The fields `LandscapeMaterialAsset.PopulateAsset` reads, for landscape materials.
    pub landscape_material: Option<LandscapeMaterialAsset>,
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
    /// The assets `LoadFile` created, in loading order. One whose `PopulateAsset` threw is here too, but in no
    /// origin's list and not in the index.
    pub headers: Vec<AssetHeader>,
    /// `defaultAssetMapping`.
    default_mapping: AssetMapping,
    /// `Assets.hasUnlinkedSpawns`.
    has_unlinked_spawns: bool,
    /// Each origin's `assets` list, as (origin, asset) in the order they were added.
    origin_assets: Vec<(OriginId, usize)>,
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
            default_mapping: AssetMapping::new(),
            has_unlinked_spawns: false,
            origin_assets: Vec::new(),
        }
    }

    /// `defaultAssetMapping`, which is also the current mapping outside a server.
    pub fn mapping(&self) -> &AssetMapping {
        &self.default_mapping
    }

    /// `Assets.hasUnlinkedSpawns`: a spawn asset was added since spawns were last linked.
    pub fn has_unlinked_spawns(&self) -> bool {
        self.has_unlinked_spawns
    }

    /// `Assets.find(Guid)`: the asset with this GUID in the current mapping, with each redirector replaced by the
    /// asset its target GUID finds. The 33rd redirect gives up with a warning.
    pub fn find(&self, guid: Guid, log: &mut dyn LogSink) -> Option<usize> {
        let found = self.default_mapping.asset_dictionary.try_get_value(&guid).copied();
        let message = || format!("Infinite asset director loop encountered when resolving: {guid}");
        self.follow_redirectors(found, log, message)
    }

    /// `Assets.find(EAssetType, ushort)`: nothing for `NONE` or ID 0; otherwise the legacy table's asset, with
    /// redirectors followed as `find` does. The table lookup throws for a category the mapping doesn't hold.
    pub fn find_legacy(
        &self,
        category: AssetCategory,
        id: u16,
        log: &mut dyn LogSink,
    ) -> Result<Option<usize>, MonoException> {
        if category == AssetCategory::from(EAssetType::None) || id == 0 {
            return Ok(None);
        }
        let found = self.default_mapping.table(category)?.try_get_value(&id).copied();
        let message =
            || format!("Infinite asset director loop encountered when resolving Type: {category} Legacy ID: {id}");
        Ok(self.follow_redirectors(found, log, message))
    }

    /// Both `find`s' loop: while the result is a redirector, the asset its target GUID finds replaces it. The 33rd
    /// redirect gives up with the warning `message` makes.
    fn follow_redirectors(
        &self,
        mut result: Option<usize>,
        log: &mut dyn LogSink,
        message: impl FnOnce() -> String,
    ) -> Option<usize> {
        let dictionary = &self.default_mapping.asset_dictionary;
        let mut redirects = 0;
        // `target_guid` is set exactly for redirectors.
        while let Some(target) = result.and_then(|index| self.headers[index].target_guid) {
            result = dictionary.try_get_value(&target).copied();
            redirects += 1;
            if redirects > 32 {
                log.log(LogLevel::Warn, &message());
                return None;
            }
        }
        result
    }

    /// `Assets.find<LandscapeMaterialAsset>(reference)`: nothing for an invalid reference, else what `find` finds
    /// if it is a landscape material, with its index.
    pub fn find_landscape_material(
        &self,
        reference: AssetReference,
        log: &mut dyn LogSink,
    ) -> Option<(usize, &LandscapeMaterialAsset)> {
        if !reference.is_valid() {
            return None;
        }
        let index = self.find(reference.guid, log)?;
        self.headers[index].landscape_material.as_ref().map(|material| (index, material))
    }

    /// `origin.GetAssets()`: the assets added for this origin, in order.
    pub fn origin_assets(&self, origin: OriginId) -> impl Iterator<Item = usize> + '_ {
        self.origin_assets.iter().filter(move |(owner, _)| *owner == origin).map(|(_, index)| *index)
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

    /// `Assets.ReportError(asset, message)`: marks the asset and reports the message after its prefix,
    /// `<origin> <friendly name> (<type>) [<guid:N>]`.
    fn report_asset_error(&mut self, index: usize, message: &str, log: &mut dyn LogSink) {
        let header = &mut self.headers[index];
        header.has_errors = true;
        let prefix = format!(
            "{} {} ({}) [{}]",
            self.origins.get(header.origin).name,
            header.friendly_name,
            header.asset_type.friendly_type_name(),
            header.guid.to_string_n()
        );
        self.report_error(format!("{prefix}: {message}"), log);
    }

    /// `FriendlyNameWithFriendlyType` and `GetOriginName()` of an asset, as the index messages name it.
    fn describe(&self, index: usize) -> (String, String) {
        let header = &self.headers[index];
        let name = format!("{} ({})", header.friendly_name, header.asset_type.friendly_type_name());
        (name, self.origins.get(header.origin).name.clone())
    }

    /// `AddToMapping(asset, overrideExistingID, defaultAssetMapping)`. `Err` is an exception it throws (the legacy
    /// table indexer's, for a category the mapping doesn't hold).
    fn add_to_mapping(
        &mut self,
        index: usize,
        override_existing_id: bool,
        log: &mut dyn LogSink,
    ) -> Result<(), MonoException> {
        let category = self.headers[index].category;
        if category == AssetCategory::from(EAssetType::Spawn) {
            self.has_unlinked_spawns = true;
        }
        let id = self.headers[index].id;
        if category == AssetCategory::from(EAssetType::Object) {
            let table = self.default_mapping.table_mut(category)?;
            if override_existing_id {
                let replaced = table.remove(&id);
                table.add(id, index)?;
                if let Some(existing) = replaced {
                    self.headers[existing].has_been_replaced = true;
                }
            } else if !table.contains_key(&id) {
                table.add(id, index)?;
            }
        } else if category != AssetCategory::from(EAssetType::None) {
            if id != 0 {
                let table = self.default_mapping.table_mut(category)?;
                if override_existing_id {
                    if let Some(existing) = table.remove(&id) {
                        self.headers[existing].has_been_replaced = true;
                    }
                } else if let Some(&other) = table.try_get_value(&id) {
                    let (other_name, other_origin) = self.describe(other);
                    let message = format!("legacy ID {id} already taken by {other_name} in {other_origin}!");
                    self.report_asset_error(index, &message, log);
                    return Ok(());
                }
                self.default_mapping.table_mut(category)?.add(id, index)?;
            } else {
                let needs_legacy_id = match category.known() {
                    Some(EAssetType::Item) => !self.headers[index].is_pro,
                    Some(EAssetType::Effect | EAssetType::Vehicle | EAssetType::Spawn | EAssetType::Npc) => false,
                    _ => true,
                };
                if needs_legacy_id {
                    self.report_asset_error(index, "needs a non-zero ID", log);
                }
            }
        }

        let guid = self.headers[index].guid;
        if guid != Guid::EMPTY {
            if override_existing_id {
                if let Some(&existing) = self.default_mapping.asset_dictionary.try_get_value(&guid) {
                    let existing_guid = self.headers[existing].guid;
                    self.default_mapping.asset_dictionary.remove(&existing_guid);
                    // `assetList.Remove(existingAsset)`: the first entry for that asset.
                    let list = &mut self.default_mapping.asset_list;
                    if let Some(position) = list.iter().position(|&entry| entry == existing) {
                        list.remove(position);
                    }
                    self.headers[existing].has_been_replaced = true;
                }
            } else if let Some(&other) = self.default_mapping.asset_dictionary.try_get_value(&guid) {
                let (other_name, other_origin) = self.describe(other);
                self.report_asset_error(index, &format!("GUID already taken by {other_name} in {other_origin}!"), log);
                return Ok(());
            }
            self.default_mapping.asset_dictionary.add(guid, index)?;
            self.default_mapping.asset_list.push(index);
        }

        self.default_mapping.modification_counter += 1;
        // The vehicle respawn check needs a loaded level, and workshop logging is off (`-LogWorkshopAssets`).
        Ok(())
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

        let asset_directory =
            mono_path::get_directory_name(STYLE, &asset_path)?.ok_or_else(MonoException::null_reference)?;
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
                            let message = format!("Unhandled asset type \"{legacy_type}\" in \"{asset_path}\"");
                            self.report_error(message, log);
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
            let hashable_path =
                format!("{}/{}", to_lower_invariant(&relative_text), to_lower_invariant(&asset_internal_name));
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
                format!(
                    "{asset_internal_name} Lowest individual asset bundle version is 1 (default), associated with 5.5."
                ),
                log,
            );
            individual_version = asset_bundle_version::UNITY_5;
        } else if individual_version > asset_bundle_version::NEWEST {
            self.report_error(
                format!(
                    "{asset_internal_name} Highest individual asset bundle version is 6, associated with 2022 LTS."
                ),
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
        let mut header = AssetHeader {
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
            friendly_name: String::new(),
            category: AssetCategory::from(asset_type.category()),
            is_pro: false,
            has_been_replaced: false,
            target_guid: None,
            landscape_material: None,
        };

        // `asset.PopulateAsset(…)`, then `asset.origin.assets.Add(asset)` and `AddToMapping`, all in one `try`.
        // `Assets.currentMasterBundle` is the master bundle chosen above.
        let current_master_bundle = master_bundle.map(|index| self.master_bundles[index].asset_bundle_name.clone());
        let (reports, populated) = populate(&mut header, current_master_bundle);
        let index = self.headers.len();
        self.headers.push(header);
        for report in reports {
            self.report_asset_error(index, &report, log);
        }
        let outcome = populated.and_then(|()| {
            self.origin_assets.push((origin, index));
            let override_existing_ids = self.origins.get(origin).should_assets_override_existing_ids;
            self.add_to_mapping(index, override_existing_ids, log)
        });
        if let Err(exception) = outcome {
            self.report_error(
                format!("Caught exception while populating \"{asset_path}\": {}", exception_message(&exception)),
                log,
            );
            log_exception_only(log, &exception);
        }
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
