//! `System.IO.Path` as Unity's Mono computes it (Mono's own `System.IO/Path.cs`), for both platform styles, so the
//! Windows reference behavior is checked on every host.

use pinewatch_io::mono_path::{
    PathStyle::{Unix, Windows},
    change_extension_to_none, combine, get_directory_name, get_file_name, get_file_name_without_extension,
    get_full_path, root_full_name,
};
use pinewatch_mono::ExceptionKind;

fn argument_message(result: Result<impl std::fmt::Debug, pinewatch_mono::MonoException>) -> String {
    let error = result.expect_err("an ArgumentException");
    assert_eq!(error.kind, ExceptionKind::Argument);
    error.message
}

#[test]
fn combine_windows() {
    assert_eq!(combine(Windows, "C:\\a", "b").unwrap(), "C:\\a\\b");
    assert_eq!(combine(Windows, "C:\\a\\", "b").unwrap(), "C:\\a\\b");
    assert_eq!(combine(Windows, "C:\\a/", "b").unwrap(), "C:\\a/b");
    assert_eq!(combine(Windows, "C:", "b").unwrap(), "C:b");
    assert_eq!(combine(Windows, "a", "C:\\b").unwrap(), "C:\\b");
    assert_eq!(combine(Windows, "a", "\\b").unwrap(), "\\b");
    assert_eq!(combine(Windows, "a", "/b").unwrap(), "/b");
    assert_eq!(combine(Windows, "", "b").unwrap(), "b");
    assert_eq!(combine(Windows, "a", "").unwrap(), "a");
    assert_eq!(argument_message(combine(Windows, "a|", "b")), "Illegal characters in path.");
    assert_eq!(argument_message(combine(Windows, "a", "b\u{1}")), "Illegal characters in path.");
    // An empty part is returned before the other is checked.
    assert_eq!(combine(Windows, "", "b|").unwrap(), "b|");
}

#[test]
fn combine_unix() {
    assert_eq!(combine(Unix, "/a", "b").unwrap(), "/a/b");
    assert_eq!(combine(Unix, "/a/", "b").unwrap(), "/a/b");
    assert_eq!(combine(Unix, "a", "/b").unwrap(), "/b");
    assert_eq!(combine(Unix, "a:", "b").unwrap(), "a:/b");
    assert_eq!(combine(Unix, "a\\", "b").unwrap(), "a\\/b");
    assert_eq!(combine(Unix, "a|", "b").unwrap(), "a|/b");
    assert_eq!(argument_message(combine(Unix, "a\0", "b")), "Illegal characters in path.");
}

#[test]
fn file_name() {
    assert_eq!(get_file_name(Windows, "C:\\a\\b.dat").unwrap(), "b.dat");
    assert_eq!(get_file_name(Windows, "C:b.dat").unwrap(), "b.dat");
    assert_eq!(get_file_name(Windows, "a/b").unwrap(), "b");
    assert_eq!(get_file_name(Windows, "a\\").unwrap(), "");
    assert_eq!(get_file_name(Windows, "").unwrap(), "");
    assert_eq!(get_file_name(Unix, "/a/b.dat").unwrap(), "b.dat");
    assert_eq!(get_file_name(Unix, "a\\b").unwrap(), "a\\b");
    assert_eq!(argument_message(get_file_name(Windows, "a<b")), "Illegal characters in path.");
}

#[test]
fn file_name_without_extension() {
    assert_eq!(get_file_name_without_extension(Windows, "C:\\a\\x.tar.gz").unwrap(), "x.tar");
    assert_eq!(get_file_name_without_extension(Windows, "C:\\a.b\\c").unwrap(), "c");
    assert_eq!(get_file_name_without_extension(Windows, "C:\\a\\.dat").unwrap(), "");
    assert_eq!(get_file_name_without_extension(Windows, "a\\b.").unwrap(), "b");
    assert_eq!(get_file_name_without_extension(Unix, "/a.b/c").unwrap(), "c");
    assert_eq!(change_extension_to_none(Windows, "C:\\a\\Item.asset").unwrap(), "C:\\a\\Item");
    assert_eq!(change_extension_to_none(Windows, "C:\\a.b\\Item").unwrap(), "C:\\a.b\\Item");
    assert_eq!(change_extension_to_none(Unix, "/a.b/c").unwrap(), "/a.b/c");
}

