# M2a: `.dat` reader Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Read every `.dat` and `.asset` file the game reads exactly as the game's `DatParser` does, with the value
helpers built on Mono's number, bool, GUID and enum parsing.

**Architecture:** `pinewatch-mono` gains every integer width, `float`/`double`, `bool`, `Guid.TryParse` and
`Enum.TryParse`, each ported from the file Unity's Windows `unityjit` corlib compiles (`docs/VERIFICATION.md`, V0).
A new `pinewatch-unity` holds the Unity value types the helpers return. A new `pinewatch-dat` ports `UnturnedDat`:
tokenizer, parser, the node tree (an arena, so metadata mode can link parents) and the value helpers. The reader-side
NUnit cases from `Assets/Tests/UnturnedDat` run as Rust table tests.

**Tech Stack:** Rust 1.99.1, edition 2024. No new production dependencies.

**Spec:** `docs/superpowers/specs/2026-11-01-file-format-readers-design.md` (sections 3, 4, 6), `COMPATIBILITY.md`,
`docs/VERIFICATION.md`.

## Where this plan sits

M2 (original content, approved 2026-11-01) is a series of plans. You review each one before its work starts.

| Plan | Delivers |
|---|---|
| **M2a `.dat` reader** (this plan) | Mono value parsing, `pinewatch-unity`, `pinewatch-dat` with the ported NUnit cases, the legacy line reader, an install-wide parse test, and the M1 review's housekeeping |
| M2b Reference harness | Unity 2022.3.62f3 batch-mode harness (`cargo xtask reference setup/run`), canonical output, `cargo xtask testdata`, runtime-table dumps; conformance for `pinewatch-mono` and `pinewatch-dat` on every copied `.dat`/`.asset`, closing the matching `VERIFICATION.md` entries |
| M2c Assets | `pinewatch-assets`: search locations and order, definition precedence, `MasterBundle.dat`, translations with English fallback, asset headers, the GUID and legacy-ID index with `AddToMapping`'s rules and messages, `UnturnedDatEx` references, holiday redirects (V14), `LandscapeMaterialAsset` |
| M2d Asset processor | `reference/processor` (C#, Unity batch mode) writes the textures that landscape materials use into the content cache (`%LOCALAPPDATA%\Pinewatch`) in their original GPU formats; `pinewatch-content` loads them |
| M2e Terrain materials | The landscape's original materials: splatmap blending, tiling and holes, compared with Unity captures |

`DateTime` parsing (only live-event config reads dates) and the writers (DAT writer, metadata-preserving writer,
editable nodes) come with the plans that first need them. Culture data comes from M2b's dumps, and M2c first uses
it: legacy asset IDs parse with the player's current culture (V13).

## Global Constraints

- Target U3-SDK `v3.26.3.12` (commit `aba323a`). C# paths below are relative to `reference/u3-sdk/Assets/Runtime/`.
- R1: all production code is Rust. R6: the install is never modified. R7: readers only.
- Parity rules P1 to P8 apply to every line. P3: where the C# can throw, the Rust returns `Result<_, MonoException>`.
- Mono behavior is ported from the file the Windows `unityjit` corlib build compiles (V0). Messages come from
  `pinewatch-mono/src/sr.rs`, and every new message is added to `sr::ALL`, which the `shipped_mscorlib` test checks.
- Format crates' production dependencies: `sha1`, `png`, `windows-sys` and their transitive dependencies only.
- No game content in the repository. Tests use synthetic inputs or ported NUnit cases. Install tests read the install
  read-only and assert properties, never committed content.
- Every ported behavior that neither the shipped IL nor a ported NUnit case confirms gets a `docs/VERIFICATION.md` entry.
- Branch `milestone/m2a-dat-reader` from `main`. Commits authored by Zeph (repository config). Nothing is pushed.
- Each new file that ports .NET or Mono code carries the header used in `pinewatch-mono` and is listed in
  `NOTICES-PORTED-CODE.md`.

## Review Focus

1. **Byte order marks and line breaks.** UTF-8 BOM files, the tokenizer's own `0xEF` check, and CR, LF or CRLF line
   ends must give the same keys and line numbers as the game (Task 6).
2. **Floats near rounding boundaries.** Long digit strings and values near `float` limits: `float` is the `(float)`
   cast of the `double` result, so double rounding must be reproduced (Task 3).
3. **Keys differing only by case,** including non-ASCII letters: the first spelling is kept, the last value wins and
   the error text is exact (Task 7).
4. **Truncated files.** A file ending inside a quoted string, an open `{` or `[`, or right after a key must produce the
   game's errors and tree (Task 7).
5. **Every real file.** All `.dat` and `.asset` files in the install parse without a panic, with a reused parser
   (Task 12).

---

### Task 1: Housekeeping from the M1 review

**Files:**
- Create: `xtask/Cargo.toml`, `xtask/src/main.rs`, `xtask/tests/dependency_allowlist.rs`, `.cargo/config.toml`
- Modify: `Cargo.toml` (members), `crates/pinewatch-io/src/install.rs`, `crates/pinewatch-io/tests/install.rs`,
  `crates/pinewatch-level/src/landscape.rs`, `crates/pinewatch-level/tests/tiles.rs`,
  `crates/pinewatch-level/tests/install.rs`, `crates/pinewatch-kvt/Cargo.toml`, `crates/pinewatch-kvt/tests/install.rs`,
  `docs/VERIFICATION.md` (V11)

**Interfaces:**
- Produces: `InstallRoot::read_prefix(&self, rel: &str, max_len: usize) -> io::Result<Vec<u8>>`, read-only.
- Produces: `cargo xtask check-deps`, which exits 0 when every crate in `FORMAT_CRATES` passes and lists each
  offending package otherwise. `ALLOWED_ROOTS = ["sha1", "png", "windows-sys"]`. `FORMAT_CRATES` starts as
  `pinewatch-mono`, `pinewatch-io`, `pinewatch-kvt`, `pinewatch-level`, and later tasks append to it.

- [ ] **Step 1: Write the failing tests**
  - `xtask/tests/dependency_allowlist.rs::format_crates_use_only_allowed_dependencies`: runs `cargo xtask check-deps`,
    asserts success.
  - `io/tests/install.rs::read_prefix_reads_at_most_max_len`: on a 10-byte file, `read_prefix(.., 4)` is the first
    4 bytes and `read_prefix(.., 100)` is all 10.
  - `level/tests/tiles.rs::message_root_is_absolute_and_normalized`: open a temporary install through
    `<temp>/<name>/../<name>`. The "missing heightmap file" message starts with the absolute path, contains no `..`,
    and still uses `\` separators with no trailing `\` (V11).
  - `level/tests/install.rs`: for every tile, the heightmap and splatmap hashes equal the SHA-1 of the whole file.
  - `kvt/tests/install.rs`: finds the install with `pinewatch_io::find_install(None)`, like the other install tests.
- [ ] **Step 2: Run** `cargo test --workspace`. Expected: the new tests fail (`read_prefix` and `check-deps`
  don't exist, the message root keeps `..`).
- [ ] **Step 3: Implement.**
  - `read_prefix`: open read-only and read at most `max_len` bytes.
  - The tile readers read only what the original consumes at most: 132,098 bytes per heightmap, 524,288 per
    splatmap, 8,193 per holes file.
  - The message root goes through `std::path::absolute` (`Path.GetFullPath`, as `DirectoryInfo.FullName` uses)
    before the existing V11 normalization. Update V11.
  - Each test that creates folders under `%TEMP%` uses a guard that removes the folder when dropped.
  - `check-deps`: list each crate's normal dependencies with `cargo tree -e normal --prefix none -p <crate>`. Every
    package not named `pinewatch-*` must appear in the same listing for one of `ALLOWED_ROOTS`.
  - `.cargo/config.toml` holds the alias `xtask = "run -q -p xtask --"`.
- [ ] **Step 4: Run** `cargo test --workspace` and `cargo xtask check-deps`. Expected: PASS, exit 0.
- [ ] **Step 5: Commit** "Housekeeping from the M1 review: dependency allowlist, prefix reads, temp cleanup".

### Task 2: Every integer width

**Files:**
- Modify: `crates/pinewatch-mono/src/number.rs`, `crates/pinewatch-mono/src/lib.rs`
- Test: `crates/pinewatch-mono/tests/integer_widths.rs`

**Interfaces:**
- Consumes: `NumberStyles`, `NumberFormat`, `try_parse_i32`/`try_parse_u32` and their validation (M1).
- Produces, same shape as `try_parse_i32`:
  ```rust
  pub fn try_parse_i8(s: Option<&NetString>, style: NumberStyles, info: &NumberFormat) -> Result<Option<i8>, MonoException>;
  // try_parse_u8 → u8, try_parse_i16 → i16, try_parse_u16 → u16, try_parse_i64 → i64, try_parse_u64 → u64
  ```

Sources: corefx `SByte.cs`, `Byte.cs`, `Int16.cs` and `UInt16.cs` narrow `Number.TryParseInt32`, each with its own
hex rule. `Int64.cs` and `UInt64.cs` call corert's `TryParseInt64`/`TryParseUInt64`: the integer-style fast paths,
`TryParseUInt64HexNumberStyle`, `NumberToInt64` and `NumberToUInt64`.

- [ ] **Step 1: Write the failing tests**
  - `widths_accept_their_range_only` (`NumberStyles::ANY`, invariant): i8 `"-128"`→-128, `"127"`→127, `"128"`→None;
    u8 `"255"`→255, `"256"`→None, `"-0"`→0, `"-1"`→None; i16 `"32768"`→None; u16 `"65535"`→65535;
    i64 `"-9223372036854775808"`→`i64::MIN`, `"9223372036854775808"`→None; u64 `"18446744073709551615"`→`u64::MAX`,
    `"18446744073709551616"`→None.
  - `hex_narrowing_follows_corefx` (`NumberStyles::HEX_NUMBER`): i8 `"80"`→-128, i8 `"100"`→None, u8 `"FF"`→255,
    i16 `"8000"`→-32768, u16 `"10000"`→None, i64 `"FFFFFFFFFFFFFFFF"`→-1, u64 `"10000000000000000"`→None.
  - `every_width_validates_the_style`: `NumberStyles::from_bits(0x400)` gives `Err` with kind `Argument` for each width.
- [ ] **Step 2: Run** `cargo test -p pinewatch-mono --test integer_widths`. Expected: FAIL (functions not defined).
- [ ] **Step 3: Implement** the six functions in `number.rs`, following the sources above.
- [ ] **Step 4: Run** `cargo test -p pinewatch-mono`. Expected: PASS.
- [ ] **Step 5: Commit** "pinewatch-mono: every integer width".

### Task 3: `float` and `double`

**Files:**
- Create: `crates/pinewatch-mono/src/number_to_double.rs`
- Modify: `crates/pinewatch-mono/src/number.rs`, `src/sr.rs`, `src/lib.rs`, `NOTICES-PORTED-CODE.md`, `docs/VERIFICATION.md`
- Test: `crates/pinewatch-mono/tests/floats.rs`

**Interfaces:**
- Produces:
  ```rust
  pub fn try_parse_f32(s: Option<&NetString>, style: NumberStyles, info: &NumberFormat) -> Result<Option<f32>, MonoException>;
  pub fn try_parse_f64(s: Option<&NetString>, style: NumberStyles, info: &NumberFormat) -> Result<Option<f64>, MonoException>;
  ```
- Produces: `NumberFormat` gains `positive_infinity_symbol` (`"Infinity"`), `negative_infinity_symbol`
  (`"-Infinity"`) and `nan_symbol` (`"NaN"`) in `invariant()`.

Sources:
- corefx `Single.cs` and `Double.cs`: the `TryParse` wrappers and their special-symbol handling.
- corert `Number.Parsing.cs`: `TryParseSingle`, `TryParseDouble`, `NumberBufferToDouble`.
- corert `src/System.Private.CoreLib/src/System/Number.CoreRT.cs`: the portable `NumberToDouble`, built on a 64-bit
  integer mantissa with no float arithmetic. The shipped IL has it as managed code.
- Reference source: `NumberFormatInfo.ValidateParseStyleFloatingPoint`.

`float` is the `(float)` cast of the `double` result and fails when that cast is infinite.

- [ ] **Step 1: Write the failing tests**
  - `exact_values` (`ANY`, invariant): `"0.5"`→0.5, `"1e3"`→1000.0, `"1,000.25"`→1000.25, `"(2)"`→-2.0, `" 3 "`→3.0,
    for both widths.
  - `negative_zero_is_normalized`: `"-0"` and `"-0.0"` give bit pattern 0 for both widths.
  - `overflow_fails_and_symbols_parse`: f64 `"1e400"`→None, f32 `"3.5e38"`→None; `"Infinity"`→+∞,
    `"-Infinity"`→−∞, `"NaN"` and `" NaN "`→NaN, for both widths.
  - `single_is_the_cast_of_double`: for `"0.1"`, `"16777217"`, `"3.4028235e38"`, `"1.17549435e-38"`, `"1e-46"`,
    `"0.000000000000000000000000000000000000000000000700649232162408535461864791644958065640130970938257885878534141944895541342930300743319094181060791015625"`:
    `try_parse_f32(s)` equals `try_parse_f64(s).map(|d| d as f32)`.
  - `hex_style_is_rejected`: `HEX_NUMBER` gives `Err` with kind `Argument` and the reference-source message (added to `sr.rs`).
- [ ] **Step 2: Run** `cargo test -p pinewatch-mono --test floats`. Expected: FAIL.
- [ ] **Step 3: Implement** `number_to_double` as a literal port of `NumberToDouble` and its helpers, then the two
  entry points. Add V16 to `VERIFICATION.md`: the float conversion is ported from source, and the harness checks it
  with a sweep of generated decimals.
- [ ] **Step 4: Run** `cargo test -p pinewatch-mono`. Expected: PASS, including `shipped_mscorlib` with the new message.
- [ ] **Step 5: Commit** "pinewatch-mono: float and double parsing".

### Task 4: `bool`, `Guid.TryParse` and `Enum.TryParse`

**Files:**
- Create: `crates/pinewatch-mono/src/boolean.rs`, `crates/pinewatch-mono/src/enums.rs`
- Modify: `crates/pinewatch-mono/src/guid.rs`, `src/lib.rs`, `NOTICES-PORTED-CODE.md`, `docs/VERIFICATION.md`
- Test: `crates/pinewatch-mono/tests/boolean.rs`, `crates/pinewatch-mono/tests/enums.rs`, `crates/pinewatch-mono/tests/guid.rs`

**Interfaces:**
- Produces:
  ```rust
  pub fn try_parse_bool(value: Option<&NetString>) -> Option<bool>;           // Boolean.TryParse
  impl Guid { pub fn try_parse(input: Option<&NetString>) -> Option<Guid>; } // GuidParseThrowStyle.None
  pub enum EnumUnderlying { I8, U8, I16, U16, I32, U32, I64, U64 }
  pub struct EnumInfo { pub underlying: EnumUnderlying, pub names: &'static [&'static str], pub values: &'static [u64] }
  pub fn try_parse_enum(info: &EnumInfo, value: Option<&NetString>, ignore_case: bool) -> Option<u64>; // raw bits
  pub trait NetEnum: Copy { const INFO: EnumInfo; fn from_bits(bits: u64) -> Self; }
  ```
- Decision for every later port: a C# enum becomes `#[repr(transparent)] pub struct Name(pub <underlying>)` with
  associated constants, because a C# enum holds any underlying value (`Enum.TryParse("7")` succeeds for an
  undefined 7). `EnumInfo` lists members in declaration order. `try_parse_enum` reproduces the lookup order of
  reference source `TryParseEnum` over Mono's `GetEnumData`.

