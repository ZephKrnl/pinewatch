# Rust port of the U3 SDK: survey and roadmap

Surveyed 2026-10-31 from `SmartlyDressedGames/U3-SDK` at commit `aba323a` (tag `v3.26.3.12`, 2026-09-21).
The SDK source will be added as a git submodule at `reference/u3-sdk`, pinned to that tag (see the step 1 spec).
It is read-only reference material, not part of the Rust code.

## License rules the port must follow

From the SDK's `LICENSE.txt`:

- **Free and non-commercial.** No sales, paid tiers or subscriptions (s.1c, s.2d).
- **PC only:** Windows, macOS, Linux (s.1).
- **No consoles, mobile, handheld devices, emulation, streaming or cloud play** (s.2). This covers the Unity version too.
  Porting either version to mobile ends the license automatically and immediately (s.7), for all of your work, including this Rust port.
  Avoid adding Steam Deck support for the same reason (it is a handheld device).
- **Ship the copyright notice, `LICENSE.txt` and `THIRDPARTYNOTICES.txt` with every build** (s.1a, s.1b).
  Anyone else must use the port under the same agreement (s.1d), so it can't be relicensed (no MIT or GPL).
- **Don't name it "Unturned".** You may say it is based on Unturned, with the disclaimers from s.5
  (not trademark use, not affiliated with or endorsed by Smartly Dressed Games, no disparagement).
- **Game content is a separate matter.** The license covers the source repo. The models, textures, sounds
  and maps in the Steam folder are not part of it. The port reads them from each player's own install
  and never ships copies. (This is my reading, not legal advice.)

## What's in the source

| Area | Files | Lines |
|---|---:|---:|
| All C# in the repo | 1,957 | 432,062 |
| Game runtime (`Assets/Runtime`) | 1,775 | 389,368 |
| Steamworks.NET bindings (replaced by the `steamworks` crate) | 107 | 29,521 |
| Unity editor tools | 48 | 6,400 |
| Tests (NetPak, UnturnedDat) | 26 | 6,697 |

Main game folder (`Runtime/Assembly-CSharp/Unturned`, 301k lines), largest parts:

| Subsystem | Lines | Subsystem | Lines |
|---|---:|---|---:|
| UI | 45,543 | Provider (game/session core) | 14,561 |
| Bundles (94 asset types) | 41,355 | Sleek (widget library) | 10,510 |
| Player | 32,923 | Settings | 8,822 |
| Managers (items, vehicles, barricades…) | 28,307 | Tools | 8,106 |
| Level | 24,637 | Edit (level editor) | 7,574 |
| Interactable | 17,933 | Command | 6,223 |
| Useable (guns, melee, consumables) | 17,540 | Files (binary readers) | 6,024 |

Also: `Framework` 19.4k (Devkit, Foliage, Landscapes, IO), `NetGen` 19.4k (generated network code),
three UI backends ~11k, network transports ~2.7k.

Largest single files: `Provider.cs` 7.2k, `UseableGun.cs` 6.6k, `InteractableVehicle.cs` 6.4k,
`VehicleManager.cs` 3.7k, `Zombie.cs` 3.6k, `BarricadeManager.cs` 3.6k.

### How much depends on Unity

952 of 1,775 runtime files (54%) touch `UnityEngine`. Specific APIs:

| Unity feature | Files | Rust replacement |
|---|---:|---|
| MonoBehaviour subclasses | 180 | Bevy components + systems |
| GetComponent | 178 | ECS queries |
| Object.Instantiate | 62 | Spawning prefabs/scenes |
| Physics queries (raycasts, overlaps) | 52 | Rapier or Avian |
| Resources.Load | 43 | Asset loader |
| AudioSource | 33 | Bevy audio |
| Rigidbody | 28 | Rapier or Avian |
| Animation / Animator | 19 | Bevy animation |
| CharacterController | 11 | Kinematic character controller |
| Unity Terrain | 6 | Custom heightmap terrain |
| WheelCollider (vehicles) | 3 | Custom raycast wheels |
| Steamworks API | 289 | `steamworks` crate |

Already free of Unity: `SystemEx` (0 of 17 files), `SDG.NetTransport` (0 of 5),
`UnturnedDat` (4 of 26), `SDG.NetPak` (only its Unity/Steam extension files).

### Existing splits that make porting easier

- **UI:** `SDG.Glazier` is a UI layer (`IGlazier`, `ISleekElement`, `ISleekButton`…) with three backends
  (uGUI, UI Toolkit, IMGUI). One Rust backend lets the game's UI code be ported on top of it.
- **Networking:** `SDG.NetTransport` defines `IServerTransport` / `IClientTransport` / `ITransportConnection`,
  with Steam, plain-socket and loopback backends. Singleplayer already runs a local server over loopback,
  so the port should keep the server/client split from day one.
