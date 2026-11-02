//! Folder paths as the game holds them: the Mono string it logs and compares, and the install-relative path that
//! Pinewatch reads it through. Every search location of the client is inside the install.

use std::io;

use pinewatch_io::InstallRoot;
use pinewatch_io::mono_path::{self, PathStyle};
use pinewatch_mono::{ExceptionKind, MonoException, chars, sr};

/// The platform whose `System.IO.Path` rules apply: the one Pinewatch runs on, as for the game.
pub(crate) const STYLE: PathStyle = PathStyle::HOST;

/// A folder or file: `full` is the game's path string, `rel` the same place relative to the install (`/`-separated,
/// empty for the install itself).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonoPath {
    pub full: String,
    pub rel: String,
}

impl MonoPath {
    /// The install folder, `ReadWrite.PATH`.
    pub(crate) fn root(install: &InstallRoot) -> Self {
        Self { full: install.full_name().to_string(), rel: String::new() }
    }

    fn rel_child(&self, name: &str) -> String {
        if self.rel.is_empty() { name.to_string() } else { format!("{}/{name}", self.rel) }
    }

    /// `Path.Combine(full, name)`.
    pub(crate) fn combine(&self, name: &str) -> Result<Self, MonoException> {
        Ok(Self { full: mono_path::combine(STYLE, &self.full, name)?, rel: self.rel_child(name) })
    }

    /// An entry as `Directory.GetDirectories` and the other enumerations return it: `Path.Join(full, name)`.
    pub(crate) fn enumerated(&self, name: &str) -> Self {
        Self { full: mono_path::join(STYLE, &self.full, name), rel: self.rel_child(name) }
    }

    /// `Path.GetDirectoryName(full)` for a file the searcher found. Only a root has none; the C# would then fail in
    /// its next `Path.Combine` with `ArgumentNullException`, which is the error here.
    pub(crate) fn parent(&self) -> Result<Self, MonoException> {
        let full = mono_path::get_directory_name(STYLE, &self.full)?.ok_or_else(|| MonoException::argument_null("path1"))?;
        let rel = self.rel.rsplit_once('/').map_or(String::new(), |(parent, _)| parent.to_string());
        Ok(Self { full, rel })
    }

    /// `Path.GetFullPath(full)`, for the same place.
    pub(crate) fn full_path(&self) -> Result<Self, MonoException> {
        Ok(Self { full: mono_path::get_full_path(STYLE, &self.full, &current_directory())?, rel: self.rel.clone() })
    }
}

/// `Directory.GetCurrentDirectory()`.
pub(crate) fn current_directory() -> String {
    std::env::current_dir().map(|dir| dir.to_string_lossy().into_owned()).unwrap_or_default()
}

/// The entries of `dir` in enumeration order, as names.
fn entry_names(install: &InstallRoot, dir: &MonoPath) -> Result<Vec<String>, MonoException> {
    let names = install.read_dir(&dir.rel).map_err(|error| enumeration_exception(dir, &error))?;
    Ok(names.into_iter().map(|name| name.to_string_lossy().into_owned()).collect())
}

/// `Directory.GetDirectories(dir)` and `EnumerateDirectories(dir)`: the subfolders, in enumeration order.
pub(crate) fn directories(install: &InstallRoot, dir: &MonoPath) -> Result<Vec<MonoPath>, MonoException> {
    let names = entry_names(install, dir)?;
    Ok(names.iter().map(|name| dir.enumerated(name)).filter(|entry| install.is_dir(&entry.rel)).collect())
}

/// `Directory.EnumerateFiles(dir, "*.asset")`, in enumeration order. The pattern is the Win32 expression `*.asset`,
/// which corefx matches with `name.EndsWith(".asset", …)`: ignoring case where the platform's file names do
/// (Windows), exactly elsewhere.
pub(crate) fn asset_files(install: &InstallRoot, dir: &MonoPath) -> Result<Vec<MonoPath>, MonoException> {
    let suffix: Vec<u16> = ".asset".encode_utf16().collect();
    let names = entry_names(install, dir)?;
    Ok(names
        .iter()
        .filter(|name| {
            let name: Vec<u16> = name.encode_utf16().collect();
            name.len() >= suffix.len() && {
                let end = &name[name.len() - suffix.len()..];
                if cfg!(windows) { chars::equals_ordinal_ignore_case(end, &suffix) } else { end == suffix }
            }
        })
        .map(|name| dir.enumerated(name))
        .filter(|entry| install.exists(&entry.rel))
        .collect())
}

/// What `FileSystemEnumerator` throws when it can't open `dir`: `DirectoryNotFoundException` with the full path of
/// a missing folder; other failures keep Rust's text for now (V12).
fn enumeration_exception(dir: &MonoPath, error: &io::Error) -> MonoException {
    if error.kind() == io::ErrorKind::NotFound {
        let full = mono_path::get_full_path(STYLE, &dir.full, &current_directory()).unwrap_or_else(|_| dir.full.clone());
        let root = mono_path::trim_ending_directory_separator(STYLE, &full);
        MonoException::new(ExceptionKind::DirectoryNotFound, sr::IO_PATH_NOT_FOUND_PATH.replace("{0}", &root))
    } else {
        MonoException::new(ExceptionKind::Io, error.to_string())
    }
}

/// `new DirectoryInfo(path).Name` (`ReadWrite.folderName`).
pub(crate) fn folder_name(path: &str) -> Result<String, MonoException> {
    let full = mono_path::get_full_path(STYLE, path, &current_directory())?;
    let trimmed = mono_path::trim_ending_directory_separator(STYLE, &full);
    if mono_path::get_directory_name(STYLE, &trimmed)?.is_none() {
        // A root is its own name.
        return Ok(full);
    }
    mono_path::get_file_name(STYLE, &trimmed)
}