Sources: corefx `Boolean.cs` (`TryParse`, `TrimWhiteSpaceAndNull`); corefx `Guid.cs` (`TryParse`); reference source
`enum.cs` (`TryParseEnum`, including the numeric path through `Convert.ChangeType`) and Mono's
`ves_icall_System_Enum_GetEnumValuesAndNames`.

- [ ] **Step 1: Write the failing tests**
  - `boolean.rs::bool_parsing`: `"True"`, `"true"`, `"TRUE"`→true; `" false "`, `"false\0\0"`→false;
    `"t"`, `"1"`, `""`→None; `None`→None.
  - `guid.rs::try_parse_never_fails_with_an_exception`: every N, D, B, P and X form from the existing tests→`Some`;
    `"0"`, `""`, `"   "`, the minus-sign case, the 9-digit X-form overflow and `"g0000000-0000-0000-0000-000000000000"`→None.
  - `enums.rs::enum_parsing` on a test enum with `A=0, B=1, C=4` and `U32` underlying: `("B", true)`→1,
    `("b", true)`→1, `("b", false)`→None, `"4"`→4, `"7"`→7, `"B, C"`→5, `" B "`→1, `"-1"`→None, `""`→None, `"X"`→None.
- [ ] **Step 2: Run** `cargo test -p pinewatch-mono`. Expected: FAIL.
- [ ] **Step 3: Implement.** If Mono's enum name order can't be settled from source, add V18 to `VERIFICATION.md`.
- [ ] **Step 4: Run** `cargo test -p pinewatch-mono`. Expected: PASS.
- [ ] **Step 5: Commit** "pinewatch-mono: bool, Guid.TryParse and Enum.TryParse".

