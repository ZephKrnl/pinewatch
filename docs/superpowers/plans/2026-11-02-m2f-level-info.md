# M2f: Level info Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Read each level's `Level.dat` and `Config.json` exactly as `Level.ReadLevelInfo` does, so level discovery
gives the original's messages (V23) and the level config exists for later steps (V14's `Allow_Holiday_Redirects`).

**Architecture:** Two new format crates from the step 1 spec: `pinewatch-binary` (the `Block` reader) and
`pinewatch-json` (Json.NET 13.0.3's `JsonTextReader` and the part of `JsonSerializerInternalReader` the config types
reach, with explicit Rust contracts in place of reflection). `LevelInfo` and the full `read_level_info` go into
`pinewatch-assets`, where level discovery already lives (`pinewatch-level` depends on it, so it can't go there).

**Tech Stack:** Rust 1.99.1, `pinewatch-mono` (numbers, `Guid`, enums, `StreamReader` decoding), `sha1`.

**Spec:** `docs/superpowers/specs/2026-11-01-file-format-readers-design.md` (section 3: "JSON deserialization with
Newtonsoft semantics for the types step 1 reads (`LevelInfoConfigData` and the types it contains)", `Level.dat`,
`Config.json`, Block; section 4: the crate table; section 10: items 6 to 8).

**Status:** written 2026-11-02 in a cloud session under the owner's autonomous order, not yet reviewed by the owner
(the spec asks for approval of each plan; M2c set the precedent of starting and flagging it).

## Global Constraints

- 1:1 behavior with the original C# wherever it is observable; where an exact match is impossible, an entry in
  `COMPATIBILITY.md` or `docs/VERIFICATION.md` with the reason. Never substitute silently.
- Production code is Rust. `reference/u3-sdk` is never edited; the install is never written.
- Tests first; one commit per task, authored `Zeph <205350396+ZephKrnl@users.noreply.github.com>`, no AI attribution.
- Ported third-party code goes in `NOTICES-PORTED-CODE.md`: Json.NET 13.0.3, commit
  `0a2e291c0d9c0c7675d445703e51750363a549ef` (the version the SDK's `Assets/Plugins/Json.NET/Newtonsoft.Json.dll`
  reports), MIT license.
- New crates join `FORMAT_CRATES` in `xtask/src/main.rs`; `cargo xtask check-deps` must still pass (only `sha1`, `png`,
  `windows-sys` from outside the workspace).
- Lines stay within 120 columns; `cargo clippy --workspace --all-targets` gives no warnings.

## Review Focus

1. A `Config.json` saved with a UTF-8 BOM (Notepad does this) parses: `StreamReader` drops the BOM first (Task 7).
2. An empty or `null` `Config.json` is not a silent default: `DeserializeObject` gives null, `config.Hash = …`
   throws, and the "Unable to parse" message is reported (Task 7).
3. A key repeated in a config appends to lists the constructor created and replaces arrays (Task 6).
4. Nesting deeper than 64 is an error, never a stack overflow (Tasks 3 and 7).
5. Versions with spaces, signs or parts over 255 behave as `uint.TryParse` and the `& 0xFF` mask make them, and the
   error message prints the partially packed value (Tasks 2 and 7).

---

### Task 1: `pinewatch-binary`: the `Block` reader

**Files:**
- Create: `crates/pinewatch-binary/Cargo.toml` (depends on `pinewatch-mono`, `pinewatch-unity`, `sha1`),
  `crates/pinewatch-binary/src/lib.rs`, `crates/pinewatch-binary/src/block.rs`
- Modify: `Cargo.toml` (workspace dependency `pinewatch-binary`), `xtask/src/main.rs` (`FORMAT_CRATES`),
  `crates/pinewatch-mono/src/text.rs` and `lib.rs` (export `utf8_get_string`)
- Test: `crates/pinewatch-binary/tests/block.rs`

**Interfaces:**
- Produces: `pinewatch_mono::utf8_get_string(bytes: &[u8]) -> NetString` (`Encoding.UTF8.GetString`: the V4 decoder,
  no BOM handling). `pinewatch_binary::Block` with `pub step: usize`, `pub long_binary_data: bool`,
  `Block::new(prefix: usize, contents: Vec<u8>)`, and `read_string -> NetString`, `read_string_array -> Vec<NetString>`,
  `read_boolean -> bool`, `read_boolean_array -> Result<Vec<bool>, MonoException>`, `read_byte -> u8`,
  `read_byte_array -> Result<Vec<u8>, MonoException>` (a negative `int` count throws `OverflowException`, V30),
  `read_i16`, `read_u16`, `read_i32`, `read_u32`, `read_f32`, `read_i64`, `read_u64`,
  `read_i32_array -> Vec<i32>`, `read_u64_array -> Vec<u64>`, `read_steam_id -> u64`,
  `read_guid -> Result<Guid, MonoException>`, `read_u16_r_vector3 -> Vector3`, `read_single_vector3 -> Vector3`,
  `read_color -> Color`, `get_hash -> [u8; 20]`. Not ported here: `readSingleQuaternion` (Unity's `Quaternion.Euler`,
  L6) and the `read(Type…)` dispatch, which come with the layouts that need them.

- [ ] **Step 1: Write the failing tests** in `tests/block.rs`, from `Files/Block.cs` and `GuidBuffer.Read`:
  - `integers_are_little_endian`: `[0x34,0x12, 0x78,0x56,0x34,0x12, 0xFF,0xFF,0xFF,0xFF, 0,0,0x80,0x3F]` reads
    `read_u16 == 0x1234`, `read_i32 == 0x1234_5678`, `read_i32 == -1`, `read_f32 == 1.0`.
  - `short_reads_give_zero_and_keep_the_step`: on `[1, 2, 3]`, `read_i32 == 0` with `step == 0`, `read_u16 == 0x0201`,
    `read_u16 == 0` with `step == 2`, `read_byte == 3`, `read_byte == 0`.
  - `the_prefix_is_the_first_step`: `Block::new(2, vec![9, 9, 7]).read_byte() == 7`.
  - `strings_have_a_one_byte_length`: on `[3, b'a', b'b', b'c', 5, b'x']`: `"abc"`, then `""` with `step == 10`, then
    `""` again with `step == 10`. On `[2, 0xC3, 0x28]`: `"\u{FFFD}("`.
  - `string_arrays`: `[2, 1, b'a', 0]` gives `["a", ""]`; an empty block gives `[]`.
  - `boolean_arrays_are_bit_packed_low_bit_first`: `[10, 0, 0b0000_0101, 0b0000_0010]` gives true at 0, 2 and 9 only;
    `[9, 0, 0xFF]` is `Err` with `ExceptionKind::IndexOutOfRange` and `Index was outside the bounds of the array.`
  - `byte_arrays`: short form `[2, 7, 8]` gives `[7, 8]`; `[5, 1]` gives five zeros with `step == 6`; with
    `long_binary_data`, `[2,0,0,0, 7,8]` gives `[7, 8]` and a count of 30000 gives `[]` with `step == 4`.
  - `integer_arrays_have_a_u16_count`: `[2,0, 1,0,0,0, 2,0,0,0]` gives `[1, 2]`; `[3,0, 1,0,0,0]` gives `[1, 0, 0]`.
  - `guids_need_16_bytes`: `[16, <16 bytes>]` gives `Guid::from_bytes(<16 bytes>)`; `[4, 1, 2, 3, 4]` is `Err` with
    `ExceptionKind::Argument` and the message `Source buffer is too small!`.
  - `vectors_and_colors`: `read_color` on `[255, 0, 51]` is `Color { r: 1.0, g: 0.0, b: 0.2, a: 1.0 }` (each `/ 255f`);
    `read_u16_r_vector3` on `[1, 0x00,0x80, 0xFF,0xFF, 2, 0,0]` is `x = 1 * 128 + (32768 / 65535) * 128 - 4096`,
    `y = 1 * 2048 - 1024`, `z = 2 * 128 - 4096`, each computed in `f64` and cast to `f32` (`Regions.REGION_SIZE` is
    `8192 / 64` = 128).
  - `the_hash_is_sha1_of_the_contents`, whatever the step.
- [ ] **Step 2: Run** `cargo test -p pinewatch-binary --test block`; expect it not to compile (no crate yet).
- [ ] **Step 3: Implement** the crate, `utf8_get_string` and the readers, each with `Block.cs`'s bounds check.
- [ ] **Step 4: Run** the test, then `cargo xtask check-deps`; expect both to pass.
- [ ] **Step 5: Commit** `pinewatch-binary: the Block reader`.

Done 2026-11-02: as planned, plus a test for a string that ends at the last byte. `readByteArray` with a negative
`int` count throws `OverflowException` (found while porting; the message is V30).

### Task 2: `Level.dat` and the packed version

**Files:**
- Create: `crates/pinewatch-assets/src/level_info.rs`
- Modify: `crates/pinewatch-assets/Cargo.toml` (`pinewatch-binary`), `src/lib.rs`
- Test: `crates/pinewatch-assets/tests/level_info.rs`

**Interfaces:**
- Consumes: `Block` (Task 1).
- Produces: `net_enum!` types `ELevelSize(i32)` (`TINY`..`INSANE`, 0..4) and `ELevelType(i32)` (`SURVIVAL`, `HORDE`,
  `ARENA`); `LevelFile { version: u8, hash: [u8; 20], owner: u64, size: ELevelSize, level_type: ELevelType }`;
  `read_level_file(contents: Vec<u8>) -> LevelFile` (`ReadLevelInfo`'s reads from `Block(0, bytes)`; the type is read
  only when `version > 1`); `try_parse_packed_version(version: Option<&NetString>) -> (bool, u32)`
  (`Parser.TryGetUInt32FromIP`, the out value included).

- [ ] **Step 1: Write the failing tests:**
  - `packed_versions`: `"3.0.0.0"` → `(true, 0x0300_0000)`; `"1.2.3.300"` → `(true, 0x0102_032C)`;
    `"1.2.x.4"` → `(false, 0x0102_0000)`; `"1.2.3"`, `""`, `"   "`, `None` → `(false, 0)`; `"+1.0.0.0"` and
    `" 1.0.0.0"` → `(true, 0x0100_0000)`; `"1.0.0.-1"` → `(false, 0x0100_0000)`. `uint.TryParse` uses the current
    culture: V29 covers it.
  - `level_file_fields`: version 2 bytes `[2, <owner u64 LE>, 3, 2]` → version 2, the owner, `ELevelSize(3)`,
    `ELevelType(2)`; version 1 `[1, <owner>, 4, 9]` → `ELevelType::SURVIVAL` (the 9 isn't read); `[]` → all zero;
    `hash == sha1(contents)`.
- [ ] **Step 2: Run** `cargo test -p pinewatch-assets --test level_info`; expect a compile failure.
- [ ] **Step 3: Implement** both in `level_info.rs`. Editability (`owner == Provider.client || .unlocker`) is left to
  Task 7.
- [ ] **Step 4: Run** the test; expect it to pass.
- [ ] **Step 5: Commit** `M2f task 2: Level.dat and the packed version`.

Done 2026-11-02: as planned, plus `"1.2.0.300"` → `(true, 0x0102_002C)`, since `"1.2.3.300"` gives the same value
with or without the `& 0xFF` mask.

### Task 3: `pinewatch-json`: `JsonTextReader` tokens

**Files:**
- Create: `crates/pinewatch-json/Cargo.toml` (`pinewatch-mono`), `src/lib.rs`, `src/reader.rs`, `src/number.rs`
  (Json.NET's `ConvertUtils.Int32TryParse`/`Int64TryParse` and `Convert.ToInt64(text, 8 or 16)`), `src/date.rs`
  (`DateTimeUtils.TryParseDateTime` for `DateParseHandling.DateTime`)
- Modify: `crates/pinewatch-mono/src/exception.rs` (`ExceptionKind::JsonReader`, `Newtonsoft.Json.JsonReaderException`;
  `ExceptionKind::JsonSerialization`, `Newtonsoft.Json.JsonSerializationException`), workspace `Cargo.toml`,
  `xtask/src/main.rs`, `NOTICES-PORTED-CODE.md`
- Test: `crates/pinewatch-json/tests/reader.rs`

**Interfaces:**
- Produces: `JsonToken` (`None`, `StartObject`, `StartArray`, `StartConstructor`, `PropertyName(NetString)`,
  `Comment(NetString)`, `Integer(JsonInteger)`, `Float(f64)`, `String(NetString)`, `Boolean(bool)`, `Null`,
  `Undefined`, `EndObject`, `EndArray`, `EndConstructor`, `Date(JsonDate)`); `JsonInteger` (`I64(i64)` or
  `Big(String)`, the decimal digits of a `BigInteger`); `JsonDate { ticks: i64, kind: DateTimeKind }`;
  `JsonTextReader::new(text: NetString)` with `JsonConvert`'s defaults (`MaxDepth` 64, `DateParseHandling.DateTime`,
  `DateTimeZoneHandling.RoundtripKind`, `FloatParseHandling.Double`), `read(&mut self) -> Result<bool, MonoException>`,
  `token(&self) -> &JsonToken`, `depth(&self) -> i32`, `path(&self) -> String`. Errors are `JsonReaderException`s with
  `JsonPosition.FormatMessage`'s ` Path '…', line N, position M.` suffix.

- [ ] **Step 1: Write the failing tests**, ported from `JsonTextReader.cs` and `JsonReader.cs`:
  - `values_and_structure`: `{"a": [1, -2.5, "s", true, null], 'b': undefined}` gives the tokens in order.
  - `lenient_syntax`: `// x` and `/* y */` give `Comment(" x")`, `Comment(" y")`; `{a_1$: 1}` gives the name
    `a_1$`; `[1,]` gives `StartArray, Integer(1), EndArray`.
  - `numbers`: `0x1F` → 31, `017` → 15 (octal), `1e3` → `Float(1000.0)`, `NaN`, `Infinity`, `-Infinity` → floats,
    `9223372036854775808` → `Big("9223372036854775808")`, `08` → `Err` (`Input string '08' is not a valid number.`).
  - `strings`: `"A\n\/"` → `A`, newline, `/`; `"\q"` → `Err`; an unterminated string → `Err`.
  - `dates`: `"2012-03-21T05:40:00Z"` and `"\/Date(1335205592410)\/"` → `Date` with `DateTimeKind::Utc` and their
    ticks; `"2012-03-21T05:40:00"` → `Date` with `Unspecified`; `"2012-03-21"` stays a `String` (Json.NET only tries
    text of 19 to 40 characters with a `T` at index 10).
  - `depth_limit`: 64 nested arrays read; 65 give `The reader's MaxDepth of 64 has been exceeded.` with its path.
  - `errors_name_the_position`: `{"a": x}` → `Unexpected character encountered while parsing value: x. Path 'a',
    line 1, position 6.`
- [ ] **Step 2: Run** `cargo test -p pinewatch-json --test reader`; expect a compile failure.
- [ ] **Step 3: Implement** by porting `JsonTextReader.Read`/`ParseValue`/`ParsePostValue`/`ParseProperty`/
  `ParseString`/`ParseNumber`/`ParseComment`/`ParseConstructor` and `JsonReader`'s state machine (`Push`, `Pop`,
  `SetToken`, `SetPostValueState`), with `pinewatch_mono::try_parse_f64` for floats. A date string whose offset isn't
  `Z` converts to local time in the original: record that in `docs/VERIFICATION.md` (no time zone data yet) and
  return the error-free UTC instant with `DateTimeKind::Local` until time zones come.
- [ ] **Step 4: Run** the test and `cargo xtask check-deps`; expect both to pass.
- [ ] **Step 5: Commit** `pinewatch-json: JsonTextReader`.

### Task 4: The typed reads and `DeserializeObject`'s frame

**Files:** Modify `crates/pinewatch-json/src/reader.rs`; Create `src/convert.rs` (`JsonConvert.DeserializeObject`'s
setup and the additional-content check); Test `crates/pinewatch-json/tests/read_as.rs`

**Interfaces:**
- Produces: `read_as_int32 -> Result<Option<i32>, _>`, `read_as_string -> Result<Option<NetString>, _>`,
  `read_as_boolean -> Result<Option<bool>, _>`, `read_as_double -> Result<Option<f64>, _>` (`JsonTextReader`'s
  overrides, which parse the raw characters); `pub(crate) fn finish(reader) -> Result<(), _>`
  (`CheckAdditionalContent`: only comments may follow the root).

- [ ] **Step 1: Write the failing tests:** `read_as_int32` on `5`, `"5"`, `0x10` → 5, 5, 16; `1.0` → `Input string
  '1.0' is not a valid integer.`; `2147483648` → `JSON integer 2147483648 is too large or small for an Int32.`; `null`
  → `None`. `read_as_string` on `5`, `1.5`, `true` → `"5"`, `"1.5"`, `"true"`; on `"2012-03-21T05:40:00Z"`, which
  `read` gives as a `Date`, → the text unchanged. `read_as_boolean` on `1`, `0`, `"false"` → true, false, false; `"yes"` → `Err`.
  `finish`: `{} // c` passes; `{} x` → `Additional text encountered after finished reading JSON content: x.` with its
  position; `{}{}` → `Err`.
- [ ] **Step 2: Run** `cargo test -p pinewatch-json --test read_as`; expect failures.
- [ ] **Step 3: Implement** by porting `ReadAsInt32`, `ReadAsString`, `ReadAsBoolean`, `ReadAsDouble`
  (`ReadNumberValue`, `ReadStringValue`, `ParseReadNumber`) and `JsonSerializer.Deserialize`'s additional-content
  loop. Confirm in `DefaultContractResolver` which types read with which `ReadType` (`Int32` → `ReadAsInt32`,
  `String` → `ReadAsString`, `Boolean` → `ReadAsBoolean`, `Double` → `ReadAsDouble`, the rest → `Read`) and write
  that table into the module docs.
- [ ] **Step 4: Run** the test; expect it to pass.
- [ ] **Step 5: Commit** `pinewatch-json: the typed reads and the root check`.

### Task 5: Values of type `object`

**Files:** Create `crates/pinewatch-json/src/value.rs`; Test `crates/pinewatch-json/tests/values.rs`

**Interfaces:**
- Produces: `JsonValue` (`Null`, `Integer(JsonInteger)`, `Float(f64)`, `String(NetString)`, `Boolean(bool)`,
  `Date(JsonDate)`, `Object(Vec<(NetString, JsonValue)>)` in order, `Array(Vec<JsonValue>)`, plus whatever the
  `JTokenWriter` copy can hold: `Undefined`, `Comment(NetString)`, `Constructor`); `read_object_value(reader) ->
  Result<Option<JsonValue>, MonoException>` (`CreateValueInternal` for `objectType == typeof(object)`: primitives as
  the boxed value, containers through `CreateJToken`, `null`/`undefined` as `None` at the top).

- [ ] **Step 1: Write the failing tests:** `{"a":1,"b":1.5,"c":"s","d":true,"e":null,"f":[1,{"g":2}],"h":
  "2012-03-21T05:40:00Z","i":123456789012345678901}` gives each type (the date as `Date`, `i` as `Big`); then the
  container quirks found while porting `JTokenWriter` (`WritePropertyName` with a repeated name, `WriteComment`
  inside an object and inside an array, `undefined` inside an array), each pinned with the result the source gives.
- [ ] **Step 2: Run** `cargo test -p pinewatch-json --test values`; expect failures.
- [ ] **Step 3: Implement** by porting `JsonSerializerInternalReader.CreateValueInternal`'s `object` path,
  `CreateJToken` and the parts of `JsonWriter.WriteToken`/`JTokenWriter`/`JContainer.Add` that it reaches.
- [ ] **Step 4: Run** the test; expect it to pass.
- [ ] **Step 5: Commit** `pinewatch-json: values of type object`.

### Task 6: Typed deserialization

**Files:** Create `crates/pinewatch-json/src/contract.rs` (the contract traits) and `src/deserialize.rs`
(`JsonSerializerInternalReader`'s object, array, list and dictionary paths, `EnsureType`); Test
`crates/pinewatch-json/tests/deserialize.rs`

**Interfaces:**
- Produces: `trait JsonObject: Sized { fn create() -> Self; fn members() -> &'static [&'static str]; fn
  member(&mut self, index: usize) -> Slot<'_>; }` (the default constructor with field initializers; the writable
  members in declaration order, `[JsonIgnore]` left out; their storage), `enum Slot<'a>` (`Bool`, `Int32`, `UInt16`,
  `UInt32`, `UInt64`, `Single`, `Guid`, `String(&mut Option<NetString>)`, `Enum(&mut dyn EnumSlot)`,
  `Object(&mut dyn ObjectSlot)`, `Struct(&mut dyn ObjectSlot)`, `Array(&mut dyn CollectionSlot)`,
  `List(&mut dyn CollectionSlot)`, `Dictionary(&mut dyn DictionarySlot)`, `Any(&mut Option<JsonValue>)`), and
  `deserialize_object<T: JsonObject>(text: &NetString) -> Result<Option<T>, MonoException>`
  (`JsonConvert.DeserializeObject<T>`). Reference-typed members are `Option`s, since JSON `null` sets them to null.

- [ ] **Step 1: Write the failing tests** on small test types that use every `Slot` kind:
  - names: an exact match first, then ignoring case (`use_legacy_ground` sets `Use_Legacy_Ground`); unknown members
    are skipped with their whole value; a `[JsonIgnore]` member is unknown.
  - `ObjectCreationHandling.Auto`: a list member from the constructor is appended to (two `Trains` keys → both
    entries kept); an array member is replaced; a dictionary member from the constructor is filled, a repeated key
    overwrites (`dictionary[key] = value`); a struct member is built new.
  - nulls: `null` sets a reference member to null and is an error for a value-type member.
  - conversions (`EnsureType`/`ConvertUtils.ConvertOrCast`, invariant culture): `float` from `1`, `"1,000"` and
    `1.5`; `ushort` from `70000` → error; `ushort` from `2.5` → 2 and `3.5` → 4 (`Convert.ToUInt16(double)` rounds
    to even); `ulong` from `18446744073709551615` (`Big`); enums from `"MISC"`, `"misc"`, `3` and `"7"`
    (`EnumUtils.ParseEnum`, numbers allowed); `Guid` from `"1b2a3d4c5e6f708192a3b4c5d6e7f809"`, an error for
    `"nope"`.
  - the root: `""` and `null` give `Ok(None)`; `[]` for an object type is an error.
- [ ] **Step 2: Run** `cargo test -p pinewatch-json --test deserialize`; expect failures.
- [ ] **Step 3: Implement** by porting `Deserialize`, `CreateValueInternal`, `PopulateObject`, `SetPropertyValue`
  with `CalculatePropertyDetails`, `CreateNewList`/`PopulateList`, `PopulateDictionary`, `EnsureType`, the
  `JsonPropertyCollection.GetClosestMatchProperty` lookup and `EnumUtils.ParseEnum`.
- [ ] **Step 4: Run** the test; expect it to pass.
- [ ] **Step 5: Commit** `pinewatch-json: typed deserialization`.

### Task 7: `LevelInfoConfigData` and `ReadLevelInfo`

**Files:**
- Modify: `crates/pinewatch-assets/src/level_info.rs` (the config types and `LevelInfo`), `src/search.rs`
  (`read_level_info`, `scan_known_levels`, `client_search_locations` report errors), `src/client.rs` (seeds
  `AssetLoader::errors`), `crates/pinewatch-assets/Cargo.toml` (`pinewatch-json`), `tests/search.rs` and
  `tests/install.rs` for the new parameter
- Test: `crates/pinewatch-assets/tests/level_info.rs`

**Interfaces:**
- Consumes: Tasks 1 to 6.
- Produces: `LevelInfoConfigData`, `LevelTrainAssociation`, `ArenaLoadout`, `ELevelWeatherOverride`,
  `ESingleplayerMapCategory` and the `AssetReference` struct contract (member `GUID`), each implementing `JsonObject`
  with the SDK's field initializers and constructor defaults; `LevelInfo { dir: MonoPath, name: String, file:
  LevelFile, editable: bool, config: LevelInfoConfigData }`; `scan_known_levels(install, root, log, reported: &mut
  Vec<String>) -> Result<Vec<LevelInfo>, _>` and `client_search_locations(install, origins, log, reported: &mut
  Vec<String>)`, where `reported` collects the `Assets.reportError` texts (logged as warnings too) that
  `load_client_assets` puts first in `AssetLoader::errors`; `should_use_holiday_redirects(config, active_holiday) ->
  bool` (`Level.updateCachedHolidayRedirects` outside the editor).

- [ ] **Step 1: Write the failing tests** with made-up levels:
  - a level without `Config.json` gets the constructor's defaults (`Version` `3.0.0.0`, `Gravity` -9.81, `Category`
    `MISC`, `Use_Legacy_Ground` true, …) and no message.
  - a valid config with a UTF-8 BOM sets its values and its hash (SHA-1 of the file).
  - `""`, `null`, `{"Gravity": "x"}` and 65 nested arrays under `Mode_Config_Overrides` each report `Unable to parse
    <name>/Config.json! Consider validating with a JSON linter` (through `Assets.reportError`: a warning and an
    `AssetLoader::errors` entry) and keep the defaults.
  - `{"Version": "1.2.x.4"}` reports `Unable to parse level "<name>" version "16908288". Expected format "#.#.#.#".
    Resetting to zero.` and leaves `Version` `0.0.0.0`; `{"Version": null}` reports `version "0"`.
  - editable: true with a `.unlocker` file, false without (no signed-in Steam user).
  - `should_use_holiday_redirects`: true only with `Allow_Holiday_Redirects` and a holiday other than `NONE`.
- [ ] **Step 2: Run** `cargo test -p pinewatch-assets --test level_info`; expect failures.
- [ ] **Step 3: Implement** the types and the full `ReadLevelInfo`: `Level.dat` through Task 2, then `Config.json` as
  bytes, decoded with `decode_stream_reader_default` and hashed whole, `deserialize_object`, the catch-all, the
  version check, and the outer `Caught exception reading level info file (…):`.
- [ ] **Step 4: Run** `cargo test --workspace`, `cargo clippy --workspace --all-targets`, `cargo xtask check-deps`.
- [ ] **Step 5: Records and commit.** `docs/VERIFICATION.md`: V23 becomes a check (the harness confirms the messages);
  a new entry for the game's own `Newtonsoft.Json.dll` build against 13.0.3; the date time zone gap from Task 3.
  `COMPATIBILITY.md` L11: levels are editable only through `.unlocker` without a Steam user. V14: only the active
  holiday stays open. `NOTICES-PORTED-CODE.md`, `docs/HANDOFF.md`, the README status. Commit `M2f task 7: level info`.
