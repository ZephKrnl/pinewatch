# Review round, 2026-11-05

**Verdict.** Both pending patches and the pending V2/V30 change are correct, and nothing blocks them. The one major
problem is in the uncommitted workflow setup: the Graphify hooks in `.claude/settings.json` would raise an error on
every tool call in a session without graphify, such as the Linux cloud sessions. A handful of notice and doc fixes
should go in with the patches. Nothing was applied, committed, merged or pushed.

**Scope.** The tree `C:\pinewatch-integration` (branch `integration/m2a-m2c`, HEAD `739b9dd`, plus its uncommitted
changes): the core patch (Unicode categories, M2f task 3 step 0), the app patch (seam normals), the uncommitted V2/V30
change, the local commits `21b8ed4`, `4da03ec` and `739b9dd`, and the uncommitted workflow and Graphify setup.

**How it ran.** Three read-only agents, QA, architecture and performance, each resumed from its saved run of
2026-11-03. The round was cut off twice: the app process exited at about 00:21, and the usage limit stopped QA and
performance at about 00:40. Their checkpoint files kept their findings, and all three finished after the limit reset.
The harness doesn't let agents write report files, so the orchestrator saved each final report as
`S\agents\<role>\report.md`, where S is
`%TEMP%\claude\c321285244580af4\b89ff58e-4fdf-4287-926a-1278241e279e\scratchpad`.

## Build and test results

| Run | Tree | Result |
|---|---|---|
| 2026-11-05 05:06 | Shared tree: `739b9dd` plus the uncommitted changes (V2/V30, workflow files, `.gitignore`, the V3 text) | `cargo test --workspace`: 540 passed, 0 failed, 0 skipped |
| 2026-11-05 00:30 | Copy of the shared tree with both patches (`S\review-tree`) | `cargo test --workspace`: 547 passed (540 plus 5 in `categories.rs` and 2 in `boot.rs`), 0 failed, 0 skipped. `cargo clippy --workspace --all-targets`: no warnings. `cargo xtask check-deps`: the 8 format crates pass |
| 2026-11-04 | Shared tree, the same code | 540 passed; clippy: no warnings; `check-deps` passes |
| 2026-11-05 | Patch checks | Both review patches apply to the shared tree and reproduce the agents' tested files byte for byte. The app agent's own `seam-normals.patch` doesn't apply (its `boot.rs` hunk fails) and is superseded |
| 2026-11-05 | Graphify output | `graphify update .` in a fresh copy rebuilt all 2,293 nodes and 5,976 edges, so `graphify-out/` is regenerable and is now in `.gitignore` |

Every run had the game installed, so no install test skipped. QA confirmed that the install's `mscorlib.dll`,
`Assembly-CSharp.dll` and `mono-2.0-bdwgc.dll` still equal the copies the evidence came from.

## QA findings

No blocker.

