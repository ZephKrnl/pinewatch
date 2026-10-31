use pinewatch_kvt::read_kvt_bytes;

/// Every official map's `Level.hierarchy` parses without an exception. Skipped without an install.
#[test]
fn every_installed_level_hierarchy_parses() {
    let Some(root) = install_root() else {
        eprintln!("skipped: no Unturned install found");
        return;
    };
    let mut parsed = 0;
    for entry in std::fs::read_dir(root.join("Maps")).unwrap() {
        let path = entry.unwrap().path().join("Level.hierarchy");
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let table = read_kvt_bytes(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(table.len() > 0, "{} parsed empty", path.display());
        parsed += 1;
    }
    assert!(parsed >= 9, "only {parsed} hierarchies found");
}

fn install_root() -> Option<std::path::PathBuf> {
    let candidates = [
        std::env::var_os("PINEWATCH_UNTURNED_DIR").map(std::path::PathBuf::from),
        Some(std::path::PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common\Unturned")),
    ];
    candidates.into_iter().flatten().find(|p| p.join("Maps").is_dir())
}
