# Pinewatch: instructions for Claude

Pinewatch is a strict 1:1 Rust port of Unturned (U3 SDK v3.26.3.12). Start every session from `docs/RECOVERY.md`:
it says where the work stands, what is in progress, which agent owns what, and the next actions. Resume from it
instead of rediscovering the project, and update it after every significant task. `docs/HANDOFF.md` has the
background and the decisions.

## The owner's standing rules

- **Wait for the owner's explicit command** before starting development work.
- **1:1 behavior** with the original C# wherever it is observable: parsing results, number conversion, error messages
  and their order, exceptions and asset skipping, edge cases and legacy quirks, binary layout. Never approximate,
  simplify or change it. If an exact match is impossible, record it in `COMPATIBILITY.md` (or in
  `docs/VERIFICATION.md` while it waits for the reference harness) with the reason. Never substitute silently.
- **Production code is Rust.** `reference/u3-sdk` is the reference C# source and is never edited.
- **The Unturned installation is never modified.** Tests and tools only read it.
- **Add tests for every meaningful change**, test first, and follow the plans in `docs/superpowers/plans/`.
- **Commit identity.** Every commit is authored `Zeph <205350396+ZephKrnl@users.noreply.github.com>`. In a fresh
  clone, run `git config user.name Zeph` and `git config user.email 205350396+ZephKrnl@users.noreply.github.com`,
  then check `git var GIT_AUTHOR_IDENT` before committing.
- **No AI attribution.** Never add `Co-Authored-By` lines or any Claude, Anthropic or AI attribution to commits or
  pull requests. Never rewrite existing history.
- **The repository stays private.** `ZephKrnl/pinewatch` on GitHub is private: never change its visibility, publish
  a release, or push credentials, game files or other third-party assets.
- **Clean history:** work on a branch, fast-forward `main`, and push.

## Environments

- **The Windows PC** (`C:\Pinewatch`) has the Unturned install, Unity 2022.3.62f3, and everything the reference
  harness (M2b) needs. All tests run there.
- **Cloud sessions run Linux,** without the game or Unity:
  - Tests that read the install print `skipped: no Unturned install found` and pass.
  - `message_root_is_absolute_and_normalized` in `crates/pinewatch-level/tests/tiles.rs` runs only on Windows.
  - `Environment.NewLine` is `\n` on Linux, so message tests check the Linux form. Windows is the reference platform
    for 1:1 checks.
  - Building the `pinewatch` app crate (Bevy) on Linux needs `pkg-config` and the ALSA, udev, Wayland and xkbcommon
    development packages.
  - `cargo test --workspace` passes on Linux (first run 2026-11-02, see `docs/HANDOFF.md`). On a fresh machine,
    install `clippy` and run `cargo fetch` first: `cargo xtask check-deps` runs `cargo tree --target all --offline`.
- **After cloning,** run `git submodule update --init`. On Windows, turn on long paths first (see
  `docs/HANDOFF.md`).

## graphify

Graphify is optional in cloud sessions. If the executable or graph is absent, skip graph operations and use
text searches plus the relevant test suite. Machine-local hooks and installed skills are not committed.

This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

Rules:
- When graphify-out/graph.json exists, run `graphify query "<question>"` before your first grep, search or source-file read for a codebase question, and put this rule in every subagent prompt that explores code. Read raw files only for lines the graph or an earlier read already located. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).
- `graphify-out/` is ignored and rebuilt locally. Do not commit generated graphs or local hook settings.
