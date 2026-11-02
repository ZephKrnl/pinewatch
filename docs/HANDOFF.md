# Handoff

The state of Pinewatch on 2026-11-02, updated during the third cloud session, written so work can resume from this
file alone.

## Where things stand

- **Current branch:** `main-9m65p0`, the branch the cloud sessions were given, pushed to `origin`. It holds `main`
  plus everything listed under "Done in the cloud sessions". **`main` was not fast-forwarded**: that work still needs
  the Windows run (see "Exact next steps").
- **Done:** M1 (the bootable terrain viewer) and M2a (the `.dat` reader) are merged into `main`. On the Windows PC,
  with the game installed, `cargo test --workspace` passes all 396 tests on `main` and skips none.
- **Linux (cloud):** on `main-9m65p0`, `cargo test --workspace` passes 534 tests (the install tests print
  `skipped: no Unturned install found`; the Windows-only test doesn't build), `cargo clippy --workspace --all-targets`
  gives no warnings, and `cargo xtask check-deps` passes for the 7 format crates. Setup a fresh Linux machine needs:
  `apt-get install libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev` (pkg-config was present),
  `rustup component add clippy`, and `cargo fetch` (`check-deps` runs `cargo tree --target all --offline`).
- **Remote:** `origin` is the private GitHub repository `ZephKrnl/pinewatch`. It stays private.
- **Branches** (all pushed to `origin`):

  | Branch | What it holds |
  |---|---|
  | `main` | M1, M2a, the M2a review fixes and the first handoff note (`480c1ca`) |
  | `main-9m65p0` | `main` plus the cloud sessions' work below |
  | `milestone/m2a-dat-reader` | The M2a work, merged into `main` by fast-forward (`b230803`) |
  | `milestone/m1-bootable` | The M1 work, merged into `main` (`ad11969`) |
  | `docs/step1-spec` | The step 1 design spec, merged into `main` (`a8c75f6`) |

- **Tags:** none.
- **Submodule:** `reference/u3-sdk` is the U3 SDK source at `aba323a` (tag `v3.26.3.12`), the code Pinewatch ports.
  After cloning, run `git submodule update --init`. On Windows, turn on long paths first, because some SDK file
  paths pass Windows' 260-character limit and the checkout otherwise fails with "Filename too long":

  ```
  git config core.longpaths true
  git -c core.longpaths=true submodule update --init
  ```

- **Working from the Windows PC:** `C:\Pinewatch` tracks `origin` (`main` is set up to track `origin/main`). Use
  `git pull` and `git push` as usual; Git Credential Manager signs in as ZephKrnl. A cloud session can't reach the
  PC (only Mac or Linux computers can be attached), so PC work runs in a Claude Code session on the PC itself.

## Rules that carry over

- **1:1 behavior.** Wherever the original C# behavior is observable, Pinewatch matches it exactly: parsing results,
  number conversion, error messages and their order, exceptions and asset skipping, quirks, binary layout. Where an
  exact match is impossible, the case gets an entry in `COMPATIBILITY.md` (or `docs/VERIFICATION.md` while it
  awaits the harness) saying why; nothing is substituted silently.
- **Rust only** for production code. The C# in `reference/u3-sdk` is a reference and is never edited.
- **The Unturned installation is never modified.** Tests and tools only read it.
- **Commits** are authored `Zeph <205350396+ZephKrnl@users.noreply.github.com>` (set in the repository's local Git
  config). No `Co-Authored-By` lines and no AI attribution, including no `Claude-Session` trailer. The existing
  history stays exactly as it is.
- **The repository stays private.** Never push credentials, game files or other third-party assets.
- **Development resumes only on the owner's explicit command.** On 2026-11-02 the owner ordered autonomous work
  through the roadmap ("do not ask me what to do next").
- `CLAUDE.md` holds these rules for every Claude session, and `.claude/settings.json` turns off the co-author line.

## Done in the cloud sessions (on `main-9m65p0`)

### M2a review findings (seven of eight)

| # | Finding | What was done |
|---|---|---|
| 1 | The install walk missed map files | `pinewatch-dat/tests/install.rs` also walks each map's root `.dat` files and `Bundles/`, and checks the walk on a made-up tree. |
| 3 | V18 | Settled: an unstable sort can't change the result. Test with equal-valued names below and above 16 members. |
| 4 | No independent float bits | 27 rows (14 corert quirks) from a separate big-integer model of upstream `Number.CoreRT.cs`; the model agreed with the port on 20,000 decimals. V16. |
| 5 | Tokenizer comment | Fixed; the case runs with LF and CRLF. |
| 6 | README status, V19 | README says the reader isn't harness-confirmed yet. `TypeResolver::resolve` returns a `Result`; `ExceptionKind::FileLoad`. |
| 7 | Message texts against the DLLs | `pinewatch-dat/tests/shipped_messages.rs`; its assembly half runs only with the game. |
| 8 | Six clippy warnings | Fixed. |

Finding 2 (V17) is now settled at source level (below); the IL check remains for the PC.

### Mono's invariant casing tables (V3, V17)

`char.ToUpperInvariant`/`ToLowerInvariant` used Rust's Unicode tables, which differ from Mono's invariant culture
for 252 code units (upper) and 207 (lower): `.dat` keys like `Size`/`ſize` merged and `Boolean.TryParse("falſe")`
succeeded. `pinewatch-mono` now uses Mono's `TextInfoToUpperData`/`TextInfoToLowerData` (`case_tables.rs`,
generated from Unity mono `b8eb56f`), with checksums of all 65,536 mappings pinned in `tests/chars.rs`.
`IsWhiteSpace` above Latin-1 is corefx's 17 separator code units, listed explicitly (it already matched). V17 lists
the source chain of `OrdinalIgnoreCase` equality and hashing, both folding through `TextInfo.ToUpper`.

