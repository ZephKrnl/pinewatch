# Pinewatch architecture

Pinewatch is a 1:1 Rust port of Unturned, targeting U3-SDK `v3.26.3.12`. The compatibility rules are in
`COMPATIBILITY.md`. The format-reader rules are in `docs/superpowers/specs/2026-11-01-file-format-readers-design.md`,
and its parity rules P1 to P8 apply to **all** production code, not only to the readers.

## Principles

1. **Port, don't redesign.** Each C# system becomes a Rust module with the same responsibilities, data, update order
   and messages. Rust structure may differ only where behavior stays identical.
2. **Simulation runs in Unity space.** Positions, rotations and all game math keep Unity's conventions: left-handed,
   Y up, meters, Euler angles in degrees applied in ZXY order. `pinewatch-unity` reproduces Unity's math. Bevy is only the
   presentation layer: one boundary module converts Unity space into Bevy's right-handed space (negating Z) for
   rendering, audio and input.
3. **.NET semantics wherever C# had them.** Strings are UTF-16 (`NetString`), integers wrap like C#, collections
   iterate in Mono's order, numbers parse with Mono's rules.
4. **Original content, never shipped.** Pinewatch reads the player's own installation read-only. Unity bundle content
   is converted locally by the asset processor into a cache outside the installation.
5. **Playable early.** Work proceeds in vertical milestones. Every milestone runs and is tested before the next one starts.

## Crates by layer

| Layer | Crates | Role |
|---|---|---|
| Foundation | `pinewatch-mono`, `pinewatch-unity`, `pinewatch-io` | Mono class-library behavior; Unity value types and math; read-only install access, hashing, logs |
| Formats | `pinewatch-dat`, `pinewatch-kvt`, `pinewatch-json`, `pinewatch-binary` | The readers from the step 1 spec |
| Content | `pinewatch-level`, `pinewatch-assets`, `pinewatch-content` | Map layouts; asset definitions; converted Unity content (meshes, textures, materials, prefabs, audio, animations) |
| Engine | `pinewatch-engine` | A Unity-semantics runtime on Bevy: GameObject and Transform hierarchy, component lifecycle and script order, `Time`, coroutines, `Invoke`, `Random`, physics, input and audio bridges |
| Presentation | `pinewatch-render`, `pinewatch-ui` | Terrain, materials ported from Unturned's shaders, lighting, sky, weather, post-processing; the Glazier and Sleek UI |
| Game | `pinewatch-game` | Ported game systems (Provider, Level, Player, managers, items, zombies, vehicles…), structured like the C# |
| Network and platform | `pinewatch-net`, `pinewatch-steam` | NetPak, transports, RPC; Steamworks |
| App | `pinewatch` (binary) | Startup, command line, install discovery. Executable `pinewatch.exe`, shown as **Pinewatch** |
| Dev tools (never shipped) | `pinewatch-scan`, `pinewatch-conformance`, `xtask`, `reference/harness` (C#), `reference/processor` (C#) | Scanning, conformance, repository tasks, the Unity reference harness, the Unity asset processor |

Each crate is created when a milestone first needs it.
Format crates keep the step 1 dependency allowlist (`sha1`, `png`, `windows-sys`).
Engine, presentation and app crates may also depend on Bevy and, later, the physics and Steamworks bindings.

## Engine and rendering

- **Bevy** (current release) provides windowing, input, ECS scheduling, asset IO and the wgpu renderer.
- **Frame loop:** `pinewatch-engine` reproduces Unity's player loop in Bevy schedules, in Unity's order:
  - fixed-update steps from `Time.fixedDeltaTime`, with the `maximumDeltaTime` clamp
  - `Update`
  - coroutines and `Invoke` calls
  - `LateUpdate`
  - rendering
- **Rendering:** Unturned uses Unity's built-in render pipeline with custom shaders. Pinewatch ports those shading models
  to WGSL. Pixel-exact equality across GPUs isn't achievable (Unity itself differs between GPUs), so the target is matching
  captures from the Unity reference. Remaining gaps go in `COMPATIBILITY.md`. Early milestones may use placeholder
  shading, always labeled as temporary.
- **Physics:** Unity 2022.3 uses PhysX 4.1. The plan is PhysX 4.1 through Rust bindings, the closest match to the original.
  A spike in the physics milestone confirms it.
- **Audio:** chosen at the audio milestone. Unity's mixer behavior is reproduced on a Rust audio backend.

## Content pipeline

- **Data files** (`.dat`, `.asset`, map files, terrain tiles, `Level.hierarchy`, JSON) are read by Rust directly from the install.
- **Unity bundles** (`core.masterbundle`, map `.unity3d` files) go through the **asset processor**. It's a C# tool that runs
  inside Unity 2022.3.62f3 in batch mode, reads the player's install, and writes a content cache:
  - meshes, skins and animations as glTF 2.0
  - textures in their original GPU formats, bit for bit
  - materials and prefabs, with their serialized component fields, as JSON
  - audio as decoded PCM

  Rust loads only the cache. The processor is tooling and never runs while the game plays.

## Data locations

- **Install (read-only):** taken from `--unturned <dir>`, then the `PINEWATCH_UNTURNED_DIR` environment variable, then Steam's library folders.
- **Pinewatch data folder:** `%LOCALAPPDATA%\Pinewatch`. It holds the content cache, saves, settings and logs. Saves live
  here instead of inside the install (L3).

## Testing

- Unit tests in every crate, including ported NUnit cases and synthetic fixtures.
- Install tests that parse real files, skipped when no install is found.
- Boot tests: the app starts headless and loads a map. Screenshot runs give visual checks.
- Conformance against the Unity reference harness once Unity is installed. Until then, every behavior ported from source
  without harness confirmation is listed in `docs/VERIFICATION.md`.

## Milestones

| Milestone | Deliverable |
|---|---|
| **M1** | Bootable Pinewatch: a window titled Pinewatch that finds the install and renders a map's real terrain from its landscape tiles (placeholder colors), with a fly camera and a screenshot mode |
| M2 | Original content: Unity asset processor and cache; `.dat` parser and asset discovery; terrain with its original materials |
| M3 | The world: level objects, trees and foliage, roads, water, lighting, sky and fog from the map's data |
| M4 | Walking around: the Unity-semantics runtime, PhysX colliders, player movement and camera ported from `PlayerMovement` and `PlayerLook` |
| M5 | Survival core: assets and items, inventory, equipment, useables, HUD and menus (Glazier), singleplayer save and load |
| M6 | Living world: zombies, animals, vehicles, barricades and structures, day and night, weather |
| M7 | Multiplayer: NetPak, transports, server and client `Provider`, dedicated server |
| M8 | Everything else: NPCs, quests and dialogue, crafting, skills, workshop content, the level editor |

## License constraints (summary)

Free and non-commercial. PC only (Windows, macOS, Linux). No Unturned content in the repository or in builds.
The README carries the required attribution. Details are in `docs/PORT_SURVEY.md` and `COMPATIBILITY.md`.
