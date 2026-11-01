# Notices for ported code

Pinewatch reproduces the behavior of the .NET class libraries that Unity's Mono compiles into the game's
`mscorlib.dll` (see `docs/VERIFICATION.md`, V0). Where a Rust file is a port of one of those source files, its
header says so, and the original's license notice is reproduced below.

The game code itself is ported from the U3 SDK, under the U3 SDK License Agreement in `LICENSE.txt`. The SDK's own
third-party notices are in `THIRDPARTYNOTICES.txt`, an unmodified copy of the SDK's file.

## .NET Foundation: corefx and corert

Ported from `mono/corefx` at `5b4edb9b0a721aed60c9c8ddeed441bdd7451e42` and `mono/corert` at
`11136ad55767485063226be08cfbd32ed574ca43`, the commits that Unity's Mono (`unity-2022.3-mbe`) pins.

| Rust file | Ported from |
|---|---|
| `crates/pinewatch-mono/src/parse_numbers.rs` | corefx `src/Common/src/CoreLib/System/ParseNumbers.cs` |
| `crates/pinewatch-mono/src/guid.rs` | corefx `src/Common/src/CoreLib/System/Guid.cs` |
| `crates/pinewatch-mono/src/number.rs` | corert `src/System.Private.CoreLib/shared/System/Number.Parsing.cs`; corefx `SByte.cs`, `Byte.cs`, `Int16.cs`, `UInt16.cs`, `Int32.cs`, `UInt32.cs`, `Int64.cs`, `UInt64.cs`, `Single.cs`, `Double.cs` |
| `crates/pinewatch-mono/src/number_to_double.rs` | corert `src/System.Private.CoreLib/src/System/Number.CoreRT.cs` (`NumberToDouble`) |
| `crates/pinewatch-mono/src/text.rs` | corefx `IO/StreamReader.cs`, `Text/UTF8Encoding.cs`, `Text/UnicodeEncoding.cs`, `Text/UTF32Encoding.cs` |
| `crates/pinewatch-mono/src/exception.rs` | corefx `ArgumentException.cs` (`Message`) and the exception classes' default messages |
| `crates/pinewatch-mono/src/chars.rs` | corefx `Char.cs` (`IsWhiteSpaceLatin1`) |
| `crates/pinewatch-mono/src/boolean.rs` | corefx `Boolean.cs` (`TryParse`, `TrimWhiteSpaceAndNull`) |
| `crates/pinewatch-mono/src/sr.rs` | corefx resource strings, as Mono's `mcs/class/corlib/corefx/SR.cs` carries them |

```
The MIT License (MIT)

Copyright (c) .NET Foundation and Contributors

All rights reserved.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Microsoft: .NET Framework reference source

Ported from `mcs/class/referencesource` in Unity's Mono at `b8eb56fab45dd52845fabba7b7ddc9a80b2b0498`.

| Rust file | Ported from |
|---|---|
| `crates/pinewatch-mono/src/number.rs` | `NumberFormatInfo.ValidateParseStyleInteger` and `ValidateParseStyleFloatingPoint` in `mscorlib/system/globalization/numberformatinfo.cs` |
| `crates/pinewatch-mono/src/sr.rs` | reference-source resource strings, as Mono's `mcs/class/corlib/ReferenceSources/SR.cs` carries them |
| `crates/pinewatch-mono/src/enums.rs` | `Enum.TryParse` and `TryParseEnum` in `mscorlib/system/enum.cs` |

```
The MIT License (MIT)

Copyright (c) Microsoft Corporation

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Mono runtime behavior

A few behaviors come from Mono's own runtime and build files in Unity's Mono (MIT): `Environment.NewLine` on Windows
(`mono/metadata/icall-windows.c`), runtime exceptions created with their default constructors
(`mono/metadata/exception.c`), and `SR.GetResourceString` returning its argument (`mcs/build/common/SR.cs`).
Pinewatch reproduces these behaviors; it contains no code copied from those files.

## Rust crates

External crates keep their own licenses (`cargo tree` lists them). Pinewatch is not distributed; a notice file for
the crates is generated before any build leaves this machine.
