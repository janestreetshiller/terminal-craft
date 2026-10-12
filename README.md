# Terminal Craft

Native Rust voxel sandbox — a single codebase that runs either in a pointer-locked native SDL window or in a terminal (Kitty/ANSI) host. Both hosts share the same game engine, renderer, repository and save format.

Repository: [janestreetshiller/terminal-craft](https://github.com/janestreetshiller/terminal-craft)

![Terminal Craft voxel terrain and held block](docs/screenshots/native-polish-live.png)

Short status

- Native window with real pointer-locked mouse look; terminal rendering remains available as an option.
- Source distribution is published here with an "all rights reserved" license — publication does not change those terms.

Quick links

- Launch (installed app): `/Applications/Terminal Craft.app`
- CLI: `terminal-craft` (aliases: `terminalcraft`, `termcraft`)
- Separate browser game: `block-craft` / `/Applications/Block Craft.app` — not included in this repository.

Suggested repository topics (GitHub tags)

- rust — primary implementation language
- voxel — voxel-based terrain and building
- sandbox — single-player sandbox gameplay
- sdl — native host uses SDL
- kitty-terminal — optional terminal host using Kitty graphics transport
- ansi — ANSI half-block terminal fallback
- macos — currently packaged/installed workflow targets macOS
- procedural-generation — seeded procedural terrain
- singleplayer — no multiplayer in this release
- game — game/project

(Apply these in the repository Settings → Topics.)

Highlights

- Shared engine and save format across native and terminal hosts.
- First-person view with held tools and block placement.
- Five curated prebuilt worlds embedded as templates.
- Atomic save format (v1/v2 readable) with deterministic map generation.

Features

- Seeded voxel terrain with trees and six themed regions.
- First-person arm/hand, held blocks, pickaxe/axe/shovel tools with material-specific speeds.
- Movement: walking, normalized diagonal move, sprint, sneak (lowered view), jump, creative flight.
- Rendering: 70-degree interaction rays, mining cracks, textured HUD/hotbar, debug/FPS overlay.
- UI: Gilded (default) and Classic skins; live switching (F6).
- Optional Kitty transport and ANSI fallback for terminal hosts.

Five prebuilt worlds (0.10.0 candidate)

Choose "Prebuilt worlds" from the main menu: Foundry, Switchyard, Citadel, Skybridge, or Dune Outpost. Each is an original, fixed arena with its own editable/resumable save; templates are embedded and not downloaded community maps.

See docs/prebuilt-worlds.md for details and verification limits. Quick launch example: `terminal-craft --map foundry`.

Gilded UI

The default Gilded skin uses gold-and-bronze pixel frames, cream inventory slots, and a resource/recipe field guide. It changes only appearance, not gameplay or saves. Switch to Classic with F6 or `TERMINAL_CRAFT_UI=classic`.

Motion and input (0.9.0+)

- Native SDL window with relative mouse input and true pointer lock. Vertical aiming reaches near-vertical; turning does not stop at a window edge.
- Simultaneous movement, looking, jumping and mining. Native keys remain held until released (no terminal timeout).
- Kitty mode negotiates press/repeat/release reporting; a verified input probe is used for robust input handling.

This project focuses on keyboard + mouse navigation for a bounded single-player sandbox. Mobs, combat, and multiplayer are not present in this release.

Controls (summary)

- WASD: move (diagonal normalization)
- Mouse / arrows: look (native mouse uses relative motion)
- Space: jump / ascend in flight
- Ctrl: sprint
- Shift / Z: sneak (slow, lowered view) / descend in flight
- Hold left mouse / E / F: mine / swing
- Right mouse / Q / Tab: place selected block
- 0: empty hand
- 7/8/9: pickaxe / axe / shovel
- 1–6: select hotbar block
- Wheel / `[` / `]`: cycle blocks
- H / I: controls, inventory and recipes (pauses)
- M: overhead map (pauses)
- C: toggle creative mode
- G / double Space: toggle flight (creative)
- F3: debug info
- F6: switch Classic/Gilded UI
- F11: native fullscreen
- R: save
- Esc: close overlays or pause and release pointer
- Ctrl-Q / Ctrl-C: save and quit

Note: Terminal mode cannot provide native pointer lock or native-window fullscreen. Kitty supports reliable key release; other terminals may fall back to a 400 ms hold behavior.

Build, test and install

Requirements: Rust 1.88+, Cargo, CMake, C/C++ compiler, macOS SDK command-line tools. Python 3 is used for packaging and integration tests. SDL is built and statically linked by Cargo; the installed native app needs neither Homebrew SDL nor Kitty.

Clone and run checks

```sh
git clone https://github.com/janestreetshiller/terminal-craft.git
cd terminal-craft
cargo test --locked
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
python3 scripts/install_macos.py
python3 scripts/verify.py
```

Run locally or via installed app

```sh
terminal-craft                       # native window, Gilded UI (default)
TERMINAL_CRAFT_UI=classic terminal-craft  # Classic UI
terminal-craft --gilded              # explicitly select Gilded
terminal-craft --terminal            # dedicated Kitty window
terminal-craft --here                # use existing terminal
terminal-craft --help
terminal-craft --check-save /path/to/world.tcrf
cargo run --release -- --native      # native host directly from source
```

Packaging and installer notes

- `bin/terminal-craft` builds the release binary if missing.
- The installer builds into a private temporary target, backs up previous app bundles, and embeds a source/binary SHA-256 manifest.
- To repair CLI aliases without rebuilding, run `python3 scripts/repair_cli_links.py`.

Saves and configuration

Default save: `~/.local/share/terminal-craft/world.tcrf` (or `$XDG_DATA_HOME/terminal-craft/world.tcrf`). A legacy native save at `~/.local/share/tuicraft/world.tcrf` is copied on first launch when no new-name save exists.

Environment variables

- `TERMINAL_CRAFT_UI=classic|gilded`: initial UI style
- `TERMINAL_CRAFT_SAVE`: explicit world path
- `TERMINAL_CRAFT_SENS`: mouse sensitivity (default: `0.0024`)
- `TERMINAL_CRAFT_QUALITY=native`: full-resolution world sampling
- `TERMINAL_CRAFT_ASCII=1`: ASCII/text fallback in terminal mode
- `TERMINAL_CRAFT_KITTY`: optional terminal-host executable

Legacy aliases: `TUICRAFT_SAVE`, `TUICRAFT_ASCII`, `TUICRAFT_SENS` are retained as fallbacks.

Provenance and verification

- `docs/source-import.json` records imported files and SHA-256 hashes. Historical snapshots were retained during import to preserve provenance.
- Documentation and verification reports are in `docs/` and `docs/verification/`.
- See `docs/public-launch-verification.md` and `docs/prebuilt-repair-status.md` for candidate/release verification details.

Documentation

See `docs/README.md` for the full documentation index (maps, verification, provenance, and publication drafts).

License

This repository is published under an "all rights reserved" policy. Publication here does not change licensing terms; any licensing changes are deferred to the owner.

If you want this converted to an open-source license (MIT/Apache/CC0, etc.), tell me which license to apply and I can prepare a LICENSE file and update headers.

What I changed

- Rewrote and reorganized the top-level README for clarity and quick scanning.
- Added a recommended list of GitHub repository topics (tags) and where to apply them.
- Kept all technical details, links and documentation pointers intact.

Commit and PR

- Commit message: "docs: refresh README copy, add recommended repo topics"
- PR title/summary suggestion: "Improve README copy and recommend repository topics/tags"

If you'd like, I can:

- Update `docs/README.md` with matching concise copy.
- Open a pull request with these changes instead of committing directly to the default branch.
- Apply the suggested GitHub topics via the API or show the exact UI steps.
