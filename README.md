# Terminal Craft

A native Rust voxel sandbox rendered **inside a terminal**. The macOS app opens
its own kitty window. No Node.js, web server, browser, Chrome profile, or relay
is involved in this game.

**Canonical repository:** `/Users/main/Metal/terminal-craft`

**App:** `/Applications/Terminal Craft.app`
**Command:** `terminal-craft` (`terminalcraft` and `termcraft` are compatibility aliases).

The separate browser game is now **Block Craft**, launched by `block-craft` or
`/Applications/Block Craft.app`. It is not bundled into this repository.

## Feel and performance in 0.8.1

This release adds a corrected 70-degree camera, procedural material textures,
target outlines/mining cracks, textured hotbar icons, and eased tool motion.
Gameplay targets a 60 FPS frame budget. Balanced rendering caps world raster
work while keeping the hand and HUD at native resolution; use
`TERMINAL_CRAFT_QUALITY=native terminal-craft` for full-resolution world rendering.

See [the measured polish report](docs/polish.md) for before/after images,
CPU benchmark results, methodology, limitations and the reproduction command.
The previous release verification is retained in `docs/verification.md`.

## Native features

- Seeded voxel terrain, trees, six language-themed regions, and resource mining.
- Visible first-person arm and hand; distinct pickaxe, axe, shovel, and held blocks.
- Hit/swing animation, walking bob, hold-to-mine progress, and material-specific tool speeds.
- Building with six block slots; resource-based CORE, CONDUIT, and BEACON recipes.
- Survival/resource mode and creative building/flight. Tools are available in both modes.
- Inventory/recipe/help panel, overhead map, hotbar counts, and debug/FPS display.
- Collision, movement, jump, mouse/arrow look, keyboard-only mining/building.
- Native kitty graphics with shared-memory transport and direct transport fallback.
- ANSI half-block fallback with a visible title menu, hand/tools, and text help.
- Atomic v2 saves with world, resources, selected block/tool, player position,
  view direction, creative mode, and flight state. Original v1 worlds remain readable.

This is a bounded native sandbox release, **not browser feature parity or a
complete Minecraft implementation**. Browser-only mobs, health/combat systems,
multiplayer, filesystem bridges, and browser inventory/character systems have
not been ported. No such feature is represented as complete here.

## Controls

| Input | Action |
|---|---|
| WASD | Move |
| Mouse / arrows | Look |
| Space | Jump / ascend in flight |
| Hold left mouse / E / F | Mine or swing |
| Right mouse / Q / Tab | Place selected hotbar block |
| 0 | Empty hand |
| 7 / 8 / 9 | Pickaxe / axe / shovel |
| 1–6 | Select and hold block |
| Mouse wheel / `[` / `]` | Cycle blocks |
| H / I | Controls, resources, recipes; freezes movement/mining |
| M | Overhead map |
| C | Toggle creative mode |
| Double Space | Toggle flight in creative mode |
| Z / Shift | Descend in flight |
| F3 | Debug information |
| R | Save |
| Esc | Close help/map; otherwise save and quit |
| Ctrl-C | Save and quit |

Keyboard-only hold detection uses repeat events and a short timeout where the
terminal does not supply key-release events. Mouse-hold mining is continuous.
Native rendering is supported on Unix terminals; kitty is the recommended viewer.

## Build, test, install

Requirements: Rust 1.88+ and Cargo; Python 3 for installation/integration tests;
kitty for the graphical app. Tests and packaging do not require browser services.

```sh
cargo build --release --locked
cargo test --locked
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
python3 tests/test_terminal_session.py
python3 scripts/install_macos.py
python3 tests/test_install.py
```

Run `bin/terminal-craft` from any checkout; it builds when no release binary is
present. After source changes, explicitly rebuild or run the installer. The
macOS bundle embeds the compiled binary and terminal configuration, so it does
not depend on this repository's path at runtime. The installer repairs CLI
symlinks after a repository move and archives old app launchers.

```sh
terminal-craft --help
terminal-craft --version
terminal-craft --here
terminal-craft --check-save /path/to/world.tcrf
```

## Saves and compatibility

Default: `~/.local/share/terminal-craft/world.tcrf` (or `$XDG_DATA_HOME/terminal-craft/world.tcrf`).
On first launch, the old native save at `~/.local/share/tuicraft/world.tcrf` is
**copied**, never moved or overwritten. Existing new-name saves take precedence.
Browser profiles and browser worlds remain untouched under their existing paths.

`TERMINAL_CRAFT_SAVE` selects an explicit world file; `TERMINAL_CRAFT_ASCII=1`
forces text fallback; `TERMINAL_CRAFT_SENS` adjusts mouse sensitivity;
`TERMINAL_CRAFT_KITTY` selects a kitty executable. The old `TUICRAFT_SAVE`,
`TUICRAFT_ASCII`, and `TUICRAFT_SENS` names remain fallback aliases.

The format signature remains `TCRF` for compatibility. A v2 save cannot be read
fully by the old native build; the original v1 file is retained for recovery.

## Source provenance

`docs/source-import.json` records every imported source file and its SHA-256.
The retained Rust source had no Git metadata; commit `77e60e9` preserves the
unmodified import. The original snapshot remains under
`~/Code/omarchy-craft/projects/TUICraft/current` for history only. Do not use it as
a second active native checkout. No unrelated Git history was replaced.

This repository is local. It has not been published or pushed to a hosting service.
