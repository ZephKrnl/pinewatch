# Pinewatch step 1: file-format readers

**Status:** draft for review · 2026-11-01
**Target:** U3-SDK tag `v3.26.3.12` (commit `aba323a`) running on Unity 2022.3.62f3
**Related:** `COMPATIBILITY.md` (rules and limitation register), `docs/PORT_SURVEY.md` (roadmap and decisions),
`docs/FORMAT_INVENTORY.md` (format research)

## 1. Purpose

Pinewatch is a complete Rust rewrite of Unturned, built from the U3 SDK source. Step 1 builds the layer that every
later system stands on: Rust readers for every file format the game reads, behaving identically to the original
wherever behavior is observable.

## 2. Requirements

| ID | Requirement |
|---|---|
| R1 | All production code is Rust. No production crate depends on Unity, Mono or any .NET runtime. |
| R2 | 1:1 observable behavior with the target, as defined in `COMPATIBILITY.md`. Every deviation has a register entry. |
| R3 | Text and binary parsing live in separate crates that can be built and tested on their own. |
| R4 | Automated tests. Differential tests against the original code are the proof of equivalence. |
| R5 | Messages, their order, exceptions and asset-skipping behavior are reproduced exactly. |
| R6 | The original game installation is never modified. |
| R7 | Readers only. Each writer comes with the step that first needs it, under the same rules. |

## 3. Scope

### In step 1

| Area | Included |
|---|---|
| Mono runtime behavior | Number parsing for every `NumberStyles` combination the in-scope code uses; culture data and current-culture selection; composite formatting and integer formatting for in-scope messages; `Guid`, `Enum`, `bool` and `DateTime` parsing; character tables; `Dictionary` iteration order; text decoding and BOM detection; exception types and messages; I/O exceptions and their messages |
| Text formats | DAT tokenizer and parser (with and without metadata); the DAT value helpers (numbers, Unity types, colors, GUIDs, asset references, master bundle references, type names); the legacy line-based reader; the KeyValueTable reader with its type readers and type redirects; JSON deserialization with Newtonsoft semantics for the types step 1 reads (`LevelInfoConfigData` and the types it contains) |
| Binary readers | Block, River (including its shared buffer), .NET `BinaryReader`, SHA-1 hashing streams |
| Map layouts | `Level.dat`; `Config.json` (`LevelInfoConfigData`); `Level/{Objects,Buildables,Visibility}.dat`; legacy `Objects/Objects_x_y.dat`; `Spawns/*.dat` and their legacy `_x_y` variants; `Environment/{Roads,Paths,Nodes,Bounds,Flags,Flags_Data,Lighting,Navigation_N}.dat`; legacy ground (`Terrain/Heights.dat`, `Terrain/Trees.dat`, `Terrain/Heightmap.png`, `Terrain/AlphamapHQ_N.png`); landscape heightmaps, splatmaps and holes; `Foliage.blob` and legacy `Foliage/*.foliage`; `Level.hierarchy` at item level, plus its landscape and foliage items |
| Assets | Discovery (search order, file precedence, `MasterBundle.dat` detection); `.dat` and `.asset` definition reading; translation lookup with English fallback; master bundle configs; per-file hashes; result items, including exceptions |
| File access | A read-only install handle. Directory enumeration in the same order the original sees |
| Tooling | `pinewatch-scan`: scans an install, dumps any in-scope file, reports every message |

### Later steps

- **Writers:** the DAT writer (plain and comment-preserving), the legacy writer, the KeyValueTable writer, binary writers, Newtonsoft JSON output.
- **Layouts owned by gameplay systems:** player and server saves, client settings, `Editor/*.dat`, devkit objects other than landscape and foliage,
  and the other JSON files (server `Config.json`, `Preferences.json`, `Status.json`, `Graphics.json` and so on).
- **Level hash:** combining per-file hashes into the level hash belongs to level loading. Step 1 produces the per-file hashes.
- **Other:** Unity bundles (step 2), network packing (step 4), BbCode, and localization formatting beyond what in-scope messages need.

## 4. Architecture

### Crates

