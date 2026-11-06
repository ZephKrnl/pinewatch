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
    | Integer `TryParse` | corefx `SByte.cs`, `Byte.cs`, `Int16.cs`, `UInt16.cs`, `Int32.cs`, `UInt32.cs`, `Int64.cs`, `UInt64.cs` |
    | `float` and `double` `TryParse` | corefx `Single.cs` and `Double.cs`; corert `src/System.Private.CoreLib/src/System/Number.CoreRT.cs` (`NumberToDouble`) |
    | Number parsing | corert `src/System.Private.CoreLib/shared/System/Number.Parsing.cs` |
    | Style validation | reference source `numberformatinfo.cs` |
    | `Boolean.TryParse` | corefx `Boolean.cs` |
    | `Enum.TryParse` | reference source `enum.cs` |
    | `Single.Equals` (for Unity's `Color.Equals`) | corefx `Single.cs` |
    | `String.IndexOf(char, int)` (for `UnityDatEx`) | corefx `String.Searching.cs` |
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
    `ThrowHelper` and `StreamReader` matched the ported source, and so did `Single.Equals`, `Single.IsNaN` and
    `String.IndexOf(char, int)` with the overload it calls, for M2a. The M2a final review also checked
    `Boolean.TryParse` and `Enum.TryParseEnum` against the IL.

  **Check:** that Unity 2022.3.62f3's Mono matches commit `b8eb56f` in every other file Pinewatch ports. Unity
  publishes no exact build commit, and the runtime DLL doesn't embed one.

## `pinewatch-mono`

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
  On 2026-11-02 the port's power-of-ten tables were compared with upstream `Number.CoreRT.cs` at `11136ad5` (equal),
  and a separate big-integer model of that file agreed with the port on 20,000 generated decimals;
  `corert_bits_from_an_independent_model` pins 27 of them, 14 where corert differs from correct rounding. On
  2026-11-03 the game's shipped `mscorlib.dll`, run by the Mono of the Unity 2022.3.62f3 Editor, gave the same bits
  for those 27 and for the 13 decimals `corert_bits_from_the_shipped_class_library` pins (3 of them quirks), and
  failed `1.7976931348623159e308` as an overflow (`double.TryParse(s, NumberStyles.Any, CultureInfo.InvariantCulture)`).
  **Check:** a harness sweep of generated decimals near rounding boundaries, long digit strings and denormals.

## `pinewatch-io`

- **V22. Mono's `Path` (gap for rare forms).** Unity's Mono compiles its own `System.IO/Path.cs`, not corefx's.
  `pinewatch_io::mono_path` ports `Combine`, `GetFileName`, `GetDirectoryName` (with `CleanPath`),
  `GetFileNameWithoutExtension`, `ChangeExtension(path, null)`, `GetPathRoot` and `GetFullPath` (`InsecureGetFullPath`,
  `WindowsDriveAdjustment`, `CanonicalizePath`, `SameRoot`) for both platforms. Not ported: `MonoIO.RemapPath`
  (`MONO_IOMAP`, off by default); the `GetFullPathName` calls for a drive-relative path on another drive than the
  current directory (`D:dir` while in `C:`), which return the path unchanged here; `char.IsLetter` for a non-ASCII
  drive letter; and the culture-aware `String.Compare` of UNC server names in `SameRoot`, compared ordinally ignoring
  case here. **Check:** a harness sweep of `Path` calls on generated paths, including these forms.

## `pinewatch-assets`

- **V23. Level discovery messages (gap).** The asset search needs only which `Maps` folders are levels:
  `ReadLevelInfo` accepts a folder whose `Level.dat` exists and reads, and Pinewatch logs `Discovered level ...` for
  it. Not yet produced: `Unable to parse <map>/Config.json! ...` and the level version error, which need the JSON
  reader, and `Level.dat`'s fields. **Fix:** level info with `pinewatch-json`.
- **V24. Folder enumeration.** Unity's Mono enumerates with corefx's `FileSystemEnumerator`: on Windows through
  `NtQueryDirectoryFile`, so entries come in the file system's own order, the order Rust's `read_dir`
  (`FindFirstFileExW`) also returns on NTFS. `EnumerateFiles(dir, "*.asset")` uses the compatible options (Win32
  matching, platform case, nothing skipped): `*.asset` becomes `name.EndsWith(".asset", OrdinalIgnoreCase)` on
  Windows and an ordinal match on Linux, and short 8.3 names aren't matched. A missing folder throws
  `DirectoryNotFoundException` (`Could not find a part of the path '<full path>'.`). The searcher calls
  `Path.Combine` and `Path.GetFileName` outside its `try` blocks; an exception there would end its thread and leave
  the location reading forever, where Pinewatch returns an error. File-system names can't hold the characters that
  throw. **Check:** enumeration order and `*.asset` matching on the reference machine, including a folder with
  mixed-case extensions.
- **V25. Loading master bundles (gap).** The reader reads each master bundle config and hashes its bundle file
  (SHA-1 of the whole file, as `SHA1Stream` sees it); the main thread then has Unity load the bundle. Pinewatch
  treats every load as successful, as the game's own bundles are. Unity refuses a second bundle with the same
  content, and the game then logs `Using "<name>" in "<dir>" as fallback asset bundle for "<dir>"` or one of the
  `Unable to ...` messages; that needs the bundle reader (M2d). **Fix:** read the bundle header with M2d.
- **V26. Culture-sensitive comparisons in asset loading (gap).** `findMasterBundleByPath` uses `string.StartsWith`
  (current culture), `findMasterBundleByName` `InvariantCultureIgnoreCase`, the core bundle check
  `InvariantCulture`, and `ItemAsset.GetTypeFriendlyName` `StartsWith("Item ")`. Pinewatch compares ordinally
  (ignoring case where the C# does), which is the same for paths and names without characters that Mono's
  collation ignores or treats specially, as in V20. **Fix:** Mono's collation tables.
- **V27. Loading order and constructor failures.** Definitions wait for the core master bundle and then load in
  arrival order; without a core master bundle none loads and the original's loading loop never ends (Pinewatch
  reports them as pending). Legacy per-asset `.unity3d` bundles aren't loaded (M2d), only named. `Activator.
  CreateInstance` on the one abstract asset class, `FoliageInfoAsset`, throws Mono's `MissingMethodException`
  built with its two-string constructor, so the message reads `Method 'Cannot create an abstract class '{0}'..
  SDG.Framework.Foliage.FoliageInfoAsset' not found.` (from Mono's `RuntimeType.CreateInstanceSlow` and corefx's
  `MissingMethodException.Message`). **Check:** the abstract-class message in the shipped runtime, and the core
  bundle wait with a test install.
