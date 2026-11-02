//! `AssetsWorker`'s reader (`ReaderThreadMain`, `AddFoundAsset`, `ReadFileWithoutHash` in
//! `Bundles/AssetsWorker.cs`): reads what the searcher found, with one `DatParser` per search location.

use std::io::Read;

use pinewatch_dat::{DatParser, DatTree};
use pinewatch_io::InstallRoot;
use pinewatch_mono::{ExceptionKind, MonoException, NetString};
use sha1::{Digest, Sha1};

use crate::master_bundle::MasterBundleConfig;
use crate::origin::OriginId;
use crate::paths::MonoPath;
use crate::search::SearchLocation;
use crate::worker::{SearchEvent, WorkItem, search_location};

/// An asset definition as the reader hands it to the main thread.
#[derive(Debug)]
pub struct AssetDefinition {
    pub path: MonoPath,
    /// SHA-1 of the bytes the parser read: the whole file.
    pub hash: [u8; 20],
    pub asset_data: DatTree,
    /// `<Language>.dat`, or `English.dat` when there's no file for the language.
    pub translation_data: Option<DatTree>,
    /// `English.dat` next to a `<Language>.dat`, when the language isn't English.
    pub fallback_translation_data: Option<DatTree>,
    /// The parser's error messages, or `None` without errors.
    pub asset_errors: Option<Vec<NetString>>,
    pub origin: OriginId,
}

/// `AssetsWorker.ResultItem`.
#[derive(Debug)]
pub enum ResultItem {
    /// A master bundle config and the SHA-1 of its bundle file. The bundle's bytes are Unity's to load (M2d).
    MasterBundle { config: MasterBundleConfig, hash: [u8; 20] },
    Asset(AssetDefinition),
    /// An exception the worker caught, with the message it logs.
    Exception { message: String, exception: MonoException },
}

/// Searches and reads one location: the searcher's events, each work item replaced by what the reader makes of it.
/// `language` is `Provider.language`. An `Err` is an exception the searcher doesn't catch (see `search_location`).
pub fn read_location(
    install: &InstallRoot,
    location: &SearchLocation,
    language: &str,
) -> Result<Vec<ResultItem>, MonoException> {
    let events = search_location(install, &location.dir)?;
    Ok(read_items(install, location.origin, events, language))
}

/// The reader for one location's events, in order, with one parser for all of them (metadata off, as
/// `-ParseAssetMetadata` is by default).
pub fn read_items(install: &InstallRoot, origin: OriginId, events: Vec<SearchEvent>, language: &str) -> Vec<ResultItem> {
    let mut reader = Reader { install, parser: DatParser::new(), origin, language };
    reader.parser.set_enable_metadata(false);
    events
        .into_iter()
        .map(|event| match event {
            SearchEvent::Exception { message, exception } => ResultItem::Exception { message, exception },
            SearchEvent::Item(WorkItem::MasterBundle(path)) => reader.master_bundle(&path).unwrap_or_else(|exception| {
                ResultItem::Exception {
                    message: format!("Caught exception reading master bundle config at: \"{}\"", path.full),
                    exception,
                }
            }),
            SearchEvent::Item(WorkItem::Asset(path)) => {
                reader.add_found_asset(&path).map(ResultItem::Asset).unwrap_or_else(|exception| ResultItem::Exception {
                    message: format!("Caught exception reading asset definition at: \"{}\"", path.full),
                    exception,
                })
            }
        })
        .collect()
}

/// An I/O failure as the exception the reader catches. The texts are Rust's for now (V12).
fn io_exception(error: std::io::Error) -> MonoException {
    MonoException::new(ExceptionKind::Io, error.to_string())
}

struct Reader<'a> {
    install: &'a InstallRoot,
    parser: DatParser,
    origin: OriginId,
    language: &'a str,
}

impl Reader<'_> {
    /// `ReadFileWithoutHash`.
    fn read_file_without_hash(&mut self, path: &MonoPath) -> Result<DatTree, MonoException> {
        let bytes = self.install.read(&path.rel).map_err(io_exception)?;
        Ok(self.parser.parse_bytes(&bytes))
    }

    /// The master bundle branch of `ReaderThreadMain`: the config, then the bundle file read through a SHA-1 stream.
    fn master_bundle(&mut self, config_path: &MonoPath) -> Result<ResultItem, MonoException> {
        let config_data = self.read_file_without_hash(config_path)?;
        let directory = config_path.parent()?;
        let config = MasterBundleConfig::new(self.install, directory, &config_data.root(), self.origin)?;
        let bundle_path = config.asset_bundle_path()?;
        let mut file = self.install.open_read(&bundle_path.rel).map_err(io_exception)?;
        let mut hasher = Sha1::new();
        let mut buffer = vec![0u8; 1 << 16];
        loop {
            let read = file.read(&mut buffer).map_err(io_exception)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        Ok(ResultItem::MasterBundle { config, hash: hasher.finalize().into() })
    }

    /// `AddFoundAsset(filePath, checkForTranslations: true)`.
    fn add_found_asset(&mut self, file_path: &MonoPath) -> Result<AssetDefinition, MonoException> {
        let dir_path = file_path.parent()?;
        let bytes = self.install.read(&file_path.rel).map_err(io_exception)?;
        let asset_data = self.parser.parse_bytes(&bytes);
        let asset_errors = self.parser.has_error().then(|| self.parser.error_messages().to_vec());
        let hash: [u8; 20] = Sha1::digest(&bytes).into();

        let language_file_path = dir_path.combine(&format!("{}.dat", self.language))?;
        let english_file_path = dir_path.combine("English.dat")?;
        let mut translation_data = None;
        let mut fallback_translation_data = None;
        if self.install.exists(&language_file_path.rel) {
            translation_data = Some(self.read_file_without_hash(&language_file_path)?);
            // `Provider.languageIsEnglish`: the language is exactly "English".
            if self.language != "English" && self.install.exists(&english_file_path.rel) {
                fallback_translation_data = Some(self.read_file_without_hash(&english_file_path)?);
            }
        } else if self.install.exists(&english_file_path.rel) {
            translation_data = Some(self.read_file_without_hash(&english_file_path)?);
        }

        Ok(AssetDefinition {
            path: file_path.clone(),
            hash,
            asset_data,
            translation_data,
            fallback_translation_data,
            asset_errors,
            origin: self.origin,
        })
    }
}
