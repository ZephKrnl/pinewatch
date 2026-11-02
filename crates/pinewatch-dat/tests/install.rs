//! Every `.dat` and `.asset` file in the installed game parses without a panic, with one reused parser as an asset
//! worker uses, with metadata off and on (`AssetsWorker` sets `EnableMetadata` from `Assets.shouldParseMetadata`).
//! The two modes can report different line numbers for the same file, as in the C#, so their errors aren't
//! compared. The install is only read. The test is skipped when no install is found.

mod common;

use common::user_strings;
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

    // `LevelInfo` loads `Maps/<map>/<language>.dat` (`English.dat` by default); `Level.dat` is binary.
    let mut map_files = 0;
    if install.is_dir("Maps") {
        let mut maps: Vec<String> = install
            .read_dir("Maps")
            .unwrap_or_else(|error| panic!("Maps: {error}"))
            .into_iter()
            .filter_map(|name| name.into_string().ok())
            .collect();
        maps.sort();
        for map in maps {
            let map_dir = format!("Maps/{map}");
            if !install.is_dir(&map_dir) {
                continue;
            }
            let language = format!("{map_dir}/English.dat");
            if install.exists(&language) {
                files.push(language);
                map_files += 1;
            }
            let bundles = format!("{map_dir}/Bundles");
            if install.is_dir(&bundles) {
                dat_files(&install, &bundles, &mut files);
            }
        }
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
    assert!(map_files >= 1, "no map language .dat files found");
}

/// The format strings the port uses are string literals in the shipped `UnturnedDat.dll`, and the duplicate-key
/// text `Data.cs` uses is in the shipped `Assembly-CSharp.dll`. This catches text taken from a source file that
/// Unity doesn't compile.
#[test]
fn message_format_strings_are_literals_in_the_shipped_assemblies() {
    let Ok(install) = find_install(None) else {
        eprintln!("skipped: no Unturned install found");
        return;
    };

    let dat = user_strings(&install.read("Unturned_Data/Managed/UnturnedDat.dll").expect("UnturnedDat.dll"));
    assert!(dat.len() > 50, "found only {} literals; the heap reader is broken", dat.len());
    let expected = [
        "duplicate key \"{0}\" on line {1} replacing existing value {2} with {3}",
        "list with {0} item(s)",
        "dictionary with {0} item(s)",
        "value(null)",
        "missing closing curly bracket '}}' for dictionary opened on line {0}",
        "missing closing bracket ']' for list opened on line {0}",
        "unrecognized escape sequence (\\{0}) on line {1} \u{2014} if this is a file path please use forward slash (/)",
        "missing closing quotation mark (\") for string opened on line {0}",
        "unexpected end of dictionary/object '}}' on line {0}",
        "unexpected end of list ']' on line {0}",
    ];
    let missing: Vec<&str> = expected.into_iter().filter(|text| !dat.contains(*text)).collect();
    assert!(missing.is_empty(), "not string literals in the shipped UnturnedDat.dll: {missing:#?}");

    let assembly = user_strings(
        &install.read("Unturned_Data/Managed/Assembly-CSharp.dll").expect("Assembly-CSharp.dll"),
    );
    for text in ["Duplicate key: \"", "\" Old value: ", " New value: "] {
        assert!(assembly.contains(text), "{text:?} is not a string literal in the shipped Assembly-CSharp.dll");
    }
}
