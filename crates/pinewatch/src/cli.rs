//! Command-line options.

use std::ffi::OsString;
use std::path::PathBuf;

pub const USAGE: &str = "Usage: pinewatch [--unturned <dir>] [--map <name>] [--screenshot <file.png>]

  --unturned <dir>         Unturned install folder (default: PINEWATCH_UNTURNED_DIR, then Steam's library folders)
  --map <name>             Map folder under Maps/ (default: PEI)
  --screenshot <file.png>  Save one frame after the terrain loads, then exit
  --help                   Show this help";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cli {
    pub unturned: Option<PathBuf>,
    pub map: String,
    pub screenshot: Option<PathBuf>,
    pub help: bool,
}

/// Parses the arguments, including the program name in first position.
pub fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Cli, String> {
    let mut cli = Cli { unturned: None, map: "PEI".into(), screenshot: None, help: false };
    let mut args = args.into_iter().skip(1);
    while let Some(arg) = args.next() {
        let name = arg.to_string_lossy().into_owned();
        let mut value = || args.next().ok_or_else(|| format!("{name} needs a value"));
        match name.as_str() {
            "--unturned" => cli.unturned = Some(PathBuf::from(value()?)),
            "--map" => cli.map = value()?.to_string_lossy().into_owned(),
            "--screenshot" => cli.screenshot = Some(PathBuf::from(value()?)),
            "--help" | "-h" => cli.help = true,
            other => return Err(format!("Unknown option: {other}")),
        }
    }
    Ok(cli)
}
