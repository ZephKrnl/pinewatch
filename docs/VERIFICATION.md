# Behaviors awaiting harness confirmation

Everything here was ported from source, but the Unity reference harness hasn't confirmed it yet, because Unity
isn't set up. Each entry says what the Rust code does, where the behavior came from, and what the harness must check.

- When the harness confirms an entry, remove it.
- If an entry can't be matched, move it to `COMPATIBILITY.md` as a limitation.
- Entries marked **gap** are known to possibly differ from the original until the harness supplies the missing data.

## Source provenance

- **V0.** Mono class-library behavior was ported from Unity's Mono fork, branch `unity-main`. The files used:
  - `mcs/class/referencesource/mscorlib/system/number.cs`
  - `mcs/class/referencesource/mscorlib/system/guid.cs`
  - `mcs/class/referencesource/mscorlib/system/io/streamreader.cs`
  - `mcs/class/corlib/ReferenceSources/ParseNumbers.cs`
  - `mcs/class/corlib/ReferenceSources/Environment.cs`

  **Check:** these match the Mono that Unity 2022.3.62f3 ships.

## `pinewatch-mono`

- **V1. Resource-key messages.** Mono's `Environment.GetResourceString(key)` returns the key itself. So `new Guid(string)`
  failures throw `FormatException` with messages like `Format_GuidInvLen`, not the English text.
  **Check:** the messages in a real Unity run.
- **V2. Runtime exception messages.** The text used for each exception type:

  | Exception | Message used |
  |---|---|
  | `NullReferenceException` | `Object reference not set to an instance of an object` |
  | `InvalidCastException` | `Specified cast is not valid.` |
  | `OverflowException()` | `Arithmetic operation resulted in an overflow.` |
  | `IndexOutOfRangeException` | `Index was outside the bounds of the array.` |
  | `ArgumentOutOfRangeException(p)` | `Specified argument was out of the range of valid values.` then `\nParameter name: p` |
  | `ArgumentNullException(p)` | `Value cannot be null.` then `\nParameter name: p` |

  **Check:** each text, including the `Parameter name:` line format.
- **V3. Unicode tables (gap).** Above U+00FF, `IsWhiteSpace`, `IsNumber`, `IsLetter`, `ToLowerInvariant` and
  `ToUpperInvariant` use Rust's Unicode data. Mono may use an older Unicode version (for example U+180E), and
  `IsLetter` is approximated with `is_alphabetic` minus numbers (combining marks differ).
  This only affects characters above Latin-1 in GUID text, trimming and `ParseNumbers` digit checks.
  **Fix:** dump Mono's tables for all 65,536 code units with the harness.
- **V4. UTF-8 replacement (gap).** Invalid UTF-8 becomes one U+FFFD per maximal invalid subpart (Rust's `from_utf8_lossy`).
  Mono's reference-source `UTF8Encoding` may replace differently. This only affects files with invalid UTF-8,
  and the official install has none. **Check:** a generated set of invalid sequences.
- **V5. UTF-16 and UTF-32 replacement.** Unpaired surrogates, an odd trailing byte, invalid code points and a partial
  trailing group each become one U+FFFD. **Check:** generated inputs.
- **V6. StreamReader buffer.** Byte order mark detection only sees the first 1024 bytes (`DefaultBufferSize`).
  **Check:** the buffer size in Unity's Mono, using files whose second kilobyte starts with a byte order mark.

## `pinewatch-kvt`

- **V7. List indexer message.** An out-of-range list index throws `ArgumentOutOfRangeException` with
  `Index was out of range. Must be non-negative and less than the size of the collection.`, then `\nParameter name: index`.
  **Check:** the text.
- **V8. Null dictionary key.** `{` or `[` with no key before it makes `Dictionary.TryGetValue(null)` throw
  `ArgumentNullException` for parameter `key`. **Check:** the type and text.
- **V9. Hierarchy hash (later step).** `LevelHierarchy.load` hashes the bytes its `StreamReader` pulled through
  `SHA1Stream`. If a stray root `}` ends reading early, the hash covers only the 1024-byte buffers read so far.
  Not implemented in M1 (the level hash belongs to level loading). **Check:** when the level hash is ported.

## `pinewatch-level`

- **V10. Hierarchy type resolution (gap).** `LevelHierarchy.read` resolves each `Type` with `Type.GetType` and creates
  the C# object. M1 doesn't port that: every non-empty valid name counts as an existing type. So M1 never logs
  `Level hierarchy item index N missing type: …` for names the real game doesn't know, though it still generates
  instance IDs for them. **Fix:** a registry of the game's hierarchy item types (devkit objects step).
- **V11. Level path string.** Messages use `<install root with \ separators, no trailing \>\Maps\<folder>`, followed by
  `/Landscape/...`, as `Path.Combine` and `Directory.GetDirectories` build it. **Check:** the root string Unity reports
  (`UnturnedPaths.RootDirectory.FullName`), including letter case.
- **V12. I/O exception messages (gap).** A file that exists but fails to read becomes an `IOException` carrying Rust's
  error text, not Mono's. **Fix:** map OS error codes to Mono's exception types and messages.
- **V13. Culture-formatted numbers in messages (gap).** `LandscapeCoord.ToString()` and `UnturnedLog`'s
  `string.Format` use the player's culture. M1 formats with the invariant culture, which only differs for negative
  numbers in cultures with a different minus sign. **Fix:** culture support (`Culture` parameter, P5).
- **V14. Holiday material redirects (later step).** When `Level.shouldUseHolidayRedirects` is on, `LandscapeTile.read`
  swaps material references for their holiday versions. M1 doesn't use materials yet. **Fix:** port this with the
  M2 asset system.
- **V15. More than 8 materials.** `materials[layer] = …` on the 8-slot `InspectableList` is assumed to throw
  `List<T>`'s `ArgumentOutOfRangeException`. **Check:** `InspectableList`'s indexer.
