# File format inventory (step 1 research)

Surveyed 2026-10-31 from the U3-SDK source (commit `aba323a`) and the installed game
(`C:\Program Files (x86)\Steam\steamapps\common\Unturned`, read-only).
C# paths are relative to `Assets/Runtime/` in the U3-SDK source at tag `v3.26.3.12`.

## What the install contains

| Type | Files | Notes |
|---|---:|---|
| Text `.dat` | 8,333 | 8,163 in Bundles, 144 in Localization, 23 in Maps, rest in Extras/Worlds |
| `.asset` (text) | 774 | Newer asset format (`Metadata { }` + `Asset { }`) |
| Binary `.dat` | 443 | 386 map files, 51 singleplayer saves in `Worlds/`, 6 in `Cloud/` and `Extras/`. 38 distinct layouts |
| `.heightmap` / `.splatmap` | 160 / 160 | Terrain tiles |
| `.png` / `.jpg` | 167 / 37 | Map previews, charts, icons, loading screens |
| `.unity3d` / `.masterbundle` / `.content` | 55 / 1 / 1 | Unity bundles (step 2, not step 1) |
| `.json` | 13 | Map and game config |
| `.hierarchy` | 9 | One per map |
| `.blob` | 7 | Foliage |
| `.bin` | 8 | `EconInfo.bin` + Germany terrain holes |

Text files: all valid UTF-8, no UTF-16. 5,806 of 9,107 `.dat`/`.asset` files start with a UTF-8 BOM.
7,891 text files use CRLF line endings, 2 use LF.

## Text formats

### 1. DAT (`UnturnedDat/DatTokenizer.cs`, `DatParser.cs`)
Used by: every asset definition (`.dat`, `.asset`), translations (`English.dat`, `Localization/**`),
`MasterBundle.dat`, map `English.dat`.

- Two stages: the tokenizer turns the whole file into tokens, then the parser builds the tree.
- Root is a dictionary. Values: string or null, dictionary `{ }`, list `[ ]`.
- **Keys are case-insensitive** (`OrdinalIgnoreCase`), insertion order kept, first spelling kept.
- **Duplicate key: last value wins**, error reported.
- Unquoted key runs to the next whitespace. Unquoted value runs to end of line, so `//` and `[` inside it are plain text.
- Quoted strings can span lines. Escapes `\n` `\t` `\\` `\"` (`\"` only when quoted). Unknown escape keeps the backslash and reports an error.
- `/` at a token start begins a comment to end of line. Same-line comments only work after a quoted value.
- One comma after `{ } [ ]` or a quoted string is skipped (mods add stray commas).
- `Key Value` followed by `{` or `[` on the **next** line: the dictionary/list wins and the value is dropped. Only one line break is skipped.
- Errors never stop parsing (third-party mods rely on this).
- Line counting: `\r`, `\n`, `\r\n` each count once. The parser counts line-break tokens, so line numbers drift after multi-line quoted strings.
- Optional metadata mode keeps comments and line numbers (editor tooling).
- Reading: `StreamReader` → UTF-8 with BOM detection, invalid bytes become U+FFFD. The file hash is SHA-1 of the raw bytes.

Value helpers (`DatValueEx`, `DatDictionaryEx`, `DatListEx`, `UnityDatEx`, `UnityDatColorEx`, `Files/UnturnedDatEx.cs`):
- Integers and floats: .NET `NumberStyles.Any` + invariant culture. Accepts `1,000`, `(5)` = -5, `5-`, `5.0` for ints, `1e3`, surrounding whitespace, `Infinity`, `NaN`.
- Bool: one character `y`/`t`/`1` or `n`/`f`/`0` (case-sensitive), otherwise .NET `bool.TryParse` (`true`/`false`, any case, trimmed).
- Enum: name (any case), number, or comma-separated flags.
- GUID: all five .NET formats (`N`, `D`, `B`, `P`, `X`). `0` or empty means "no GUID" in reference helpers.
- Vector2/3: `x, y, z`, optionally in parentheses, or a `{ X Y Z }` dictionary. Legacy form `Key_X`, `Key_Y`, `Key_Z`.
- Colors: hex `#RRGGBB` / `#RRGGBBAA` (`HexNumber` style, so `" F"` is a valid pair), a `{ R G B A }` dictionary, or legacy `Key_R`… floats.
- Asset refs: GUID, `Type:ID` (legacy ID parsed with `NumberStyles.Integer` + *current* culture), or `{ GUID }` / `{ Type ID }`.
- Master bundle refs: `bundle:path` or bare `path` (uses the current bundle).
- `Type` values: assembly-qualified C# type names, rejected if they contain `\` `:` `/`.
- Edge cases where C# **throws** (e.g. vector value `"("`): Rust must report a failure instead.