- **Generated code:** `NetGen` is machine-generated serialization; Rust derive macros can replace it.

## Game content (read from the Steam install)

Installed at `C:\Program Files (x86)\Steam\steamapps\common\Unturned` (2.1 GB).

- `Bundles/`: 8,163 `.dat` and 774 `.asset` files. Text definitions for items, vehicles, zombies and so on,
  in the format `UnturnedDat` parses. Rust can read these directly.
- `Bundles/core.masterbundle`: one 112 MB Unity bundle (UnityFS v8, Unity 2022.3.62f3) holding the models,
  textures, sounds, prefabs and animations. **This is the hard part.**
- `Maps/<name>/`: binary `.dat` level data (read by `River`/`Block`), raw terrain heightmap and splatmap tiles,
  `Foliage.blob`, plus small Unity bundles for roads, ambience and terrain materials.
  10 maps installed, including PEI, Washington, Russia, Germany and Yukon.

### Getting models, textures and sounds out of Unity's format

To be decided in step 2's design:

- **A. Native Rust reader** for Unity bundles. Players need no Unity, but it is the most work:
  the meshes, textures, audio and animation formats all need decoding.
- **B. One-time converter.** An editor script inside the U3-SDK Unity project exports the bundle to
  open formats (glTF, PNG, OGG, JSON) in a cache on the player's PC. Unity does the decoding,
  but Unity 2022.3.62f3 must be installed.
- **C. B first, A later.** Gameplay work starts sooner, and players stop needing Unity once A lands.

Header check: the bundle's index is LZ4HC-compressed (flags `0x243`), so the outer container is easy to open.
Whether it includes type information (which makes generic reading much easier) is still unknown.

## Proposed order

Each step gets its own design, spec, plan and build.

1. **File-format readers.** Every text format (`.dat`, the old line format, the hierarchy format, JSON), the binary readers,
   every map file layout, terrain tiles, foliage, navmesh and asset discovery. Readers only: each writer comes with the step
   that first needs it, under the same 1:1 rules. Done when the Unity reference harness shows identical results for every
   game file and generated input. No engine needed.
2. **Content pipeline.** Get meshes, textures and sounds out of `core.masterbundle` (A, B or C above).
3. **World viewer.** A Bevy app that loads a map (terrain, splatmaps, objects, foliage, lighting) with a free-fly camera.
4. **Core netcode.** NetPak bit packer, transport layer, and a local server/client over loopback like the original.
5. **Player.** Movement, collision, first- and third-person camera, animations.
6. **Items.** Asset types, inventory, equipment, useables.
7. **World simulation.** Zombies, animals, vehicles, barricades and structures, day/night, weather.
8. **UI.** A Rust Glazier backend, then menus and HUD.
9. **Online multiplayer.** Your own dedicated servers over Steam networking.
   It won't connect to official servers, which run BattlEye anti-cheat.
10. **Later.** Quests and NPC dialogue, crafting, commands, mod hooks, level editor (~13k lines; skip at first).

## Decisions so far

- **1:1 observable behavior** with the original C# code. Rules and every known limitation are in `COMPATIBILITY.md`.
- **Reference environment:** Unity 2022.3.62f3, used only as the reference runtime and for asset processing. All production code is Rust.
- **Target source version:** 3.26.3.12. The install stays untouched.
- **Step 1 scope:** readers only.
- **Name:** Pinewatch.
  - Lowercase `pinewatch` for the repository and crate names (`pinewatch-<role>`) and the executable file (`pinewatch.exe` on Windows).
  - `Pinewatch` for the window title and every displayed executable name (Windows product name and description, shortcuts, macOS app name).
- **README:** carries the attribution and all disclaimers the U3 SDK license requires (section 1: copyright notice,
  license text, third-party notices, free and non-commercial terms; section 5: not trademark use, accurately based on
  the U3 SDK, not affiliated with or endorsed by Smartly Dressed Games, no disparagement). Exact wording goes in the step 1 spec.
- **Git:** local and private, at `C:\Pinewatch`. No remotes and no pushes. Commits are authored as Zeph, configured in this repository only.

## Tentative tech choices (each confirmed in its step's design)

- Engine: Bevy 0.20 (its ECS maps well onto MonoBehaviour-style code).
- Physics: Rapier or Avian, decided in step 5.
- Steam: `steamworks` 0.13.
- Rendering: Bevy's PBR plus custom WGSL shaders for the game's look (the repo has 70 Unity shaders as reference).

## Environment

- Rust 1.99.1 and Git LFS 3.7.1 are installed.
- The repository sets `core.longpaths true`, because some SDK file paths are long.