| Crate | Responsibility | Depends on |
|---|---|---|
| `pinewatch-mono` | The Mono class-library behavior listed in section 3 | none |
| `pinewatch-unity` | Unity value types exactly as stored (`Vector2/3/4`, `Vector3Int`, `Quaternion`, `Color`, `Color32`, `Matrix4x4`) and the pure managed helpers in-scope code calls | none |
| `pinewatch-io` | Read-only install handle, directory enumeration, hashing streams, log capture, diagnostic details | mono |
| `pinewatch-dat` | DAT tokenizer, parser, tree and value helpers; the legacy line-based reader | mono, unity, io |
| `pinewatch-kvt` | KeyValueTable reader, type readers, type redirects | mono, unity, io |
| `pinewatch-json` | JSON deserialization with Newtonsoft semantics | mono, io |
| `pinewatch-binary` | Block, River and `BinaryReader` readers | mono, unity, io |
| `pinewatch-level` | Map file layouts, terrain tiles, foliage, navmesh, hierarchy items, level info | binary, kvt, json, dat, unity, mono, io |
| `pinewatch-assets` | Asset discovery, definitions, translations, master bundle configs | dat, io, mono |
| `pinewatch-scan` | Command-line tool | all of the above |
| `pinewatch-conformance` | **Dev only, never shipped.** Canonical output format, comparison with reference outputs, generated inputs, fuzz targets | all of the above |
| `xtask` | **Dev only.** Repository tasks (test-data copy, harness setup and runs, conformance, install check) | as needed |

Dependency rules:
- Dependencies point one way: `mono`/`unity` → `io` → format crates → `level`/`assets` → tools.
- No production crate depends on Bevy, Unity, a .NET runtime, `pinewatch-conformance` or `xtask`.
- External production dependencies are an allowlist, checked by a test:
  - `sha1`
  - `png` (legacy terrain images, see L8)
  - `windows-sys` (Windows locale and file-system calls)
  - their transitive dependencies

  Adding a crate needs a stated reason. Dev-only crates aren't restricted.

### Parity rules

These rules apply to every line of production code in step 1.

| ID | Rule |
|---|---|
| P1 | **Same algorithm, same order.** Each original function becomes a Rust function that performs the same reads, comparisons and state changes in the same order. Rust idioms are allowed only where results stay identical. |
| P2 | **State kept identical.** C# statics and reused objects become explicit Rust context values with the same sharing and lifetime. For example: River's process-wide read buffer; the KeyValueTable reader's static `StringBuilder`; the `DatParser` each asset worker reuses (which carries leftover root comments into the next file when metadata is on); `FoliageStorageV2`'s GUID buffer. |
| P3 | **Exceptions are values.** Any C# operation that can throw returns `Result<T, MonoException>`, carrying the exception type and Mono's exact message. It propagates to the Rust counterpart of the same C# catch site, so the same assets are skipped with the same messages. Rust code never panics on any input. A panic is a bug. |
| P4 | **Messages are exact.** Every message the original produces is byte-identical, raised at the same point, in the same order. That covers parser error lists, log calls and exception messages. Extra details (column, byte offset, path inside the file) sit beside a message and never add, remove, change or reorder messages. |
| P5 | **Culture is a parameter.** Code that uses the current culture, explicitly or implicitly, takes a `Culture` argument. It's created once at startup the way Mono picks the current culture from the OS. Invariant-culture call sites use the invariant culture. |
| P6 | **Raw values.** Readers return values exactly as the original produces them. Where the original converts through engine-native code (for example `Quaternion.Euler`), the reader keeps the stored inputs, which determine the result exactly, and the conversion is a separate function tracked by L6. |
| P7 | **C# integer semantics.** Arithmetic, casts and overflow behave as in the compiled C#: wrapping where C# wraps, exceptions where C# checks. |
| P8 | **No silent substitution.** If a behavior can't be reproduced, work on that case stops, `COMPATIBILITY.md` gets an entry, and you're told before it's merged. |

### Key types (sketch)

- **DAT tree:** `DatNode` is one of `Value(DatValue)`, `Dictionary(DatDictionary)` or `List(DatList)`.
  - `DatValue` holds an optional string, because the original's values can be null.
  - `DatDictionary` is a Mono-ordered dictionary with case-insensitive keys (`OrdinalIgnoreCase`) that keeps the first spelling.
  - Metadata (comments, line numbers) is stored alongside when enabled.
