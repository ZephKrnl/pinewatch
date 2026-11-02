// Ported from Mono's `mcs/class/corlib/System.IO/Path.cs` (Unity mono b8eb56f), which Unity's Mono compiles instead
// of corefx's `Path`, and from corefx `DirectoryInfo.Parent` (mono/corefx 5b4edb9b). MIT license; see
// NOTICES-PORTED-CODE.md.

//! `System.IO.Path` as Unity's Mono computes it, so Pinewatch builds the same path strings the game logs and
//! compares.
//!
//! Each function takes the [`PathStyle`] of the platform Mono runs on, which picks the separators, the invalid
//! characters and the Windows-only steps (`Environment.IsRunningOnWindows`), so the Windows reference behavior can be
//! tested on any host. Strings are handled as UTF-16 code units, as in C#.
//!
//! Not ported: `MonoIO.RemapPath` (Mono's `MONO_IOMAP` path remapping, off unless that variable is set), and the two
//! `GetFullPathName` calls of `WindowsDriveAdjustment`, which only a drive-relative path on another drive than the
//! current directory reaches (for example `D:dir` while the current directory is on `C:`). `char.IsLetter` for the
//! drive letter is ported for ASCII only. These are recorded in `docs/VERIFICATION.md` (V22).

use pinewatch_mono::{MonoException, chars};

/// The platform whose `System.IO.Path` rules apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStyle {
    Windows,
    Unix,
}

impl PathStyle {
    /// The platform Pinewatch runs on.
    pub const HOST: Self = if cfg!(windows) { Self::Windows } else { Self::Unix };

    /// `Path.DirectorySeparatorChar`.
    pub const fn separator(self) -> char {
        match self {
            Self::Windows => '\\',
            Self::Unix => '/',
        }
    }

    fn sep(self) -> u16 {
        self.separator() as u16
    }

    /// `Path.AltDirectorySeparatorChar`.
    fn alt(self) -> u16 {
        u16::from(b'/')
    }

    /// `Path.VolumeSeparatorChar`.
    fn volume(self) -> u16 {
        match self {
            Self::Windows => u16::from(b':'),
            Self::Unix => u16::from(b'/'),
        }
    }

    fn is_windows(self) -> bool {
        self == Self::Windows
    }

    /// `IsDirectorySeparator`.
    fn is_separator(self, c: u16) -> bool {
        c == self.sep() || c == self.alt()
    }

    /// One of `PathSeparatorChars`: the directory, alternative and volume separators.
    fn is_path_separator_char(self, c: u16) -> bool {
        self.is_separator(c) || c == self.volume()
    }

    /// `InvalidPathChars` (`GetInvalidPathChars`).
    fn is_invalid(self, c: u16) -> bool {
        match self {
            Self::Windows => c < 0x20 || matches!(c, 0x22 | 0x3C | 0x3E | 0x7C),
            Self::Unix => c == 0,
        }
    }

    /// `dirEqualsVolume`.
    fn dir_equals_volume(self) -> bool {
        self.sep() == self.volume()
    }
}

const ILLEGAL_CHARACTERS: &str = "Illegal characters in path.";

fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn text(units: &[u16]) -> String {
    String::from_utf16_lossy(units)
}

fn argument(message: &str) -> MonoException {
    MonoException::argument(message, None)
}

fn has_invalid(style: PathStyle, path: &[u16]) -> bool {
    path.iter().any(|&c| style.is_invalid(c))
}

fn last_path_separator(style: PathStyle, path: &[u16]) -> Option<usize> {
    path.iter().rposition(|&c| style.is_path_separator_char(c))
}

/// `string.TrimEnd()`.
fn trim_end(s: &[u16]) -> &[u16] {
    let end = s.iter().rposition(|&c| !chars::is_white_space(c)).map_or(0, |i| i + 1);
    &s[..end]
}

/// `IsPathRooted(ReadOnlySpan<char>)`.
fn is_rooted_span(style: PathStyle, path: &[u16]) -> bool {
    let Some(&c) = path.first() else {
        return false;
    };
    c == style.sep() || c == style.alt() || (!style.dir_equals_volume() && path.len() > 1 && path[1] == style.volume())
}

/// `IsPathRooted(string)`: also checks for invalid characters.
fn is_rooted(style: PathStyle, path: &[u16]) -> Result<bool, MonoException> {
    if path.is_empty() {
        return Ok(false);
    }
    if has_invalid(style, path) {
        return Err(argument(ILLEGAL_CHARACTERS));
    }
    Ok(is_rooted_span(style, path))
}

