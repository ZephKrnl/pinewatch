# Behaviors awaiting harness confirmation

Everything here was ported from source, but the Unity reference harness hasn't confirmed it yet, because Unity
isn't set up. Each entry says what the Rust code does, where the behavior came from, and what the harness must check.

- When the harness confirms an entry, remove it.
- If an entry can't be matched, move it to `COMPATIBILITY.md` as a limitation.
- Entries marked **gap** are known to possibly differ from the original until the harness supplies the missing data.
- Entry numbers never change. Entries settled another way are listed at the end with their numbers.

## Source provenance

- **V0. Which source files Unity's Mono compiles.** The game runs Unity's Mono 6.13.0 (`MonoBleedingEdge`). Its
  class libraries are built from Unity's Mono fork, branch `unity-2022.3-mbe`, ported here at commit `b8eb56f`
  (2025-06-06). That commit pins corefx at `5b4edb9b` and corert at `11136ad5`.
  - **Build list.** The Windows player's corlib is the `unityjit` profile:
    `win32_unityjit_corlib.dll.sources` includes `win32_net_4_x_corlib.dll.sources`, which includes
    `corlib.dll.sources`.
  - **Ported files.** `pinewatch-mono` follows the files that list compiles:

    | Behavior | Source |
    |---|---|
    | `ParseNumbers` | corefx `src/Common/src/CoreLib/System/ParseNumbers.cs` |
    | `Guid` parsing | corefx `src/Common/src/CoreLib/System/Guid.cs` |
    | `Int32.TryParse` and `UInt32.TryParse` | corefx `Int32.cs` and `UInt32.cs` |
    | Number parsing | corert `src/System.Private.CoreLib/shared/System/Number.Parsing.cs` |
    | Style validation | reference source `numberformatinfo.cs` |
    | `StreamReader` | corefx `IO/StreamReader.cs` |
    | Text decoding | corefx `Text/UTF8Encoding.cs`, `Text/UnicodeEncoding.cs`, `Text/UTF32Encoding.cs`, `DecoderNLS.cs`, `DecoderReplacementFallback.cs` |
    | Exception messages | corefx exception classes; `mono/metadata/exception.c`; `mono/metadata/icall-windows.c` |

  - **Messages are English.** Both resource lookups (`SR.GetResourceString` in `mcs/build/common/SR.cs` and
    `Environment.GetResourceString` in `ReferenceSources/Environment.cs`) return their argument. In the shipped
    IL, every call site examined passes the English text, including those whose source passes a key:
    `SR.GetResourceString(nameof(SR.Format_GuidInvLen))` in corefx and
    `Environment.GetResourceString("Argument_InvalidNumberStyles")` in reference source.
    `pinewatch-mono/src/sr.rs` holds the texts.
  - **Checked against the shipped `mscorlib.dll`.** The install test `shipped_mscorlib` checks that every text is a
    string literal there, and that the old Mono-only texts and the resource keys are not. A read-only IL dump
    (`ldstr` literals, call targets, branch structure) of `ParseNumbers.IsDigit`, `Guid.TryParseGuid*`,
    `Guid.StringToInt`, `GuidResult.GetGuidParseException`, `ArgumentException.Message`,
    `NumberFormatInfo.ValidateParseStyleInteger`, `Int32.TryParse`, `List<T>`'s indexer, `Dictionary.FindEntry`,
    `ThrowHelper` and `StreamReader` matched the ported source.

  **Check:** that Unity 2022.3.62f3's Mono matches commit `b8eb56f` in every other file Pinewatch ports. Unity
  publishes no exact build commit, and the runtime DLL doesn't embed one.

## `pinewatch-mono`