- **DAT parser:** `DatParser` is reusable and holds the same persistent fields as the original. `parse` returns the root dictionary, and `error_messages` returns the exact list.
- **Binary readers:**
  - `BlockReader` has the original's `block` and `step` semantics.
  - `RiverReader` takes a `&mut RiverSharedBuffer` from the caller's context.
  - `DotNetBinaryReader` wraps any seekable reader.
- **Install access:** `InstallRoot` can only open, read, check existence and enumerate. It has no way to write, create, rename or delete anything.
- **Exceptions:** `MonoException` carries the exception type and its exact message. I/O failures map to the same exception types and messages Mono produces for the same OS errors.
- **Messages:** each is a `Message` with its exact `text` plus optional details. Logs go to a `LogSink` that records the level and the exact text.
- **Strict mode:** tools and tests can turn any message into a failure. The game never uses it.

## 5. Reference harness and conformance

### Setup

- `reference/u3-sdk`: a git submodule of `SmartlyDressedGames/U3-SDK`, pinned to tag `v3.26.3.12`. Never edited.
- `reference/harness`: the test-only C# harness, as a Unity package.
- `cargo xtask reference setup`:
  - makes a working copy of the pinned SDK in `reference/work/` (ignored by git)
  - adds the harness package
  - replaces the SDK's `Newtonsoft.Json.dll` with the game's own build
- `cargo xtask reference run`: runs Unity 2022.3.62f3 in batch mode on that working copy.
- `cargo xtask testdata`: copies the in-scope files from the install into `testdata/` (ignored by git). It opens the install read-only and writes a manifest recording the game version and Steam build (currently 3.26.3.13 and 25613056) and the SHA-1 of every copied file.

### What the harness does

- Runs the original code, unmodified, against `testdata/` and against generated inputs.
- Writes canonical output:
  - JSON Lines, UTF-8, LF line endings, fixed key order
  - floats as their exact IEEE-754 bit patterns, plus a decimal form for reading
  - strings exactly as produced
- Records, per input:
  - the resulting tree or values
  - every message, in order
  - every exception (type and message)
  - bytes consumed and final positions
  - hashes
  - logs
- **Map layouts:** runs the original loaders on copied maps and records every value read through Block, River and `BinaryReader`, plus each loader's results.
  - Reads are recorded by observing calls without changing behavior, for example with runtime method hooks (Harmony).
  - If that isn't reliable for some reader (for instance, calls the Mono JIT inlines), the harness compares the loader's resulting data and the bytes it consumed instead, and you're told before that fallback is used.
- **Runtime tables:** dumps character classes and case mappings for all 65,536 UTF-16 code units, and the number-format data for every culture Mono knows.
  - Rust's tables are generated from those dumps and compared exhaustively.
  - The dumps derive from Mono's MIT-licensed data and are committed.

### Test layers

| Layer | Command | Needs Unity or game files |
|---|---|---|
| Ported NUnit cases from `Assets/Tests/UnturnedDat` (~1,040) | `cargo test` | No |
| Synthetic fixtures: generated inputs with committed reference output | `cargo test` | No |
| Runtime tables, compared exhaustively | `cargo test` | No |
| Game-file conformance: every in-scope file in `testdata/`, DAT in both metadata modes | `cargo xtask conformance` | Yes |
| Differential fuzzing: at least 1,000,000 generated inputs per reader family, batched through the harness | `cargo xtask fuzz` | Yes |
| Install check: SHA-1 of every install file before and after a full scan | `cargo xtask check-install` | Game install |

The reader families are DAT, legacy, KeyValueTable, JSON, Block, River and `BinaryReader`.

**Never committed:** game files, or anything derived from them (trees, values, messages quoting their content).
That content belongs to Smartly Dressed Games, so it stays out of the history even though the repository is local and private. Committed fixtures come only from synthetic inputs.

**Mismatch policy:** any mismatch fails the run. It gets fixed. Only if exact reproduction is impossible does it become a `COMPATIBILITY.md` entry instead, and you're told first (P8).

## 6. Behaviors the tests must pin down

