//! Every resource string Pinewatch puts in an exception message is a string literal in the installed game's
//! `mscorlib.dll`. This catches messages taken from source files that Unity's Mono doesn't compile.
//! The install is only read. The test is skipped when no install is found.

use std::collections::HashSet;

use pinewatch_io::find_install;
use pinewatch_mono::sr;

fn u16_at(d: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([d[at], d[at + 1]]))
}

fn u32_at(d: &[u8], at: usize) -> usize {
    u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]]) as usize
}

/// The string literals of a .NET assembly: the entries of its `#US` metadata heap (ECMA-335 II.24.2.4).
fn user_strings(d: &[u8]) -> HashSet<String> {
    let pe = u32_at(d, 0x3C);
    let section_count = u16_at(d, pe + 6);
    let optional_size = u16_at(d, pe + 20);
    let optional = pe + 24;
    let directories = optional + if u16_at(d, optional) == 0x10B { 96 } else { 112 };
    let section_table = optional + optional_size;
    let offset = |rva: usize| -> usize {
        for i in 0..section_count {
            let s = section_table + i * 40;
            let (virtual_size, va, raw_size, raw) = (u32_at(d, s + 8), u32_at(d, s + 12), u32_at(d, s + 16), u32_at(d, s + 20));
            if va <= rva && rva < va + virtual_size.max(raw_size) {
                return rva - va + raw;
            }
        }
        panic!("RVA {rva:#x} is outside every section");
    };
    let cli = offset(u32_at(d, directories + 14 * 8));
    let metadata = offset(u32_at(d, cli + 8));
    let mut p = metadata + 16 + u32_at(d, metadata + 12);
    let stream_count = u16_at(d, p + 2);
    p += 4;
    for _ in 0..stream_count {
        let (start, size) = (u32_at(d, p), u32_at(d, p + 4));
        p += 8;
        let name_end = p + d[p..].iter().position(|&b| b == 0).expect("stream name");
        let name = &d[p..name_end];
        p = (name_end + 4) & !3;
        if name == b"#US" {
            return heap_entries(&d[metadata + start..metadata + start + size]);
        }
    }
    panic!("no #US heap");
}

fn heap_entries(heap: &[u8]) -> HashSet<String> {
    let mut strings = HashSet::new();
    let mut i = 1; // Entry 0 is the empty string.
    while i < heap.len() {
        let b = usize::from(heap[i]);
        let (length, header) = if b & 0x80 == 0 {
            (b, 1)
        } else if b & 0xC0 == 0x80 {
            (((b & 0x3F) << 8) | usize::from(heap[i + 1]), 2)
        } else {
            let rest = (usize::from(heap[i + 1]) << 16) | (usize::from(heap[i + 2]) << 8) | usize::from(heap[i + 3]);
            (((b & 0x1F) << 24) | rest, 4)
        };
        let start = i + header;
        if length > 0 {
            // UTF-16 code units, then one flag byte.
            let units: Vec<u16> =
                heap[start..start + length - 1].as_chunks::<2>().0.iter().map(|&c| u16::from_le_bytes(c)).collect();
            strings.insert(String::from_utf16_lossy(&units));
        }
        i = start + length;
    }
    strings
}

#[test]
fn every_resource_string_is_a_literal_in_the_shipped_mscorlib() {
    let Ok(install) = find_install(None) else {
        eprintln!("skipped: no Unturned install found");
        return;
    };
    let dll = install.read("Unturned_Data/Managed/mscorlib.dll").expect("mscorlib.dll");
    let literals = user_strings(&dll);
    assert!(literals.len() > 1000, "found only {} literals; the heap reader is broken", literals.len());
    let missing: Vec<&str> = sr::ALL.iter().copied().filter(|s| !literals.contains(*s)).collect();
    assert!(missing.is_empty(), "not string literals in the shipped mscorlib.dll: {missing:#?}");
}

#[test]
fn messages_from_uncompiled_sources_are_absent() {
    let Ok(install) = find_install(None) else {
        eprintln!("skipped: no Unturned install found");
        return;
    };
    let literals = user_strings(&install.read("Unturned_Data/Managed/mscorlib.dll").expect("mscorlib.dll"));
    // Mono's own ParseNumbers.cs and the resource keys are not what Unity compiles (docs/VERIFICATION.md, V0).
    for absent in [
        "Could not find any parsable digits.",
        "Additional unparsable characters are at the end of the string.",
        "Format_GuidInvLen",
        "Object reference not set to an instance of an object",
    ] {
        assert!(!literals.contains(absent), "{absent:?} is a literal after all");
    }
}
