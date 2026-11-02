# Handoff

The state of Pinewatch on 2026-11-02 (updated at the end of the first cloud session), written so work can resume
from this file alone.

## Where things stand

- **Current branch:** `main-9m65p0`, the branch the cloud session was given. It holds `main` plus the M2a review
  fixes below, and is pushed. **`main` was not fast-forwarded**: the fixes still need the Windows run (see "Exact
  next steps").
- **Done:** M1 (the bootable terrain viewer) and M2a (the `.dat` reader) are merged into `main`. On the Windows PC,
  with the game installed, `cargo test --workspace` passes all 396 tests on `main` and skips none.
- **Linux (cloud session, 2026-11-02):** on `main-9m65p0`, `cargo test --workspace` passes 403 tests (the install
  tests print `skipped: no Unturned install found`; the Windows-only test doesn't build), `cargo clippy --workspace
  --all-targets` gives no warnings, and `cargo xtask check-deps` passes. Setup it needed: `apt-get install
  libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev` (pkg-config was present), `rustup component add
  clippy`, and `cargo fetch`, because `check-deps` runs `cargo tree --target all --offline` and a fresh machine
  lacks the Windows-only crates.
- **Remote:** `origin` is the private GitHub repository `ZephKrnl/pinewatch`. It stays private.
- **Branches** (all pushed to `origin`):

  | Branch | What it holds |
  |---|---|
  | `main` | M1, M2a, the M2a review fixes and the handoff note (`480c1ca`) |
  | `main-9m65p0` | `main` plus the M2a minor findings and two Linux test fixes (cloud session) |
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

## Unfinished work

### 1. Eight minor findings from the M2a final review (seven done, on `main-9m65p0`)

Done in the cloud session, one commit each, every one with its test:

| # | Finding | What was done |
|---|---|---|
| 1 | The install walk missed map files | `crates/pinewatch-dat/tests/install.rs` also walks each map's root `.dat` files and `Bundles/`, asserts that map files and a `Maps/*/English.dat` were found, and checks the walk on a made-up install tree. |
| 3 | V18 | Settled and moved to the end of `docs/VERIFICATION.md`; the `ordered_members` comment is fixed; `equal_values_give_the_same_bits_in_any_order` covers `[1, 1, 0]` and a 20-member enum. |
| 4 | No independent float bits | `corert_bits_from_an_independent_model` in `crates/pinewatch-mono/tests/floats.rs` pins 27 rows (14 corert quirks, both review cases among them). They came from a separate big-integer model of upstream `Number.CoreRT.cs` (`mono/corert` `11136ad5`); the upstream tables equal the port's, and the model agreed with the port on 20,000 generated decimals. V16 records this. |
| 5 | Tokenizer comment | Fixed (the SDK blob is LF); the case runs with LF and CRLF. |
| 6 | README status, V19 | The status line now says the reader isn't yet confirmed by the harness. `TypeResolver::resolve` and both `parse_type` helpers return `Result<_, MonoException>`, `ExceptionKind::FileLoad` is new, and tests check that a resolver's exception propagates. V19 updated. |
| 7 | Message texts against `UnturnedDat.dll` | New `crates/pinewatch-dat/tests/shipped_messages.rs`: every tokenizer, parser and `Data.cs` message is produced through the port and looked up in the `#US` heap of `UnturnedDat.dll` or `Assembly-CSharp.dll`, as a `string.Format` template or as `string.Concat` pieces. **The assembly half has only been skipped so far**; its first real run is on the PC. |
| 8 | Six clippy warnings | Fixed; `clippy --workspace --all-targets` is clean on Linux. |

Also fixed, because they failed on Linux: the missing-heightmap test in `crates/pinewatch-level/tests/tiles.rs` now
expects the `/` form there, and the screenshot test in `crates/pinewatch/tests/exe.rs` checks an upper-case install
path only on Windows and macOS (on Linux it is a different folder). Neither changes production behavior.

Still open:

| # | Finding | Suggested fix, with its regression test |
|---|---|---|
| 2 | V17 can be narrowed. In the shipped IL, `OrdinalIgnoreCase` equality and hashing both fold each `char` through `TextInfo.ToUpper`, so only the table data (V3) is open. | Re-check with the IL dump of the shipped `mscorlib.dll` (Windows PC only), then update V17 in `docs/VERIFICATION.md`. |

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

1. **Finish the minor findings on the Windows PC.** `git fetch` and check out `main-9m65p0` (it is `main` plus the
   fixes, so `main` can fast-forward to it). Do finding 2 with the IL dump and update V17. Run `cargo test
   --workspace` (expect 404 tests, none skipped; the new install checks, the map walk and the assembly half of
   `shipped_messages`, run there for the first time), `cargo clippy --workspace --all-targets` and `cargo xtask check-deps`. If `shipped_messages` finds a
   text missing, compare it with the shipped literal before changing anything: the port must match the DLL. Then
   fast-forward `main` and push it.
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
- `TypeResolver::resolve` returns `Result<Option<TypeHandle>, MonoException>` (cloud session): `Type.GetType` can
  throw even with `throwOnError: false`, and `ParseType` doesn't catch.
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
