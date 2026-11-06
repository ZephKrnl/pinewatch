# Cloud continuation checkpoint

Prepared 2026-11-06 for the owner's request to save the current project to private `ZephKrnl/pinewatch` and
continue through GitHub in the cloud. The owner explicitly requested no local build or local development workload.
This authorization supersedes the older notes requiring approval to save the pending work.

## Snapshot and history

The current source comes from `C:\pinewatch-integration`, local branch `integration/m2a-m2c`, commit `739b9dd`
plus the pending working-tree changes. `C:\Pinewatch` is an older M2a checkout and is not the continuation point.
The original local directories remain untouched.

The upload snapshot is based on the already published `main-9m65p0` commit
`eb9368b1be2c25cfef3b7febe6b3af8abdfd8063`. It includes the content of the three local integration commits
`21b8ed4`, `4da03ec` and `739b9dd`, plus the pending runtime, Unicode, JSON, workflow and review files.
Those local commits remain intact in the original repository. Published history is not rewritten.

Machine-local Graphify hooks, its installed skill, backups, graph output, build outputs and game files are excluded.
The shared Claude settings retain `includeCoAuthoredBy: false`. The SDK remains a submodule pinned to
`aba323a81558487ad8d2167ac20da9c0fd72b6d9`.

## Where development stands

- M1 and M2a: implemented.
- M2c: asset discovery and indexing implemented. The recorded Windows run indexed 6,114 assets with no warnings.
- M2f tasks 1 and 2: binary `Block` and `Level.dat` reading implemented.
- M2f task 3: Unicode categories and the Json.NET token reader implemented. The recorded 409-case reference corpus
  matches the shipped library. The final review was still pending in the latest recovery record.
- M2f tasks 4 to 7: typed reads, object values, typed deserialization and level configuration remain.
- M2b, M2d and M2e: reference harness, Unity asset processing and original terrain materials remain.

The seam-normal patch in `docs/reviews/2026-11-05/` is review material only. It is not applied to production code.
The older `docs/HANDOFF.md` and some checkboxes in the plans are stale. Use source plus `docs/RECOVERY.md` for detail.

## Validation and next work

No build or tests were run for this upload, as requested. The last recorded Windows result is 2026-11-05:
555 tests passed, clippy clean, and dependency checks passed for 9 format crates. These are historical results,
not a new validation of the upload. Known JSON gaps remain in V31.

1. Read `docs/RECOVERY.md`, `docs/reviews/2026-11-05/review.md` and the M2f plan. Recover any final task 3 review if
   available, or review task 3 before extending it. Fix important findings with regression tests.
2. Initialize the SDK submodule. Install the pinned Rust 1.99.1 toolchain and Linux build dependencies as needed:
   `pkg-config`, `libasound2-dev`, `libudev-dev`, `libwayland-dev`, `libxkbcommon-dev`.
   Run `cargo fetch` before `cargo xtask check-deps`, whose dependency traversal is offline.
3. Validate in the cloud with `cargo test --workspace`, `cargo clippy --workspace --all-targets` and
   `cargo xtask check-deps`. Report installation-dependent skips separately.
4. Continue M2f task 4 from `docs/superpowers/plans/2026-11-02-m2f-level-info.md`, then tasks 5 to 7.
   Move `HashedBytes`/`SHA1Stream` from `pinewatch-level` to `pinewatch-io` before task 7 as recorded in the review.
5. Update the recovery record after each completed task. Author new commits as Zeph with the noreply identity in
   `AGENTS.md`. Keep Unity and shipped-runtime verification for the Windows environment.