- **V28. Asset classes' own `PopulateAsset` (gap).** The index is ported exactly (`AddToMapping`'s rules, messages
  and `Dictionary` order), and so is what it reads from each asset: the friendly name of the 10 classes that
  override `FriendlyName`, `ItemAsset.isPro`, and all of `RedirectorAsset`'s and `LandscapeMaterialAsset`'s
  `PopulateAsset`. A ported `PopulateAsset` that throws takes `LoadFile`'s path (`Caught exception while populating
  ...`, not indexed). The other classes' `PopulateAsset` isn't ported yet, so their own `ReportError` messages are
  missing, and an asset whose `PopulateAsset` throws in the game is indexed here.
  `VendorAsset` names with `<` tags skip `ItemTool.filterRarityRichText` and `RichTextUtil.replaceColorTags` (a
  regular expression), and `ServerListCurationAsset` uses its name, as when it has no curation file. **Fix:** port
  each class's `PopulateAsset` with the step that first needs it.
- **V29. Current-culture integer parsing (gap).** `CachingBcAssetRef.TryParse(string, …)` reads a legacy ID with
  `ushort.TryParse(text, out id)`, and `Parser.TryGetUInt32FromIP` (the packed level version) each part with
  `uint.TryParse(part, out value)`: `NumberStyles.Integer` in the player's culture. Pinewatch parses in the invariant
  culture, which differs only for cultures whose positive or negative sign isn't `+` or `-`. **Fix:** culture
  support (`Culture` parameter, P5), with V13.

## `pinewatch-binary`

## `pinewatch-json`

- **V31. Json.NET's reader (gaps).** `JsonTextReader` with `JsonConvert`'s defaults gives the same tokens, values,
  depths, paths, line positions and exceptions as the shipped `Newtonsoft.Json.dll` for every case of
  `crates/pinewatch-json/tests/fixtures/reader-corpus.txt`. The 409 texts cover structure, comments, unquoted names,
  numbers, strings and escapes, surrogates, dates, depth, errors and paths, line ends, Unicode whitespace,
  constructors, and texts longer than the reader's 1,024-character buffer. The expected output comes from
  `reference/checks/jsontokens.cs`, run on 2026-11-05 by the Unity Editor's Mono with copies of the shipped
  `mscorlib.dll` and `Newtonsoft.Json.dll` (nothing in the reader is decided by the runtime). It includes the two
  `ArgumentOutOfRangeException`s that escape the reader: `DateTime.AddDays` for 24:00 on 9999-12-31, and the
  `DateTime` constructor for `/Date(n)/` out of range. Two gaps remain:
  - **Local time.** A date with an offset (`+01:00`, or `/Date(n+0100)/`) is converted to the local time zone.
    Time zones aren't ported, so the local zone is taken to be UTC: the token holds the UTC instant, marked
    `Local`. The corpus compares local dates by their UTC instant, so it can't see this. The clamp at the ends of
    the range (an offset past `DateTime.MaxValue` or below `MinValue`) also uses a zero local offset. **Check:**
    port `TimeZoneInfo.Local`, or decide which zone Pinewatch reports.
  - **Lone surrogates in messages and paths.** Exception messages and `path()` are Rust strings, so a lone
    surrogate in a property name, or in a character a message quotes, becomes U+FFFD there; token values keep it.

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
  `/Landscape/...`, as `Path.Combine` and `Directory.GetDirectories` build it. The root is
  `UnturnedPaths.RootDirectory.FullName`, Unity's `new DirectoryInfo(Application.dataPath).Parent`: since
  2026-11-02, `InstallRoot::full_name` computes it with Mono's own `Path.GetFullPath` (ported in
  `pinewatch_io::mono_path`, V22) from `<install>/Unturned_Data`, where M1 used `GetFullPathNameW`. **Check:** the
  root string Unity reports, including letter case and 8.3 short names (`Application.dataPath` comes from Unity's
  native code).
- **V12. I/O exception messages (gap).** A file that exists but fails to read becomes an `IOException` carrying Rust's
  error text, not Mono's. **Fix:** map OS error codes to Mono's exception types and messages.
- **V13. Culture-formatted numbers in messages (gap).** `LandscapeCoord.ToString()` and `UnturnedLog`'s
  `string.Format` use the player's culture, and so do the interpolated tile coordinates in the asset integrity
  contexts (`Landscape tile (x: … y: … layer: …)`). Pinewatch formats with the invariant culture, which only differs
  for negative numbers in cultures with a different minus sign. **Fix:** culture support (`Culture` parameter, P5).
- **V14. Holiday material redirects (inputs open).** M2c task 11 ports `LandscapeMaterialAsset` and its
  `getHolidayRedirect` (Christmas, Halloween and April Fools pick their redirect; every other holiday value gives
  none, including values no member has, since the server sends the holiday), and the tile side: `LandscapeTile.read`
  swaps a material for its redirect when `TileAssets::use_holiday_redirects` is on, records materials the assets lack
  as known missing, and `updatePrototypes` queues every valid slot for the integrity check, in the original's order.
  Still open are the two inputs, which `TileAssets::new` leaves off (no holiday): `Level.shouldUseHolidayRedirects`
  (not the editor, the level's `Allow_Holiday_Redirects` from `Config.json`, a holiday active) and the active holiday
  (`Provider.authorityHoliday`: the server's, or in singleplayer the holiday schedule when the mode config allows
  holidays). Until they come, Pinewatch reads tiles as the game does outside holidays. `updatePrototypes`' Unity
  terrain layers (`getOrCreateLayer`, which loads the textures) come with the terrain materials (M2e). **Fix:** the
  inputs with the level config and the provider.

## `pinewatch-dat`

- **V19. Type names in `.dat` values (gap).** `ParseType` calls `Type.GetType(name, false, ignoreCase: true)`, which
  searches the calling assembly (`UnturnedDat.dll`) and `mscorlib.dll` unless the name says which assembly, and
  matches case-insensitively. Pinewatch asks a `TypeResolver`. Since M2c, `pinewatch_assets::AssetTypeResolver`
  knows the 96 asset classes of the SDK: an assembly-qualified name with `Assembly-CSharp` (ignoring case, with any
  further properties) and the full class name (ignoring ASCII case) resolve; an unqualified name doesn't, since
  `UnturnedDat.dll` and `mscorlib.dll` hold no asset class. Other types `Type.GetType` would find (non-asset classes,
  arrays, nested or generic types) aren't known, so a definition naming one gets `Unable to parse Metadata.Type`
  where the game says `Type "<name>" is not a valid asset type`.
  The checks before the lookup (null or empty value, `\`, `:` or `/` in the name) are ported. `throwOnError: false`
  doesn't stop every exception: `Type.GetType` still throws some, for example `FileLoadException` for an
  assembly-qualified name whose assembly is found but can't be loaded, and `ParseType` doesn't catch them. So
  `TypeResolver::resolve` returns a `Result`, and `parse_type` passes its error on. **Fix:** a type registry built
  from the shipped assemblies, with `Type.GetType`'s name matching. **Check:** which exceptions Mono's
  `Type.GetType(name, false, true)` throws for malformed and assembly-qualified names; how its type-name parser
  treats spaces, `+`, `[]` and version properties; and how it compares names ignoring case.
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

Settled 2026-11-02 from the source, without the harness:

- **V18. Enum member order.** `Enum.TryParse` matches names in the order `GetCachedValuesAndNames` returns: Mono's
  declaration order, sorted by value with `Array.Sort` when the values don't ascend. `Array.Sort` is unstable at
  every size (the three-element case reorders `[1, 1, 0]`), but it only reorders members with equal values, and
  those give the same bits whichever matches first. Pinewatch's stable sort therefore gives the same results. The
  test `equal_values_give_the_same_bits_in_any_order` covers it.
- **V15. More than 8 materials.** `InspectableList<T>` (`Framework/Debug/InspectableList.cs`) derives from `List<T>`
  and hides only `Add`, `Remove` and `RemoveAt`, not the indexer, and `LandscapeTile`'s constructor fills it with 8
  entries (the default material, then 7 invalid references). So `materials[layer] = …` for layer 8 or more throws
  `List<T>`'s `ArgumentOutOfRangeException`, V7's message, as `more_than_eight_materials_throws` expects. Settled
  during the stabilization pass.

Settled 2026-11-03 on the Windows PC, from the shipped assemblies (copies of `Unturned_Data/Managed`, game version
3.26.3.13; the programs are in `reference/checks/`):

- **V3. Unicode tables.** `char.ToUpperInvariant` and `char.ToLowerInvariant` of the shipped `mscorlib.dll`, run for
  all 65,536 code units by the Mono of the Unity 2022.3.62f3 Editor (`MONO_PATH` set to the copied `Managed` folder,
  so `typeof(object).Assembly.Location` is the game's corlib), give the FNV-1a checksums `whole_tables_match_monos`
  pins (`0x48ed4531e9399927` and `0xb94ec82258b5db7b`) and change 973 code units each. `TextInfo.ToUpper` and
  `ToLower` of `CultureInfo.InvariantCulture` and `string.ToUpperInvariant` give the same checksums. `IsWhiteSpace` is
  true for exactly the 8 Latin-1 and 17 other code units `chars.rs` lists. The Editor's own corlib gives a
  byte-identical dump. Pinewatch's tables were generated from Unity mono `b8eb56f` (V0); this run shows the shipped
  build has the same tables. For M2f task 3 the run also recorded the Unicode categories, from two functions that
  differ: the dump holds `char.GetUnicodeCategory` of every code unit (FNV-1a over the category numbers:
  `0x5cd45c712e21a2e5`), and the program prints only the checksum of `CharUnicodeInfo.GetUnicodeCategory`
  (`0x4280054dea7fbc49`), which corefx's `CharUnicodeInfoData` (mono/corefx `5b4edb9b`) reproduces. The two differ at
  exactly five code units, U+00A7, U+00AA, U+00AD, U+00B6 and U+00BA, because below U+0100 `char` reads its own
  table, `s_categoryForLatin1`. `tests/categories.rs` pins both checksums and the dump's `IsNumber` and
  `IsLetterOrDigit`, which Json.NET's reader uses (M2f task 3).
- **V17. Keys that differ only by case.** `DatDictionary` uses `StringComparer.OrdinalIgnoreCase`. The shipped
  `mscorlib.dll`'s IL has the chain the source shows: `OrdinalIgnoreCaseComparer.Equals` is `String.Equals(x, y,
  OrdinalIgnoreCase)`, whose `CompareOrdinalIgnoreCase` folds `a` to `z` while both code units are ASCII
  (`GlobalizationMode.Invariant` is the constant `false`) and passes the rest to `CompareStringOrdinalIgnoreCase`,
  which compares `TextInfo.ToUpper` of each; `GetHashCode` is `CompareInfo.GetIgnoreCaseHash`, Marvin over
  `MemoryExtensions.ToUpperInvariant` of the key. Run on the shipped corlib for every code unit against its upper
  case, lower case, ASCII case flip and neighbours (327,678 pairs), the comparer says equal exactly when the
  `ToUpperInvariant` forms are equal, and equal strings hash alike. So two keys merge exactly when every code unit is
  equal after V3's `ToUpperInvariant`, which is what Pinewatch does; the first spelling stays and the last value
  wins, with a `duplicate key` error.
- **V21. Build configuration.** The shipped Windows client is built with `WITH_NOREDIST` and without `UNITY_EDITOR`,
  `DEVELOPMENT_BUILD` and `DEDICATED_SERVER`. In the shipped `Assembly-CSharp.dll`:
  - `Assets.LoadAllAssets` (`<LoadAllAssets>d__118.MoveNext`) passes `Path.Combine(..., "Bundles")` straight to
    `AddSearchLocation`, with no `Directory.Exists` (the `!WITH_NOREDIST` block), and calls
    `Provider.initAutoSubscribeMaps` (the `WITH_NOREDIST` block) and `ResourceHash.Initialize` (`!DEDICATED_SERVER`).
  - `MasterBundleConfig.getAssetBundlePath` is `Path.Combine(directoryPath, assetBundleName)` and `getHashFilePath`
    is `getHashFileName(Path.Combine(directoryPath, assetBundleName))`, with no `core.masterbundle` case.
  - `Level.ScanKnownLevels` lists `PathEx.Join(UnturnedPaths.RootDirectory, "Maps")` without a Steam fallback.
  - `Provider.steamAppInstallDirectory`, which the SDK uses only inside `!WITH_NOREDIST` blocks, isn't in the
    assembly at all.

  So the core `Bundles` location is added without checking that it exists, `Maps` isn't looked up in the Steam
  install, and the master bundle paths are `Path.Combine(directory, name)`, as Pinewatch does. Pinewatch follows the
  Windows client on every host; the Linux client would first look for `<name>_linux` bundles
  (`UNITY_STANDALONE_LINUX`).

- **V2. Exception messages.** The texts below come from the shipped constructors and `ArgumentException.Message`
  (IL-checked).

  | Exception | Message |
  |---|---|
  | `NullReferenceException` | `Object reference not set to an instance of an object.` |
  | `InvalidCastException` | `Specified cast is not valid.` |
  | `OverflowException()` | `Arithmetic operation resulted in an overflow.` |
  | `IndexOutOfRangeException` | `Index was outside the bounds of the array.` |
  | `ArgumentOutOfRangeException(p)` | `Specified argument was out of the range of valid values.` + NewLine + `Parameter name: p` |
  | `ArgumentNullException(p)` | `Value cannot be null.` + NewLine + `Parameter name: p` |
  | `ArgumentException(message, p)` | the message + NewLine + `Parameter name: p`; nothing is added for a null or empty `p` |

  Run on the game's own x64 runtime (`gamehost` with the shipped class libraries, `runtime_exceptions.cs`), the
  runtime raises a null dereference with its own text, `Object reference not set to an instance of an object`
  without the final period, in every form tried (a call, a field, an array, `throw null`, unboxing, a delegate, an
  interface); the text is a C string in `mono-2.0-bdwgc.dll`, and `MonoException::null_reference` now uses it.
  Failed casts and unboxing, array and string indexes, checked arithmetic and conversions and `int.MinValue / -1`
  get the default constructor's message. `Environment.NewLine` is `"\r\n"`.
- **V30. `new byte[n]` with a negative `n`.** On the game's x64 runtime, `newarr` with a negative `int` size (and
  `new int[-1, 1]`) throws `OverflowException` with an empty message, whose `ToString()` is just
  `System.OverflowException`; a negative `long` size fails in its conversion first, with the default message. The
  32-bit runtime of the Unity Editor gives the default message instead, so only the game's runtime counts.
  `Block.readByteArray` now throws `MonoException::negative_array_size`, and `UnturnedLog.exception` would log it
  as `(empty exception message)`.
