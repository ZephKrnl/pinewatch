# M1: Bootable Pinewatch with real terrain — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `pinewatch.exe` opens a window titled Pinewatch, finds the Unturned install, and renders a chosen map's real terrain from its landscape tiles, with a fly camera and a screenshot mode for tests.

**Architecture:**
- Ported 1:1 readers for `Level.hierarchy` (KeyValueTable) and the landscape tiles (heightmap, splatmap, holes) feed a Bevy app.
- Terrain is built in Unity space and converted to Bevy space at one boundary (negate Z).
- Colors are temporary placeholders per splat layer until M2 brings the original materials.

**Tech Stack:** Rust 1.99.1 (edition 2024), Bevy 0.20, `sha1`, `winresource` (build-time, Windows executable metadata).

**Spec:** `docs/superpowers/specs/2026-11-01-file-format-readers-design.md` and `docs/ARCHITECTURE.md`.

## Global Constraints

- Production code is Rust only. No Unity, Mono or .NET dependency.
- The install is opened read-only. Nothing is ever written, created, renamed or deleted inside it.
- Ported behavior follows parity rules P1–P8 (spec section 4). Every behavior ported from source but not yet confirmed by the Unity harness gets a line in `docs/VERIFICATION.md`.
- Format crates may depend only on `sha1`, `png`, `windows-sys` (and their transitive dependencies).
- Names: crates `pinewatch-<role>`, executable `pinewatch.exe`, window title and displayed name `Pinewatch`.
- Constants from the source:
  - tile size 1024
  - tile height 2048
  - tile origin `(x * 1024, -1024, y * 1024)`
  - heightmap 257×257 big-endian u16, first index = world Z row, second = world X column
  - splatmap 256×256×8 bytes
  - holes: 1 version byte + 256×256 bits (LSB first), where a 1 bit means solid ground
- Git is local only. No remotes, no pushes.

## Review Focus

1. **No install found** (non-default Steam library, missing game): a clear message naming the three lookup sources and a non-zero exit. Never a panic.
2. **Map without a landscape** (legacy-ground workshop maps): an explicit "not supported until a later milestone" message, then exit.
3. **Truncated or missing tile files:** the original's semantics (missing bytes read as −1, so heights become 65535; the original's warning text for missing files). No crash.
4. **Non-ASCII install paths:** paths stay `OsString`/`PathBuf` end to end, with no lossy conversion before the filesystem.
5. **GPU or window init failure** (this PC also has a virtual display adapter): Bevy's error surfaces readably. Covered manually by the screenshot run.

---

### Task 1: Workspace and licensing

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `.gitattributes`, `LICENSE.txt`, `THIRDPARTYNOTICES.txt`, `README.md`
- Modify: `.gitignore`

- [ ] **Step 1:** Workspace `Cargo.toml` with `members = ["crates/*"]`, `resolver = "3"`, shared `edition = "2024"` and `rust-version = "1.99"`.
- [ ] **Step 2:** `rust-toolchain.toml` pins `1.99.1`.
- [ ] **Step 3:** `.gitattributes`: `*.rs`/`*.toml`/`*.md` as text; `crates/**/tests/fixtures/** -text` and `**/*.bin -text` so fixture bytes are never converted.
- [ ] **Step 4:** `.gitignore` gains `/target`, `/testdata`, `/reference/work`.
- [ ] **Step 5:** `LICENSE.txt` and `THIRDPARTYNOTICES.txt` are byte copies of the SDK's files.
- [ ] **Step 6:** `README.md` contains the spec's section 8 wording verbatim, plus build and run instructions.
- [ ] **Step 7: Verify.** Run `cmp LICENSE.txt reference/u3-sdk/LICENSE.txt && cmp THIRDPARTYNOTICES.txt reference/u3-sdk/THIRDPARTYNOTICES.txt`. Expected: no output. Then `cargo metadata --format-version 1 > /dev/null`. Expected: exit 0.
- [ ] **Step 8: Commit.**

### Task 2: `pinewatch-mono` foundations

**Files:**
- Create: `crates/pinewatch-mono/src/{lib.rs,net_string.rs,text.rs,number.rs,guid.rs,exception.rs}`
- Test: unit tests in each module; `crates/pinewatch-mono/tests/dat_value_int_cases.rs`