### Task 5: `pinewatch-unity` value types

**Files:**
- Create: `crates/pinewatch-unity/Cargo.toml`, `crates/pinewatch-unity/src/lib.rs`, `crates/pinewatch-unity/tests/values.rs`
- Modify: `Cargo.toml` (workspace dependency), `xtask/src/main.rs` (`FORMAT_CRATES` += `pinewatch-unity`)

**Interfaces:**
- Produces, each `#[repr(C)]` and `Clone, Copy, Debug, Default, PartialEq` with exact per-component equality,
  as Unity's `Equals` compares:
  ```rust
  pub struct Vector2 { pub x: f32, pub y: f32 }
  pub struct Vector3 { pub x: f32, pub y: f32, pub z: f32 }
  pub struct Color { pub r: f32, pub g: f32, pub b: f32, pub a: f32 }
  pub struct Color32 { pub r: u8, pub g: u8, pub b: u8, pub a: u8 } // also Eq
  ```
  Also any conversion `UnityDatColorEx` calls (for example `Color` to `Color32`), ported from the managed code in the
  Unity install's `UnityEngine.CoreModule.dll`, checked with the IL dump the way `mscorlib.dll` was.

- [ ] **Step 1: Write the failing test** `equality_is_exact_per_component`: `Vector3(0.1, 0, 0)` differs from
  `Vector3(0.1000001, 0, 0)`; a NaN component makes a value unequal to itself. If a conversion is ported, add a test
  of its rounding at the midpoints, with expected values taken from its IL.
