//! Checks around `--screenshot`: the file must not land inside the install, and a failed save must be noticed.

use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// `path` made absolute, with its longest existing prefix canonicalized (links, junctions, letter case and short
/// names resolved) and the rest appended as given.
fn resolve(path: &Path) -> io::Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    let mut existing = absolute.as_path();
    let mut rest = Vec::new();
    loop {
        if let Ok(canonical) = std::fs::canonicalize(existing) {
            let mut resolved = canonical;
            for part in rest.iter().rev() {
                resolved.push(part);
            }
            return Ok(resolved);
        }
        let (Some(parent), Some(name)) = (existing.parent(), existing.file_name()) else {
            return Ok(absolute);
        };
        rest.push(name.to_owned());
        existing = parent;
    }
}

/// True when writing `path` would put a file inside the install (or replace the install folder itself).
pub fn inside_install(path: &Path, install_root: &Path) -> io::Result<bool> {
    let root = std::fs::canonicalize(install_root)?;
    Ok(resolve(path)?.starts_with(&root))
}

/// The file's modification time, or `None` if it doesn't exist.
pub fn modified_time(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// True when the file exists and is new or rewritten since `before` was taken with [`modified_time`].
pub fn was_written(path: &Path, before: Option<SystemTime>) -> bool {
    modified_time(path).is_some_and(|now| before != Some(now))
}