### M2c: assets (plan `docs/superpowers/plans/2026-11-02-m2c-assets.md`)

The plan was written and started without the owner's review, under the autonomous order; review it with the code
on the branch. M2c comes before M2b because M2b needs the PC.

| Task | State | What exists |
|---|---|---|
| 1 Mono's path strings | Done | `pinewatch_io::mono_path` (Mono's own `Path.cs`: `Combine`, `Join`, `GetFileName`, `GetDirectoryName`, `GetFullPath`, …, both platform styles); `InstallRoot::full_name` (`UnturnedPaths.RootDirectory.FullName`); the `pinewatch-assets` crate |
| 2 Origins and search locations | Done | `Origins`, `client_search_locations` (core `Bundles`, `Sandbox` folders, map `Bundles`), `scan_known_levels` |
| 3 Folder walk | Done | `search_location`: breadth first, `MasterBundle.dat`, `FindAssets` precedence, `*.asset` matching, exact messages |
| 4 Definitions and translations | Done | `read_location`/`read_items`: one parser per location, SHA-1, parser errors, `<Language>.dat` with English fallback |
| 5 Master bundles | Done | `MasterBundleConfig` with its exceptions and version rules; bundle file hashed as a stream |
| 6 Asset headers | Done | `AssetLoader`: master bundles, core bundle wait, `TryLoadFile`, `LoadFile` up to creating the asset (`AssetHeader`) |
| 7 Asset types | Done | 95 asset classes, the 70 legacy keys, `GetTypeFriendlyName`, `AssetTypeResolver` for `Metadata.Type` |
| 8 Localization | Done | `Local`: `format`, `format_or_null`, `format_or_empty`, the English fallback, `has` |
| 9 Asset index | Done | `AssetMapping` (legacy tables, GUID dictionary, asset list, modification counter), `AddToMapping` with its messages, the 10 `FriendlyName` overrides, `ItemAsset.isPro`, `RedirectorAsset` |
| 10 Asset references | Done | `AssetReference`, `ContentReference`, `MasterBundleReference`, `AudioReference` and their readers; `CachingAssetRef`, `CachingBcAssetRef` (cache, `IsReferenceTo`, `ToString`, parsers); `AssetLoader::find_legacy`; the rest of `UnturnedDatEx` except its writers |
| 11 `LandscapeMaterialAsset`, holiday redirects | Done, inputs open | `LandscapeMaterialAsset` (`AssetHeader::landscape_material`), `EPhysicsMaterial`, `ERayMask`, `ENPCHoliday`, `holiday_redirect`; `AssetLoader::find` (redirectors) and `find_landscape_material`; `ClientAssetIntegrity`; `load_client_assets`; tiles read with `TileAssets` (holiday swap, known-missing check, `updatePrototypes`' requests). Open: the holiday inputs (V14) |
| 12 The whole install | Done, not yet run | `pinewatch-assets/tests/install.rs` loads and indexes every location; asserts the core bundle, 5,000+ assets, item ID 1 with a friendly name, landscape materials and `Fallback_Green`. It skips without the install, so it has never run |

Shared pieces added on the way: `MonoDictionary` (`Dictionary<K, V>`'s slot order, which the asset list depends
on), `Guid`'s `D` format, and the `net_enum!` macro, which declares a ported C# enum from one member list. A
class's `PopulateAsset` that throws now takes `LoadFile`'s "Caught exception while populating" path.

Since task 11, `pinewatch-level` depends on `pinewatch-assets`: `read_landscape` takes a `TileAssets` (the loaded
assets, the holiday switch and the integrity state). The app loads every client asset before the map, as the game
does, so it starts slower and prints the asset system's messages; the level install test does the same, and its
"no warnings" check now also means every official map's tile materials are among the loaded assets.

New register and verification entries: L3 (d) no `Sandbox` folder is created, L3 (e) a missing GUID is never
written, L5 made concrete for the asset worker, L11 no workshop yet; V21 the `WITH_NOREDIST` build assumption, V22
Mono's `Path` rare forms, V23 level discovery's `Config.json` messages, V24 folder enumeration, V25 master bundle
loading, V26 culture-sensitive comparisons, V27 loading order and constructor failures, V28 the classes'
own `PopulateAsset`; V14 is narrowed to the tile side and the holiday inputs.

New resource strings, checked by `shipped_mscorlib` on the PC: `IO_PATH_NOT_FOUND_PATH`,
`ARGUMENT_OUT_OF_RANGE_START_INDEX_LARGER_THAN_LENGTH`, `MISSING_METHOD_NAME`, `ARGUMENT_ADDING_DUPLICATE_WITH_KEY`,
`ARG_KEY_NOT_FOUND_WITH_KEY`.

### M2f: level info (plan `docs/superpowers/plans/2026-11-02-m2f-level-info.md`)

Written and started in the cloud under the autonomous order, **not reviewed by the owner** (the spec asks for each
plan's approval; M2c set the precedent). It reads `Level.dat` and `Config.json` as `Level.ReadLevelInfo` does, for
V23's messages and the level's `Allow_Holiday_Redirects` (V14). It adds the step 1 spec's `pinewatch-binary` and
`pinewatch-json` (Json.NET 13.0.3, the version the SDK's `Newtonsoft.Json.dll` reports).

| Task | State | What exists |
|---|---|---|
| 1 `Block` | Done | `pinewatch-binary::Block` (every reader but `readSingleQuaternion` and `read(Type…)`); `pinewatch_mono::utf8_get_string` (`Encoding.UTF8.GetString`, flushed) |
| 2 `Level.dat`, packed version | Done | `read_level_file` (`LevelFile`, `ELevelSize`, `ELevelType`), `try_parse_packed_version` (`Parser.TryGetUInt32FromIP`, out value included) |
| 3 to 6 `pinewatch-json` | Not started | Reader, typed reads, `object` values, typed deserialization |
| 7 `LevelInfoConfigData`, `ReadLevelInfo` | Not started | |

New entry: V30 (`new byte[negative]`'s message).

### Linux test fixes

The missing-heightmap test expects the `/` form on Linux, and the screenshot test checks an upper-case install path
only where file systems ignore case. Neither changes production behavior.

## Unfinished work

### 1. On the Windows PC (needs the game, Unity or the shipped DLLs)

- **The Windows run** of everything on `main-9m65p0`, then fast-forwarding `main` (see "Exact next steps").
- **Finding 2 / V17:** confirm in the IL of `Unturned_Data\Managed\mscorlib.dll` the chain V17 lists
  (`OrdinalIgnoreCaseComparer`, `String.Equals(…, OrdinalIgnoreCase)`, `CompareInfo.CompareOrdinalIgnoreCase`,
  `CompareStringOrdinalIgnoreCase`, `GetIgnoreCaseHash`, `MemoryExtensions.ToUpperInvariant`,
  `TextInfo.ChangeCase`, `TextInfo.ToUpper(char)`).
- **V3:** confirm the shipped `TextInfoToUpperData`, `TextInfoToLowerData` and `CharUnicodeInfoData` equal the
  source (their field data in `mscorlib.dll`, or a harness dump of all 65,536 code units).
- **V21:** confirm `WITH_NOREDIST` in the shipped `Assembly-CSharp.dll` (V21 names the methods to look at).
- **M2b, the reference harness** (not started; no plan yet). Scope in the M2a plan's table: a batch-mode harness
  (`cargo xtask reference setup/run`), canonical output, `cargo xtask testdata`, runtime-table dumps, conformance
  for `pinewatch-mono`, `pinewatch-dat` and now `pinewatch-assets`. Found on the PC on 2026-11-02: Unity Editor
  2022.3.62f3 and 6000.6.7f1 under `C:\Program Files\Unity\Hub\Editor` and Unity Hub (license activation for batch
  mode not checked); `csc.exe`, `ilasm.exe` and the `dotnet` CLI; the game's own Mono runtime
  (`MonoBleedingEdge\EmbedRuntime\mono-2.0-bdwgc.dll` with `Unturned_Data\Managed\*.dll`), a candidate for the
  class-library checks, read-only.

### 2. In the cloud (no PC needed)

- **M2c is written in full.** What remains of it is the PC run of its install tests (section 1).
- **M2f tasks 2 to 7** (level info; the plan lists each).
- Cloud disk is a fixed allowance: if linking the app fails with `signal 7 [Bus error]`, the disk is full of old
  app binaries (about 1.8 GB each); `cargo clean -p pinewatch` frees them.
- Later plans: M2d (asset processor) and M2e (terrain materials) need Unity on the PC.

### 3. Phone and cloud access

Section 2 of the U3 SDK License (`LICENSE.txt`) bars using or developing the SDK or any Mod "for or in connection
with ... mobile or handheld devices". The owner was told and chose to work from his phone through Claude Code cloud
sessions anyway. No dev container exists. `CLAUDE.md` carries his rules into those sessions.

### 4. Loose ends

- 376 old test folders (`pinewatch-*-test-*`) are left in `%TEMP%` on the Windows PC, for the owner to delete.
- M1's review fixes were never re-reviewed independently.

## Exact next steps

1. **On the Windows PC:** `git fetch`, check out `main-9m65p0`. Run `cargo test --workspace` (expect 535 tests,
   none skipped: the Windows-only test, plus the install tests that skip on Linux run for the first time, among them
   the map walk, `shipped_messages`' assembly half, `shipped_mscorlib` with the five new strings, and the assets
   install test). If the level install test reports `Context "Landscape tile …" missing asset` warnings, look for the
   same lines in the original game's `Logs/Client.log` after loading that map before changing anything: the game
   warns too when a material is really missing. If a shipped-text test fails, compare with the DLL before changing
   anything: the port must match it. Run `cargo clippy --workspace --all-targets` and `cargo xtask check-deps`. Run
   the app once (`cargo run -p pinewatch --release`): it now loads every asset before the map, which has never run
   on the real game; expect a slower start and the asset system's messages on stderr. Do the V17, V3 and V21 checks
   above and update `docs/VERIFICATION.md`. Then fast-forward `main` and push it.
2. **M2c is written in full** (tasks 1 to 12), but its install tests have never run: they skip without the game.
   The PC run in step 1 is their first. **In the cloud:** M2f tasks 2 to 7, test first, one commit per task, after
   the owner reviews the plan if he wants to. The active holiday (V14) waits for the provider. The next Unity plans,
   M2d (asset processor) and M2e (terrain materials), need the PC.
3. **On the PC:** M2b: write its spec and plan, choosing between the Unity Editor in batch mode and the game's own
   Mono runtime for each check, then implement it test first and record what the comparisons show.

## Decisions worth knowing

- From M2a: `DatParseable` has an associated `Context` type and `try_parse` returns `Result<bool, MonoException>`;
  `try_parse_struct` returns `(bool, T)`; `try_get_string` returns `Option<Option<&NetString>>` (a flag's null counts
  as found); `try_parse_color32_rgb` returns `(bool, Color32)`; the vector helpers return `Result`;
  `DatComment::join_lines` returns an `Option`; `TypeResolver::resolve` returns `Result<Option<TypeHandle>, _>`.
- From M2c: Pinewatch runs as the shipped Windows client (`WITH_NOREDIST`, V21) on every host; it behaves as a Steam
  client with no workshop subscriptions (L11); the asset worker's threads become one deterministic order (L5);
  every path is a Mono string paired with an install-relative path (`MonoPath`); master bundles count as loaded
  (V25).
- Not ported yet: the writers and editable nodes, the twelve `DateTime` test rows, the hash in `Data.cs`,
  `readByteArray`, and `UnturnedDatEx`'s writers (`SetAssetRefWithInlineComment`).
- A ported C# enum is a transparent integer newtype with a constant per member, so values no member has survive;
  `pinewatch_mono::net_enum!` declares one (older enums were written out by hand and work the same way).
- Open gaps: V19 (types beyond the asset classes), V20, V22, V23, V25 and V26 in `docs/VERIFICATION.md`, and L7 in
  `COMPATIBILITY.md`.

## Where to read more

- `docs/superpowers/specs/2026-11-01-file-format-readers-design.md`: the design spec, with parity rules P1 to P8.
- `docs/superpowers/plans/`: the M1, M2a and M2c plans.
- `COMPATIBILITY.md`: the compatibility rules and the register of known limitations.
- `docs/VERIFICATION.md`: behaviors that wait for the reference harness.
- `NOTICES-PORTED-CODE.md`: license notices for the ported .NET and Mono code.
