//! Startup: options, install discovery, map loading, then the Bevy app.

use std::path::PathBuf;
use std::process::ExitCode;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};
use pinewatch_io::{InstallRoot, StderrLog, find_install};
use pinewatch_level::read_landscape;

use crate::camera::FlyCameraPlugin;
use crate::cli::{USAGE, parse_args};
use crate::terrain::{LoadedTerrain, TerrainPlugin};

/// Exit codes: 2 = bad options, no install or unknown map; 3 = map has no landscape; 4 = map failed to load.
pub fn main() -> ExitCode {
    let cli = match parse_args(std::env::args_os()) {
        Ok(cli) => cli,
        Err(message) => {
            eprintln!("{message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    if cli.help {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }

    let install = match find_install(cli.unturned.clone()) {
        Ok(install) => install,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };

    let maps = list_maps(&install);
    let Some(map) = resolve_map(&maps, &cli.map) else {
        eprintln!(
            "Map '{}' not found in {}.\nAvailable maps: {}",
            cli.map,
            install.root().join("Maps").display(),
            maps.join(", ")
        );
        return ExitCode::from(2);
    };

    let tiles = match read_landscape(&install, &map, &mut StderrLog) {
        Ok(Some(tiles)) if !tiles.is_empty() => tiles,
        Ok(_) => {
            eprintln!(
                "Map '{map}' has no landscape. Maps that use the legacy ground system aren't supported until a later milestone."
            );
            return ExitCode::from(3);
        }
        Err(error) => {
            eprintln!("Failed to load map '{map}': {error}");
            return ExitCode::from(4);
        }
    };

    if let Some(path) = &cli.screenshot {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            if let Err(error) = std::fs::create_dir_all(parent) {
                eprintln!("Can't create {}: {error}", parent.display());
                return ExitCode::from(2);
            }
        }
    }

    run(LoadedTerrain { map, tiles }, cli.screenshot)
}

/// Map folder names under `Maps/`, in the order the OS lists them.
fn list_maps(install: &InstallRoot) -> Vec<String> {
    install
        .read_dir("Maps")
        .unwrap_or_default()
        .into_iter()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| install.is_dir(&format!("Maps/{name}")))
        .collect()
}

/// The folder's own spelling for a requested map name: an exact match, else a case-insensitive one.
fn resolve_map(maps: &[String], requested: &str) -> Option<String> {
    maps.iter()
        .find(|m| *m == requested)
        .or_else(|| maps.iter().find(|m| m.eq_ignore_ascii_case(requested)))
        .cloned()
}

#[derive(Resource)]
struct ScreenshotRequest {
    path: PathBuf,
    frames_left: u32,
    taken: bool,
}

fn run(terrain: LoadedTerrain, screenshot: Option<PathBuf>) -> ExitCode {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.53, 0.70, 0.92)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: "Pinewatch".into(), ..default() }),
            ..default()
        }))
        .insert_resource(terrain)
        .add_plugins((TerrainPlugin, FlyCameraPlugin));

    if let Some(path) = screenshot {
        // Wait for shader pipelines to finish compiling before capturing.
        app.insert_resource(ScreenshotRequest { path, frames_left: 240, taken: false })
            .add_systems(Update, take_screenshot);
    }

    match app.run() {
        AppExit::Success => ExitCode::SUCCESS,
        AppExit::Error(code) => ExitCode::from(code.get()),
    }
}

fn take_screenshot(mut commands: Commands, mut request: ResMut<ScreenshotRequest>) {
    if request.taken {
        return;
    }
    if request.frames_left > 0 {
        request.frames_left -= 1;
        return;
    }
    request.taken = true;
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(request.path.clone()))
        .observe(|_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
            exit.write(AppExit::Success);
        });
}
