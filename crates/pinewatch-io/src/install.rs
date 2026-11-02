use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use crate::mono_path::{self, PathStyle};
use crate::steam;

/// Environment variable that points at the original game's install folder.
pub const INSTALL_DIR_ENV: &str = "PINEWATCH_UNTURNED_DIR";

/// The original game's installation, opened read-only.
///
/// This type has no way to write, create, rename or delete anything: Pinewatch never modifies the install.
#[derive(Debug, Clone)]
pub struct InstallRoot {
    root: PathBuf,
    full_name: String,
}

impl InstallRoot {
    /// Opens an existing directory as the install root. A folder whose path Mono's `Path` refuses (an invalid
    /// character) is refused too.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let root = path.as_ref().to_path_buf();
        if !fs::metadata(&root)?.is_dir() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("{} is not a folder", root.display())));
        }
        // The current directory only matters for a relative path.
        let cwd = std::env::current_dir().map(|dir| dir.to_string_lossy().into_owned()).unwrap_or_default();
        let full_name = mono_path::root_full_name(PathStyle::HOST, &root.to_string_lossy(), &cwd)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.message))?;
        Ok(Self { root, full_name })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `UnturnedPaths.RootDirectory.FullName` (`ReadWrite.PATH`): the install folder as the game's messages show it.
    pub fn full_name(&self) -> &str {
        &self.full_name
    }

    /// Turns a path relative to the install (using `/` or `\`) into an OS path.
    /// Paths that would leave the install (`..`, drive letters) are refused.
    pub fn resolve(&self, rel: &str) -> io::Result<PathBuf> {
        let mut path = self.root.clone();
        for part in rel.split(['/', '\\']) {
            match part {
                "" | "." => {}
                ".." => return Err(outside(rel)),
                p if p.contains(':') => return Err(outside(rel)),
                p => path.push(p),
            }
        }
        Ok(path)
    }

    /// Reads a whole file, opened with read access only.
    pub fn read(&self, rel: &str) -> io::Result<Vec<u8>> {
        let path = self.resolve(rel)?;
        let mut file = fs::OpenOptions::new().read(true).open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    /// Reads at most `max_len` bytes from the start of a file, opened with read access only. Readers that consume a
    /// bounded number of bytes use it, so a huge or corrupt file never causes a huge allocation.
    pub fn read_prefix(&self, rel: &str, max_len: usize) -> io::Result<Vec<u8>> {
        let path = self.resolve(rel)?;
        let file = fs::OpenOptions::new().read(true).open(path)?;
        let mut bytes = Vec::new();
        file.take(max_len as u64).read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    /// True if `rel` is an existing file (like `File.Exists`).
    pub fn exists(&self, rel: &str) -> bool {
        self.resolve(rel).is_ok_and(|p| p.is_file())
    }

    /// True if `rel` is an existing folder (like `Directory.Exists`).
    pub fn is_dir(&self, rel: &str) -> bool {
        self.resolve(rel).is_ok_and(|p| p.is_dir())
    }

    /// Names of the entries in a folder, in the order the OS returns them.
    pub fn read_dir(&self, rel: &str) -> io::Result<Vec<OsString>> {
        fs::read_dir(self.resolve(rel)?)?.map(|entry| entry.map(|e| e.file_name())).collect()
    }
}

fn outside(rel: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, format!("path leaves the install folder: {rel}"))
}

/// Why no install was found, with the outcome of each source tried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallNotFound {
    pub cli: String,
    pub env: String,
    pub steam: String,
}

impl fmt::Display for InstallNotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Unturned installation not found. Sources tried, in order:")?;
        writeln!(f, "  1. --unturned <dir>: {}", self.cli)?;
        writeln!(f, "  2. {INSTALL_DIR_ENV}: {}", self.env)?;
        writeln!(f, "  3. Steam library folders: {}", self.steam)?;
        write!(f, "Pass --unturned <dir> or set {INSTALL_DIR_ENV} to the folder that contains Maps and Bundles.")
    }
}

impl std::error::Error for InstallNotFound {}

fn looks_like_install(path: &Path) -> bool {
    path.join("Maps").is_dir() && path.join("Bundles").is_dir()
}

fn not_an_install(path: &Path) -> String {
    format!("{} does not contain Maps and Bundles folders", path.display())
}

/// Finds the install: the `--unturned` path, then [`INSTALL_DIR_ENV`], then Steam's library folders.
/// An explicit path or variable that isn't an install is an error; there is no fallback past it.
pub fn find_install(cli: Option<PathBuf>) -> Result<InstallRoot, InstallNotFound> {
    let skipped = "not checked".to_string();

    if let Some(path) = cli {
        if looks_like_install(&path)
            && let Ok(root) = InstallRoot::open(&path)
        {
            return Ok(root);
        }
        return Err(InstallNotFound { cli: not_an_install(&path), env: skipped.clone(), steam: skipped });
    }

    let env = match std::env::var_os(INSTALL_DIR_ENV) {
        Some(value) if !value.is_empty() => {
            let path = PathBuf::from(value);
            if looks_like_install(&path)
                && let Ok(root) = InstallRoot::open(&path)
            {
                return Ok(root);
            }
            return Err(InstallNotFound { cli: "not given".into(), env: not_an_install(&path), steam: skipped });
        }
        _ => "not set".to_string(),
    };

    let steam = match steam::find_app_install(steam::UNTURNED_APP_ID) {
        Ok(Some(path)) if looks_like_install(&path) => {
            if let Ok(root) = InstallRoot::open(&path) {
                return Ok(root);
            }
            not_an_install(&path)
        }
        Ok(Some(path)) => not_an_install(&path),
        Ok(None) => format!("no Steam library contains app {}", steam::UNTURNED_APP_ID),
        Err(e) => format!("could not read Steam's library list ({e})"),
    };

    Err(InstallNotFound { cli: "not given".into(), env, steam })
}