/// `Path.Combine(path1, path2)`.
pub fn combine(style: PathStyle, path1: &str, path2: &str) -> Result<String, MonoException> {
    Ok(text(&combine_units(style, &units(path1), &units(path2))?))
}

fn combine_units(style: PathStyle, path1: &[u16], path2: &[u16]) -> Result<Vec<u16>, MonoException> {
    if path1.is_empty() {
        return Ok(path2.to_vec());
    }
    if path2.is_empty() {
        return Ok(path1.to_vec());
    }
    if has_invalid(style, path1) || has_invalid(style, path2) {
        return Err(argument(ILLEGAL_CHARACTERS));
    }
    if is_rooted(style, path2)? {
        return Ok(path2.to_vec());
    }
    let end = path1[path1.len() - 1];
    let mut combined = path1.to_vec();
    if end != style.sep() && end != style.alt() && end != style.volume() {
        combined.push(style.sep());
    }
    combined.extend_from_slice(path2);
    Ok(combined)
}

/// `Path.Join(path1, path2)`: no invalid-character check, and a separator only where neither part has one.
pub fn join(style: PathStyle, path1: &str, path2: &str) -> String {
    let (first, second) = (units(path1), units(path2));
    if first.is_empty() {
        return path2.to_string();
    }
    if second.is_empty() {
        return path1.to_string();
    }
    if style.is_separator(first[first.len() - 1]) || style.is_separator(second[0]) {
        format!("{path1}{path2}")
    } else {
        format!("{path1}{}{path2}", style.separator())
    }
}

/// `PathInternal.TrimEndingDirectorySeparator`: drops one trailing separator unless the path is a root (judged
/// with `GetPathRoot`, see V22).
pub fn trim_ending_directory_separator(style: PathStyle, path: &str) -> String {
    let units = units(path);
    match units.last() {
        Some(&last) if style.is_separator(last) && get_path_root(style, &units).ok().as_deref() != Some(&units[..]) => {
            text(&units[..units.len() - 1])
        }
        _ => path.to_string(),
    }
}

/// `Path.GetFileName(path)`.
pub fn get_file_name(style: PathStyle, path: &str) -> Result<String, MonoException> {
    Ok(text(get_file_name_units(style, &units(path))?))
}

fn get_file_name_units(style: PathStyle, path: &[u16]) -> Result<&[u16], MonoException> {
    if path.is_empty() {
        return Ok(path);
    }
    if has_invalid(style, path) {
        return Err(argument(ILLEGAL_CHARACTERS));
    }
    Ok(match last_path_separator(style, path) {
        Some(last) => &path[last + 1..],
        None => path,
    })
}

/// `findExtension`: the index of the last `.` after the last separator.
fn find_extension(style: PathStyle, path: &[u16]) -> Option<usize> {
    let dot = path.iter().rposition(|&c| c == u16::from(b'.'))?;
    match last_path_separator(style, path) {
        Some(separator) if separator >= dot => None,
        _ => Some(dot),
    }
}

/// `Path.ChangeExtension(path, null)`: the path without its extension.
pub fn change_extension_to_none(style: PathStyle, path: &str) -> Result<String, MonoException> {
    Ok(text(change_extension_to_none_units(style, &units(path))?))
}

fn change_extension_to_none_units(style: PathStyle, path: &[u16]) -> Result<&[u16], MonoException> {
    if has_invalid(style, path) {
        return Err(argument(ILLEGAL_CHARACTERS));
    }
    Ok(match find_extension(style, path) {
        Some(extension) => &path[..extension],
        None => path,
    })
}

/// `Path.GetFileNameWithoutExtension(path)`.
pub fn get_file_name_without_extension(style: PathStyle, path: &str) -> Result<String, MonoException> {
    let path = units(path);
    let name = get_file_name_units(style, &path)?;
    Ok(text(change_extension_to_none_units(style, name)?))
}

