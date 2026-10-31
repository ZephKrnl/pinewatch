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