- **V2. Exception messages.** The texts below come from the shipped constructors and `ArgumentException.Message`
  (IL-checked). `Environment.NewLine` is `"\r\n"` on Windows (Mono's `mono_icall_get_new_line`).

  | Exception | Message |
  |---|---|
  | `NullReferenceException` | `Object reference not set to an instance of an object.` |
  | `InvalidCastException` | `Specified cast is not valid.` |
  | `OverflowException()` | `Arithmetic operation resulted in an overflow.` |
  | `IndexOutOfRangeException` | `Index was outside the bounds of the array.` |
  | `ArgumentOutOfRangeException(p)` | `Specified argument was out of the range of valid values.` + NewLine + `Parameter name: p` |
  | `ArgumentNullException(p)` | `Value cannot be null.` + NewLine + `Parameter name: p` |
  | `ArgumentException(message, p)` | the message + NewLine + `Parameter name: p`; nothing is added for a null or empty `p` |

  **Check:** two runtime behaviors come from Mono's C source, not the shipped IL:
  - A null dereference and a failed cast make the runtime create the exception with its default constructor
    (`mono_exception_from_name` in `exception.c`).
  - `Environment.NewLine` is `"\r\n"` on Windows.
- **V3. Unicode tables (gap).** Above U+00FF, `IsWhiteSpace` uses Rust's `White_Space` property.
  `ToLowerInvariant` and `ToUpperInvariant` use Rust's mappings when they give a single code unit. .NET's simple
  mappings differ for a few characters, for example U+0130 lowercases to U+0069 in .NET but stays U+0130 here.
  Mono may also use an older Unicode version. In M1 this only affects GUID text: trimming, the hex-digit check,
  and the `x` of a hex prefix. **Fix:** dump Mono's tables for all 65,536 code units with the harness.
- **V4. UTF-8 decoding.** Ported from corefx `UTF8Encoding.GetChars`, the slow loop, which the fast loop hands every
  irregular sequence to. Each fallback is one U+FFFD. These lead bytes are invalid on their own: `80`–`BF`, `C0`,
  `C1` and `F5`–`FF`. An interrupted sequence is one fallback, and the interrupting byte is read again. A bad
  second byte (overlong form, encoded surrogate, beyond U+10FFFF) is one fallback together with its lead byte, so
  `E0 80 80` gives two U+FFFD where Rust's `from_utf8_lossy` gives three. A sequence still incomplete at the end is
  dropped, because `StreamReader` never flushes its decoder. **Check:** a generated set of invalid sequences.
- **V5. UTF-16 and UTF-32 decoding.** Ported from corefx `UnicodeEncoding.GetChars` and `UTF32Encoding.GetChars`.
  Each unpaired surrogate is one U+FFFD, emitted when the next code unit arrives, and so is each invalid UTF-32
  value. An odd trailing byte, a trailing high surrogate and a partial UTF-32 group are dropped. **Check:**
  generated inputs.

- **V16. Float conversion.** `float` and `double` parsing is ported from corefx `Single.TryParse` and
  `Double.TryParse` over corert's `TryParseSingle`, `TryParseDouble` and the portable `NumberToDouble`
  (`Number.CoreRT.cs`), which the shipped IL has as managed code. Only the first 18 significant digits count, the
  value is rounded once to 53 bits, and `float` is the cast of that `double`. NaN has the C# constants' bits.
  **Check:** a harness sweep of generated decimals near rounding boundaries, long digit strings and denormals.

- **V18. Enum member order.** `Enum.TryParse` matches names in the order `GetCachedValuesAndNames` returns: Mono's
  declaration order, sorted by value with `Array.Sort` when the values don't ascend. `Array.Sort` is an introsort,
  stable up to 16 members but not above. Pinewatch sorts stably. The order only matters for names that differ only
  by letter case, parsed with `ignoreCase`. **Check:** when each game enum is ported, confirm no two of its names are
  equal ignoring case; if one is, dump its member order with the harness.

## `pinewatch-kvt`

- **V9. Hierarchy hash (later step).** `LevelHierarchy.load` hashes the bytes its `StreamReader` pulled through
  `SHA1Stream`. If a stray root `}` ends reading early, the hash covers only the 1024-byte buffers read so far.
  Not implemented in M1 (the level hash belongs to level loading). **Check:** when the level hash is ported.

## `pinewatch-level`

- **V10. Hierarchy type resolution (gap).** `LevelHierarchy.read` resolves each `Type` with `Type.GetType` and creates
  the C# object. M1 doesn't port that: every non-empty valid name counts as an existing type. So M1 never logs
  `Level hierarchy item index N missing type: …` for names the real game doesn't know, though it still generates
  instance IDs for them. The landscape item is found by comparing the name with `LANDSCAPE_TYPE` exactly. `Type.GetType`
  also resolves other spellings of the same type, for example `SDG.Framework.Landscapes.Landscape, Assembly-CSharp`,
  so a map written that way would load in the game but exit with "no landscape" here. The type names in
  `LANDSCAPE_TYPE` and in the two built-in type redirects are taken from the SDK source and haven't been checked
  against a real `Level.hierarchy` written by the shipped game. **Fix:** a registry of the game's hierarchy item
  types (devkit objects step), with `Type.GetType`'s name matching.
- **V11. Level path string.** Messages use `<install root with \ separators, no trailing \>\Maps\<folder>`, followed by
  `/Landscape/...`, as `Path.Combine` and `Directory.GetDirectories` build it. The root is `DirectoryInfo.FullName`:
  Pinewatch makes the install path absolute and resolves `.` and `..` the way `Path.GetFullPath` does
  (`GetFullPathNameW` on Windows). **Check:** the root string Unity reports
  (`UnturnedPaths.RootDirectory.FullName`), including letter case and 8.3 short names.
- **V12. I/O exception messages (gap).** A file that exists but fails to read becomes an `IOException` carrying Rust's
  error text, not Mono's. **Fix:** map OS error codes to Mono's exception types and messages.
- **V13. Culture-formatted numbers in messages (gap).** `LandscapeCoord.ToString()` and `UnturnedLog`'s
  `string.Format` use the player's culture. M1 formats with the invariant culture, which only differs for negative
  numbers in cultures with a different minus sign. **Fix:** culture support (`Culture` parameter, P5).
- **V14. Holiday material redirects (later step).** When `Level.shouldUseHolidayRedirects` is on, `LandscapeTile.read`
  swaps material references for their holiday versions. M1 doesn't use materials yet. **Fix:** port this with the
  M2 asset system.
- **V15. More than 8 materials.** `materials[layer] = …` on the 8-slot `InspectableList` is assumed to throw
  `List<T>`'s `ArgumentOutOfRangeException` (`Index was out of range. …` + NewLine + `Parameter name: index`).
  **Check:** `InspectableList`'s indexer.

## `pinewatch-dat`

- **V17. Keys that differ only by case (gap).** `DatDictionary` uses `StringComparer.OrdinalIgnoreCase`. Pinewatch
  treats two keys as equal when every code unit matches after `ToUpperInvariant`, and above ASCII that mapping comes
  from Rust's Unicode tables, not Mono's (V3). Equal keys merge: the first spelling stays and the last value wins,
  with a `duplicate key` error. **Check:** non-ASCII key pairs that differ only by case merge, or stay apart, exactly
  as in the game. That depends on both `Equals` and `GetHashCode` of Mono's `OrdinalIgnoreCase` comparer.
- **V19. Type names in `.dat` values (gap).** `ParseType` calls `Type.GetType(name, false, ignoreCase: true)`, which
  searches the calling assembly (`UnturnedDat.dll`) and `mscorlib.dll` unless the name says which assembly, and
  matches case-insensitively. Pinewatch asks a `TypeResolver`, and no resolver knows the game's types yet, as in V10.
  The checks before the lookup (null or empty value, `\`, `:` or `/` in the name) are ported. **Fix:** a type
  registry built from the shipped assemblies, with `Type.GetType`'s name matching.
- **V20. Legacy `readBoolean` comparison (gap).** `Data.readBoolean` compares with `"y"` and `"true"` using
  `StringComparison.InvariantCultureIgnoreCase`, a linguistic comparison through Mono's collation tables, which skips
  ignorable characters such as U+00AD (soft hyphen). Pinewatch compares code units ignoring case, so it matches for
  every value without such characters. **Check:** `readBoolean` on `y` and `true` with ignorable characters inserted.

## Settled without the harness

Settled 2026-11-01 from the compiled source and the shipped `mscorlib.dll` (V0):

- **V1. Resource-key messages.** Wrong: messages are the English texts, not the resource keys. `new Guid("0")` throws
  `FormatException` with `Guid should contain 32 digits with 4 dashes (xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx).`
- **V6. StreamReader buffer.** `new StreamReader(stream)` passes a 1024-byte buffer (corefx `DefaultBufferSize`;
  `ldc.i4 1024` in the shipped constructor). Byte order marks are only detected in the first buffer.
- **V7. List indexer message.** `List<T>`'s indexer calls `ThrowHelper.ThrowArgumentOutOfRange_IndexException`:
  `ArgumentOutOfRangeException("index", "Index was out of range. Must be non-negative and less than the size of the
  collection.")`. Its NewLine is V2's.
- **V8. Null dictionary key.** `Dictionary.FindEntry(null)` throws `ArgumentNullException("key")`. Its NewLine is V2's.
