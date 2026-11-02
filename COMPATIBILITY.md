# Compatibility rules

## The rule

The Rust implementation reproduces the original C# implementation 1:1 wherever behavior is observable.
Nothing is approximated, simplified or intentionally changed.

- The original C# code is used **only** as a reference in tests. All production code is Rust.
- When an exact match is impossible, the case gets an entry in the register below: what the original does,
  why it can't be matched, what Rust does instead, and how a test detects it.
- **A difference without a register entry is a bug.** Tests fail on it.

## What counts as observable

- Parsed values, defaults, and the results of every conversion
- Which files and assets load, fail or are skipped
- Error and log messages: exact text, order, and the point where each is produced
- Exceptions: type, message, where each is raised, and where it is caught
- Bytes read and consumed, including end-of-file behavior
- State carried between calls (shared buffers, reused parser objects)
- Collection iteration order
- Files written: names, locations, contents
- Hashes
- File-system side effects

## The reference

Approved 2026-10-31:

- **Unity 2022.3.62f3 is the reference environment.** The original C# code runs there, unmodified, for differential tests.
- **Unity's only roles are reference runtime and asset-processing environment.** No production game code runs in or depends on Unity.
  All production game code is Rust.
- **Target source version:** 3.26.3.12. Later versions are adopted deliberately, never silently (see L4).
- **The original game installation stays untouched** (see L3).

| Part | Reference |
|---|---|
| Game code | U3-SDK tag `v3.26.3.12` (commit `aba323a`, 2026-09-21), the newest published source |
| Runtime | Unity 2022.3.62f3 with its Mono runtime (`MonoBleedingEdge`) and class libraries (`Unturned_Data/Managed`) |
| JSON library | The game's own `Newtonsoft.Json.dll`. It is **not** the same build as the SDK's copy (SHA-1 `25c01768…` vs `a4ea782c…`) |
| Test data | A copy of the installed game files (version 3.26.3.13). The install itself is never opened for writing |

The Rust code reproduces the behavior of **that runtime**, not of modern .NET and not of Rust's standard library:
number parsing and formatting, culture data and how the current culture is chosen, UTF-8 decoding, BOM detection,
character tables (whitespace, case mapping), `Dictionary` iteration order (including slot reuse after removals),
`Guid`/`Enum`/`bool`/`DateTime` parsing, and exception types and messages.

Ported runtime behavior follows the source files that Unity's Mono actually compiles for the Windows player, as
its `unityjit` build lists name them. Those are mostly corefx and corert files, not Mono's own files or the
reference-source files of the same names. Ports are checked against the shipped `mscorlib.dll`
(`docs/VERIFICATION.md`, V0). Those sources are MIT-licensed. Each ported Rust file names its source in a header,
and the license notices are in `NOTICES-PORTED-CODE.md`.

## How equivalence is proven

- **Differential tests:** a test-only C# harness runs the original code unmodified, against the copied game files
  and generated inputs. It records every observable result in a canonical format. The Rust output must be byte-identical.
- **Exhaustive tables:** character classification and case mapping are compared for all 65,536 UTF-16 code units.
- **Generated edge cases:** number grammars, escape sequences, invalid UTF-8, binary files cut off at every byte, deep nesting.
- **Differential fuzzing:** random inputs run through both implementations and must produce identical output.
- **Ported tests:** the ~1,040 original NUnit cases also run as plain Rust tests.

## Register of known limitations

### L1. Stack traces in logged exceptions
- **Original:** some catch sites log the exception message followed by a C# stack trace
  (method names, IL offsets, assembly build IDs).
- **Why it can't match:** those frames describe compiled C# methods that don't exist in Rust.
- **Rust:** logs the identical message and exception type. The stack-trace lines are omitted.
- **Detection:** the harness compares everything up to the stack trace and checks that only that part differs.

### L2. Out-of-memory points for absurd sizes
- **Original:** a corrupt count makes it allocate as much as the file says, and it throws `OutOfMemoryException`
  only if the allocation fails.
- **Why it can't match:** whether a huge allocation fails depends on the machine's memory and the allocator.
- **Rust:** the same loop counts and the same allocation requests (no capping). It raises the same exception when
  an allocation fails. Deterministic cases (negative sizes, runtime size limits) match exactly.
- **Detection:** tests cover the deterministic cases. Large-but-possible sizes are documented, not tested.

### L3. The install is never written (your requirement)
- **Original:** the streamed reader opens files with write access and creates missing folders and files.
  The whole-file reader creates a missing parent folder even when only reading. The game keeps its saves inside its own folder.
- **Why it can't match:** matching it would mean modifying the install, which you ruled out.
- **Rust:** inside the install it opens files read-only and creates nothing. Observable differences:
  - (a) reading from a missing folder inside the install does not create that folder
  - (b) on a read-only install, or when another program blocks write access, the original fails to open the file and Rust doesn't
  - (c) Rust keeps saves in its own folder with byte-identical contents
  - (d) when the install has no `Sandbox` folder, the asset loader creates it; Rust doesn't, so no sandbox locations
    are searched in either case
  - (e) a definition without `GUID` (and without `Metadata`) gets a new random GUID that the asset loader writes
    into the file, logging `Assigned GUID <guid> to asset "<path>"`; Rust uses a new random GUID in memory, writes
    nothing and logs nothing, so the GUID changes on every run
  - Inside save folders, Rust reproduces the original's folder and file creation exactly.