**Interfaces (produces):**
- `pub struct NetString(Vec<u16>)` with `from_str`, `as_units`, `len` (UTF-16 units), `is_empty`, `to_string_lossy`, `PartialEq<&str>`
- `pub fn decode_stream_reader_default(bytes: &[u8]) -> NetString`: Mono `StreamReader` defaults, i.e. BOM detection for UTF-8, UTF-16 LE/BE and UTF-32 LE/BE, otherwise UTF-8, with invalid bytes replaced by U+FFFD
- `pub enum ExceptionKind { ArgumentNull, ArgumentOutOfRange, Format, InvalidCast, NullReference, Overflow, OutOfMemory, IndexOutOfRange, Io }` and `pub struct MonoException { pub kind: ExceptionKind, pub message: String }`
- `pub fn try_parse_i32(s: Option<&NetString>, styles: NumberStyles, culture: &NumberFormat) -> Option<i32>`, plus the same for `u32`; `NumberStyles::{ANY, INTEGER}`; `NumberFormat::invariant()`
- `pub struct Guid([u8; 16])` (.NET byte layout) with `Guid::EMPTY`, `Guid::parse(&NetString) -> Result<Guid, MonoException>` (the `new Guid(string)` constructor) and `to_string_n()`

- [ ] **Step 1: Write failing tests.**
  - NetString: a surrogate pair counts as 2 units; a lone surrogate survives a round trip.
  - Decoding: each BOM kind; invalid UTF-8 → U+FFFD.
  - Integers: every `TryParseInt32` and `TryParseUInt32` case from `reference/u3-sdk/Assets/Tests/UnturnedDat/DatValueTests.cs`, plus `"1,000"`→1000, `"(5)"`→-5, `"5-"`→-5, `" 7 "`→7, `"5.0"`→5, `"5.5"`→fail, `"1e3"`→1000, `"¤5"`→5, `"2147483648"`→fail for i32, `None`→fail.
  - GUID: N, D, B, P and X forms parse to the same bytes; `"0"` fails with `Format`; `to_string_n` gives 32 lowercase hex digits.
- [ ] **Step 2:** Run `cargo test -p pinewatch-mono`. Expected: FAIL (items not defined).
- [ ] **Step 3: Implement.** Port Mono's `Number.ParseNumber` state machine (reference source) for integers: whitespace set `0x09..=0x0D, 0x20`; signs, parentheses, currency, group separator only after a digit; trailing `\0` accepted; non-zero fractional digits fail. Port the `Guid` parser from the same reference source.
- [ ] **Step 4:** Run `cargo test -p pinewatch-mono`. Expected: PASS.
- [ ] **Step 5:** Add every ported rule whose exact Mono wording or edge behavior isn't harness-confirmed yet to `docs/VERIFICATION.md`.
- [ ] **Step 6: Commit.**

### Task 3: `pinewatch-io` install access and logs

**Files:**
- Create: `crates/pinewatch-io/src/{lib.rs,install.rs,steam.rs,log.rs}`

**Interfaces (produces):**
- `pub struct InstallRoot` with `open(path) -> io::Result<InstallRoot>`, `root() -> &Path`, `read(&self, rel) -> io::Result<Vec<u8>>`, `exists(&self, rel) -> bool`, `is_dir`, `read_dir(&self, rel) -> io::Result<Vec<OsString>>` (OS order). No write API exists.
- `pub fn find_install(cli: Option<PathBuf>) -> Result<InstallRoot, InstallNotFound>`: checks `--unturned`, then `PINEWATCH_UNTURNED_DIR`, then Steam `libraryfolders.vdf` (Windows registry `SteamPath`, default `C:\Program Files (x86)\Steam`) for app 304930. `InstallNotFound` displays the three sources tried.
- `pub enum LogLevel { Info, Warn, Error }`, `pub trait LogSink { fn log(&mut self, level: LogLevel, text: &str); }`, `pub struct MemoryLog` for tests.

- [ ] **Step 1: Write failing tests.**
  - VDF parsing of a synthetic `libraryfolders.vdf` with two libraries.
  - `find_install` with an explicit path that lacks `Maps/` returns `InstallNotFound`.
  - `InstallRoot` refuses paths that escape the root (`..`).
