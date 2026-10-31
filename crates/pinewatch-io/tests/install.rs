use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use pinewatch_io::{InstallRoot, LogLevel, LogSink, MemoryLog, find_install};

fn temp_dir(name: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("pinewatch-io-test-{}-{n}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn fake_install(name: &str) -> PathBuf {
    let dir = temp_dir(name);
    fs::create_dir_all(dir.join("Maps").join("PEI")).unwrap();
    fs::create_dir_all(dir.join("Bundles")).unwrap();
    fs::write(dir.join("Maps").join("PEI").join("Level.dat"), [1u8, 2, 3]).unwrap();
    dir
}

#[test]
fn explicit_path_without_maps_is_not_an_install() {
    let dir = temp_dir("no-maps");
    let err = find_install(Some(dir)).unwrap_err();
    let text = err.to_string();
    assert!(text.contains("--unturned"), "{text}");
    assert!(text.contains("PINEWATCH_UNTURNED_DIR"), "{text}");
    assert!(text.contains("Steam"), "{text}");
}

#[test]
fn explicit_valid_path_is_used_as_is() {
    let dir = fake_install("valid");
    let install = find_install(Some(dir.clone())).unwrap();
    assert_eq!(install.root(), dir.as_path());
}

#[test]
fn reads_files_and_lists_directories() {
    let dir = fake_install("read");
    let install = InstallRoot::open(&dir).unwrap();
    assert_eq!(install.read("Maps/PEI/Level.dat").unwrap(), vec![1, 2, 3]);
    assert!(install.exists("Maps/PEI/Level.dat"));
    assert!(!install.exists("Maps/PEI/Missing.dat"));
    assert!(install.is_dir("Maps/PEI"));
    assert_eq!(install.read_dir("Maps").unwrap(), vec![std::ffi::OsString::from("PEI")]);
}

#[test]
fn paths_that_leave_the_install_are_refused() {
    let dir = fake_install("escape");
    let install = InstallRoot::open(&dir).unwrap();
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
