//! Json.NET 13.0.3 (`Newtonsoft.Json.dll`, the .NET 4.5 build the game ships), ported: `JsonTextReader`'s tokens so
//! far (M2f task 3). The rest of what `Level.ReadLevelInfo` needs, `JsonConvert.DeserializeObject` into
//! `LevelInfoConfigData`, follows in tasks 4 to 6.

mod date;
mod number;
mod reader;

pub use date::{DateTimeKind, JsonDate};
pub use reader::{JsonInteger, JsonTextReader, JsonToken};