Verified in the 3.26.3.12 source. The full list is in `docs/FORMAT_INVENTORY.md`.

- **DAT:**
  - The whole file is tokenized first, then parsed.
  - When `{` or `[` follows a value on the next line, the value is dropped. Only one line break is skipped there.
  - A duplicate key keeps the last value and adds an error.
  - Keys are case-insensitive.
  - An unquoted value runs to the end of its line.
  - Comment, comma and escape rules as listed in the inventory.
  - Line numbers in messages drift after multi-line quoted strings.
  - The reused parser carries leftover root comments into the next file when metadata is on.
  - The tokenizer has its own `0xEF` BOM check on top of `StreamReader`'s BOM handling.
- **Numbers:**
  - `NumberStyles.Any` with the invariant culture at most call sites.
  - `NumberStyles.Integer` with the current culture for legacy asset IDs.
  - `HexNumber` for colors.
  - The vector helpers' substring arithmetic, including the `IndexOf` exception on a lone `(`.
- **Legacy line reader:** keys split at the first space, keys are case-sensitive, and `//` comments count only at the start of a line.
- **KeyValueTable:**
  - A duplicate key keeps the first value.
  - Repeated object keys merge and repeated list keys append.
  - Nested lists aren't supported.
  - All nested readers share one static builder.
  - It has its own boolean rules.
- **Block:**
  - Out-of-range reads return 0 without advancing.
  - GUIDs have a 1-byte length prefix.
  - Boolean arrays aren't bounds-checked and throw `IndexOutOfRangeException`.
- **River:**
  - Byte arrays and GUIDs have a 2-byte length prefix.
  - At end of file, a single byte reads as 0, multi-byte reads leave stale bytes from the shared buffer, and string reads decode those stale bytes.
- **Landscape tiles:** heightmaps are big-endian. A missing byte reads as −1, so a short heightmap yields 65535 (height 1.0) and a short splatmap or hole mask yields 255.
- **Foliage:** only the header is hashed, and GUID reads can leave stale bytes in the instance's buffer.
- **Assets:**
  - Definition precedence: `<Folder>.asset`, then `<Folder>.dat`, then `Asset.dat`, then every `*.asset`.
  - Folders are searched breadth-first.
  - Translations use `<Language>.dat` with English fallback.
  - One parser per worker is reused for definitions, translations and `MasterBundle.dat`.

## 7. Repository layout

```
C:\Pinewatch
├─ Cargo.toml                  workspace
├─ rust-toolchain.toml         pinned stable toolchain (currently 1.99.1), edition 2024
├─ README.md                   attribution and disclaimers (section 8)
├─ LICENSE.txt                 U3 SDK License Agreement, verbatim
├─ THIRDPARTYNOTICES.txt       the SDK's notices, an unmodified copy
├─ NOTICES-PORTED-CODE.md      MIT notices for code ported from the .NET class libraries Unity's Mono compiles
├─ COMPATIBILITY.md
├─ docs/                       PORT_SURVEY.md, FORMAT_INVENTORY.md, superpowers/specs/
├─ crates/                     pinewatch-* crates
├─ xtask/
├─ reference/
│  ├─ u3-sdk/                  submodule, pinned to v3.26.3.12
│  ├─ harness/                 C# Unity package (tests only)
│  └─ work/                    ignored: working copy and Unity cache
└─ testdata/                   ignored: copied game files and reference outputs
```

> **Amended 2026-11-01 (M1 review).** The MIT notices moved from `THIRDPARTYNOTICES.txt` to `NOTICES-PORTED-CODE.md`,
> so `THIRDPARTYNOTICES.txt` stays an exact copy of the SDK's file (as the M1 plan checks). Notices for external
> crates will be generated into a notice file before any build leaves this machine; nothing is distributed today.

Pinewatch is a Mod under the U3 SDK License Agreement, so the repository follows that agreement (section 1). The repository sets `core.longpaths true`.

A `.gitattributes` file turns off line-ending conversion for test fixtures and generated tables, so their bytes are
stored exactly. Git for Windows converts line endings by default, which would change byte-exact test inputs.