| Id | Severity | Finding | Proposed change |
|---|---|---|---|
| M1 | Major | `.claude/settings.json` runs `graphify hook-guard` on every Bash, Grep, Read and Glob call. Where graphify isn't installed the command fails with exit 127: Claude Code only blocks on exit 2, so the calls go through, but each one shows a hook error (inferred from the documented exit codes, not run on Linux). On the PC, `hook-guard` always exits 0 and never blocks | Move both hooks to `.claude/settings.local.json`, which is ignored; keep the committed `settings.json` as `{"includeCoAuthoredBy": false}` with a final newline |
| m1 | Minor | V3's wording mixed up `char.GetUnicodeCategory` (the dump, `0x5cd45c712e21a2e5`) and `CharUnicodeInfo.GetUnicodeCategory` (checksum only, `0x4280054dea7fbc49`) | **Done**: corrected in `docs/VERIFICATION.md`. When the core patch lands, add that `tests/categories.rs` pins both |
| m2 | Minor | `NOTICES-PORTED-CODE.md:23` covers only the whitespace parts of `chars.rs`; `chars/category_table.rs` has no row; the table's generator exists only in a temp folder | Extend the row (Char.cs `GetUnicodeCategory`, `GetLatin1UnicodeCategory`, `IsNumber`, `IsLetterOrDigit`, `CheckNumber`, `CheckLetterOrDigit`; CharUnicodeInfo.cs `GetUnicodeCategory`, `InternalGetCategoryValue`; UnicodeCategory.cs); add a row for the table; keep `gen_category_table.py` in `reference/checks/` |
| m3 | Minor | `NOTICES-PORTED-CODE.md:137-138` names `mono/metadata/exception.c`. In Unity mono `b8eb56f` the period-less text is in `mono/mini/mini-exceptions.c` (`mono_handle_exception_internal`) and the empty overflow message in `mono/metadata/object.c` (`mono_array_new_specific_internal`, `mono_array_new_full_checked`) | Name those files and functions |
| m4 | Minor | The viewer's normals approximate Unity's linked terrain (`Landscape.linkNeighbors` calls `Terrain.SetNeighbors`; Unity's own filter is engine code), so they aren't a port | Record it in `COMPATIBILITY.md` (or under M2e in `docs/VERIFICATION.md`) |
| m5 | Minor | The seam test covers tiles (0,0), (1,0) and (0,1) only; the shipped maps have negative coordinates | Add a 2x2 case with (-1,-1), (0,-1), (-1,0) and (0,0), checking the shared corner and the one-sided difference at the landscape's end |
| m6 | Minor | `graphify query`, `explain` and `path` write `graphify-out/cache/last_query_stamp`, and the hook writes `cache/hook_sessions/*.nudges`, so "read-only" agents do write into the tree (harmless now that the folder is ignored) | Say in `docs/MULTI_AGENT_WORKFLOW.md` that read-only agents query a scratch copy with `--graph` |
| m7 | Minor | `.claude/skills/graphify/` (148 KB) is graphify 0.9.101, Apache-2.0 with MIT parts; committing it needs its license and NOTICE files and sits badly with the rule against third-party assets. `SKILL.md.bak` and `.claude/settings.json.graphify-bak` are leftovers | Don't commit the skill (it reinstalls with `graphify install`), nor `.claude/CLAUDE.md`; drop both backups |
| m8 | Minor | Without `graphify-out` nothing breaks, but `CLAUDE.md:52` ("A committed graphify-out/graph.json ...") no longer applies | Delete that line; add a fallback when graphify is missing (skip the graph steps, run the whole suite) to the workflow and the recovery checklist |
| m9 | Minor | `tools/graph_impact.py:5` documents `--all-files`, which its parser rejects; leaving out `--type` already searches every file | Fix the docstring |
| m10 | Minor | `landscape_height` scans all tiles per edge sample: negligible on 64 tiles, about 10^9 checks for a crafted 1,000-tile level | See "Conflicts": performance measured the alternative |
| n1-n6 | Nits | V2 table row should read `NullReferenceException()`; a doc line in `pinewatch-assets/tests/load.rs:184` sits on the wrong test (from `4da03ec`); `load.rs:405` throws earlier than the C# for a root path only; the staged `boot.rs` was LF (now CRLF); `CLAUDE.md`'s graphify lines pass 120 columns and `settings.json` has no final newline; `coord as isize * 256` can overflow on 32-bit targets only | Fix with the related changes |

QA's verdicts: the core patch is correct (its own model of the Rust code agrees with the shipped corlib's dump on all
65,536 code units, and the table equals the corefx C# model); the app patch is correct and well tested for the
viewer (0 mismatched shared edge samples over 160 seams on PEI, Washington and Russia; the seam line in the
screenshots is gone, and the thin outline left is a ledge in the heightmap); the V2/V30 change is correct (all 11
production `null_reference()` sites port runtime null dereferences, and the SDK runtime has no explicit
`NullReferenceException` or `OverflowException` throw); the three commits are correct apart from m1 and n2, and use the
Zeph noreply identity with no AI trailers; robustness: no panic, overflow or unbounded allocation found, no new
`unsafe`, no writes.

## Architecture findings

| Severity | Finding | Proposed change |
|---|---|---|
| High | The graph has almost no cross-crate method calls (1 `calls` edge into 446 production methods; `pinewatch-level` calls `KvtCursor` methods 26 times with no edge), and items declared through `net_enum!` (`ENPCHoliday`, `ELevelSize`, `ELevelType`, and the core patch's `UnicodeCategory`) have no node | Add both gaps to the Graphify protocol; confirm every impact query with a text search |
| Medium | The only `SHA1Stream` port is private (`HashedBytes`, `pinewatch-level/src/landscape.rs:130`); M2f task 7 needs it in `pinewatch-assets`, which can't depend on level | Move it to `pinewatch-io` as a public type before task 7 |
| Medium | The spec's crate table (spec lines 61-62) is out of date: level depends on assets, and assets on binary, as the M2f plan chose. A dev-only cycle exists: mono -[dev]-> io -> mono | Record the real layering and its reason in the spec or `docs/ARCHITECTURE.md` |
| Medium | No Rust file is touched by two pending changes; `NOTICES-PORTED-CODE.md` and `docs/VERIFICATION.md` are. The core patch needs no `lib.rs` re-export | Land V2/V30 before M2f task 3, which also edits `exception.rs` |
| Medium | The app patch rebuilds landscape logic in the app crate: `Landscape.getTile` (a dictionary lookup) and `linkNeighbors` aren't ported, and `view_target` (`terrain.rs:115-116`) truncates where `HeightmapCoord` rounds and clamps | Port `getTile`, `LandscapeCoord(Vector3)` and `HeightmapCoord` into `pinewatch-level` later |
| Medium | `Assets.linkSpawns` (called at `Assets.cs:2374`, with observable `ReportError` messages) is unported and unrecorded | Add a V entry |
| Low | Duplications: `Substring` and `ToLowerInvariant` on `&str` in `assets/load.rs:636,648` (a lone surrogate becomes U+FFFD); `IndexOf(char)` twice; `io::Error` to `IOException` in four places (V12); `MapPaths` duplicates `MonoPath`; a 73-line PE reader copied between two test files | Move them to `pinewatch-mono` and `pinewatch-io` helpers |
| Low | `MonoException::overflow()` has no production caller after V2/V30 | Keep it; note it |

Translation map: 632 of 1,153 production items (55%) name the C# member they port; per crate, binary and unity are
nearly complete and the app has none. The top unported callees for M2f are Json.NET's
`JsonConvert.DeserializeObject<LevelInfoConfigData>` (94 methods), `LevelInfoConfigData`'s defaults, `SHA1Stream`,
the full `LevelInfo`, and `ReadWrite`'s file helpers; for M2b, the logging and `ReadWrite` folder calls the harness must
capture, including `createFolder`, which writes and so must be redirected (L3). The full lists are in
`S\agents\architecture\translation_map.csv` and `unported_callees.csv`.

## Performance findings

Measured in release, warm file cache, n = 7 unless noted (medians):

| Phase | Time |
|---|---|
| `load_client_assets` (6,114 assets) | 1,487 ms: the folder walk 888, the reader 568, `take_results` 42 |
| `read_landscape` | PEI 38 ms, Washington 38, Germany 92, Russia 164 |
| Level discovery | under 1.1 ms |
| `.dat` parsing from memory | 24 ms for 2 MB (83 MB/s) |
| App start to window (`--map PEI --screenshot`, n = 5) | 1.92 s; assets loaded at 1.50 s, so asset loading is about 78% of it; peak 508 MiB |

Prototypes in a scratch copy, each with output identical to today's (the loader's full state, the message log, and all
10 maps' tiles and messages):

| Variant | Walk | Reader | End to end |
|---|---|---|---|
| Today | 888 | 568 | 1,487 |
| File types from the listing ("types") | 496 | 559 | 1,122 |
| Parallel reader, results in item order | 876 | 119 | 1,036 |
| Parallel walk per breadth-first level ("pwalk") | 181 | 576 | 799 |
| All of these, plus the repeated `File.Exists` | 95 | 121 | **252 (-83%)** |

1. **The folder walk** (`worker.rs:33-54`, `paths.rs:68-71,89`): take each entry's kind from the listing (keep the
   extra check for links) and walk each breadth-first level on worker threads, joined in queue order. Low 1:1 risk;
   guard with `tests/walk.rs`, a serial-equals-parallel install test, and temp installs with a junction, a broken link,
   a folder named `X.asset` and an unreadable folder.
2. **The reader** (`reader.rs:56-82`): worker threads with one `DatParser` each, results in item order; serial when
   `-ParseAssetMetadata` is on. Low risk; guard with the equality test and broken-input installs.
3. **CPU fixes, estimated, not measured:** `fold_case` allocates per lookup (`pinewatch-dat/src/tree.rs:78-84`,
   10-20 ms); the KVT reader clones its builder and hashes keys twice (5-45 ms per map); tile decoding byte by byte
   (about 5 ms).
4. **Not recommended:** answering `File.Exists` from the listing (NTFS case folding, 8.3 names and trailing-dot
   trimming make it risky for 49 ms).
5. **Memory:** the app keeps every asset alive for the whole run (`app.rs:67-97`); dropping them before Bevy starts
   would free an estimated 80 MiB.

The seam patch costs about 1.2 ms on 16 tiles and 8 ms on 64. A `HashMap` index gave identical normals but was slower
(9.5 ms on 16 tiles, 40.6 ms on 64).

## Conflicts and disagreements

- **Files:** no Rust file is touched by two pending changes. Two docs are shared: `NOTICES-PORTED-CODE.md` (V2/V30's
  lines 134-138, and the core patch's row 23 plus a new row) and `docs/VERIFICATION.md` (V2/V30's moved entries, the
  V3 text). Doing all the notice edits in one pass avoids conflicts.
- **Order:** V2/V30 must land before M2f task 3, which also changes `pinewatch-mono/src/exception.rs`.
- **The neighbor lookup:** QA proposed a `HashMap` (m10); performance measured it slower than today's scan on real
  maps. Recommendation: keep the scan; if crafted large levels matter, use a dense grid indexed by coordinate, or the
  ported `Landscape.getTile` the architecture review proposes.
- **Line endings:** the app agent's `boot.rs` was LF in a CRLF checkout. The staged copy is now CRLF; git normalizes on
  commit either way.

## Orchestrator's notes on the patches

- **Core patch:** additive inside `pinewatch-mono`; the header keeps the provenance and license lines it rewrites; no
  `lib.rs` change is needed. Independently recomputed from the recorded dump: `char.GetUnicodeCategory` checksums to
  `0x5cd45c712e21a2e5`, the patch's `CharUnicodeInfo` table to the printed `0x4280054dea7fbc49`, and they differ at
  exactly U+00A7, U+00AA, U+00AD, U+00B6 and U+00BA. Needs m2 before it lands.
- **App patch:** replaces Bevy's per-tile `compute_smooth_normals` with central differences that read neighbor tiles
  past each edge. Interior normals change slightly too, not only the seams. The one caller, `spawn_terrain`, and three
  tests are updated. Its one line over 120 columns (`terrain.rs:158`) was already there. Needs m4 and m5 with it.
- The repository doesn't use rustfmt (the existing files don't follow its defaults), so formatting is by hand within 120
  columns.

