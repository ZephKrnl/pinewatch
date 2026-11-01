//! The executable's failure paths. Each exits before any window opens.

use std::process::Command;

fn run(args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_pinewatch")).args(args).output().expect("runs");
    (out.status.code(), String::from_utf8_lossy(&out.stderr).into_owned())
}

fn have_install() -> bool {
    pinewatch_io::find_install(None).is_ok()
}

#[test]
fn folder_that_is_not_an_install_exits_2_and_names_every_source() {
    let dir = std::env::temp_dir().join(format!("pinewatch-exe-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (code, stderr) = run(&["--unturned", dir.to_str().unwrap()]);
    assert_eq!(code, Some(2), "{stderr}");
    assert!(stderr.contains("--unturned") && stderr.contains("PINEWATCH_UNTURNED_DIR") && stderr.contains("Steam"), "{stderr}");
}

/// A temporary folder that looks like an install (`Maps` and `Bundles`), removed when dropped.
struct FakeInstall(std::path::PathBuf);

impl FakeInstall {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("pinewatch-exe-{name}-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("Maps")).unwrap();
        std::fs::create_dir_all(dir.join("Bundles")).unwrap();
        Self(dir)
    }
}

impl Drop for FakeInstall {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn screenshot_inside_the_install_is_refused_before_anything_is_written() {
    let install = FakeInstall::new("screenshot");
    let root = install.0.to_str().unwrap().to_string();
    let dir_name = install.0.file_name().unwrap().to_str().unwrap().to_string();
    let targets = [
        format!("{root}/new-folder/shot.png"),
        format!("{root}/Maps/shot.png"),
        format!("{root}/../{dir_name}/shot.png"),
        format!("{}/MAPS/shot.png", root.to_uppercase()),
    ];
    for target in targets {
        let (code, stderr) = run(&["--unturned", &root, "--screenshot", &target]);
        assert_eq!(code, Some(2), "{target}: {stderr}");
        assert!(stderr.contains("inside the Unturned install"), "{target}: {stderr}");
    }
    assert!(!install.0.join("new-folder").exists(), "a folder was created inside the install");
    let entries: Vec<_> = std::fs::read_dir(&install.0).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(entries.len(), 2, "{entries:?}");
}

#[test]
fn unknown_option_exits_2_with_usage() {
    let (code, stderr) = run(&["--frobnicate"]);
    assert_eq!(code, Some(2));
    assert!(stderr.contains("Usage"), "{stderr}");
}

#[test]
fn unknown_map_exits_2_and_lists_available_maps() {
    if !have_install() {
        eprintln!("skipped: no Unturned install found");
        return;
    }
    let (code, stderr) = run(&["--map", "NoSuchMap"]);
    assert_eq!(code, Some(2), "{stderr}");
    assert!(stderr.contains("Available maps") && stderr.contains("PEI"), "{stderr}");
}

#[test]
fn map_without_a_landscape_exits_3() {
    if !have_install() {
        eprintln!("skipped: no Unturned install found");
        return;
    }
    let (code, stderr) = run(&["--map", "Destruction"]);
    assert_eq!(code, Some(3), "{stderr}");
    assert!(stderr.contains("no landscape"), "{stderr}");
}
