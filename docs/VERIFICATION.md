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