## Remaining risks

- **The hooks (M1):** committing `.claude/settings.json` as it is puts a hook error on every tool call in cloud
  sessions.
- **Third-party skill (m7):** committing it without its licenses would break the rule against third-party assets.
- **Not proven 1:1:** the viewer's normals (Unity engine code); `CharUnicodeInfo` per code unit (a 64-bit checksum
  only); V17's IL chain (behavior run only, no IL dump).
- **Graph gaps:** cross-crate method calls, calls through struct fields, `net_enum!` items and many test links are
  missing, so graph impact lists are incomplete. The whole test suite stays the real check. The hosted index is still
  at `main` (`480c1ca`) until `main` moves.
- **Unpushed and temporary:** `integration/m2a-m2c`'s 3 commits exist only on the PC, and the patches' originals and
  the shipped-assembly evidence live in temp folders (copies of both patches are next to this report).
- **Usage limits:** parallel agents hit the 5-hour limit twice; the checkpoint rule kept their work both times.

## Recommended next steps

Each needs the owner's approval; nothing is committed or pushed until then.

1. **Fix the workflow setup:** move the hooks to `.claude/settings.local.json` (M1); leave the skill and
   `.claude/CLAUDE.md` uncommitted and drop the two backups (m7); delete `CLAUDE.md:52` and add the no-graphify fallback
   (m8); document the graph gaps and the scratch-copy rule in `docs/MULTI_AGENT_WORKFLOW.md` (architecture High, m6);
   fix `graph_impact.py`'s docstring (m9).
2. **Fix the docs:** the notices (m2, m3), the V2 row (n1), the misplaced doc line in `load.rs` (n2).
3. **Apply the core patch** with its notice rows, the generator in `reference/checks/`, and V3's mention of
   `tests/categories.rs`.
4. **Apply the app patch** with the 2x2 negative-coordinate test (m5) and the `COMPATIBILITY.md` entry (m4).
5. **Verify:** `cargo test --workspace`, `cargo clippy --workspace --all-targets`, `cargo xtask check-deps`, `graphify
   update .`, and a release run of the viewer on PEI, Washington and Russia with screenshots.
6. **Commit, one per change:** V2/V30; the workflow and Graphify setup with `.gitignore`; the core patch; the app patch;
   the docs (recovery record, this review, the HANDOFF refresh). Then fast-forward `main` and push, which also lets the
   hosted Graphify index catch up.
7. **Then:** M2f task 3 (the Json.NET reader), moving `SHA1Stream` to `pinewatch-io` before task 7; the loading speed-up
   (types, parallel walk, parallel reader) as its own task, test first with the equality test; a V entry for
   `Assets.linkSpawns`; the spec's crate table.

## Changes made in this round (all uncommitted)

- `.gitignore`: ignores `graphify-out/`.
- `docs/VERIFICATION.md`: the V3 wording fix.
- `docs/RECOVERY.md`: the review round's state.
- `docs/reviews/2026-11-05/`: this report and the two review patches.

## Appendix A: the app patch (seam normals), in full

`app-seam-normals.patch`, next to this report: the review diff from the shared tree to the app agent's
tested files.

