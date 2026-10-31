//! Locating Steam's library folders and an app's install folder.
//!
//! Steam's `libraryfolders.vdf` and `appmanifest_*.acf` use Valve's KeyValues text format. This is not one of the
//! original game's formats, so it is Pinewatch's own code (the original simply runs from its own folder).

use std::fs;
use std::io;
use std::path::PathBuf;

/// Steam app ID of the original game.
pub const UNTURNED_APP_ID: &str = "304930";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Value {
    Str(String),
    Object(Vec<(String, Value)>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Str(String),
    Open,
    Close,
}

fn tokenize(text: &str) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {}
            '{' => tokens.push(Token::Open),
            '}' => tokens.push(Token::Close),
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '"' => {
                let mut s = String::new();
                let mut closed = false;
                while let Some(c) = chars.next() {
                    match c {
                        '"' => {
                            closed = true;
                            break;
                        }
                        '\\' => match chars.next()? {
                            'n' => s.push('\n'),
                            't' => s.push('\t'),
                            other => s.push(other),
                        },
                        other => s.push(other),
                    }
                }
                if !closed {
                    return None;
                }
                tokens.push(Token::Str(s));
            }
            other => {
                let mut s = String::from(other);
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() || matches!(c, '{' | '}' | '"') {
                        break;
                    }
                    s.push(c);
                    chars.next();
                }
                tokens.push(Token::Str(s));
            }
        }
    }
    Some(tokens)
}

fn parse_pairs(tokens: &[Token], i: &mut usize, nested: bool) -> Option<Vec<(String, Value)>> {
    let mut pairs = Vec::new();
    loop {
        match tokens.get(*i) {
            None => return if nested { None } else { Some(pairs) },
            Some(Token::Close) => return if nested { Some(pairs) } else { None },
            Some(Token::Open) => return None,
            Some(Token::Str(key)) => {
                *i += 1;
                match tokens.get(*i)? {
                    Token::Str(value) => {
                        pairs.push((key.clone(), Value::Str(value.clone())));
                        *i += 1;
                    }
                    Token::Open => {
                        *i += 1;
                        let inner = parse_pairs(tokens, i, true)?;
                        *i += 1; // the closing brace
                        pairs.push((key.clone(), Value::Object(inner)));
                    }
                    Token::Close => return None,
                }
            }
        }
    }
}

fn parse(text: &str) -> Option<Vec<(String, Value)>> {
    let tokens = tokenize(text)?;
    let mut i = 0;
    parse_pairs(&tokens, &mut i, false)
}

fn object<'a>(pairs: &'a [(String, Value)], key: &str) -> Option<&'a [(String, Value)]> {
    pairs.iter().find_map(|(k, v)| match v {
        Value::Object(inner) if k.eq_ignore_ascii_case(key) => Some(inner.as_slice()),
        _ => None,
    })
}

fn string<'a>(pairs: &'a [(String, Value)], key: &str) -> Option<&'a str> {
    pairs.iter().find_map(|(k, v)| match v {
        Value::Str(s) if k.eq_ignore_ascii_case(key) => Some(s.as_str()),
        _ => None,
    })
}

/// Library folder paths listed in a `libraryfolders.vdf`, in file order.
pub fn library_paths(vdf: &str) -> Vec<String> {
    let Some(root) = parse(vdf) else { return Vec::new() };
    let Some(libraries) = object(&root, "libraryfolders") else { return Vec::new() };
    libraries
        .iter()
        .filter_map(|(_, library)| match library {
            Value::Object(fields) => string(fields, "path").map(str::to_string),
            Value::Str(_) => None,
        })
        .collect()
}

/// The `installdir` of an `appmanifest_<id>.acf`.
pub fn app_install_dir(acf: &str) -> Option<String> {
    let root = parse(acf)?;
    string(object(&root, "AppState")?, "installdir").map(str::to_string)
}

/// Steam's own folder: the `SteamPath` registry value on Windows, otherwise the usual locations.
pub fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    #[cfg(windows)]
    {
        if let Some(path) = registry_steam_path() {
            roots.push(path);
        }
        roots.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
    }
    #[cfg(not(windows))]
    {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            roots.push(home.join(".local/share/Steam"));
            roots.push(home.join(".steam/steam"));
            roots.push(home.join("Library/Application Support/Steam"));
        }
    }
    roots
}

/// Finds an app's install folder through Steam's library folders. `Ok(None)` if no library has it.
pub fn find_app_install(app_id: &str) -> io::Result<Option<PathBuf>> {
    let mut last_error = None;
    let mut read_any = false;
    for steam in steam_roots() {
        let vdf = match fs::read_to_string(steam.join("steamapps").join("libraryfolders.vdf")) {
            Ok(text) => text,
            Err(e) => {
                last_error = Some(e);
                continue;
            }
        };
        read_any = true;
        let mut libraries: Vec<PathBuf> = library_paths(&vdf).into_iter().map(PathBuf::from).collect();
        libraries.push(steam.clone());
        for library in libraries {
            let steamapps = library.join("steamapps");
            let Ok(acf) = fs::read_to_string(steamapps.join(format!("appmanifest_{app_id}.acf"))) else { continue };
            if let Some(dir) = app_install_dir(&acf) {
                return Ok(Some(steamapps.join("common").join(dir)));
            }
        }
    }
    match (read_any, last_error) {
        (false, Some(e)) => Err(e),
        _ => Ok(None),
    }
}

#[cfg(windows)]
fn registry_steam_path() -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW};

    let subkey: Vec<u16> = "Software\\Valve\\Steam\0".encode_utf16().collect();
    let value: Vec<u16> = "SteamPath\0".encode_utf16().collect();
    let mut buffer = vec![0u16; 2048];
    let mut size = (buffer.len() * 2) as u32;
    // SAFETY: valid NUL-terminated strings, a writable buffer and its size in bytes.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if status != 0 {
        return None;
    }
    let len = (size as usize / 2).saturating_sub(1);
    Some(PathBuf::from(OsString::from_wide(&buffer[..len.min(buffer.len())])))
}
