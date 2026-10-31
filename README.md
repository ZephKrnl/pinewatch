# Pinewatch

**Pinewatch** is a free, non-commercial project based on the U3 SDK by Smartly Dressed Games Ltd. It is a Rust
implementation of a modified game based on the U3 SDK (a "Mod" as defined in the U3 SDK License Agreement).

**Trademark notice.** UNTURNED is a registered trademark of Smartly Dressed Games Ltd. References to UNTURNED in
this project do not constitute use as a trademark pursuant to the Trademarks Act, RSC 1985, c T-13, or applicable
trademark legislation in other jurisdictions; such references are solely for non-commercial, identification,
informational and descriptive purposes. Pinewatch is based on the U3 SDK. Pinewatch and its contributors are not
affiliated with or endorsed by Smartly Dressed Games Ltd., and do not make, use or sell any goods or services
provided by Smartly Dressed Games Ltd.

**License.** Copyright © 2026 Smartly Dressed Games Ltd. All rights reserved, unless expressly granted herein.
Pinewatch is provided free of charge and strictly on a non-commercial basis under the U3 SDK License Agreement
(`LICENSE.txt`), which also governs any use or further distribution by third parties. Third-party notices are in
`THIRDPARTYNOTICES.txt`. Pinewatch is for personal computers running Windows, macOS or Linux only. The license
prohibits use on gaming consoles, mobile or handheld devices, and through emulation, streaming, cloud or
subscription services.

**Game content.** Pinewatch ships no Unturned game content: no models, textures, sounds, maps or data files. It reads
that content from your own installation of Unturned and never modifies it.

## Status

Early development, milestone M1: Pinewatch boots, finds your Unturned installation and renders a map's terrain
from its landscape tiles. Terrain colors are temporary placeholders until M2 brings the original materials.
See `docs/ARCHITECTURE.md` for the milestones.

## Requirements

- Windows 10 or 11, x64, with a DirectX 12 or Vulkan capable GPU
- Rust 1.99.1 (pinned in `rust-toolchain.toml`) with the MSVC toolchain
- Unturned installed through Steam (Pinewatch reads it, never writes to it)

## Build and run

```
cargo run --release -p pinewatch -- --map PEI
```

Options:

| Option | Meaning |
|---|---|
| `--unturned <dir>` | Unturned install folder. Otherwise `PINEWATCH_UNTURNED_DIR`, then Steam's library folders |
| `--map <name>` | Map folder name under `Maps/` (default `PEI`) |
| `--screenshot <file.png>` | Save one frame after the terrain loads, then exit |

Controls: WASD to move, Q/E down/up, hold the right mouse button to look around, hold Shift to move faster.

## Tests

```
cargo test --workspace
```

Tests that read the real game files run when an install is found and are skipped otherwise.

## Repository

| Path | Contents |
|---|---|
| `crates/` | Pinewatch's Rust crates |
| `docs/` | Architecture, survey, format inventory, specs and plans |
| `COMPATIBILITY.md` | The 1:1 compatibility rules and every known limitation |
| `reference/u3-sdk` | The U3 SDK source, pinned to tag `v3.26.3.12` (reference only, never edited) |

This repository is local and private. Git is used for version control only; nothing is pushed anywhere.
