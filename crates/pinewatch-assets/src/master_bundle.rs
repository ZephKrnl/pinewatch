//! `MasterBundleConfig` (`Bundles/MasterBundleConfig.cs`, `Bundles/MasterBundleHelper.cs`): a folder's
//! `MasterBundle.dat` and the asset bundle it names. The bundle itself is loaded by Unity, which comes with M2d.

use pinewatch_dat::DictRef;
use pinewatch_io::{InstallRoot, mono_path};
use pinewatch_mono::{ExceptionKind, MonoException, NetString};

use crate::origin::OriginId;
use crate::paths::{MonoPath, STYLE};

/// `AssetBundleVersion`.
pub mod asset_bundle_version {
    /// Unity 5.5 and earlier per-asset `.unity3d` file.
    pub const UNITY_5: i32 = 1;
    /// When master bundles were introduced.
    pub const UNITY_2017_LTS: i32 = 2;
    pub const UNITY_2018_AND_2019_LTS: i32 = 3;
    pub const UNITY_2020_LTS: i32 = 4;
    pub const UNITY_2021_LTS: i32 = 5;
    /// 2022 LTS and later.
    pub const NEWEST: i32 = 6;
}

/// A master bundle's configuration, as its constructor builds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterBundleConfig {
    /// The folder holding `MasterBundle.dat` and the bundle.
    pub directory_path: MonoPath,
    /// The bundle's file name, for example `core.masterbundle`.
    pub asset_bundle_name: String,
    /// `asset_bundle_name` without its extension.
    pub asset_bundle_name_without_extension: String,
    /// Put before every asset path loaded from the bundle, for example `Assets/Bundles`.
    pub asset_prefix: String,
    /// The master bundle version (`AssetBundleVersion`).
    pub version: i32,
    pub origin: OriginId,
    /// Whether `<bundle>.hash` exists next to the bundle.
    pub does_hash_file_exist: bool,
}

fn exception(message: impl Into<String>) -> MonoException {
    MonoException::new(ExceptionKind::Exception, message)
}

/// `GetString(key)`: the value, or null.
fn get_string(data: &DictRef<'_>, key: &str) -> Option<String> {
    data.get_string(key, None).map(NetString::to_string_lossy)
}

impl MasterBundleConfig {
    /// `new MasterBundleConfig(absoluteDirectory, data, origin)`, with its exceptions.
    pub fn new(
        install: &InstallRoot,
        directory_path: MonoPath,
        data: &DictRef<'_>,
        origin: OriginId,
    ) -> Result<Self, MonoException> {
        let asset_bundle_name = get_string(data, "Asset_Bundle_Name").unwrap_or_default();
        if asset_bundle_name.is_empty() {
            return Err(exception(
                "Unspecified Asset_Bundle_Name! This should be the file name and extension of the master asset bundle \
                 exported from Unity.",
            ));
        }
        let asset_bundle_name_without_extension = mono_path::get_file_name_without_extension(STYLE, &asset_bundle_name)?;
        let asset_prefix = get_string(data, "Asset_Prefix").unwrap_or_default();
        if asset_prefix.is_empty() {
            return Err(exception(
                "Unspecified Asset_Prefix! This should be the portion of the Unity asset path prior to the /Bundles/ \
                 path, e.g. Assets/Bundles/",
            ));
        }
        let version = if data.contains_key("Master_Bundle_Version") {
            data.parse_int32("Master_Bundle_Version", 0)
        } else {
            data.parse_int32("Asset_Bundle_Version", asset_bundle_version::UNITY_2017_LTS)
        };
        if version < asset_bundle_version::UNITY_2017_LTS {
            return Err(exception("Lowest master bundle version is 2 (default), associated with 2017.4 LTS."));
        } else if version > asset_bundle_version::NEWEST {
            return Err(exception("Highest master bundle version is 6, associated with 2022 LTS."));
        }
        let mut config = Self {
            directory_path,
            asset_bundle_name,
            asset_bundle_name_without_extension,
            asset_prefix,
            version,
            origin,
            does_hash_file_exist: false,
        };
        let expected_path = config.asset_bundle_path()?;
        if !install.exists(&expected_path.rel) {
            return Err(exception(format!(
                "Unable to find specified Asset_Bundle_Name next to the config file! Expected path: {}",
                expected_path.full
            )));
        }
        config.does_hash_file_exist = install.exists(&config.hash_file_path()?.rel);
        Ok(config)
    }

    /// `getAssetBundlePath` for the Windows client with `WITH_NOREDIST` (V21): the bundle next to the config.
    pub fn asset_bundle_path(&self) -> Result<MonoPath, MonoException> {
        self.directory_path.combine(&self.asset_bundle_name)
    }

    /// `getHashFilePath`: `MasterBundleHelper.getHashFileName` of the bundle path, which appends `.hash`.
    pub fn hash_file_path(&self) -> Result<MonoPath, MonoException> {
        let bundle = self.directory_path.combine(&self.asset_bundle_name)?;
        Ok(MonoPath { full: format!("{}.hash", bundle.full), rel: format!("{}.hash", bundle.rel) })
    }

    /// `formatAssetPath`: the prefix, then the path, with one `/` between them unless either has it.
    pub fn format_asset_path(&self, asset_path: &str) -> String {
        if self.asset_prefix.is_empty() {
            asset_path.to_string()
        } else if self.asset_prefix.ends_with('/') || asset_path.starts_with('/') {
            format!("{}{asset_path}", self.asset_prefix)
        } else {
            format!("{}/{asset_path}", self.asset_prefix)
        }
    }
}