/// `Path.GetPathRoot(path)`.
fn get_path_root(style: PathStyle, path: &[u16]) -> Result<Vec<u16>, MonoException> {
    if chars::trim(path).is_empty() {
        return Err(argument("The specified path is not of a legal form."));
    }
    if !is_rooted(style, path)? {
        return Ok(Vec::new());
    }
    if !style.is_windows() {
        return Ok(if style.is_separator(path[0]) { vec![style.sep()] } else { Vec::new() });
    }
    if path.len() == 1 && style.is_separator(path[0]) {
        return Ok(vec![style.sep()]);
    } else if path.len() < 2 {
        return Ok(Vec::new());
    }
    let mut len = 2;
    if style.is_separator(path[0]) && style.is_separator(path[1]) {
        // UNC: \\server or \\server\share.
        while len < path.len() && !style.is_separator(path[len]) {
            len += 1;
        }
        if len < path.len() {
            len += 1;
            while len < path.len() && !style.is_separator(path[len]) {
                len += 1;
            }
        }
        let mut root = vec![style.sep(), style.sep()];
        root.extend(path[2..len].iter().map(|&c| if c == style.alt() { style.sep() } else { c }));
        return Ok(root);
    } else if style.is_separator(path[0]) {
        return Ok(vec![style.sep()]);
    }
    // The path is rooted and doesn't start with a separator, so `path[1]` is the volume separator.
    if path.len() >= 3 && style.is_separator(path[2]) {
        len += 1;
    }
    Ok(path[..len].to_vec())
}

/// `CleanPath`: removes repeated and trailing separators and turns alternative separators into the directory
/// separator, keeping a leading `\\`.
fn clean_path(style: PathStyle, s: &[u16]) -> Vec<u16> {
    let l = s.len();
    let mut sub = 0;
    let mut alt = 0;
    let mut start = 0;
    let s0 = s[0];
    if l > 2 && s0 == u16::from(b'\\') && s[1] == u16::from(b'\\') {
        start = 2;
    }
    if l == 1 && style.is_separator(s0) {
        return s.to_vec();
    }
    for i in start..l {
        let c = s[i];
        if !style.is_separator(c) {
            continue;
        }
        if style.sep() != style.alt() && c == style.alt() {
            alt += 1;
        }
        if i + 1 == l || style.is_separator(s[i + 1]) {
            sub += 1;
        }
    }
    if sub == 0 && alt == 0 {
        return s.to_vec();
    }
    let len = l - sub;
    let mut copy = vec![0u16; len];
    if start != 0 {
        copy[0] = u16::from(b'\\');
        copy[1] = u16::from(b'\\');
    }
    let mut i = start;
    let mut j = start;
    while i < l && j < len {
        let c = s[i];
        if !style.is_separator(c) {
            copy[j] = c;
            j += 1;
            i += 1;
            continue;
        }
        if j + 1 != len {
            copy[j] = style.sep();
            j += 1;
            while i < l - 1 {
                if !style.is_separator(s[i + 1]) {
                    break;
                }
                i += 1;
            }
        }
        i += 1;
    }
    copy
}

/// `Path.GetDirectoryName(path)`: `None` where the C# returns null (the path is a root).
pub fn get_directory_name(style: PathStyle, path: &str) -> Result<Option<String>, MonoException> {
    Ok(get_directory_name_units(style, &units(path))?.map(|name| text(&name)))
}

fn get_directory_name_units(style: PathStyle, path: &[u16]) -> Result<Option<Vec<u16>>, MonoException> {
    if path.is_empty() {
        return Err(argument("Invalid path"));
    }
    if get_path_root(style, path)? == path {
        return Ok(None);
    }
    if chars::trim(path).is_empty() {
        return Err(argument("Argument string consists of whitespace characters only."));
    }
    if has_invalid(style, path) {
        return Err(argument("Path contains invalid characters"));
    }
    let Some(mut last) = last_path_separator(style, path) else {
        return Ok(Some(Vec::new()));
    };
    if last == 0 {
        last += 1;
    }
    let ret = &path[..last];
    let l = ret.len();
    if l >= 2 && style.is_windows() && ret[l - 1] == style.volume() {
        let mut name = ret.to_vec();
        name.push(style.sep());
        Ok(Some(name))
    } else if l == 1 && style.is_windows() && path.len() >= 2 && path[last] == style.volume() {
        let mut name = ret.to_vec();
        name.push(style.volume());
        Ok(Some(name))
    } else {
        Ok(Some(clean_path(style, ret)))
    }
}