#[test]
fn directory_name_windows() {
    assert_eq!(get_directory_name(Windows, "C:\\a\\b.dat").unwrap().as_deref(), Some("C:\\a"));
    assert_eq!(get_directory_name(Windows, "C:\\a").unwrap().as_deref(), Some("C:\\"));
    assert_eq!(get_directory_name(Windows, "C:\\").unwrap(), None);
    assert_eq!(get_directory_name(Windows, "C:\\a\\\\b").unwrap().as_deref(), Some("C:\\a"));
    assert_eq!(get_directory_name(Windows, "a/b/c").unwrap().as_deref(), Some("a\\b"));
    assert_eq!(get_directory_name(Windows, "a//b").unwrap().as_deref(), Some("a"));
    assert_eq!(get_directory_name(Windows, "C:a").unwrap().as_deref(), Some("C:"));
    assert_eq!(get_directory_name(Windows, "a").unwrap().as_deref(), Some(""));
    assert_eq!(
        get_directory_name(Windows, "\\\\server\\share\\x").unwrap().as_deref(),
        Some("\\\\server\\share")
    );
    assert_eq!(argument_message(get_directory_name(Windows, "")), "Invalid path");
    assert_eq!(argument_message(get_directory_name(Windows, "  ")), "The specified path is not of a legal form.");
    assert_eq!(argument_message(get_directory_name(Windows, "a\\b|")), "Illegal characters in path.");
}

#[test]
fn directory_name_unix() {
    assert_eq!(get_directory_name(Unix, "/a/b").unwrap().as_deref(), Some("/a"));
    assert_eq!(get_directory_name(Unix, "/a").unwrap().as_deref(), Some("/"));
    assert_eq!(get_directory_name(Unix, "/").unwrap(), None);
    assert_eq!(get_directory_name(Unix, "a").unwrap().as_deref(), Some(""));
    assert_eq!(get_directory_name(Unix, "/a//b/").unwrap().as_deref(), Some("/a/b"));
}

#[test]
fn full_path_windows() {
    let cwd = "C:\\x";
    assert_eq!(get_full_path(Windows, "C:\\a\\..\\b", cwd).unwrap(), "C:\\b");
    assert_eq!(get_full_path(Windows, "C:/a/./b/", cwd).unwrap(), "C:\\a\\b\\");
    assert_eq!(get_full_path(Windows, "rel\\p", cwd).unwrap(), "C:\\x\\rel\\p");
    assert_eq!(get_full_path(Windows, "C:\\a \\b", cwd).unwrap(), "C:\\a\\b");
    // A leading space hides the drive, so the path counts as relative to the current directory.
    assert_eq!(get_full_path(Windows, " C:\\a\\b ", cwd).unwrap(), "C:\\x\\ C:\\a\\b");
    assert_eq!(get_full_path(Windows, "\\abc", "D:\\x").unwrap(), "D:\\abc");
    assert_eq!(get_full_path(Windows, "C:", cwd).unwrap(), "C:\\x");
    assert_eq!(get_full_path(Windows, "C:sub", cwd).unwrap(), "C:\\x\\sub");
    assert_eq!(get_full_path(Windows, "C:\\..\\..", cwd).unwrap(), "C:\\");
    assert_eq!(
        argument_message(get_full_path(Windows, "  ", cwd)),
        "The specified path is not of a legal form (empty)."
    );
    assert_eq!(
        argument_message(get_full_path(Windows, "\\\\server", cwd)),
        "UNC paths should be of the form \\\\server\\share."
    );
}

#[test]
fn full_path_unix() {
    let cwd = "/x";
    assert_eq!(get_full_path(Unix, "/a/../b", cwd).unwrap(), "/b");
    assert_eq!(get_full_path(Unix, "/a/b/", cwd).unwrap(), "/a/b/");
    assert_eq!(get_full_path(Unix, "/a//b", cwd).unwrap(), "/a/b");
    assert_eq!(get_full_path(Unix, "./a", cwd).unwrap(), "/x/a");
    // A relative path is only canonicalized when some `.` ends a segment.
    assert_eq!(get_full_path(Unix, "a//b", cwd).unwrap(), "/x/a//b");
    assert_eq!(get_full_path(Unix, "a.b", cwd).unwrap(), "/x/a.b");
    assert_eq!(get_full_path(Unix, "/a /b", cwd).unwrap(), "/a /b");
}

/// `UnturnedPaths.RootDirectory` is `new DirectoryInfo(Application.dataPath).Parent`: the install folder through
/// `GetFullPath`, without a trailing separator (V11).
#[test]
fn install_root_full_name() {
    assert_eq!(root_full_name(Windows, "C:\\Steam\\Unturned", "C:\\x").unwrap(), "C:\\Steam\\Unturned");
    assert_eq!(root_full_name(Windows, "C:/Steam/Unturned/", "C:\\x").unwrap(), "C:\\Steam\\Unturned");
    assert_eq!(root_full_name(Windows, "C:\\Steam\\x\\..\\Unturned", "C:\\x").unwrap(), "C:\\Steam\\Unturned");
    assert_eq!(root_full_name(Windows, "C:\\", "C:\\x").unwrap(), "C:\\");
    assert_eq!(root_full_name(Unix, "/home/u/Unturned", "/x").unwrap(), "/home/u/Unturned");
    assert_eq!(root_full_name(Unix, "/home/u/x/../Unturned/", "/x").unwrap(), "/home/u/Unturned");
    assert_eq!(root_full_name(Unix, "Unturned", "/x").unwrap(), "/x/Unturned");
}