- [ ] **Step 2:** Run `cargo test -p pinewatch-io`. Expected: FAIL.
- [ ] **Step 3: Implement.** Files are opened with read-only access only.
- [ ] **Step 4:** Run `cargo test -p pinewatch-io`. Expected: PASS. An install test (skipped if none is found) asserts that the real install has `Maps/PEI/Level.dat`.
- [ ] **Step 5: Commit.**

### Task 4: `pinewatch-kvt` reader

**Files:**
- Create: `crates/pinewatch-kvt/src/{lib.rs,reader.rs,values.rs}`
- Test: `crates/pinewatch-kvt/tests/{syntax.rs,install.rs}`

**Interfaces:**
- Consumes: `NetString`, `decode_stream_reader_default`, `try_parse_i32`/`u32`, `Guid`, `MonoException`.
- Produces:
  - `pub enum KvtValue { String(NetString), Table(KvtTable), List(Vec<KvtValue>) }`
  - `pub struct KvtTable` (insertion-ordered, exact-match keys) with `get(&str) -> Option<&KvtValue>` and `contains_key`
  - `pub fn read_kvt(text: &NetString) -> KvtTable`: a port of `KeyValueTableReader(StreamReader)`, including the shared static builder
  - `pub struct KvtCursor<'a>` mirroring `IFormattedFileReader`: `read_key`, `read_array_length(key) -> Result<i32, MonoException>`, `read_array_index`, `read_value() -> Result<Option<NetString>, MonoException>`, `read_object() -> Option<KvtCursor>`, `read_i32() -> Result<i32, MonoException>`, `read_u32() -> Result<u32, MonoException>`, `read_guid() -> Result<Guid, MonoException>`, `read_type_name() -> Result<Option<NetString>, MonoException>` (redirects chased, invalid characters rejected)

- [ ] **Step 1: Write failing tests** (`syntax.rs`), each asserting the original's result:
  - quoted key/value pairs
  - backslash makes the next character literal, even outside quotes
  - `{}` nested tables
  - a list of strings and tables
  - a duplicate scalar key keeps the first value
  - a repeated table key merges, a repeated list key appends
  - an inner `[` is ignored and its `]` ends the outer list
  - a stray root `}` ends parsing
  - text outside quotes is ignored
  - `read_array_length` on a string value → `NullReference`
  - `read_value` on a table element → `InvalidCast`
  - `read_guid` of `"0"` or empty → `Guid::EMPTY`; of `"xyz"` → `Format`
- [ ] **Step 2:** Run `cargo test -p pinewatch-kvt`. Expected: FAIL.
- [ ] **Step 3: Implement** as a direct port of `KeyValueTableReader.cs` and the type readers used (`Int`, `UInt`, `GUID`, `Type`, `KeyValueTableTypeRedirectorRegistry`).
- [ ] **Step 4:** Run `cargo test -p pinewatch-kvt`. Expected: PASS. `install.rs` (skipped without an install) reads all 9 `Maps/*/Level.hierarchy` files without an exception.
- [ ] **Step 5:** Update `docs/VERIFICATION.md`, then commit.

### Task 5: `pinewatch-level` landscape

**Files:**
- Create: `crates/pinewatch-level/src/{lib.rs,hierarchy.rs,landscape.rs}`
- Test: `crates/pinewatch-level/tests/{tiles.rs,install.rs}`

**Interfaces:**
- Consumes: `KvtCursor`, `InstallRoot`, `LogSink`.
- Produces:
  - `pub struct HierarchyItem { pub index: i32, pub type_name: Option<NetString>, pub instance_id: Option<u32> }` and `pub fn read_hierarchy_items(table: &KvtTable) -> Result<Vec<HierarchyItem>, MonoException>`
  - `pub struct LandscapeCoord { pub x: i32, pub y: i32 }`
  - `pub struct LandscapeTile { pub coord: LandscapeCoord, pub materials: Vec<Guid>, pub heightmap: Box<[[f32; 257]; 257]>, pub splatmap: Box<[[[f32; 8]; 256]; 256]>, pub holes: Option<Box<[[bool; 256]; 256]>>, pub hashes: TileHashes }`
  - `pub fn read_landscape(install: &InstallRoot, map_dir: &str, log: &mut dyn LogSink) -> Result<Option<Vec<LandscapeTile>>, MonoException>`. It returns `None` when the hierarchy has no `SDG.Framework.Landscapes.Landscape` item. Tile file names: `Tile_{x}_{y}_Source.heightmap` and `.splatmap`, `Holes/Tile_{x}_{y}.bin`.

