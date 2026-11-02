use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use pinewatch_io::{InstallRoot, LogLevel, LogSink, MemoryLog, find_install};

/// A folder under `%TEMP%`, removed when dropped.
struct TempDir(PathBuf);

impl std::ops::Deref for TempDir {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temp_dir(name: &str) -> TempDir {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("pinewatch-io-test-{}-{n}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    TempDir(dir)
}

fn fake_install(name: &str) -> TempDir {
    let dir = temp_dir(name);
    fs::create_dir_all(dir.join("Maps").join("PEI")).unwrap();
    fs::create_dir_all(dir.join("Bundles")).unwrap();
    fs::write(dir.join("Maps").join("PEI").join("Level.dat"), [1u8, 2, 3]).unwrap();
    dir
}

#[test]
fn explicit_path_without_maps_is_not_an_install() {
    let dir = temp_dir("no-maps");
    let err = find_install(Some(dir.to_path_buf())).unwrap_err();
    let text = err.to_string();
    assert!(text.contains("--unturned"), "{text}");
    assert!(text.contains("PINEWATCH_UNTURNED_DIR"), "{text}");
    assert!(text.contains("Steam"), "{text}");
}

#[test]
fn explicit_valid_path_is_used_as_is() {
    let dir = fake_install("valid");
    let install = find_install(Some(dir.to_path_buf())).unwrap();
    assert_eq!(install.root(), &*dir);
}

#[test]
fn reads_files_and_lists_directories() {
    let dir = fake_install("read");
    let install = InstallRoot::open(&*dir).unwrap();
    assert_eq!(install.read("Maps/PEI/Level.dat").unwrap(), vec![1, 2, 3]);
    assert!(install.exists("Maps/PEI/Level.dat"));
    assert!(!install.exists("Maps/PEI/Missing.dat"));
    assert!(install.is_dir("Maps/PEI"));
    assert_eq!(install.read_dir("Maps").unwrap(), vec![std::ffi::OsString::from("PEI")]);
}

#[test]
fn read_prefix_reads_at_most_max_len() {
    let dir = temp_dir("prefix");
    fs::write(dir.join("ten.bin"), [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9]).unwrap();
    let install = InstallRoot::open(&*dir).unwrap();
    assert_eq!(install.read_prefix("ten.bin", 4).unwrap(), vec![0, 1, 2, 3]);
    assert_eq!(install.read_prefix("ten.bin", 100).unwrap(), (0u8..10).collect::<Vec<_>>());
    assert_eq!(install.read_prefix("../ten.bin", 4).unwrap_err().kind(), ErrorKind::InvalidInput);
}

#[test]
fn paths_that_leave_the_install_are_refused() {
    let dir = fake_install("escape");
    let install = InstallRoot::open(&*dir).unwrap();
    assert_eq!(install.read("../outside.txt").unwrap_err().kind(), ErrorKind::InvalidInput);
    assert_eq!(install.read("Maps/../../outside.txt").unwrap_err().kind(), ErrorKind::InvalidInput);
    assert_eq!(install.read("C:/Windows/win.ini").unwrap_err().kind(), ErrorKind::InvalidInput);
    assert!(!install.exists("../outside.txt"));
}

#[test]
fn memory_log_records_level_and_exact_text() {
    let mut log = MemoryLog::default();
    log.log(LogLevel::Warn, "LandscapeTile missing heightmap file: x");
    assert_eq!(log.entries, vec![(LogLevel::Warn, "LandscapeTile missing heightmap file: x".to_string())]);
}

#[test]
fn real_install_has_pei() {
    let Ok(install) = find_install(None) else {
        eprintln!("skipped: no Unturned install found");
        return;
    };
    assert!(install.exists("Maps/PEI/Level.dat"));
}

/// `full_name` is `UnturnedPaths.RootDirectory.FullName`: absolute, `.` and `..` resolved, no trailing separator,
/// built with the platform's `Path` rules (V11).
#[test]
fn full_name_is_the_mono_root_directory() {
    let dir = temp_dir("full-name");
    let name = dir.file_name().unwrap().to_str().unwrap().to_string();
    let detour = dir.join("..").join(&name).join(".");
    let install = InstallRoot::open(&detour).unwrap();
    let expected = dir.to_str().unwrap().to_string();
    assert_eq!(install.full_name(), expected);
    assert!(!install.full_name().contains(".."));
}