/// `SameRoot`.
fn same_root(style: PathStyle, root: &[u16], path: &[u16]) -> bool {
    if root.len() < 2 || path.len() < 2 {
        return false;
    }
    if style.is_separator(root[0]) && style.is_separator(root[1]) {
        if !(style.is_separator(path[0]) && style.is_separator(path[1])) {
            return false;
        }
        // `String.Compare(rootShare, pathShare, true, InvariantCulture)`: the server and share names here are what
        // `GetPathRoot` built, compared ignoring case.
        let fold = |s: &[u16]| s.iter().map(|&c| chars::to_upper_invariant(c)).collect::<Vec<u16>>();
        return fold(&server_and_share(style, root)) == fold(&server_and_share(style, path));
    }
    if root[0] != path[0] {
        return false;
    }
    if path[1] != style.volume() {
        return false;
    }
    if root.len() > 2 && path.len() > 2 {
        return style.is_separator(root[2]) && style.is_separator(path[2]);
    }
    true
}

/// `GetServerAndShare`.
fn server_and_share(style: PathStyle, path: &[u16]) -> Vec<u16> {
    let mut len = 2;
    while len < path.len() && !style.is_separator(path[len]) {
        len += 1;
    }
    if len < path.len() {
        len += 1;
        while len < path.len() && !style.is_separator(path[len]) {
            len += 1;
        }
    }
    path[2..len].iter().map(|&c| if c == style.alt() { style.sep() } else { c }).collect()
}

/// `CanonicalizePath`.
fn canonicalize_path(style: PathStyle, path: &[u16], cwd: &[u16]) -> Result<Vec<u16>, MonoException> {
    let path = if style.is_windows() { chars::trim(path) } else { path };
    if path.is_empty() {
        return Ok(path.to_vec());
    }
    let root = get_path_root(style, path)?;
    let mut dirs: Vec<&[u16]> = path.split(|&c| style.is_separator(c)).collect();
    let is_unc = style.is_windows() && root.len() > 2 && style.is_separator(root[0]) && style.is_separator(root[1]);
    let limit = if is_unc { 3 } else { 0 };
    let dot: &[u16] = &[u16::from(b'.')];
    let dot_dot: &[u16] = &[u16::from(b'.'), u16::from(b'.')];
    let mut target = 0;
    for i in 0..dirs.len() {
        if style.is_windows() {
            dirs[i] = trim_end(dirs[i]);
        }
        if (!(is_unc && i == 2) && dirs[i] == dot) || (i != 0 && dirs[i].is_empty()) {
            continue;
        } else if dirs[i] == dot_dot {
            if target > limit {
                target -= 1;
            }
        } else {
            dirs[target] = dirs[i];
            target += 1;
        }
    }
    if target == 0 || (target == 1 && dirs[0].is_empty()) {
        return Ok(root);
    }
    let mut ret = dirs[..target].join(&style.sep());
    if !style.is_windows() {
        if !root.is_empty() && !ret.is_empty() && ret[0] != u16::from(b'/') {
            let mut rooted = root;
            rooted.extend_from_slice(&ret);
            ret = rooted;
        }
        return Ok(ret);
    }
    if is_unc {
        ret.insert(0, style.sep());
    }
    if !same_root(style, &root, &ret) {
        let mut rooted = root.clone();
        rooted.extend_from_slice(&ret);
        ret = rooted;
    }
    if is_unc {
        Ok(ret)
    } else if !style.is_separator(path[0]) && same_root(style, &root, path) {
        if ret.len() <= 2 && ret.last() != Some(&style.sep()) {
            ret.push(style.sep());
        }
        Ok(ret)
    } else if cwd.len() > 1 && cwd[1] == style.volume() {
        // DOS local file path.
        if ret.is_empty() || style.is_separator(ret[0]) {
            ret.push(u16::from(b'\\'));
        }
        let mut full = cwd[..2].to_vec();
        full.extend_from_slice(&ret);
        Ok(full)
    } else if cwd.last().is_some_and(|&c| style.is_separator(c)) && style.is_separator(ret[0]) {
        let mut full = cwd.to_vec();
        full.extend_from_slice(&ret[1..]);
        Ok(full)
    } else {
        let mut full = cwd.to_vec();
        full.extend_from_slice(&ret);
        Ok(full)
    }
}

/// `WindowsDriveAdjustment`, except the `GetFullPathName` calls (see the module notes).
fn windows_drive_adjustment(style: PathStyle, path: &[u16], cwd: &[u16]) -> Result<Vec<u16>, MonoException> {
    if path.len() < 2 {
        if path.len() == 1 && (path[0] == u16::from(b'\\') || path[0] == u16::from(b'/')) {
            return get_path_root(style, cwd);
        }
        return Ok(path.to_vec());
    }
    let is_ascii_letter = u8::try_from(path[0]).is_ok_and(|c| c.is_ascii_alphabetic());
    if path[1] != u16::from(b':') || !is_ascii_letter {
        return Ok(path.to_vec());
    }
    let same_drive = cwd.first() == Some(&path[0]);
    if path.len() == 2 {
        if same_drive {
            return Ok(cwd.to_vec());
        }
    } else if path[2] != style.sep() && path[2] != style.alt() && same_drive {
        return combine_units(style, cwd, &path[2..]);
    }
    Ok(path.to_vec())
}

