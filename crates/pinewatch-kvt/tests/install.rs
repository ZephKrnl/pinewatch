use pinewatch_io::find_install;
use pinewatch_kvt::read_kvt_bytes;

/// Every official map's `Level.hierarchy` parses without an exception. Skipped without an install.
#[test]
fn every_installed_level_hierarchy_parses() {
    let Ok(install) = find_install(None) else {
        eprintln!("skipped: no Unturned install found");
        return;
    };
    let mut parsed = 0;
    for map in install.read_dir("Maps").unwrap() {
        let rel = format!("Maps/{}/Level.hierarchy", map.to_string_lossy());
        if !install.exists(&rel) {
            continue;
        }
        let bytes = install.read(&rel).unwrap();
        let table = read_kvt_bytes(&bytes).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert!(!table.is_empty(), "{rel} parsed empty");
        parsed += 1;
    }
    assert!(parsed >= 9, "only {parsed} hierarchies found");
}