```diff
diff --git a/crates/pinewatch/src/terrain.rs b/crates/pinewatch/src/terrain.rs
index d3bca3e..d97eae1 100644
--- a/crates/pinewatch/src/terrain.rs
+++ b/crates/pinewatch/src/terrain.rs
@@ -55,14 +55,57 @@ fn placeholder_color(tile: &LandscapeTile, row: usize, column: usize) -> [f32; 4
     [rgb[0], rgb[1], rgb[2], 1.0]
 }
 
-/// One tile as a triangle mesh in Bevy space, facing up (+Y).
-pub fn build_tile_mesh(tile: &LandscapeTile) -> Mesh {
+/// The landscape's height (normalized, as in the heightmap) at `row`, `column` counted from `tile`'s first sample.
+/// One step past the tile's edges the sample comes from whichever tile in `landscape` holds it (a diagonal one at the
+/// corners); `None` where the landscape ends.
+fn landscape_height(tile: &LandscapeTile, landscape: &[LandscapeTile], row: isize, column: isize) -> Option<f32> {
+    let last = (HEIGHTMAP_RESOLUTION - 1) as isize;
+    if (0..=last).contains(&row) && (0..=last).contains(&column) {
+        return Some(tile.heightmap[row as usize][column as usize]);
+    }
+    let global_row = tile.coord.y as isize * last + row;
+    let global_column = tile.coord.x as isize * last + column;
+    landscape.iter().find_map(|other| {
+        let row = global_row - other.coord.y as isize * last;
+        let column = global_column - other.coord.x as isize * last;
+        ((0..=last).contains(&row) && (0..=last).contains(&column))
+            .then(|| other.heightmap[row as usize][column as usize])
+    })
+}
+
+/// Unity-space normal at heightmap sample `(row, column)`, from central differences of the landscape's heights. Both
+/// tiles of a seam see the same samples there, so their shared vertices get the same normal and the seam doesn't
+/// show; where the landscape ends the difference is one-sided.
+fn tile_normal_unity(tile: &LandscapeTile, landscape: &[LandscapeTile], row: usize, column: usize) -> Vec3 {
+    let center = tile.heightmap[row][column];
+    let (row, column) = (row as isize, column as isize);
+    let height = |row, column| landscape_height(tile, landscape, row, column);
+    // Meters of height per meter between the samples before and after this one.
+    let slope = |before: Option<f32>, after: Option<f32>| {
+        let (low, high, cells) = match (before, after) {
+            (Some(before), Some(after)) => (before, after, 2.0),
+            (None, Some(after)) => (center, after, 1.0),
+            (Some(before), None) => (before, center, 1.0),
+            (None, None) => return 0.0,
+        };
+        (high - low) * TILE_HEIGHT / (cells * CELL)
+    };
+    let along_x = slope(height(row, column - 1), height(row, column + 1));
+    let along_z = slope(height(row - 1, column), height(row + 1, column));
+    Vec3::new(-along_x, 1.0, -along_z).normalize()
+}
+
+/// One tile as a triangle mesh in Bevy space, facing up (+Y). `landscape` holds the map's tiles (it may include
+/// `tile` itself): their samples next to `tile`'s edges give the edge vertices the same normals as in the neighbors.
+pub fn build_tile_mesh(tile: &LandscapeTile, landscape: &[LandscapeTile]) -> Mesh {
     let n = HEIGHTMAP_RESOLUTION;
     let mut positions = Vec::with_capacity(n * n);
+    let mut normals = Vec::with_capacity(n * n);
     let mut colors = Vec::with_capacity(n * n);
     for row in 0..n {
         for column in 0..n {
             positions.push(unity_to_bevy(tile_vertex_unity(tile, row, column)).to_array());
+            normals.push(unity_to_bevy(tile_normal_unity(tile, landscape, row, column).to_array()).to_array());
             colors.push(placeholder_color(tile, row, column));
         }
     }
@@ -81,9 +124,9 @@ pub fn build_tile_mesh(tile: &LandscapeTile) -> Mesh {
 
     let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
     mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
+    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
     mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
     mesh.insert_indices(Indices::U32(indices));
-    mesh.compute_smooth_normals();
     mesh
 }
 
@@ -145,7 +188,7 @@ fn spawn_terrain(
     });
     for tile in &terrain.tiles {
         commands.spawn((
-            Mesh3d(meshes.add(build_tile_mesh(tile))),
+            Mesh3d(meshes.add(build_tile_mesh(tile, &terrain.tiles))),
             MeshMaterial3d(material.clone()),
             Transform::IDENTITY,
             Name::new(format!("Terrain {}", tile.coord)),
diff --git a/crates/pinewatch/tests/boot.rs b/crates/pinewatch/tests/boot.rs
index 752de83..e6cef86 100644
--- a/crates/pinewatch/tests/boot.rs
+++ b/crates/pinewatch/tests/boot.rs
@@ -27,7 +27,7 @@ fn unity_to_bevy_negates_z() {
 #[test]
 fn flat_half_height_tile_sits_at_zero_and_spans_one_tile() {
     let tile = LandscapeTile::new(LandscapeCoord { x: 1, y: -2 });
-    let mesh = build_tile_mesh(&tile);
+    let mesh = build_tile_mesh(&tile, &[]);
     let p = positions(&mesh);
     assert_eq!(p.len(), 257 * 257);
     assert!(p.iter().all(|v| v[1] == 0.0));
@@ -42,7 +42,7 @@ fn heightmap_rows_follow_unity_z_and_columns_follow_unity_x() {
     let mut tile = LandscapeTile::new(LandscapeCoord { x: 0, y: 0 });
     tile.heightmap[3][5] = 1.0;
     tile.heightmap[4][5] = 0.0;
-    let mesh = build_tile_mesh(&tile);
+    let mesh = build_tile_mesh(&tile, &[]);
     let p = positions(&mesh);
     assert_eq!(p[3 * 257 + 5], [20.0, 1024.0, -12.0]);
     assert_eq!(p[4 * 257 + 5], [20.0, -1024.0, -16.0]);
@@ -50,10 +50,58 @@ fn heightmap_rows_follow_unity_z_and_columns_follow_unity_x() {
 
 #[test]
 fn flat_tile_normals_point_up() {
-    let mesh = build_tile_mesh(&LandscapeTile::new(LandscapeCoord { x: 0, y: 0 }));
+    let mesh = build_tile_mesh(&LandscapeTile::new(LandscapeCoord { x: 0, y: 0 }), &[]);
     assert!(normals(&mesh).iter().all(|n| (n[1] - 1.0).abs() < 1e-5 && n[0].abs() < 1e-5 && n[2].abs() < 1e-5));
 }
 
+/// A tile whose heights (normalized, as in the heightmap) come from `height(unity_x, unity_z)`, so neighbors built
+/// from the same function share their edge samples exactly.
+fn tile_from(x: i32, y: i32, height: impl Fn(f32, f32) -> f32) -> LandscapeTile {
+    let mut tile = LandscapeTile::new(LandscapeCoord { x, y });
+    for (row, samples) in tile.heightmap.iter_mut().enumerate() {
+        for (column, sample) in samples.iter_mut().enumerate() {
+            *sample = height(x as f32 * 1024.0 + column as f32 * 4.0, y as f32 * 1024.0 + row as f32 * 4.0);
+        }
+    }
+    tile
+}
+
+fn hills(x: f32, z: f32) -> f32 {
+    0.5 + 0.01 * (x / 50.0).sin() * (z / 70.0).cos()
+}
+
+fn assert_same_normal(a: [f32; 3], b: [f32; 3], at: &str) {
+    let angle = Vec3::from(a).angle_between(Vec3::from(b));
+    assert!(angle < 1e-4, "{at}: {a:?} vs {b:?} ({angle} rad apart)");
+}
+
+#[test]
+fn neighboring_tiles_share_their_edge_normals() {
+    // Tile (0, 0), its east neighbor (1, 0) and its north neighbor (0, 1), with the same hills across both seams.
+    let landscape = vec![tile_from(0, 0, hills), tile_from(1, 0, hills), tile_from(0, 1, hills)];
+    let meshes: Vec<Mesh> = landscape.iter().map(|tile| build_tile_mesh(tile, &landscape)).collect();
+    let (origin, east, north) = (normals(&meshes[0]), normals(&meshes[1]), normals(&meshes[2]));
+    for i in 0..257 {
+        // Column 256 of (0, 0) is column 0 of (1, 0); row 256 of (0, 0) is row 0 of (0, 1).
+        assert_same_normal(origin[i * 257 + 256], east[i * 257], &format!("east seam, row {i}"));
+        assert_same_normal(origin[256 * 257 + i], north[i], &format!("north seam, column {i}"));
+    }
+}
+
+#[test]
+fn normals_lean_away_from_the_uphill_side() {
+    // Rising 1 m every 4 m toward Unity +x, then toward Unity +z (Bevy -z), up to the tile's edges.
+    let slope = 0.25;
+    let along_x = build_tile_mesh(&tile_from(0, 0, |x, _| 0.25 + x * slope / 2048.0), &[]);
+    let along_z = build_tile_mesh(&tile_from(0, 0, |_, z| 0.25 + z * slope / 2048.0), &[]);
+    let expected_x = Vec3::new(-slope, 1.0, 0.0).normalize().to_array();
+    let expected_z = Vec3::new(0.0, 1.0, slope).normalize().to_array();
+    for (i, (x, z)) in normals(&along_x).iter().zip(normals(&along_z)).enumerate() {
+        assert_same_normal(*x, expected_x, &format!("slope along x, vertex {i}"));
+        assert_same_normal(*z, expected_z, &format!("slope along z, vertex {i}"));
+    }
+}
+
 fn args(list: &[&str]) -> Result<Cli, String> {
     parse_args(list.iter().map(std::ffi::OsString::from))
 }
```

