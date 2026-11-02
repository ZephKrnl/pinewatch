# Handoff

The state of Pinewatch on 2026-11-02, written so work can resume from this file alone. Nothing is half-done in the
tree: the work below was requested and then paused before it started.

## Where things stand

- **Current branch:** `main`, at the commit that adds this file. The working tree is clean.
- **Done:** M1 (the bootable terrain viewer) and M2a (the `.dat` reader) are merged into `main`. On the Windows PC,
  with the game installed, `cargo test --workspace` passes all 396 tests and skips none.
- **Remote:** `origin` is the private GitHub repository `ZephKrnl/pinewatch`. It stays private.
- **Branches** (all pushed to `origin`):

  | Branch | What it holds |
  |---|---|
  | `main` | Everything: M1, M2a, the M2a review fixes and this note |
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
  `git pull` and `git push` as usual; Git Credential Manager signs in as ZephKrnl.

## Rules that carry over

- **1:1 behavior.** Wherever the original C# behavior is observable, Pinewatch matches it exactly: parsing results,
  number conversion, error messages and their order, exceptions and asset skipping, quirks, binary layout. Where an
  exact match is impossible, the case gets an entry in `COMPATIBILITY.md` (or `docs/VERIFICATION.md` while it
  awaits the harness) saying why; nothing is substituted silently.
- **Rust only** for production code. The C# in `reference/u3-sdk` is a reference and is never edited.
- **The Unturned installation is never modified.** Tests and tools only read it.
- **Commits** are authored `Zeph <205350396+ZephKrnl@users.noreply.github.com>` (set in the repository's local Git
  config). No `Co-Authored-By` lines and no AI attribution. The existing history stays exactly as it is.
- **The repository stays private.** Never push credentials, game files or other third-party assets.
- **Development resumes only on the owner's explicit command.**

## Unfinished work

### 1. Eight minor findings from the M2a final review (not started)

| # | Finding | Suggested fix, with its regression test |
|---|---|---|
| 1 | `crates/pinewatch-dat/tests/install.rs` walks only `Bundles/` and `Localization/`. The game also parses `Maps/*/English.dat` with a reused `DatParser` (`LevelInfo.getLocalization` through `ReadWrite.ReadDataWithoutHash`). | Add the map files (and any `Maps/*/Bundles/`) to the walk, and assert that map files were found. |
| 2 | V17 can be narrowed. In the shipped IL, `OrdinalIgnoreCase` equality and hashing both fold each `char` through `TextInfo.ToUpper`, so only the table data (V3) is open. | Re-check with the IL dump, then update V17 in `docs/VERIFICATION.md`. |
| 3 | V18 can be settled: an unstable sort only reorders members with equal values, and those give the same bits. The "stable up to 16 members" wording is also wrong (the three-element case reorders `[1, 1, 0]`). | Move V18 to "Settled", fix the comment on `ordered_members` in `crates/pinewatch-mono/src/enums.rs`, and add a test with equal-valued names out of order. |
| 4 | `crates/pinewatch-mono/tests/floats.rs` has no independently computed expected bits. | Pin corert quirks, for example `"-1.79769313486231571982965e-323"` → `0x8000000000000003` and `"2.4703282292062328e-324"` → `0x0000000000000001`, plus more cases from an independent model of corert's algorithm. |
| 5 | The comment on `compare_with_and_without_comments` in `crates/pinewatch-dat/tests/tokenizer.rs` says the C# file has CRLF line ends. The SDK blob is LF; the CRLF comes from `core.autocrlf` in this checkout. | Fix the comment and run the case with both LF and CRLF. |
| 6 | The README's status line claims more than the reference harness has confirmed. V19 could note that `Type.GetType` still throws some exceptions with `throwOnError: false` (for example `FileLoadException` for an assembly-qualified name). | Reword the status line; update V19, and consider letting `TypeResolver::resolve` return a `Result`, with a test that its error propagates through `parse_type`. |
| 7 | Nothing checks the message format strings against the shipped `UnturnedDat.dll` the way `shipped_mscorlib` checks `mscorlib.dll`. | Add an install test that finds each format string the port uses among the DLL's string literals, and `Data.cs`'s "Duplicate key" text in `Assembly-CSharp.dll`. |
| 8 | Six clippy warnings in M1 code: `crates/pinewatch-io/src/install.rs:122,133` and `crates/pinewatch/src/app.rs:79,80,137` (collapsible `if`), and `crates/pinewatch-mono/tests/net_string.rs:21` (comparison to an empty slice). | Collapse the `if`s. In `net_string.rs` the comparison tests `PartialEq<&str>` with `""` on purpose, so allow the lint there. |

### 2. M2b, the reference harness (not started; no plan written yet)

The owner's paused request: design and implement the Unity 2022.3.62f3 reference harness, resolve the compatibility
gaps V3, V17, V19 and V20 using the original game as the reference, run the reference comparisons, and document any
remaining differences.

