# Recovery record

**2026-11-06 cloud handoff:** Read `docs/CLOUD_HANDOFF.md` first. The owner has authorized saving this work to
GitHub and continuing in the cloud. The status and approval wording below is the historical 2026-11-05 checkpoint.

Where Pinewatch stands, kept short so a session can resume from here instead of rediscovering the project. Start
here and read only what the next action needs. `docs/HANDOFF.md` has the background and the decisions; its status
sections are older than this record. Update this record after every significant task.

**Last updated:** 2026-11-05 07:30 (UTC+8), on the Windows PC: M2f task 3 is implemented and verified, uncommitted,
with its final review running. The plan's ledger is `.superpowers/sdd/2026-11-02-m2f-level-info/progress.md`; the
review round's report is `docs/reviews/2026-11-05/review.md`.

PREV below is the 2026-11-03 session's scratchpad,
`%TEMP%\claude\C--pinewatch-integration\7db6febe-1341-4a80-9486-46f78cbd850c\scratchpad`, and S is the review round's,
`%TEMP%\claude\c321285244580af4\b89ff58e-4fdf-4287-926a-1278241e279e\scratchpad`. Both are temp folders.

## Resume checklist

1. `git -C C:\pinewatch-integration status -sb` and `git log --oneline -3`: compare with "Commits and pushes".
2. Local graph: `built_at_commit` in `graphify-out/graph.json` should equal `git rev-parse HEAD`, and no source file
   should be newer than `graph.json` (`find crates xtask tools reference/checks -newer graphify-out/graph.json
   -type f`). Otherwise run `graphify update .` and check again. Without graphify, skip the graph steps and rely on
   the whole test suite.
3. Hosted graph (the Graphify MCP tools): `graph_stats` gives its `commitSha`. Use it only for files unchanged since
   that commit (`git diff --stat <commitSha>..HEAD -- <path>`).
4. Run graph queries on a copy of `graph.json` (`--graph <copy>`): `query`, `explain` and `path` write
   `graphify-out/cache/last_query_stamp`.
5. For M2f, read the ledger first: a task with a `Task <N>: complete` line is done.

## Current milestone and task

- **Active line:** `integration/m2a-m2c` in the worktree `C:\pinewatch-integration`, as the owner set on 2026-11-03.
- **Milestones:** M1 and M2a are on `main`. M2c (assets) is done, and M2f (level info) has tasks 1 to 3 done, all only
  on the active line. M2b (reference harness), M2d (asset processor) and M2e (terrain materials) have not started.