## Appendix B: the core patch (Unicode categories)

`core-unicode-categories.patch`, next to this report. The generated `category_table.rs` (394 lines: 2,864 runs
and the 256-entry Latin-1 table) is cut after its header here.

```diff
diff --git a/crates/pinewatch-mono/src/chars/category_table.rs b/crates/pinewatch-mono/src/chars/category_table.rs
new file mode 100644
index 0000000..20c9473
--- /dev/null
+++ b/crates/pinewatch-mono/src/chars/category_table.rs
@@ -0,0 +1,394 @@
+// Generated from corefx's `CharUnicodeInfo.GetUnicodeCategory(int)` and `InternalGetCategoryValue` with the
+// category tables of `src/Common/src/CoreLib/System/Globalization/CharUnicodeInfoData.cs`, and from
+// `s_categoryForLatin1` in `src/Common/src/CoreLib/System/Char.cs` (mono/corefx 5b4edb9b). Copyright (c) .NET
+// Foundation and Contributors. MIT license; see NOTICES-PORTED-CODE.md.
+
+//! The Unicode category of every UTF-16 code unit, as corefx's tables give it.
+//!
+//! `CharUnicodeInfoData` holds the categories of all code points in an 11:5:4 three-level table, which
+//! `InternalGetCategoryValue` reads (the second level as little-endian `ushort`s). `CATEGORY_RUNS` holds the
+//! result for U+0000 to U+FFFF, a lone surrogate included, as runs of equal categories.
+//! `CATEGORY_FOR_LATIN1` is `char`'s own table for U+0000 to U+00FF.
+
+/// `CharUnicodeInfo.GetUnicodeCategory(char)` as runs, sorted by code unit: each entry is the first code unit of
+/// a run and the `UnicodeCategory` value of every code unit up to the next entry. The first run starts at U+0000.
+pub(super) const CATEGORY_RUNS: [(u16, u8); 2864] = [
+    (0x0000, 14), (0x0020, 11), (0x0021, 24), (0x0024, 26), (0x0025, 24), (0x0028, 20), (0x0029, 21), (0x002A, 24),
+    (0x002B, 25), (0x002C, 24), (0x002D, 19), (0x002E, 24), (0x0030,  8), (0x003A, 24), (0x003C, 25), (0x003F, 24),
+    (0x0041,  0), (0x005B, 20), (0x005C, 24), (0x005D, 21), (0x005E, 27), (0x005F, 18), (0x0060, 27), (0x0061,  1),
+    (0x007B, 20), (0x007C, 25), (0x007D, 21), (0x007E, 25), (0x007F, 14), (0x00A0, 11), (0x00A1, 24), (0x00A2, 26),
+    (0x00A6, 28), (0x00A7, 24), (0x00A8, 27), (0x00A9, 28), (0x00AA,  4), (0x00AB, 22), (0x00AC, 25), (0x00AD, 15),
+... (374 more generated table lines; the full patch is core-unicode-categories.patch)
diff --git a/crates/pinewatch-mono/src/chars.rs b/crates/pinewatch-mono/src/chars.rs
index e727881..63a4784 100644
--- a/crates/pinewatch-mono/src/chars.rs
+++ b/crates/pinewatch-mono/src/chars.rs
@@ -1,5 +1,7 @@
-// The Latin-1 whitespace rule is ported from corefx `src/Common/src/CoreLib/System/Char.cs` (mono/corefx 5b4edb9b).
-// Copyright (c) .NET Foundation and Contributors. MIT license; see NOTICES-PORTED-CODE.md.
+// The Latin-1 rules, `GetUnicodeCategory`, `IsNumber` and `IsLetterOrDigit` are ported from corefx
+// `src/Common/src/CoreLib/System/Char.cs`, and `CharUnicodeInfo.GetUnicodeCategory` from
+// `src/Common/src/CoreLib/System/Globalization/CharUnicodeInfo.cs` (mono/corefx 5b4edb9b). Copyright (c) .NET
+// Foundation and Contributors. MIT license; see NOTICES-PORTED-CODE.md.
 // The case mapping follows `TextInfo.ToUpper(char)` and `ToLower(char)` in reference source `textinfo.cs` (MONO
 // branch) and Mono's `ReferenceSources/TextInfo.cs` (Unity mono b8eb56f). MIT license; see NOTICES-PORTED-CODE.md.
 
@@ -7,14 +9,61 @@
 //!
 //! - `IsWhiteSpace` follows corefx: `IsWhiteSpaceLatin1` up to U+00FF, then the space, line and paragraph separator
 //!   categories of `CharUnicodeInfoData` (mono/corefx 5b4edb9b), listed here.
+//! - `GetUnicodeCategory`, `IsNumber` and `IsLetterOrDigit` follow corefx too: `char`'s own table up to U+00FF, then
+//!   `CharUnicodeInfo.GetUnicodeCategory`, whose results `chars/category_table.rs` lists. That is corefx's Unicode
+//!   data, not Rust's: Rust's `char` methods know later Unicode versions.
 //! - `ToUpperInvariant` and `ToLowerInvariant` are `CultureInfo.InvariantCulture.TextInfo.ToUpper` and `ToLower`:
 //!   ASCII is folded directly, everything else comes from Mono's tables (`case_tables.rs`).
 //!
-//! These are ported from source; the harness confirms them against the shipped `mscorlib.dll`
+//! These are ported from source, and the shipped `mscorlib.dll` gives the same results for every code unit
 //! (`docs/VERIFICATION.md`, V3).
 
+mod category_table;
+
 use crate::case_tables::{TO_LOWER, TO_UPPER};
 
+mod unicode_category {
+    #![allow(non_upper_case_globals)] // .NET's member names, which `Enum.TryParse` matches
+
+    crate::net_enum! {
+        /// `System.Globalization.UnicodeCategory` (corefx `Globalization/UnicodeCategory.cs`).
+        pub struct UnicodeCategory(i32) {
+            UppercaseLetter = 0,
+            LowercaseLetter = 1,
+            TitlecaseLetter = 2,
+            ModifierLetter = 3,
+            OtherLetter = 4,
+            NonSpacingMark = 5,
+            SpacingCombiningMark = 6,
+            EnclosingMark = 7,
+            DecimalDigitNumber = 8,
+            LetterNumber = 9,
+            OtherNumber = 10,
+            SpaceSeparator = 11,
+            LineSeparator = 12,
+            ParagraphSeparator = 13,
+            Control = 14,
+            Format = 15,
+            Surrogate = 16,
+            PrivateUse = 17,
+            ConnectorPunctuation = 18,
+            DashPunctuation = 19,
+            OpenPunctuation = 20,
+            ClosePunctuation = 21,
+            InitialQuotePunctuation = 22,
+            FinalQuotePunctuation = 23,
+            OtherPunctuation = 24,
+            MathSymbol = 25,
+            CurrencySymbol = 26,
+            ModifierSymbol = 27,
+            OtherSymbol = 28,
+            OtherNotAssigned = 29,
+        }
+    }
+}
+
+pub use unicode_category::UnicodeCategory;
+
 /// `char.IsWhiteSpace`.
 pub fn is_white_space(c: u16) -> bool {
     if c < 0x100 {
@@ -26,6 +75,68 @@ pub fn is_white_space(c: u16) -> bool {
     }
 }
 
+/// `CharUnicodeInfo.GetUnicodeCategory(char)`: the category in corefx's Unicode data (`CharUnicodeInfoData`). A
+/// lone surrogate is `Surrogate`; an unassigned code unit or a noncharacter is `OtherNotAssigned`.
+pub fn char_unicode_info_category(c: u16) -> UnicodeCategory {
+    let runs = &category_table::CATEGORY_RUNS;
+    // The first run starts at U+0000, so `c` is in the last run that starts at or before it.
+    let run = runs.partition_point(|&(start, _)| start <= c) - 1;
+    UnicodeCategory(i32::from(runs[run].1))
+}
+
+/// `GetLatin1UnicodeCategory`: `char`'s own table, `s_categoryForLatin1`, for U+0000 to U+00FF. It differs from
+/// `CharUnicodeInfo` for five code units: `§` and `¶` are `OtherSymbol` here, `ª` and `º` `LowercaseLetter`, and the
+/// soft hyphen U+00AD `DashPunctuation`.
+fn latin1_category(c: u16) -> UnicodeCategory {
+    UnicodeCategory(i32::from(category_table::CATEGORY_FOR_LATIN1[usize::from(c)]))
+}
+
+/// `char.GetUnicodeCategory(char)`: `char`'s own table up to U+00FF, then `CharUnicodeInfo.GetUnicodeCategory`.
+pub fn get_unicode_category(c: u16) -> UnicodeCategory {
+    if c < 0x100 { latin1_category(c) } else { char_unicode_info_category(c) }
+}
+
+/// `CheckNumber`.
+fn check_number(category: UnicodeCategory) -> bool {
+    matches!(
+        category,
+        UnicodeCategory::DecimalDigitNumber | UnicodeCategory::LetterNumber | UnicodeCategory::OtherNumber
+    )
+}
+
+/// `CheckLetterOrDigit`.
+fn check_letter_or_digit(category: UnicodeCategory) -> bool {
+    matches!(
+        category,
+        UnicodeCategory::UppercaseLetter
+            | UnicodeCategory::LowercaseLetter
+            | UnicodeCategory::TitlecaseLetter
+            | UnicodeCategory::ModifierLetter
+            | UnicodeCategory::OtherLetter
+            | UnicodeCategory::DecimalDigitNumber
+    )
+}
+
+/// `char.IsNumber`: an ASCII digit, or any character in a number category (`DecimalDigitNumber`, `LetterNumber`,
+/// `OtherNumber`), so `²`, `½` and `Ⅰ` are numbers too.
+pub fn is_number(c: u16) -> bool {
+    if c < 0x80 {
+        // `IsAscii`.
+        (0x30..=0x39).contains(&c)
+    } else if c < 0x100 {
+        check_number(latin1_category(c))
+    } else {
+        check_number(char_unicode_info_category(c))
+    }
+}
+
+/// `char.IsLetterOrDigit`: any character in a letter category or `DecimalDigitNumber`.
+pub fn is_letter_or_digit(c: u16) -> bool {
+    // `IsLatin1(c) ? GetLatin1UnicodeCategory(c) : CharUnicodeInfo.GetUnicodeCategory(c)`, which is
+    // `GetUnicodeCategory`.
+    check_letter_or_digit(get_unicode_category(c))
+}
+
 /// The table's mapping for `c`, or `c` itself.
 fn mapped(table: &[(u16, u16)], c: u16) -> u16 {
     table.binary_search_by_key(&c, |&(from, _)| from).map_or(c, |index| table[index].1)
diff --git a/crates/pinewatch-mono/tests/categories.rs b/crates/pinewatch-mono/tests/categories.rs
new file mode 100644
index 0000000..8a2eb78
--- /dev/null
+++ b/crates/pinewatch-mono/tests/categories.rs
@@ -0,0 +1,148 @@
+//! `CharUnicodeInfo.GetUnicodeCategory`, `char.GetUnicodeCategory`, `char.IsNumber` and `char.IsLetterOrDigit` for
+//! every UTF-16 code unit, as the shipped `mscorlib.dll` computes them (`docs/VERIFICATION.md`, V3).
+//!
+//! The pinned values come from `reference/checks/chardump.cs` run on the shipped corlib (game 3.26.3.13, 2026-11-03).
+//! It prints the checksum of `CharUnicodeInfo.GetUnicodeCategory` itself. The other three are [`checksum`] over its
+//! dump: each record's category byte (`char.GetUnicodeCategory`) and its flags 8 (`IsNumber`) and 64
+//! (`IsLetterOrDigit`), as 1 or 0. The rows below are the dump's values too. A model of the C# (mono/corefx
+//! `5b4edb9b`), run on the source independently of the Rust table, gives the same values.
+
+use pinewatch_mono::chars::UnicodeCategory as Category;
+use pinewatch_mono::chars::{
+    char_unicode_info_category, get_unicode_category, is_letter_or_digit, is_number, is_white_space,
+};
+use pinewatch_mono::{EnumUnderlying, NetEnum};
+
+/// FNV-1a (64 bits) over a value for every code unit, each as two little-endian bytes (as in `tests/chars.rs`).
+fn checksum(map: impl Fn(u16) -> u16) -> u64 {
+    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
+    for c in 0..=u16::MAX {
+        for byte in map(c).to_le_bytes() {
+            hash ^= u64::from(byte);
+            hash = hash.wrapping_mul(0x0100_0000_01b3);
+        }
+    }
+    hash
+}
+
+fn number(category: Category) -> u16 {
+    u16::try_from(category.0).unwrap()
+}
+
+/// `UnicodeCategory`'s 30 members in .NET's order, `UppercaseLetter = 0` to `OtherNotAssigned = 29` (corefx
+/// `Globalization/UnicodeCategory.cs`).
+#[test]
+fn members_in_dotnet_order() {
+    let info = Category::INFO;
+    assert_eq!(
+        info.names,
+        [
+            "UppercaseLetter", "LowercaseLetter", "TitlecaseLetter", "ModifierLetter", "OtherLetter", "NonSpacingMark",
+            "SpacingCombiningMark", "EnclosingMark", "DecimalDigitNumber", "LetterNumber", "OtherNumber",
+            "SpaceSeparator", "LineSeparator", "ParagraphSeparator", "Control", "Format", "Surrogate", "PrivateUse",
+            "ConnectorPunctuation", "DashPunctuation", "OpenPunctuation", "ClosePunctuation",
+            "InitialQuotePunctuation", "FinalQuotePunctuation", "OtherPunctuation", "MathSymbol", "CurrencySymbol",
+            "ModifierSymbol", "OtherSymbol", "OtherNotAssigned",
+        ]
+    );
+    assert_eq!(info.values, (0..30).collect::<Vec<u64>>());
+    assert_eq!(info.underlying, EnumUnderlying::I32);
+    assert_eq!((Category::UppercaseLetter, Category::OtherNotAssigned), (Category(0), Category(29)));
+}
+
+#[test]
+fn whole_tables_match_the_shipped_corlib() {
+    assert_eq!(checksum(|c| number(char_unicode_info_category(c))), 0x4280_054d_ea7f_bc49);
+    assert_eq!(checksum(|c| number(get_unicode_category(c))), 0x5cd4_5c71_2e21_a2e5);
+    assert_eq!(checksum(|c| u16::from(is_number(c))), 0x1c7d_69a7_ff0f_4834);
+    assert_eq!(checksum(|c| u16::from(is_letter_or_digit(c))), 0xc59a_8dd0_5270_38ed);
+    assert_eq!((0..=u16::MAX).filter(|&c| is_number(c)).count(), 735);
+    assert_eq!((0..=u16::MAX).filter(|&c| is_letter_or_digit(c)).count(), 49_202);
+}
+
+/// What Json.NET's reader asks: `ParseValue` sends a `char.IsNumber` character to the number parser, and unquoted
+/// property names take `char.IsLetterOrDigit` characters (plus `_` and `$`, which it tests itself).
+#[test]
+fn rows_json_net_reads() {
+    for (c, category, numeric, letter_or_digit) in [
+        (0x0030, Category::DecimalDigitNumber, true, true), // 0
+        (0x0039, Category::DecimalDigitNumber, true, true), // 9
+        (0x0041, Category::UppercaseLetter, false, true),   // A
+        (0x005A, Category::UppercaseLetter, false, true),   // Z
+        (0x0061, Category::LowercaseLetter, false, true),   // a
+        (0x007A, Category::LowercaseLetter, false, true),   // z
+        (0x005F, Category::ConnectorPunctuation, false, false), // _
+        (0x0024, Category::CurrencySymbol, false, false),   // $
+        (0x002B, Category::MathSymbol, false, false),       // +
+        (0x002D, Category::DashPunctuation, false, false),  // -
+        (0x002E, Category::OtherPunctuation, false, false), // .
+        (0x00B2, Category::OtherNumber, true, false),       // ² SUPERSCRIPT TWO: a number, not a digit
+        (0x00BD, Category::OtherNumber, true, false),       // ½
+        (0x00B5, Category::LowercaseLetter, false, true),   // µ
+        (0x00C0, Category::UppercaseLetter, false, true),   // À
+        (0x00FF, Category::LowercaseLetter, false, true),   // ÿ
+        (0x01C5, Category::TitlecaseLetter, false, true),   // ǅ
+        (0x02B0, Category::ModifierLetter, false, true),    // ʰ
+        (0x0391, Category::UppercaseLetter, false, true),   // Α GREEK CAPITAL LETTER ALPHA
+        (0x05D0, Category::OtherLetter, false, true),       // א HEBREW LETTER ALEF
+        (0x4E00, Category::OtherLetter, false, true),       // 一
+        (0x0660, Category::DecimalDigitNumber, true, true), // ٠ ARABIC-INDIC DIGIT ZERO
+        (0x0669, Category::DecimalDigitNumber, true, true), // ٩
+        (0x0966, Category::DecimalDigitNumber, true, true), // ० DEVANAGARI DIGIT ZERO
+        (0xFF10, Category::DecimalDigitNumber, true, true), // ０ FULLWIDTH DIGIT ZERO
+        (0x2160, Category::LetterNumber, true, false),      // Ⅰ ROMAN NUMERAL ONE
+        (0x3007, Category::LetterNumber, true, false),      // 〇
+        (0x0300, Category::NonSpacingMark, false, false),   // COMBINING GRAVE ACCENT
+        (0x9FEF, Category::OtherLetter, false, true),       // the last CJK ideograph this data has
+        (0x9FF0, Category::OtherNotAssigned, false, false), // a letter only in later Unicode versions
+        (0x0378, Category::OtherNotAssigned, false, false), // unassigned
+        (0xFDD0, Category::OtherNotAssigned, false, false), // noncharacters
+        (0xFFFE, Category::OtherNotAssigned, false, false),
+        (0xFFFF, Category::OtherNotAssigned, false, false),
+        (0xD800, Category::Surrogate, false, false), // lone surrogates
+        (0xDBFF, Category::Surrogate, false, false),
+        (0xDC00, Category::Surrogate, false, false),
+        (0xDFFF, Category::Surrogate, false, false),
+        (0xE000, Category::PrivateUse, false, false),
+    ] {
+        assert_eq!(get_unicode_category(c), category, "U+{c:04X}");
+        assert_eq!(char_unicode_info_category(c), category, "U+{c:04X}");
+        assert_eq!(is_number(c), numeric, "U+{c:04X}");
+        assert_eq!(is_letter_or_digit(c), letter_or_digit, "U+{c:04X}");
+    }
+}
+
+/// Up to U+00FF, `char` reads its own table, `s_categoryForLatin1`, which differs from `CharUnicodeInfo`'s data for
+/// exactly five code units (`char` from the dump, `CharUnicodeInfo` from the source). `IsNumber` and
+/// `IsLetterOrDigit` give the same answers from either table.
+#[test]
+fn latin1_comes_from_chars_own_table() {
+    for (c, from_char, from_info) in [
+        (0x00A7, Category::OtherSymbol, Category::OtherPunctuation), // §
+        (0x00AA, Category::LowercaseLetter, Category::OtherLetter),  // ª
+        (0x00AD, Category::DashPunctuation, Category::Format),       // SOFT HYPHEN
+        (0x00B6, Category::OtherSymbol, Category::OtherPunctuation), // ¶
+        (0x00BA, Category::LowercaseLetter, Category::OtherLetter),  // º
+    ] {
+        assert_eq!(get_unicode_category(c), from_char, "U+{c:04X}");
+        assert_eq!(char_unicode_info_category(c), from_info, "U+{c:04X}");
+    }
+    let differing: Vec<u16> =
+        (0..=u16::MAX).filter(|&c| get_unicode_category(c) != char_unicode_info_category(c)).collect();
+    assert_eq!(differing, [0x00A7, 0x00AA, 0x00AD, 0x00B6, 0x00BA]);
+    assert!(is_letter_or_digit(0x00AA) && is_letter_or_digit(0x00BA));
+    assert!(!is_number(0x00AA) && !is_letter_or_digit(0x00AD));
+}
+
+/// `char.IsWhiteSpace` above Latin-1 is `CharUnicodeInfo.IsWhiteSpace`: the space, line and paragraph separator
+/// categories. `chars.rs` lists those code units itself; the category table agrees.
+#[test]
+fn white_space_above_latin1_is_the_separator_categories() {
+    for c in 0x100..=u16::MAX {
+        let separator = matches!(
+            char_unicode_info_category(c),
+            Category::SpaceSeparator | Category::LineSeparator | Category::ParagraphSeparator
+        );
+        assert_eq!(is_white_space(c), separator, "U+{c:04X}");
+    }
+}
```