Git is local only:
- The repository has no remotes, and nothing is ever pushed.
- All git settings are local to `C:\Pinewatch`, with commits authored as Zeph.
- The `reference/u3-sdk` submodule only **fetches** from `SmartlyDressedGames/U3-SDK` to get the pinned tag. It is never pushed to.

## 8. README attribution (draft wording for your review, not legal advice)

> **Pinewatch** is a free, non-commercial project based on the U3 SDK by Smartly Dressed Games Ltd. It is a Rust
> implementation of a modified game based on the U3 SDK (a "Mod" as defined in the U3 SDK License Agreement).
>
> **Trademark notice.** UNTURNED is a registered trademark of Smartly Dressed Games Ltd. References to UNTURNED in
> this project do not constitute use as a trademark pursuant to the Trademarks Act, RSC 1985, c T-13, or applicable
> trademark legislation in other jurisdictions; such references are solely for non-commercial, identification,
> informational and descriptive purposes. Pinewatch is based on the U3 SDK. Pinewatch and its contributors are not
> affiliated with or endorsed by Smartly Dressed Games Ltd., and do not make, use or sell any goods or services
> provided by Smartly Dressed Games Ltd.
>
> **License.** Copyright © 2026 Smartly Dressed Games Ltd. All rights reserved, unless expressly granted herein.
> Pinewatch is provided free of charge and strictly on a non-commercial basis under the U3 SDK License Agreement
> (`LICENSE.txt`), which also governs any use or further distribution by third parties. Third-party notices are in
> `THIRDPARTYNOTICES.txt`. Pinewatch is for personal computers running Windows, macOS or Linux only. The license
> prohibits use on gaming consoles, mobile or handheld devices, and through emulation, streaming, cloud or
> subscription services.
>
> **Game content.** Pinewatch ships no Unturned game content: no models, textures, sounds, maps or data files. It reads
> that content from your own installation of Unturned and never modifies it.

## 9. Done criteria

Step 1 is done when all of these hold:

1. Every in-scope reader exists in Rust, and the dependency allowlist test passes.
2. Canonical output is identical for:
   - every in-scope file in `testdata/`
   - every ported NUnit case
   - every synthetic fixture
   - the full fuzzing campaign, with zero unexplained mismatches
3. The runtime tables match exhaustively.
4. Every remaining difference has a `COMPATIBILITY.md` entry.
5. The install check passes after a full scan.
6. `pinewatch-scan` scans the install and reports every message.
7. `README.md`, `LICENSE.txt` and `THIRDPARTYNOTICES.txt` are in place.

## 10. Order of work

Step 1 is too large for one implementation plan. It will be planned as a series of smaller plans, starting with
stages 1 to 3, and you approve each plan before its work starts.

1. Repository setup: workspace, `xtask`, licenses, README, submodule, ignore rules.
2. Harness skeleton, canonical format, test-data copy, runtime-table dumps.
3. `pinewatch-mono` and `pinewatch-unity`.
4. `pinewatch-io`.
5. `pinewatch-dat`, including the legacy reader, then conformance on every `.dat` and `.asset`.
6. `pinewatch-kvt` and `pinewatch-json`.
7. `pinewatch-binary`.
8. `pinewatch-level`.
9. `pinewatch-assets`.
10. `pinewatch-scan`, the fuzzing campaign, and the done-criteria check.

## 11. Risks and open items

- **Unity setup needs you:**
  - install Unity Hub and Unity 2022.3.62f3
  - sign in with a free Unity account and activate a Personal license (I can't sign in for you)

  Batch mode won't run without an activated license.
- **Recording reads from the original loaders** may miss calls the Mono JIT inlines. The fallback is described in section 5.
- **Harness speed:** batch-mode runs are slow to start, so fuzzing runs in large batches.
- **Measured before porting:**
  - L6 (engine math), L7 (stack depth), L8 (PNG decoding)
  - Mono details: Unicode table version, float-parsing algorithm, UTF-8 replacement behavior, current-culture selection
- **Platforms:** Windows 11 x64 is the reference platform for step 1. Linux and macOS conformance runs come later. Until then, platform-specific behavior there is unverified.
- **Version drift (L4):** new SDK tags are adopted deliberately, after a diff of the in-scope files.