- [ ] **Step 2: Run** `cargo test -p pinewatch-unity`. Expected: FAIL (crate not defined).
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test -p pinewatch-unity` and `cargo xtask check-deps`. Expected: PASS.
- [ ] **Step 5: Commit** "pinewatch-unity: Vector2, Vector3, Color, Color32".

### Task 6: DAT tokenizer

**Files:**
- Create: `crates/pinewatch-dat/Cargo.toml`, `crates/pinewatch-dat/src/lib.rs`, `crates/pinewatch-dat/src/tokenizer.rs`,
  `crates/pinewatch-dat/tests/tokenizer.rs`
- Modify: `Cargo.toml`, `xtask/src/main.rs` (`FORMAT_CRATES` += `pinewatch-dat`)

**Interfaces:**
- Produces:
  ```rust
  pub enum DatTokenType { /* EDatTokenType's members: same names, same order */ }
  pub struct DatToken { pub token_type: DatTokenType, pub value: Option<NetString> }
  pub struct DatTokenizer { /* the C# fields */ }
  impl DatTokenizer {
      pub fn new() -> Self;
      pub fn tokenize(&mut self, input: &[u16]);
      pub fn read_token(&mut self) -> Option<DatToken>;
      pub fn error_messages(&self) -> &[NetString];
      pub fn debug_dump_tokens_to_string(&self) -> NetString;
  }
  ```

Source: `UnturnedDat/DatTokenizer.cs`. `Tokenize(TextReader)` works on characters already decoded by `StreamReader`,
so `tokenize` takes code units.

- [ ] **Step 1: Write the failing tests**
  - All 8 tests of `Assets/Tests/UnturnedDat/DatTokenizerTests.cs`, same names in snake_case, same assertions.
  - `utf8_bom_characters_are_skipped_one_by_one`: the units `EF BB BF` before `key value` give the same tokens as
    `key value`. A lone `EF` before `key value` is swallowed too, and so is `EF BB` (`SkipUtf8Bom` reads before
    comparing).
  - `cr_lf_and_crlf_each_count_one_line`: `"a 1\rb 2\nc 3\r\nd 4"` puts `d` on line 4 in its error messages and tokens.
- [ ] **Step 2: Run** `cargo test -p pinewatch-dat --test tokenizer`. Expected: FAIL.
- [ ] **Step 3: Implement** as a literal port.
- [ ] **Step 4: Run** `cargo test -p pinewatch-dat`. Expected: PASS.
- [ ] **Step 5: Commit** "pinewatch-dat: tokenizer".

### Task 7: Node tree and parser

**Files:**
- Create: `crates/pinewatch-dat/src/tree.rs`, `crates/pinewatch-dat/src/parser.rs`, `crates/pinewatch-dat/src/dump.rs`
- Test: `crates/pinewatch-dat/tests/parser.rs` (217 cases), `crates/pinewatch-dat/tests/parser_errors.rs` (39 cases)

**Interfaces:**
- Consumes: Task 6's tokenizer; `decode_stream_reader_default` (M1).
- Produces:
  ```rust
  pub struct DatTree;                       // arena; nodes addressed by NodeId
  #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub struct NodeId(u32);
  pub enum DatNodeType { Value, Dictionary, List }                       // EDatNodeType
  #[derive(Clone, Copy)] pub enum NodeRef<'t> { Value(ValueRef<'t>), Dictionary(DictRef<'t>), List(ListRef<'t>) }
  impl DatTree { pub fn root(&self) -> DictRef<'_>; }
  impl<'t> ValueRef<'t> { pub fn value(&self) -> Option<&'t NetString>; }
  impl<'t> DictRef<'t> {
      pub fn len(&self) -> usize;
      pub fn contains_key(&self, key: &str) -> bool;
      pub fn try_get_node(&self, key: &str) -> Option<NodeRef<'t>>;
      pub fn iter(&self) -> impl Iterator<Item = (&'t NetString, NodeRef<'t>)>; // insertion order
  }
  impl<'t> ListRef<'t> {
      pub fn len(&self) -> usize;
      pub fn try_get_node(&self, index: i32) -> Option<NodeRef<'t>>;
      pub fn iter(&self) -> impl Iterator<Item = NodeRef<'t>>;
      pub fn values(&self) -> impl Iterator<Item = ValueRef<'t>>;            // GetValues
  }
  impl NodeRef<'_> { pub fn debug_dump_to_string(&self) -> NetString; }
  pub struct DatParser { /* the C# fields, kept between parses (P2) */ }
  impl DatParser {
      pub fn new() -> Self;
      pub fn set_enable_metadata(&mut self, enable: bool);
      pub fn parse_text(&mut self, input: &NetString) -> DatTree; // Parse(TextReader)
      pub fn parse_str(&mut self, input: &str) -> DatTree;        // Parse(string)
      pub fn parse_bytes(&mut self, input: &[u8]) -> DatTree;     // Parse(byte[]): StreamReader decoding
      pub fn has_error(&self) -> bool;
      pub fn error_message(&self) -> Option<&NetString>;         // the first one
      pub fn error_messages(&self) -> &[NetString];
  }
  ```
- Keys compare with `StringComparer.OrdinalIgnoreCase`: code unit by code unit through
  `pinewatch_mono::chars::to_upper_invariant`. The first spelling is kept, and a duplicate replaces the value in place.
  Add V17 to `VERIFICATION.md`: `OrdinalIgnoreCase` above ASCII depends on Mono's case tables (V3).

Sources: `UnturnedDat/DatParser.cs`, `DatDictionary.cs`, `DatList.cs`, `DatValue.cs`, `DatNode.cs`, `DatNodeEx.cs`
(`DebugDumpToString`) and the `DebugDumpToStringBuilder` methods.

- [ ] **Step 1: Write the failing tests**
  - Every row of `DatParserTests.cs` and `DatParserErrorTests.cs`, as one table per C# test method with the same
    input strings and asserted values. Each table records its C# method name, and a test checks each file's row
    count (217 and 39).
  - `duplicate_key_with_other_case_keeps_first_spelling`: `"Key 1\nKEY 2\n\u{00E9}t\u{00E9} a\n\u{00C9}T\u{00C9} b"`
    gives two keys spelled `Key` and `été` with values `2` and `b`, and exactly the duplicate-key errors the C# message
    format produces.
  - `truncated_input`: a file ending inside a quoted string, after an unclosed `{`, after an unclosed `[`, and right
    after a key. Each gives the C# error messages in order and a tree with the values read so far.
- [ ] **Step 2: Run** `cargo test -p pinewatch-dat`. Expected: FAIL.
- [ ] **Step 3: Implement** the arena tree, the parser and `debug_dump_to_string`, as literal ports.
- [ ] **Step 4: Run** `cargo test -p pinewatch-dat`. Expected: PASS.
- [ ] **Step 5: Commit** "pinewatch-dat: node tree and parser".

### Task 8: Metadata mode

**Files:**
- Create: `crates/pinewatch-dat/src/comment.rs`
- Modify: `crates/pinewatch-dat/src/tree.rs`, `crates/pinewatch-dat/src/parser.rs`
- Test: `crates/pinewatch-dat/tests/metadata_inline_comments.rs` (18), `metadata_line_numbers.rs` (17),
  `metadata_node_paths.rs` (9), `metadata_prefix_comments.rs` (37)

**Interfaces:**
- Produces:
  ```rust
  pub struct DatComment { /* DatComment's fields */ }
  impl DatComment { pub fn join_lines(&self, separator: &str) -> NetString; }
  impl<'t> NodeRef<'t> {
      pub fn is_metadata_available(&self) -> bool;
      pub fn parsed_comment(&self) -> Option<&'t DatComment>;
      pub fn parent(&self) -> Option<NodeRef<'t>>;
      pub fn parsed_line_number(&self) -> Option<i32>;
      pub fn parsed_line_number_range(&self) -> Option<(i32, i32)>;
      pub fn node_path(&self) -> Option<NetString>;                         // TryGetNodePath
  }
  impl<'t> ValueRef<'t> { pub fn parsed_inline_comment(&self) -> Option<&'t NetString>; }
  impl<'t> DictRef<'t> { pub fn key_line_number(&self, key: &str) -> Option<i32>; }
  ```

Sources: `DatComment.cs`, `DatNodeWithMetadata.cs`, `DatDictionaryWithMetadata.cs`, `DatListWithMetadata.cs`,
`DatValueWithMetadata.cs`, the metadata paths of `DatParser.cs`, and `DatNodeEx.TryGetNodePath`.

- [ ] **Step 1: Write the failing tests**
  - Every row of the four `DatParserMetadata*Tests.cs` files, ported as in Task 7, with row-count checks.
  - `reused_parser_carries_leftover_root_comment_into_next_file` (spec section 6, P2): with metadata on, parse a file
    that ends with a root comment, then a file starting with `key value`. The second file's `key` has the first
    file's comment as its prefix comment.
  - `line_numbers_drift_after_multiline_quoted_strings` (spec section 6): in `"a \"x\ny\"\nb 1"`, `b` sits on
    physical line 3 but its parsed line number is 2. The parser counts line-break tokens, and the break inside
    the quoted string isn't one.
- [ ] **Step 2: Run** `cargo test -p pinewatch-dat`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test -p pinewatch-dat`. Expected: PASS.
- [ ] **Step 5: Commit** "pinewatch-dat: metadata mode".

### Task 9: Value helpers

**Files:**
- Create: `crates/pinewatch-dat/src/value_ex.rs`, `dictionary_ex.rs`, `list_ex.rs`, `structs.rs`, `types.rs`
- Modify: `docs/VERIFICATION.md` (V19)
- Test: `crates/pinewatch-dat/tests/values.rs` (168 − 6 rows), `dictionary.rs` (168 − 6), `list.rs` (6), `structs.rs` (30)

**Interfaces:**
- Consumes: Tasks 2 to 4 (Mono parsing), Task 7 (tree).
- Produces, methods named after `DatValueEx`, `DatDictionaryEx` and `DatListEx` in snake_case. Each `TryParseX`
  returns `Option` (or `Result<Option<_>, MonoException>` where the C# can throw), and each `ParseX` takes the
  default:
  ```rust
  impl ValueRef<'_> {
      pub fn is_value_null_or_empty(&self) -> bool;
      pub fn try_parse_int32(&self) -> Option<i32>;   pub fn parse_int32(&self, default: i32) -> i32;
      // int8, uint8, int16, uint16, uint32, int64, uint64, float, double, bool, guid: the same pair each
      pub fn try_parse_enum<T: NetEnum>(&self) -> Option<T>; pub fn parse_enum<T: NetEnum>(&self, default: T) -> T;
      pub fn parse_type(&self, resolver: &dyn TypeResolver, default: Option<TypeHandle>) -> Option<TypeHandle>;
  }
  impl DictRef<'_> {
      pub fn try_get_value(&self, key: &str) -> Option<ValueRef<'_>>;
      pub fn try_get_dictionary(&self, key: &str) -> Option<DictRef<'_>>;
      pub fn try_get_list(&self, key: &str) -> Option<ListRef<'_>>;
      pub fn try_get_string(&self, key: &str) -> Option<&NetString>;
      pub fn get_string(&self, key: &str, default: Option<&NetString>) -> Option<NetString>;
      // every ValueRef parse pair again, with a key first
  }
  impl ListRef<'_> { /* DatListEx's methods, index: i32 */ }
  pub trait DatParseable { fn try_parse(&mut self, node: NodeRef<'_>) -> bool; }
  // try_parse_struct / parse_struct / parse_list_of_structs / parse_array_of_structs on NodeRef, DictRef, ListRef
  pub trait TypeResolver { fn resolve(&self, assembly_qualified_name: &NetString) -> Option<TypeHandle>; }
  #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub struct TypeHandle(pub u32);
  ```
- `TryParseBool` checks the one-character forms (`y`, `t`, `1`, `n`, `f`, `0`, case-sensitive) before
  `Boolean.TryParse`. `ParseType` rejects names containing `\`, `:` or `/`, then asks the resolver. Add V19: type
  resolution needs the game's type registry, as in V10.

Sources: `UnturnedDat/DatValueEx.cs`, `DatDictionaryEx.cs`, `DatListEx.cs`, `DatNodeEx.cs`, `DatParseable.cs`,
`DatListValueEnumerator.cs`.

- [ ] **Step 1: Write the failing tests:** every row of `DatValueTests.cs`, `DatDictionaryTests.cs`,
  `DatListTests.cs` and `DatStructTests.cs` except the 12 `TryParseDateTimeUtc`/`ParseDateTimeUtc` rows (deferred
  with `DateTime` parsing), with row-count checks. The C# test enums become `NetEnum` types in the test files. The
  `ParseType` rows use a test resolver that knows exactly `System.Int32` and `SDG.Unturned.IDatValue`.
- [ ] **Step 2: Run** `cargo test -p pinewatch-dat`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test -p pinewatch-dat`. Expected: PASS.
- [ ] **Step 5: Commit** "pinewatch-dat: value helpers".

### Task 10: Unity value helpers

**Files:**
- Create: `crates/pinewatch-dat/src/unity_ex.rs`, `crates/pinewatch-dat/src/unity_color_ex.rs`
- Test: `crates/pinewatch-dat/tests/unity_ex.rs` (101), `crates/pinewatch-dat/tests/unity_color_ex.rs` (138)

**Interfaces:**
- Consumes: Task 5's types, Task 9's helpers.
- Produces (results as `Result<Option<_>, MonoException>` where the C# substring arithmetic can throw, for example
  `IndexOf` on a lone `(`, spec section 6):
  ```rust
  impl ValueRef<'_> { pub fn try_parse_vector2(&self) -> ...; pub fn try_parse_vector3(&self) -> ...;
                      pub fn try_parse_color32_rgb(&self) -> ...; pub fn try_parse_color32_rgba(&self) -> ...; }
  impl DictRef<'_> { /* the same, with a key; plus */
      pub fn legacy_parse_vector3(&self, key: &str) -> Vector3;
      pub fn legacy_parse_color(&self, key: &str, default: Color) -> Color;
      pub fn legacy_parse_color32_rgb(&self, key: &str, default: Color32) -> Color32; }
  // plus the Parse* forms with defaults, as in UnityDatEx and UnityDatColorEx
  ```

Sources: `UnturnedDat/UnityDatEx.cs`, `UnturnedDat/UnityDatColorEx.cs`.

- [ ] **Step 1: Write the failing tests:** every row of `UnityDatExTests.cs` and `UnityDatColorExTests.cs`, with
  row-count checks.
- [ ] **Step 2: Run** `cargo test -p pinewatch-dat`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test -p pinewatch-dat`. Expected: PASS.
- [ ] **Step 5: Commit** "pinewatch-dat: Unity value helpers".

### Task 11: Legacy line reader

**Files:**
- Create: `crates/pinewatch-dat/src/legacy.rs`, `crates/pinewatch-dat/tests/legacy.rs`

**Interfaces:**
- Produces `pub struct LegacyData` with a constructor from decoded text and the read methods of
  `Assembly-CSharp/Unturned/Files/Data.cs` in snake_case. Writers stay unported (R7).

- [ ] **Step 1: Write the failing tests** (synthetic):
  - `key_is_split_at_the_first_space`: `"Name Big Rock 2"` reads `Name` as `Big Rock 2`.
  - `keys_are_case_sensitive`: `name` is not `Name`.
  - `comments_only_at_line_start`: `"// x"` is skipped, `"Key value // x"` keeps `value // x`.
  - `duplicate_key_last_wins_with_error`: the C# error text and the last value.
  - `bool_forms`: `y`, `1` and `true` in any case read as true; anything else reads as false.
- [ ] **Step 2: Run** `cargo test -p pinewatch-dat --test legacy`. Expected: FAIL.
- [ ] **Step 3: Implement** as a literal port of the reading parts of `Data.cs`.
- [ ] **Step 4: Run** `cargo test -p pinewatch-dat`. Expected: PASS.
- [ ] **Step 5: Commit** "pinewatch-dat: legacy line reader".

### Task 12: Every installed file, and the docs

**Files:**
- Create: `crates/pinewatch-dat/tests/install.rs`
- Modify: `crates/pinewatch-dat/Cargo.toml` (dev-dependency `pinewatch-io`), `docs/VERIFICATION.md`,
  `NOTICES-PORTED-CODE.md`, `README.md` (status line)

- [ ] **Step 1: Write the failing test** `every_installed_dat_and_asset_file_parses` (skipped without an install):
  - Decode and parse every `*.dat` and `*.asset` under `Bundles/` and `Localization/` with one reused `DatParser`,
    as an asset worker does.
  - Assert at least 8,000 files and no panic. Print the number of files with errors and the first few messages to
    stderr only.
  - It fails first because nothing walks the install yet.
- [ ] **Step 2: Run** `cargo test -p pinewatch-dat --test install -- --nocapture`. Expected: FAIL.
- [ ] **Step 3: Implement** the walk with `InstallRoot::read_dir` and `read`. Then update the docs:
  `VERIFICATION.md` V16 to V19, the notices table for the new ported files, and the README status line for M2a.
- [ ] **Step 4: Run** `cargo test --workspace`, `cargo xtask check-deps` and `cargo clippy --workspace --all-targets`.
  Expected: all tests pass, no install test skipped, no new clippy warnings.
- [ ] **Step 5: Commit** "pinewatch-dat: every installed file parses; docs".