- **Detection:** the read-only file handle has no write methods, and a test checks the install is unchanged after a full scan.

### L4. Installed game is newer than the published source
- **Original:** your install is 3.26.3.13. The newest published source is 3.26.3.12.
- **Why it can't match:** any code changes in 3.26.3.13 can't be seen until Smartly Dressed Games publishes them.
- **Rust:** targets 3.26.3.12 exactly. Real-file tests use the 3.26.3.13 data files.
- **Detection and follow-up:** on each new SDK tag, diff the reference files, update the Rust code, and re-run every differential test.

### L5. Output order across threads (later steps)
- **Original:** where messages come from different threads (asset loading workers, foliage worker), their relative order
  depends on timing. The original itself has no fixed order.
- **Rust:** matches the order within each thread and every ordering the original does guarantee.
  - Asset loading (M2c): the original searches and reads every search location at once, each with a searcher and a
    reader thread. Rust takes the locations in the order they were requested and finishes each before the next.
    Inside a location it keeps the searcher's order, places each searcher exception where the searcher raised it,
    and each read result where the reader takes the work item, which is one order the original can produce.
- **Detection:** comparisons are made per thread.

### L6. Engine-native math (to be measured)
- **Original:** some Unity math runs in native engine code, for example `Quaternion.Euler`.
- **Open question:** whether Rust can match it bit for bit on every platform. Step 1 parsers keep the raw stored values,
  so this first matters when map loading converts them.
- **Plan:** the harness measures each function. Exact matches need no entry. Any gap gets its own entry with the measured difference.

### L7. Stack overflow from extremely deep nesting
- **Original:** the `.dat` and hierarchy parsers recurse once per nesting level, so a deeply enough nested file overflows the stack.
  Depending on how Mono handles stack overflow on the platform, the game either crashes or raises an exception that the
  asset loader catches (skipping that asset). This is not yet established.
- **Why it can't match everywhere:** the depth at which it happens depends on Mono's stack frame sizes and the thread's stack size.
- **Rust:** the harness establishes the outcome and the exact depth on the reference platform. Rust then does the same thing
  (crash, or the same exception at the same point) at that same depth. Other platforms may differ.
- **Detection:** a test feeds increasing depths and compares where and how each implementation stops.

### L8. Image decoding for legacy terrain (to be measured)
- **Original:** legacy-ground maps load `Terrain/Heightmap.png` and `Terrain/AlphamapHQ_N.png` through Unity's native
  `Texture2D.LoadImage` and read pixels back with `GetPixel`.
- **Open question:** how Unity's decoder treats 16-bit, palette and gamma-tagged PNGs. That decides the exact pixel values.
- **Rust:** decodes with the `png` crate and must reproduce Unity's pixel values exactly. The harness measures this on
  generated PNGs covering every PNG color type and bit depth. Any gap gets its own entry. The official maps don't use legacy ground,
  but workshop maps can.

### L9. Navmesh reader in the shipped game
- **Original:** the shipped game reads `Navigation_N.dat` through `UnturnedPathfinding_ASPFP`, a wrapper around the third-party
  A* Pathfinding Project library. That code isn't in the SDK. The SDK build uses `UnturnedPathfinding_Empty`, which reads the same file layout.
- **Why it can't match fully:** the shipped reader's behavior can't be seen, especially on malformed files.
- **Rust:** matches the SDK's reader (`UnturnedPathfinding_Empty`) exactly, since that is what the reference runs.
  Pathfinding behavior itself (a later step) has the same gap and will get its own entry then.
- **Detection:** none possible against the shipped reader. Conformance covers the SDK's reader.

### L10. C# assembly mods (mod-loading step)
- **Original:** `*.module` manifests make the game load C# DLLs that are compiled against the original game's C# API.
- **Why it can't match:** those assemblies need the original C# types and a .NET runtime. Pinewatch has neither, by design (R1).
- **Rust:** will read the manifests exactly, but can't run the assemblies. Data-only mods (asset folders, maps) are unaffected.
- **Detection:** the mod-loading step will report every assembly it can't load.

### L11. No Steam workshop yet
- **Original:** the client's asset and level discovery includes the workshop items the player subscribes to
  (`AddClientUgcSearchLocations`, the workshop part of `ScanKnownLevels`).
- **Why it can't match yet:** Pinewatch has no Steam integration (M7 and later).
- **Rust:** behaves like a client with the workshop service running and no subscriptions: no workshop search
  locations or levels. `ScanKnownLevels`' branch for a missing workshop service (legacy server folders, which it
  creates) is never taken, as on any Steam client.
- **Detection:** reference runs use an account without subscriptions, or compare only non-workshop locations.

