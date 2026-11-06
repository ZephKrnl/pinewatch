"""Generates crates/pinewatch-mono/src/chars/category_table.rs from corefx's C# (mono/corefx 5b4edb9b).

Usage: python -I gen_category_table.py <corefx dir with Char.cs, CharUnicodeInfoData.cs, UnicodeCategory.cs> <out.rs>

- CATEGORY_RUNS: CharUnicodeInfo.GetUnicodeCategory(char) for U+0000..U+FFFF, computed by running
  InternalGetCategoryValue(ch, UNICODE_CATEGORY_OFFSET) on CharUnicodeInfoData's CategoryLevel1Index,
  CategoryLevel2Index (little-endian ushorts, read unaligned), CategoryLevel3Index and CategoriesValue, and
  compressed to runs of equal categories.
- CATEGORY_FOR_LATIN1: Char.s_categoryForLatin1, translated to numbers with UnicodeCategory.cs.
"""

import re
import sys
from pathlib import Path


def parse_tables(text):
    tables = {}
    for m in re.finditer(r"ReadOnlySpan<byte>\s+(\w+)\s*=>\s*new byte\[(\d+)\]\s*\{(.*?)\};", text, re.S):
        name, size, body = m.group(1), int(m.group(2)), re.sub(r"//[^\n]*", "", m.group(3))
        data = [int(x, 16) for x in re.findall(r"0x([0-9a-fA-F]{2})\b", body)]
        assert len(data) == size, (name, len(data), size)
        tables[name] = data
    return tables


def main():
    corefx, out = Path(sys.argv[1]), Path(sys.argv[2])
    text = (corefx / "CharUnicodeInfoData.cs").read_text(encoding="utf-8")
    assert text.startswith("#if !__MonoCS__"), "expected the csc-only corefx file"
    tables = parse_tables(text)
    l1, l2, l3, values = (tables[n] for n in ("CategoryLevel1Index", "CategoryLevel2Index", "CategoryLevel3Index",
                                               "CategoriesValue"))
    enum = dict((n, int(v)) for n, v in re.findall(r"^\s*(\w+)\s*=\s*(\d+),", (corefx / "UnicodeCategory.cs")
                                                    .read_text(encoding="utf-8"), re.M))
    assert list(enum.values()) == list(range(30))

    def category(ch):
        index = l1[ch >> 9]
        pos = (index << 6) + ((ch >> 3) & 0b111110)
        index = l2[pos] | (l2[pos + 1] << 8)
        index = l3[(index << 4) + (ch & 0x000F)]
        return values[index * 2 + 0]

    runs = []
    for c in range(0x10000):
        cat = category(c)
        assert 0 <= cat < 30
        if not runs or runs[-1][1] != cat:
            runs.append((c, cat))

    char_text = (corefx / "Char.cs").read_text(encoding="utf-8")
    body = re.search(r"s_categoryForLatin1\s*=\s*\{(.*?)\};", char_text, re.S).group(1)
    latin1 = [enum[n] for n in re.findall(r"\(byte\)UnicodeCategory\.(\w+)", body)]
    assert len(latin1) == 256

    lines = [
        "// Generated from corefx's `CharUnicodeInfo.GetUnicodeCategory(int)` and `InternalGetCategoryValue` with the",
        "// category tables of `src/Common/src/CoreLib/System/Globalization/CharUnicodeInfoData.cs`, and from",
        "// `s_categoryForLatin1` in `src/Common/src/CoreLib/System/Char.cs` (mono/corefx 5b4edb9b). Copyright (c) .NET",
        "// Foundation and Contributors. MIT license; see NOTICES-PORTED-CODE.md.",
        "",
        "//! The Unicode category of every UTF-16 code unit, as corefx's tables give it.",
        "//!",
        "//! `CharUnicodeInfoData` holds the categories of all code points in an 11:5:4 three-level table, which",
        "//! `InternalGetCategoryValue` reads (the second level as little-endian `ushort`s). `CATEGORY_RUNS` holds the",
        "//! result for U+0000 to U+FFFF, a lone surrogate included, as runs of equal categories.",
        "//! `CATEGORY_FOR_LATIN1` is `char`'s own table for U+0000 to U+00FF.",
        "",
        "/// `CharUnicodeInfo.GetUnicodeCategory(char)` as runs, sorted by code unit: each entry is the first code unit of",
        "/// a run and the `UnicodeCategory` value of every code unit up to the next entry. The first run starts at U+0000.",
        "pub(super) const CATEGORY_RUNS: [(u16, u8); %d] = [" % len(runs),
    ]
    for i in range(0, len(runs), 8):
        lines.append("    " + " ".join("(0x%04X, %2d)," % run for run in runs[i:i + 8]))
    lines += [
        "];",
        "",
        "/// `Char.s_categoryForLatin1`: the `UnicodeCategory` value `char` itself gives each code unit up to U+00FF.",
        "pub(super) const CATEGORY_FOR_LATIN1: [u8; 256] = [",
    ]
    for i in range(0, 256, 16):
        lines.append("    " + " ".join("%2d," % v for v in latin1[i:i + 16]) + " // %04X - %04X" % (i, i + 15))
    lines.append("];")
    for line in lines:
        assert len(line) <= 120, line
    out.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    print("wrote %s: %d runs, %d lines" % (out, len(runs), len(lines)))


main()
