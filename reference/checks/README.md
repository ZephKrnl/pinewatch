# Checks against the shipped game

Small programs that compare what Pinewatch ports with what the installed game's own files do, on the Windows PC.
They settle entries in `docs/VERIFICATION.md` until the reference harness (M2b) replaces them. They are not
production code and are not built by the workspace.

**Never run anything inside the install.** Copy what a check needs to a scratch folder first:

- `Unturned_Data/Managed/*.dll`: the game's class libraries and `Assembly-CSharp.dll`.
- `MonoBleedingEdge/EmbedRuntime/mono-2.0-bdwgc.dll`: the game's own runtime, 64-bit.

## Tools

- **Unity 2022.3.62f3 Editor's Mono** (`Editor/Data/MonoBleedingEdge`), the Unity version the game is built with
  (`UnityPlayer.dll` reports 2022.3.62f3, `96770f904ca7`). Its `bin/mono.exe` is **32-bit**, so it is not the
  game's runtime; use it to compile (`lib/mono/4.5/mcs.exe`) and disassemble (`lib/mono/4.5/ikdasm.exe`). With
  `MONO_PATH` set to the copied `Managed` folder it runs the game's class libraries (each program prints
  `typeof(object).Assembly.Location` to prove it). That is enough for managed code, not for what the runtime itself
  does.
- **`gamehost/`**, a dependency-free x64 Rust program that loads a copy of the game's `mono-2.0-bdwgc.dll` through
  the embedding API (`mono_jit_init_version`, `mono_jit_exec`) and runs a .NET program on it with the game's class
  libraries. Use it for anything the runtime decides: the 32-bit Editor runtime and the game's x64 runtime already
  differ for `new byte[-1]`.

  ```
  cargo build --release --manifest-path reference/checks/gamehost/Cargo.toml
  reference/checks/gamehost/target/release/gamehost.exe <copied mono-2.0-bdwgc.dll> <copied Managed dir> \
      "<install>/MonoBleedingEdge/etc" program.exe [args...]
  ```

## Checks

| Program | What it checks | Entry | Result (2026-11-03, game 3.26.3.13) |
|---|---|---|---|
| `chardump.cs` | `ToUpperInvariant`, `ToLowerInvariant`, `IsWhiteSpace` and `GetUnicodeCategory` for all 65,536 code units, with FNV-1a checksums in the form `crates/pinewatch-mono/tests/chars.rs` pins | V3 | Same checksums and counts as the port; the Editor's corlib gives a byte-identical dump |
| `ignorecase.cs` | `StringComparer.OrdinalIgnoreCase` equality and hashing against `ToUpperInvariant` folding | V17 | 327,678 pairs, no mismatches |
| `runtime_exceptions.cs` | Type and exact message of each exception the runtime raises (null dereference, casts, overflow, negative array size, indexes, division) | V2, V30 | Run with `gamehost`; see V2 and V30 |
| `asset_classes.py` | Rebuilds `crates/pinewatch-assets/src/types/table.rs` from an `ikdasm` dump of `Assembly-CSharp.dll` and diffs every field | V19 (asset classes) | 96 classes, the table matches |
| `il_method.py` | Prints the IL (or only its calls and strings) of methods matching a pattern in an `ikdasm` dump | V21 | `WITH_NOREDIST` confirmed |
| `jsontokens.cs` | Every token, value, depth, path and position, and every exception, that the shipped `Newtonsoft.Json.dll`'s `JsonTextReader` gives for each case of `crates/pinewatch-json/tests/fixtures/reader-corpus.txt` (compile with `-r:Newtonsoft.Json.dll -r:System.Numerics.dll`) | V31 | Written to `tests/fixtures/reader-expected.txt`; the port matches all 409 cases |
| `gen_category_table.py` | Generates `crates/pinewatch-mono/src/chars/category_table.rs` from corefx's `Char.cs`, `CharUnicodeInfoData.cs` and `UnicodeCategory.cs` (mono/corefx `5b4edb9b`); `tests/categories.rs` checks the result against `chardump.cs`'s run | V3 | Regenerated 2026-11-05: identical to the committed table |

Compile a C# check with the Editor's `mcs` and run it either way:

```
MONO="<Editor>/Data/MonoBleedingEdge"
"$MONO/bin/mono.exe" "$MONO/lib/mono/4.5/mcs.exe" -out:chardump.exe chardump.cs
MONO_PATH=<copied Managed dir> "$MONO/bin/mono.exe" chardump.exe chardump.bin          # 32-bit runtime
gamehost.exe <copied runtime dll> <copied Managed dir> "<install>/MonoBleedingEdge/etc" chardump.exe chardump.bin
```

Disassemble with `"$MONO/bin/mono.exe" "$MONO/lib/mono/4.5/ikdasm.exe" Assembly-CSharp.dll > Assembly-CSharp.il`
(run in the copied `Managed` folder), then `python -I asset_classes.py Assembly-CSharp.il
crates/pinewatch-assets/src/types/table.rs` or `python -I il_method.py Assembly-CSharp.il
'^MasterBundleConfig::getAssetBundlePath$' --calls`.