- **Scope in the M2 plan series** (see the table in `docs/superpowers/plans/2026-11-01-m2a-dat-reader.md`): a
  batch-mode harness (`cargo xtask reference setup/run`), canonical output, `cargo xtask testdata`, runtime-table
  dumps, and conformance for `pinewatch-mono` and `pinewatch-dat` on every `.dat` and `.asset` file, closing the
  matching `docs/VERIFICATION.md` entries. `.gitignore` already reserves `/reference/work` (the Unity working copy)
  and `/testdata` (local copies of game files, never committed).
- **Found on the Windows PC on 2026-11-02:**
  - Unity Editor 2022.3.62f3 and 6000.6.7f1 under `C:\Program Files\Unity\Hub\Editor`, and Unity Hub. Whether a
    license is activated for batch mode wasn't checked.
  - The .NET Framework C# compiler (`csc.exe`, `ilasm.exe`) and the `dotnet` CLI.
  - The game ships its own Mono runtime (`MonoBleedingEdge\EmbedRuntime\mono-2.0-bdwgc.dll` and
    `MonoBleedingEdge\etc`). That runtime with the game's `Unturned_Data\Managed\*.dll` is exactly what the game
    executes, so it is a candidate for the class-library checks (V3, V17, V20), read-only. Not decided.
- **What each gap needs:**
  - V3 (Unicode tables): `IsWhiteSpace`, `ToUpperInvariant` and `ToLowerInvariant` for all 65,536 code units.
  - V17 (key case folding): the same tables, plus `OrdinalIgnoreCase` equality and hashing on key pairs.
  - V19 (type names): which names `Type.GetType(name, false, true)` resolves when called from `UnturnedDat.dll`.
  - V20 (legacy `readBoolean`): `InvariantCultureIgnoreCase` results for `y` and `true` with inserted or replaced
    characters, including ignorable ones such as U+00AD.
- **This work needs the Windows PC**, where the game install and Unity are.

### 3. Codespaces and phone access (skipped on purpose)

The owner asked for a GitHub Codespaces setup so work could continue from a phone, then chose to push to GitHub
only. The reason is section 2 of the U3 SDK License (`LICENSE.txt`), which bars using or developing the SDK or any
Mod "for or in connection with ... mobile or handheld devices", and any environment that enables gameplay on them
through cloud-based delivery. Revisit only if the owner decides that use is permitted.

### 4. Loose ends

- 376 old test folders (`pinewatch-*-test-*`) are left in `%TEMP%` on the Windows PC, for the owner to delete.
- M1's review fixes were never re-reviewed independently.

## Exact next steps, when the owner gives the command

1. **The eight minor findings.** Branch `maintenance/m2a-minors` from `main`. Fix each finding with its regression
   test, run `cargo test --workspace`, `cargo clippy --workspace --all-targets` and `cargo xtask check-deps` on the
   Windows PC, fast-forward `main`, and push `main` and the branch.
2. **M2b.** Write the spec and the plan (`docs/superpowers/plans/<date>-m2b-reference-harness.md`) from the scope
   above, choosing between the Unity Editor in batch mode and the game's own Mono runtime for each check. Then
   implement it on the Windows PC with tests first, run the comparisons, and update `docs/VERIFICATION.md` and
   `COMPATIBILITY.md` with what they show.

## Decisions from M2a worth knowing

- `DatParseable` has an associated `Context` type, and `try_parse` returns `Result<bool, MonoException>`, because
  the game's implementations throw and read static game state. The struct helpers pass both along.
- `try_parse_struct` returns `(bool, T)`: on failure the value can be partly filled, as the C# out value is.
- `try_get_string` returns `Option<Option<&NetString>>`: a flag's null text counts as found, so `get_string` returns
  that null rather than the default, as `GetString` does.
- `try_parse_color32_rgb` returns `(bool, Color32)`, because it fails to opaque black rather than the default.
- The vector helpers, `legacy_parse_vector3` included, return `Result`, because `String.IndexOf` throws for a value
  that ends with `(`.
- `DatComment::join_lines` returns an `Option`, because the C# returns null.
- Not ported yet: the writers and editable nodes, the twelve `DateTime` test rows, the asset, master-bundle and
  audio references and the hash in `Data.cs`, `readByteArray`, and `UnturnedDatEx`.
- Open gaps: V3, V17, V19 and V20 in `docs/VERIFICATION.md`, and L7 (stack depth for extremely deep nesting) in
  `COMPATIBILITY.md`.

## Where to read more

- `docs/superpowers/specs/2026-11-01-file-format-readers-design.md`: the design spec, with parity rules P1 to P8.
- `docs/superpowers/plans/`: the M1 and M2a plans.
- `COMPATIBILITY.md`: the compatibility rules and the register of known limitations.
- `docs/VERIFICATION.md`: behaviors that wait for the reference harness.
- `NOTICES-PORTED-CODE.md`: license notices for the ported .NET code.
