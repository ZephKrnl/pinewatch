//! The format crates' production dependencies stay within the allowlist (spec section 4, done criterion 1).

use std::path::Path;
use std::process::Command;

#[test]
fn format_crates_use_only_allowed_dependencies() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("workspace root");
    let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("check-deps")
        .current_dir(workspace)
        .output()
        .expect("xtask runs");
    assert!(
        out.status.success(),
        "check-deps failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