/// `Path.GetFullPath(path)` (`InsecureGetFullPath`), with `cwd` as `Directory.GetCurrentDirectory()`.
pub fn get_full_path(style: PathStyle, path: &str, cwd: &str) -> Result<String, MonoException> {
    Ok(text(&get_full_path_units(style, &units(path), &units(cwd))?))
}

fn get_full_path_units(style: PathStyle, path: &[u16], cwd: &[u16]) -> Result<Vec<u16>, MonoException> {
    if chars::trim(path).is_empty() {
        return Err(argument("The specified path is not of a legal form (empty)."));
    }
    let mut path = if style.is_windows() { windows_drive_adjustment(style, path, cwd)? } else { path.to_vec() };
    let end = path[path.len() - 1];
    let mut canonicalize = true;
    if path.len() >= 2 && style.is_separator(path[0]) && style.is_separator(path[1]) {
        if path.len() == 2 || !path[2..].contains(&path[0]) {
            return Err(argument("UNC paths should be of the form \\\\server\\share."));
        }
        if path[0] != style.sep() {
            path = path.iter().map(|&c| if c == style.alt() { style.sep() } else { c }).collect();
        }
    } else if !is_rooted(style, &path)? {
        if !style.is_windows() {
            canonicalize = has_dot_segment(style, &path);
        }
        let mut full = cwd.to_vec();
        if cwd.last() != Some(&style.sep()) {
            full.push(style.sep());
        }
        full.extend_from_slice(&path);
        path = full;
    } else if style.is_windows() && path.len() >= 2 && style.is_separator(path[0]) && !style.is_separator(path[1]) {
        // Like `\abc\def`: the current directory's drive.
        if cwd.len() > 1 && cwd[1] == style.volume() {
            let mut full = cwd[..2].to_vec();
            full.extend_from_slice(&path);
            path = full;
        } else {
            path = cwd_unc_prefix(cwd);
        }
    }
    if canonicalize {
        path = canonicalize_path(style, &path, cwd)?;
    }
    if style.is_separator(end) && path.last() != Some(&style.sep()) {
        path.push(style.sep());
    }
    Ok(path)
}

/// The non-Windows shortcut: a relative path is canonicalized only if some `.` ends a segment (`./`, `../`, a
/// trailing `.`).
fn has_dot_segment(style: PathStyle, path: &[u16]) -> bool {
    let mut start = 0;
    while let Some(found) = path[start..].iter().position(|&c| c == u16::from(b'.')) {
        start += found + 1;
        if start == path.len() || style.is_separator(path[start]) {
            return true;
        }
    }
    false
}

/// `current.Substring(0, current.IndexOf('\\', current.IndexOf("\\\\") + 1))`: everything before the backslash that
/// follows the first `\\` of a UNC current directory (the C# replaces the path with it).
fn cwd_unc_prefix(cwd: &[u16]) -> Vec<u16> {
    let backslash = u16::from(b'\\');
    let double = cwd.windows(2).position(|pair| pair == [backslash, backslash]).map_or(-1, |i| i as isize);
    let from = (double + 1) as usize;
    let end = cwd[from..].iter().position(|&c| c == backslash).map_or(cwd.len(), |i| from + i);
    cwd[..end].to_vec()
}

/// `UnturnedPaths.RootDirectory.FullName` for the install at `root`: Unity's `new DirectoryInfo(Application.dataPath)
/// .Parent`, where `Application.dataPath` is the install folder followed by `/Unturned_Data` (V11).
pub fn root_full_name(style: PathStyle, root: &str, cwd: &str) -> Result<String, MonoException> {
    let cwd = units(cwd);
    let mut data_path = units(root);
    data_path.extend(units("/Unturned_Data"));
    let data_full = get_full_path_units(style, &data_path, &cwd)?;
    // `DirectoryInfo.Parent`: the trailing-separator trim is a no-op, since the full path doesn't end with one.
    let parent = get_directory_name_units(style, &data_full)?.unwrap_or_default();
    Ok(text(&get_full_path_units(style, &parent, &cwd)?))
}
