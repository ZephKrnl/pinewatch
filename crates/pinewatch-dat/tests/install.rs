//! Every `.dat` and `.asset` file in the installed game parses without a panic, with one reused parser as an asset
//! worker uses, with metadata off and on. The install is only read. The test is skipped when no install is found.

use pinewatch_dat::DatParser;
use pinewatch_io::{InstallRoot, find_install};

/// Appends the `.dat` and `.asset` files under `dir` (install-relative) to `files`, depth first in name order.
fn dat_files(install: &InstallRoot, dir: &str, files: &mut Vec<String>) {
    let entries = install.read_dir(dir).unwrap_or_else(|error| panic!("{dir}: {error}"));
    let mut names: Vec<String> = entries.into_iter().filter_map(|name| name.into_string().ok()).collect();
    names.sort();
    for name in names {
        let path = format!("{dir}/{name}");
        if install.is_dir(&path) {
            dat_files(install, &path, files);
        } else {
            let lower = name.to_ascii_lowercase();
            if lower.ends_with(".dat") || lower.ends_with(".asset") {
                files.push(path);
            }
        }
    }
}

#[test]
fn every_installed_dat_and_asset_file_parses() {
    let Ok(install) = find_install(None) else {
        eprintln!("skipped: no Unturned install found");
        return;
    };

    let mut files = Vec::new();
    for dir in ["Bundles", "Localization"] {
        dat_files(&install, dir, &mut files);
    }

    let mut parser = DatParser::new();
    let mut metadata_parser = DatParser::new();
    metadata_parser.set_enable_metadata(true);
    let mut with_errors = 0;
    let mut samples = Vec::new();
    for file in &files {
        let bytes = install.read(file).unwrap_or_else(|error| panic!("{file}: {error}"));
        parser.parse_bytes(&bytes);
        metadata_parser.parse_bytes(&bytes);
        assert_eq!(parser.error_messages(), metadata_parser.error_messages(), "{file}: metadata changes the errors");
        if parser.has_error() {
            with_errors += 1;
            if samples.len() < 5 {
                samples.push(format!("{file}: {}", parser.error_messages()[0]));
            }
        }
    }

    eprintln!("parsed {} files, {with_errors} with errors", files.len());
    for sample in &samples {
        eprintln!("  {sample}");
    }
    assert!(files.len() >= 8000, "only {} .dat and .asset files found", files.len());
}
