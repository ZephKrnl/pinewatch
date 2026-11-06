# Pinewatch development

Read `docs/CLOUD_HANDOFF.md`, then `docs/RECOVERY.md` and the relevant milestone plan before changing code.
`CLAUDE.md` contains the shared project rules. The newer cloud handoff supersedes older approval and status notes.

- The owner authorized uploading the current project and continuing development in the cloud on 2026-11-06.
- Author every new commit as `Zeph <205350396+ZephKrnl@users.noreply.github.com>`.
  Configure `user.name` and `user.email` locally, then check `git var GIT_AUTHOR_IDENT` before committing.
- Do not add co-author or AI attribution trailers. Preserve existing history and keep the repository private.
- Production code is Rust. Match the target C# behavior exactly. Record unresolved differences in
  `COMPATIBILITY.md` or `docs/VERIFICATION.md` instead of silently approximating.
- Never modify the Unturned installation or `reference/u3-sdk`. Never commit game content, credentials,
  build output, local tooling installations or caches.
- Use the milestone plans and test meaningful changes. Record skipped installation-dependent tests accurately.
- Work on a development branch. Update `main` only by fast-forward after the required checks pass.
- The cloud environment has no Unity or installed game. Do not claim fresh Windows parity results from Linux.
- Graphify is optional. If unavailable, use text searches and the relevant test suite. Do not install machine-local
  hooks in shared settings. If using a graph, check its freshness and confirm call relationships in source.
