# Handoff

The state of Pinewatch on 2026-11-03, written so work can resume from this file alone. Nothing is half-done in the
tree: the eight M2a review findings are implemented on `opencode/development` and are waiting for review and merge.

## Where things stand

- **Current branch:** `opencode/development`. The working tree is clean.
- **Done:** M1 (the bootable terrain viewer) and M2a (the `.dat` reader) are merged into `main`. The eight M2a
  review findings are done on `opencode/development`. On the Windows PC, with the game installed,
  `cargo test --workspace` passes all 400 tests and skips none; `cargo clippy --workspace --all-targets` and
  `cargo xtask check-deps` are clean.
- **Remote:** `origin` is the private GitHub repository `ZephKrnl/pinewatch`. It stays private.
- **Branches** (all pushed to `origin`):

  | Branch | What it holds |
  |---|---|
  | `main` | M1, M2a and this note; not yet the eight review fixes |
  | `opencode/development` | The eight M2a review fixes, ready to review and merge into `main` |
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
- `CLAUDE.md` holds these rules for every Claude session, and `.claude/settings.json` turns off the co-author line.

## Work status

### 1. Eight minor findings from the M2a final review (done on `opencode/development`)

Implemented on 2026-11-03, with a regression test for each code change:

- The install walk now includes `Maps/*/English.dat` and `Maps/*/Bundles/`, skips the binary `Level.dat`, and asserts
  that map files were found.
- V17 now records what the shipped IL shows: equality and hashing both fold each `char` through
  `TextInfo.ToUpper`; only Mono's Unicode table data (V3) is open.
- V18 is settled. `Array.Sort` is not stable even for a few members, but it can only reorder members with equal
  values, which contribute the same bits. The `ordered_members` comment is fixed, and an equal-valued out-of-order
  test covers it.
- `floats.rs` now pins independently computed corert bits, taken from Unity 2022.3.62f3's Mono runtime, including
  the `-1.79769313486231571982965e-323` -> `0x8000000000000003` and `2.4703282292062328e-324` ->
  `0x0000000000000001` quirks, and the overflow at `1.7976931348623159e308`.
- The tokenizer comment now says the SDK blob is LF and this checkout's CRLF comes from `core.autocrlf`; the case
  runs with both line endings.
- The README status no longer claims more than the harness confirmed. V19 records that `Type.GetType` can still
  throw with `throwOnError: false` (for example `FileLoadException`). `TypeResolver::resolve` returns
  `Result<Option<TypeHandle>, MonoException>`, `ExceptionKind::FileLoad` exists, and a test checks that the error
  propagates through `parse_type`.
- An install test finds the port's message format strings among the `UnturnedDat.dll` literals and `Data.cs`'s
  duplicate-key text in `Assembly-CSharp.dll`.
- The clippy warnings are fixed; `net_string.rs` allows the intentional empty-slice comparison lint.

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
  - V17 (key case folding): only the same Unicode case tables (V3); the comparer algorithm is settled.
  - V19 (type names): which names `Type.GetType(name, false, true)` resolves when called from `UnturnedDat.dll`,
    and which exceptions it throws with `throwOnError: false`.
  - V20 (legacy `readBoolean`): `InvariantCultureIgnoreCase` results for `y` and `true` with inserted or replaced
    characters, including ignorable ones such as U+00AD.
- **This work needs the Windows PC**, where the game install and Unity are.

### 3. Phone and cloud access

The owner first asked for a GitHub Codespaces setup, then chose to push to GitHub only, because section 2 of the
U3 SDK License (`LICENSE.txt`) bars using or developing the SDK or any Mod "for or in connection with ... mobile or
handheld devices", and any environment that enables gameplay on them through cloud-based delivery. No dev container
exists. Later the same day the owner chose to continue from his phone through a Claude Code cloud session that works
on this repository while the PC is off. `CLAUDE.md` carries his rules into those sessions.

### 4. Loose ends

- 376 old test folders (`pinewatch-*-test-*`) are left in `%TEMP%` on the Windows PC, for the owner to delete.
- M1's review fixes were never re-reviewed independently.

## Exact next steps, when the owner gives the command

1. **Review and merge the eight minor findings.** `opencode/development` is pushed and green: 400 tests,
   `cargo clippy --workspace --all-targets` and `cargo xtask check-deps` pass on the Windows PC. Review it, then
   fast-forward `main` and push `main` when the owner asks.
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
