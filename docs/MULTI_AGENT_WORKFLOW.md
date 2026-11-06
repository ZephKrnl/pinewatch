# Multi-agent workflow

How Pinewatch work is split across parallel Claude Code subagents. The main session is the orchestrator: it assigns
tasks and file ownership, integrates every change, runs the tests and asks the owner before any commit. The owner's
rules in `CLAUDE.md` bind every agent.

## Roles

Pinewatch is a port of a game: it has no services, database or web UI. The five roles map onto its real parts.

| Agent (`.claude/agents/`) | Scope | Mode |
|---|---|---|
| `pinewatch-architecture` | Crate layering, call relationships, Rust-to-C# translation map, architectural issues | Read-only |
| `pinewatch-core` | The format and runtime crates (`pinewatch-mono`, `-unity`, `-dat`, `-io`, `-kvt`, `-binary`, `-assets`, `-level`) | Implements, only in assigned files |
| `pinewatch-app` | The Bevy viewer (`crates/pinewatch`): startup, rendering, camera, screenshots | Investigates; fixes as patches |
| `pinewatch-qa` | Independent review of every change: correctness against the C#, tests, regressions, robustness | Read-only |
| `pinewatch-performance` | Hot paths in asset and level loading and startup, measured | Read-only |

## Rules for parallel work

- **Ownership.** Each task names the files an agent may change. No two agents own the same file at the same time. The
  orchestrator owns `docs/`, `COMPATIBILITY.md`, `NOTICES-PORTED-CODE.md` and anything shared (`lib.rs` of a crate
  another agent touches, `Cargo.toml`).
- **Isolation.** An implementing agent works in its own copy of the worktree (a plain directory copy in the session
  scratchpad, no git operations), so other agents' builds and test runs on the shared tree never see half-done edits.
  The orchestrator copies the reviewed files back.
- **Read-only agents** change no repository file; their notes and scripts go in the scratchpad.
- **Builds.** Agents that build concurrently use separate `CARGO_TARGET_DIR`s when they would otherwise wait on the
  same lock for long.
- **Git.** No agent commits, pushes, stashes, resets, checks out or creates branches or worktrees. Only the orchestrator
  commits, and only with the owner's approval.
- **Never written:** the Unturned install, `C:\Pinewatch` and the other worktrees, `reference/u3-sdk`.
- **Integration:** implementing agent → QA review → orchestrator applies → targeted tests, then `cargo test
  --workspace` and `cargo clippy --workspace --all-targets` → `graphify update .` → owner approval → commit.
- **Reports** are short: findings with `file:line`, the evidence (graph output, source lines, a measured number or a
  run on the shipped assemblies), severity, and the proposed change. No unverified claims about the original game.

## Graphify protocol

Every agent reads `.claude/skills/graphify/SKILL.md` and `references/query.md` first, then:

1. **Graph first.** Before reading code in an unfamiliar area, query the local graph (`graphify query "<question>"`,
   `graphify explain "<symbol>"`, `graphify path "<A>" "<B>"`, `graphify affected "<symbol>" --relation calls
   --relation references`, `graphify god-nodes`). Read source files only where the graph points, and check what the
   graph says against those lines before relying on it.
2. **Two graphs.** `graphify-out/graph.json` is the Rust workspace (it records `built_at_commit`). The C# SDK graph is
   built from a scratch copy of `reference/u3-sdk/Assets/Runtime` (copy the `*.cs` files, then `graphify extract .
   --code-only` there; never build inside the SDK checkout) and answers "what does this C# method call" with
   `--graph <that graph.json>`.
3. **Known gap: `Type::function` uses.** The Rust extraction links calls like `MonoException::null_reference()` from
   other files, and functions passed as values, to the type rather than the function, so `affected` can report
   nothing. Use `python -I tools/graph_impact.py graphify-out/graph.json . "<text>" --type <Type>`: it finds the exact
   sites in the files the graph links to the type, maps them to their enclosing functions, and walks the reverse
   `calls` edges to callers and tests.
4. **The hosted Graphify workspace** (MCP tools such as `graphify_callers`, `graphify_tests_for`, `impact_and_risk`,
   `graphify_file_neighbors`) indexes GitHub `main`. Check its `commitSha` (`graph_stats`) and use it only for files
   unchanged since then (`git diff --stat <commitSha>..HEAD -- <path>`); it can't be refreshed without a push.
5. **Refresh.** After structural code changes run `graphify update .`, then check that `built_at_commit` in
   `graphify-out/graph.json` equals `git rev-parse HEAD`. The graph reads the working tree, so uncommitted changes are
   in it even though the stamp names HEAD. Git hooks are not installed: `.git` is shared by every worktree.
6. **Feedback.** After a useful query, `graphify save-result --question ... --answer ... --outcome useful` keeps it
   for later sessions (`graphify reflect` summarizes them).

## Shipped-game checks

`reference/checks/README.md`: the Unity Editor's Mono (32-bit) runs the shipped class libraries; `gamehost` runs code
on the game's own 64-bit runtime, which is what counts for anything the runtime decides.