### 2. Legacy Data (`Assembly-CSharp/Unturned/Files/Data.cs`)
Line-based `key value` split at the first space. `//` comments only at line start. **Case-sensitive keys.**
Duplicate: last wins, error reported. Bool: `y`, `1`, `true` (case-insensitive). Euler rotations stored as bytes.
Today only the editor's spawn-table tool uses it. Small; port for completeness.

### 3. KeyValueTable (`Assembly-CSharp/Framework/IO/FormattedFiles/KeyValueTables/`)
Used by: `Level.hierarchy` (devkit objects, landscape tiles, foliage system) and editor tool option files.
- `"key" "value"`, `"key" { … }`, `"key" [ "a" { … } ]`. Everything outside quotes except braces/brackets is ignored.
- Backslash makes the next character literal (no `\n` translation).
- **Keys case-sensitive. Duplicate key: first wins.** A repeated object key merges, a repeated list key appends.
- Nested lists are not supported (an inner `[` is ignored and its `]` closes the outer list).
- Own value rules: bool accepts `true/false/0/1/yes/no/y/n/t/f` (any case). Vectors, quaternions (`X Y Z W`, not Euler) and colors are sub-objects.
- C# type names go through a redirector registry for renamed types.

### 4. JSON (Newtonsoft.Json, `Files/ReadWrite.cs`)
Used by: map `Config.json`, server `Config.json`, `Preferences.json`, `Status.json`, `StreamerNames.json`, `ModInfo.json`,
`Settings/Graphics.json`, `Cloud/ConvenientSavedata.json`, `WorkshopDownloadConfig.json`, `*.module` (mod manifests), economy JSON.
Newtonsoft defaults matter for user-edited files: case-insensitive property names, comments allowed, missing fields keep
defaults ("populate"), enums by name or number. The SDK's XML helpers have no callers.

## Binary formats

### 5. Block (`Files/Block.cs`), whole file in memory
Little-endian. Strings: u8 length + UTF-8. Byte arrays: u8 length (or i32 < 30,000 in "long" mode).
**GUID: u8 length (16) + 16 bytes in .NET layout** (first three fields little-endian).
Bool arrays: u16 count + bits, least significant first. Compressed position `UInt16RVector3`: region byte + u16 fractions
(region size 128, offsets −4096 / −1024). Rotation: three f32 Euler degrees. Color: three bytes.
**Out-of-range reads return 0 or empty and do not advance.** Some arrays are not bounds-checked in C# (would throw).
Used by: `Level.dat`, `Spawns/Zombies.dat`, `Spawns/Items.dat`, `Environment/Lighting.dat`, `Terrain/Heights.dat`,
`Editor/*.dat`, player saves (`Player`, `Life`, `Inventory`, `Clothing`, `Skills`, `Anim`, `Quests`…),
client settings (`Options`, `Controls`, `Play`, `Filters`), `Characters.dat`, `Ignored_Blueprints.dat`.