- **Current task:** M2f task 3, the JSON reader (owner's order, 2026-11-05): done and verified, waiting for the final
  review's findings and then the owner's approval to commit. Nothing is committed, merged or pushed until then.

## Completed

- M2c tasks 1 to 12 and M2f tasks 1 and 2 (cloud sessions, `main-9m65p0`). V15 settled (stabilization D1, `eb9368b`).
- The M2a integration with the PC's review fixes (`21b8ed4`), `WeatherAsset` (`4da03ec`), and V3, V17 and V21 settled
  from the shipped assemblies (`739b9dd`).
- Stabilization B1 closed: an exhaustive check of every `mono_path` function (17.7 million calls) found no panic.
- The app ran on the PC with M2c's asset loading: window 1.92 s after start (median of 5), peak 508 MiB.
- **The review round (2026-11-05):** qa, architecture and performance reviewed the pending work read-only; both patches
  and the V2/V30 change are correct, with no blocker (`docs/reviews/2026-11-05/review.md`).
- **M2f task 3 (2026-11-05), uncommitted:**
  - Step 0, the Unicode category table: the reviewed core patch, applied test first, with its notice rows, the
    generator in `reference/checks/gen_category_table.py` (it rebuilds the table byte for byte) and V3's mention.
  - The new crate `pinewatch-json`: `JsonTextReader.Read` with `JsonConvert`'s defaults (`reader.rs`), Json.NET's
    integer and hex readers (`number.rs`), and its date parsing (`date.rs`). It keeps Json.NET's 1,024-character
    buffer, because `TryReadOffset` depends on a string's position in it.
  - In `pinewatch-mono`: `chars::is_digit`, the exception kinds `JsonReader` and `JsonSerialization`,
    `convert_to_int64` (`Convert.ToInt64(string, fromBase)`), `DOUBLE_NAN` (.NET's NaN bits) and the two `DateTime`
    resource strings, all checked against the shipped corlib.
  - Evidence: `reference/checks/jsontokens.cs` runs the game's own `Newtonsoft.Json.dll` over a 409-case corpus
    (`crates/pinewatch-json/tests/fixtures/`), and the port matches every token, value, depth, path, position and
    exception. Gaps are in `docs/VERIFICATION.md`, V31 (the local time zone is taken as UTC; lone surrogates are
    lossy in messages and paths).
  - The M2f plan now names the Json.NET .NET 4.5 build and its define set.

## Waiting for the owner's approval

Uncommitted in the shared tree, `C:\pinewatch-integration` (555 tests pass, see "Tests"):

| Change | Files | State |
|---|---|---|
| V2 and V30 from the game's x64 runtime | `pinewatch-mono`: `src/exception.rs`, `src/lib.rs`, `tests/exception.rs`, `tests/shipped_mscorlib.rs`; `pinewatch-binary`: `src/block.rs`, `tests/block.rs`; `pinewatch-assets/tests/load.rs`; `NOTICES-PORTED-CODE.md`; `docs/VERIFICATION.md` | Reviewed, correct. Open: the Mono source names in the notices (m3), the V2 row (n1) |
| The multi-agent workflow | `docs/MULTI_AGENT_WORKFLOW.md`, `.claude/agents/pinewatch-*.md`, `tools/graph_impact.py` | Open: the graph gaps and the scratch-copy rule (m6); the script's docstring (m9) |
| The local Graphify setup | `.claude/skills/graphify/`, `.claude/CLAUDE.md`, the `graphify` section of `CLAUDE.md`, the hooks in `.claude/settings.json`, `.claude/settings.json.graphify-bak` | **Major (M1), open:** move the hooks to `.claude/settings.local.json`; don't commit the skill (license, m7); drop the backups; delete `CLAUDE.md:52` (m8) |
| `graphify-out/` ignored; V3 wording | `.gitignore`; `docs/VERIFICATION.md` | Done |
| M2f task 3 step 0: Unicode categories | `pinewatch-mono`: `src/chars.rs`, `src/chars/category_table.rs`, `tests/categories.rs`; `reference/checks/gen_category_table.py`; `NOTICES-PORTED-CODE.md` rows; V3 | Done; the plan commits it alone |
| M2f task 3: the JSON reader | `crates/pinewatch-json/` (new), `pinewatch-mono` (`chars.rs` `is_digit`, `parse_numbers.rs`, `number.rs`, `sr.rs`, `exception.rs`, `lib.rs`, `tests/categories.rs`), workspace `Cargo.toml`, `xtask/src/main.rs`, `NOTICES-PORTED-CODE.md` (Json.NET), `docs/VERIFICATION.md` (V31), `reference/checks/jsontokens.cs` and `README.md`, the M2f plan | Done and verified; final review running |
| This record, the review | `docs/RECOVERY.md`, its pointer in `CLAUDE.md`, `docs/reviews/2026-11-05/` | |

Not applied: the **app patch** (seam normals; `crates/pinewatch/src/terrain.rs`, `tests/boot.rs`). Reviewed, correct
for the viewer; it still needs a negative-coordinate 2x2 test (m5) and a `COMPATIBILITY.md` entry (m4). The files as
they would be installed are in S\patches\app\new; the review diff is `docs/reviews/2026-11-05/app-seam-normals.patch`.

## Remaining work

1. The approvals above, then fast-forward `main` to the active line and push; the hosted Graphify index catches up
   after that push.
2. M2f tasks 4 to 7: typed reads, `object` values, typed deserialization (`JsonSerializerInternalReader`), then
   `LevelInfoConfigData` and `ReadLevelInfo`. Move `SHA1Stream` (`HashedBytes`, `pinewatch-level/src/landscape.rs:130`)
   to `pinewatch-io` as a public type before task 7.
3. Loading speed-up, measured in the review: list-based file types, a parallel walk and a parallel reader cut client
   asset loading from 1,487 ms to 252 ms with identical output. Its own task, test first.
4. Architecture items: a V entry for `Assets.linkSpawns`; the spec's crate table; port `Landscape.getTile` and
   `HeightmapCoord` into `pinewatch-level`; the duplicated helpers the review lists.
5. M2b, the reference harness, on the PC: spec and plan first. M2d and M2e need Unity on the PC.
6. Open entries in `docs/VERIFICATION.md` (gaps V10, V12, V13, V19, V20, V22, V23, V25, V26, V28, V29, V31; V14's
   inputs) and L7 in `COMPATIBILITY.md`.
7. Loose ends: 376 old `pinewatch-*-test-*` folders in `%TEMP%`; M1's review fixes never re-reviewed;
   `docs/HANDOFF.md`'s status sections predate `739b9dd`.

## Agent ownership

Roles and rules are in `docs/MULTI_AGENT_WORKFLOW.md`. No file has two owners.

- **Orchestrator** (the main session): `docs/`, `COMPATIBILITY.md`, `NOTICES-PORTED-CODE.md`, `CLAUDE.md`, `.claude/`,
  every crate's `lib.rs` and `Cargo.toml`; integration, tests, and commits once the owner approves. M2f task 3 was
  implemented inline by the orchestrator (superpowers executing-plans).
- **core:** only the files its task assigns, in its own copy. **app:** `crates/pinewatch/` files, as patches.
- **qa, architecture, performance, final reviewer:** read-only. Reports are saved by the orchestrator (the harness
  refuses agent-written report files) to S\agents\<role>\report.md, with checkpoint.md kept current by each agent.
- Order: V2/V30 lands before M2f task 3, which also edits `pinewatch-mono/src/exception.rs` and `lib.rs`.
  `NOTICES-PORTED-CODE.md` and `docs/VERIFICATION.md` are shared by the pending changes.

## Important paths

- Worktrees: `C:\pinewatch-integration` is the active one. Not written: `C:\Pinewatch` (`pinewatch/development`, which
  also holds two untracked line-count reports of the owner's), `C:\pinewatch-claude` and `C:\pinewatch-chatgpt`, all
  three clean at `e14ac6d`. C# sources: `C:\Pinewatch\reference\u3-sdk` (the integration worktree's submodule is empty).
- Plans: `docs/superpowers/plans/2026-11-02-m2c-assets.md` and `2026-11-02-m2f-level-info.md`. Spec:
  `docs/superpowers/specs/2026-11-01-file-format-readers-design.md`.
- Shipped-game checks: `reference/checks/` (now with `jsontokens.cs` and `gen_category_table.py`). In PREV: copies of
  the shipped DLLs (`managed\`, used by `jsontokens.cs`), the game's runtime (`gamert\`), the V3 dumps (`v3\`), the
  Json.NET 13.0.3 sources (`jsonnet-src\`), the corefx sources for the category table (`agents\core\src\corefx\`),
  the C# SDK graph (`sdk-graph\`).
- Rebuild the reader's expected output: compile `reference/checks/jsontokens.cs` with the Editor's `mcs`
  (`-r:<Managed copy>/Newtonsoft.Json.dll -r:System.Numerics.dll`) and run it with `MONO_PATH=<Managed copy>` on
  `tests/fixtures/reader-corpus.txt`; the 2026-11-05 rerun was byte-identical.

## Graphify

| Graph | Covers | Built | Size | State on 2026-11-05 |
|---|---|---|---|---|
| Local: `graphify-out/graph.json` (graphify 0.9.101, ignored by git) | The active line's working tree, `pinewatch-json` included | `739b9dd` (HEAD), 2026-11-05 07:20 | 2,538 nodes, 6,611 edges | Current: no source file is newer |
| Hosted: the MCP tools, `ZephKrnl/pinewatch` | GitHub `main` | `480c1ca`, 2026-11-02 | 1,289 nodes, 2,753 edges | 40 commits behind: use only for unchanged files |
| C# SDK: PREV\sdk-graph | A copy of `reference/u3-sdk/Assets/Runtime` | 2026-11-03 | | Use with `--graph`; `ReadLevelInfo` needs its node id |

Known gaps, so confirm impact with a text search and always run the whole suite:

- `Type::function` uses and functions passed as values link to the type: use `tools/graph_impact.py` (omit `--type` to
  search every file; `--all-files` doesn't exist).
- Cross-crate method calls are almost all missing (1 edge into 446 production methods), and so are calls through
  struct fields (`self.install.exists`): `InstallRoot::exists`, called 27,911 times while loading, has no caller edge.
- Items declared through `net_enum!` have no node (`UnicodeCategory`, `DateTimeKind`, `ENPCHoliday`, `ELevelSize`).
- Test links are incomplete (`pinewatch-binary/tests/block.rs` `byte_arrays`, `pinewatch-assets/tests/load.rs`
  `bundle_override_path_flag_throws`); the hosted `graphify_tests_for` misses `crates/pinewatch/tests/boot.rs`.
- Graphify memory: notes from 2026-11-03 to 2026-11-05 wait in the hosted workspace's Memory > Needs review.

## Tests

| Date | Tree | Result |
|---|---|---|
| 2026-11-05 07:20 | Shared tree with M2f task 3 | `cargo test --workspace`: 555 passed, 0 failed, 0 skipped; `cargo clippy --workspace --all-targets`: no warnings; `cargo xtask check-deps`: 9 format crates pass. `pinewatch-json`: 10 (9 named, plus the 409-case corpus against the shipped Json.NET) |
| 2026-11-05 05:06 | Shared tree before task 3 | 540 passed, 0 failed, 0 skipped |
| 2026-11-05 00:30 | Shared tree plus both patches (S\review-tree) | 547 passed; clippy: no warnings; `check-deps` passes |
| 2026-11-02 | `main-9m65p0`, Linux cloud | 534 passed, the install tests skipped |

## Commits and pushes

| Branch | Commit | On `origin` |
|---|---|---|
| `integration/m2a-m2c` (active) | `739b9dd` | **No**: never pushed; 3 commits ahead of `origin/main-9m65p0` |
| `main-9m65p0` | `eb9368b` | Yes: the cloud sessions' work |
| `main` | `480c1ca` | Yes; not yet fast-forwarded to the active line |
| `pinewatch/development`, `claude/development`, `chatgpt/development`, and `opencode/development` (only on `origin`) | `e14ac6d` | Yes; superseded, their content is in `21b8ed4` |

No tags. The submodule `reference/u3-sdk` is at `aba323a` (`v3.26.3.12`).

## Immediate next actions

Each waits for the owner's approval. Nothing is committed, merged or pushed without it.

1. Act on the task 3 final review: fix Critical and Important findings test first, ledger the rest.
2. Commit in this order, one change each: V2/V30; the workflow and Graphify setup with `.gitignore` (after M1, m6 to
   m9); M2f task 3 step 0 (`pinewatch-mono: Unicode categories`); M2f task 3 (`pinewatch-json: JsonTextReader`); the
   docs (this record, the review). Then fast-forward `main` and push.
3. The app patch with its 2x2 test (m5) and `COMPATIBILITY.md` entry (m4), when the owner wants it.
4. M2f task 4, typed reads.
