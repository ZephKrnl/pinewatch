use pinewatch_io::steam::{app_install_dir, library_paths};

const LIBRARY_FOLDERS: &str = r#""libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"label"		""
		"apps"
		{
			"228980"		"1"
		}
	}
	// A second library on another drive.
	"1"
	{
		"path"		"D:\\SteamLibrary"
		"apps"
		{
			"304930"		"2156"
		}
	}
}
"#;

const APP_MANIFEST: &str = r#""AppState"
{
	"appid"		"304930"
	"name"		"Unturned"
	"installdir"		"Unturned"
	"buildid"		"25613056"
}
"#;

#[test]
fn library_paths_in_file_order_with_escapes_resolved() {
    assert_eq!(
        library_paths(LIBRARY_FOLDERS),
        vec!["C:\\Program Files (x86)\\Steam".to_string(), "D:\\SteamLibrary".to_string()]
    );
}

#[test]
fn app_manifest_install_dir() {
    assert_eq!(app_install_dir(APP_MANIFEST), Some("Unturned".to_string()));
}

#[test]
fn malformed_vdf_yields_nothing_instead_of_panicking() {
    assert!(library_paths("\"libraryfolders\" { \"0\" { \"path\" \"unterminated").is_empty());
    assert_eq!(app_install_dir("{ } } {"), None);
}