### 6. River (`Files/River.cs`), streamed from a file
Same primitives, except **byte arrays and GUIDs use a u16 length prefix**.
At end of file, bytes read as 0, and numbers can return **stale bytes from the previous read** (C# bug).
**Opens files read-write and creates missing folders even when reading.** The Rust version must never do this.
Used by: `Level/Objects.dat`, `Level/Buildables.dat`, `Level/Visibility.dat`, legacy `Objects/Objects_x_y.dat`,
`Spawns/{Players,Jars,Vehicles,Animals,Fauna}.dat` and legacy `_x_y` variants, `Environment/{Roads,Paths,Nodes,Bounds,Flags,Flags_Data}.dat`,
`Environment/Navigation_N.dat`, `Terrain/Trees.dat`, server world saves, `ServerBookmarks.bin`, `BlockedPlayers.bin`.

Map files start with a version byte that gates optional fields. Versions seen in the install:

| File | Versions seen | File | Versions seen |
|---|---|---|---|
| `Objects.dat` | 1, 2, 9, 11, 12 | `Nodes.dat` | 2, 5, 6, 9 |
| `Buildables.dat` | 9, 12 | `Paths.dat` | 4, 5, 6 |
| `Trees.dat` | 5, 6, 8 | `Flags_Data.dat` | 3, 5 |
| `Zombies.dat` | 8, 9, 10 | `Lighting.dat` | 7, 9, 11, 12 |
| `Vehicles.dat` | 4, 16, 17 | `Roads.dat` | 1, 2 |
| `Level.dat` | 1, 2 | `Navigation_N.dat` (121 files) | 1 |

`Terrain/Materials.dat`, `Details.dat`, `Resources.dat` exist in maps but are only copied for new levels. Current code does not read them.

### 7. .NET BinaryReader dialect
Little-endian, strings with a 7-bit variable-length prefix, bool as one byte.
Used by: `Foliage.blob`, legacy `Foliage/Tile_x_y.foliage`, `EconInfo.bin` (version 3, 16,184 entries).

### 8. Terrain tiles (`Framework/Landscapes/LandscapeTile.cs`)
- `Landscape/Heightmaps/Tile_X_Y_Source.heightmap`: 257 × 257 **big-endian** u16, x-major.
- `Landscape/Splatmaps/Tile_X_Y_Source.splatmap`: 256 × 256 × layers u8.
- `Landscape/Holes/Tile_X_Y.bin`: version byte + 256 × 256 bits.
- Tile coordinates and material references live in `Level.hierarchy`.
- Legacy terrain: `Terrain/Heights.dat` (Block), `Terrain/Heightmap.png`, `Terrain/AlphamapHQ_N.png`.

### 9. `Foliage.blob` (`Framework/Foliage/FoliageStorageV2.cs`)
Header: i32 version (1 or 2), i32 tile count, per tile (i32 x, i32 y, i64 offset), v2 adds an asset GUID table.
Tile blobs are read on demand: per asset, a list of 4×4 f32 matrices (column-major) + bool. Only the header is hashed.

### 10. Navmesh (`Level/UnturnedPathfinding_Empty.cs`)
`Navigation_N.dat` after its version byte: bounds center and size, tile counts, per tile u16 triangle indices
and i32 × 3 vertices. The real pathfinding library is not in the SDK, but this stub documents the file layout.

### 11. Other
- Images: PNG/JPG through a standard decoder.
- `core.masterbundle.hash`: per-platform bundle hashes for multiplayer checks (step 2).
- `*.manifest`: Unity build manifests, not read by the game.
- Unity bundles (`core.masterbundle` UnityFS v8 / Unity 2022.3.62f3, `core.content` UnityFS v6 / Unity 2017.4.16f1, map `.unity3d`): step 2.
- `*.module` manifests point at C# DLL mods. Those cannot run in a Rust port.

## Behaviors the Rust code must reproduce

| Area | Original behavior |
|---|---|
| Text decoding | UTF-8 with BOM detection, invalid bytes → U+FFFD |
| Whitespace | .NET `char.IsWhiteSpace` set (same as Rust `char::is_whitespace`) |
| Key comparison | DAT: `OrdinalIgnoreCase`. Legacy Data and KVT: exact match |
| Numbers | `NumberStyles.Any`, `Integer` and `HexNumber`, invariant or current culture, depending on the call site |
| Bool, enum, GUID, date | .NET `bool.TryParse`, `Enum.TryParse(ignoreCase)`, `Guid.TryParse`, `DateTime.TryParse` |
| GUID bytes | .NET in-memory layout, text form `N` (32 lowercase hex) |
| Math types | Unity left-handed Y-up, Euler degrees (ZXY order), column-major matrices |
| Hashes | SHA-1 of raw file bytes; the level hash combines named file hashes |
| Translations | .NET composite formatting (`{0}`), English fallback |
| Storage | Client saves can live in Steam Cloud |

## Existing tests to port

`Assets/Tests/UnturnedDat/`: about 1,040 table-driven NUnit cases (tokenizer, parser, errors, line numbers, comments,
values, vectors, colors, structs, writer). Some assert exact error text.
`Assets/Tests/SDG.NetPak/`: about 94 cases for network packing (step 4).