- [ ] **Step 1: Write failing tests** (`tiles.rs`, synthetic bytes):
  - A full heightmap decodes big-endian (`0x8000` → `0x8000 as f32 / 65535.0`).
  - A heightmap 1 byte short makes its last sample 1.0 (65535).
  - An empty splatmap yields 1.0 in every layer.
  - A holes file with byte `0b0000_0001` makes only offset 0 solid.
  - A missing heightmap logs exactly `LandscapeTile missing heightmap file: <path>` at warn level and leaves heights at 0.
  - Hashes cover exactly the bytes consumed.
- [ ] **Step 2:** Run `cargo test -p pinewatch-level`. Expected: FAIL.
- [ ] **Step 3: Implement** as ports of `LandscapeTile.read`, `readHeightmap`, `readSplatmap`, `ReadHoles`, `LandscapeCoord.read`, `AssetReference.read` and the item loop of `LevelHierarchy.read`.
- [ ] **Step 4:** Run `cargo test -p pinewatch-level`. Expected: PASS. `install.rs` (skipped without an install) loads every official map's landscape: tile counts sum to 160, and every heightmap and splatmap is read in full.
- [ ] **Step 5:** Update `docs/VERIFICATION.md`, then commit.

### Task 6: `pinewatch` app

**Files:**
- Create: `crates/pinewatch/{Cargo.toml,build.rs,src/main.rs,src/cli.rs,src/terrain.rs,src/camera.rs,src/space.rs}`
- Test: `crates/pinewatch/tests/boot.rs`

**Interfaces:**
- Consumes: `find_install`, `read_landscape`, `LandscapeTile`.
- Produces:
  - `pub fn unity_to_bevy(p: [f32; 3]) -> bevy::math::Vec3` (negates Z) in `space.rs`
  - `pub fn build_tile_mesh(tile: &LandscapeTile) -> bevy::render::mesh::Mesh` in `terrain.rs`, with vertex `(i, j)` at Unity position `(x*1024 + j*4, -1024 + h*2048, y*1024 + i*4)`
  - CLI: `pinewatch [--unturned <dir>] [--map <name>] [--screenshot <file.png>]`. The default map is `PEI`. `--screenshot` saves one frame after the terrain is spawned, then exits.

- [ ] **Step 1: Write a failing test** (`boot.rs`): `build_tile_mesh` on a flat 0.5 tile puts every vertex at Bevy y = 0 and the corner `(256, 256)` at Bevy `(x*1024+1024, 0, -(y*1024+1024))`.
- [ ] **Step 2:** Run `cargo test -p pinewatch`. Expected: FAIL.
- [ ] **Step 3: Implement.**
  - The window title is `Pinewatch`.
  - `build.rs` sets the Windows ProductName and FileDescription to `Pinewatch`.
  - Placeholder layer colors are a fixed 8-color palette, labeled temporary in code.
  - Normals come from height differences.
  - The fly camera uses WASD/QE, right mouse button to look, and Shift to go faster.
  - A directional light and ambient light.
- [ ] **Step 4:** Run `cargo test -p pinewatch`. Expected: PASS.
- [ ] **Step 5: Commit.**

### Task 7: Milestone check

- [ ] **Step 1:** Run `cargo test --workspace`. Expected: all pass. Install tests run, not skipped.
- [ ] **Step 2:** Snapshot the install: path, size and modified time of every file, plus SHA-1 of every file Pinewatch read.
- [ ] **Step 3:** Run `cargo run --release -p pinewatch -- --map PEI --screenshot target/m1/pei.png`, and the same for `Washington`. Expected: PNGs that show recognizable terrain.
- [ ] **Step 4:** Repeat the snapshot. Expected: identical.
- [ ] **Step 5:** Commit, with `docs/VERIFICATION.md` up to date.
