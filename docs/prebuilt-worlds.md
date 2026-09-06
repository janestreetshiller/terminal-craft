# Original prebuilt worlds — 0.10.0 source candidate

Open **Prebuilt worlds** from Terminal Craft's native or terminal menu.
Each entry creates a separate editable save on first play and resumes that save
on later visits. The templates are embedded in the executable; no runtime
download, Minecraft installation, importer, or repository checkout is needed.

| Map | ID | Layout |
|---|---|---|
| Foundry | `foundry` | Industrial tower, service shed, tanks and crates |
| Switchyard | `switchyard` | Freight cars, rail lanes and an overhead gantry |
| Citadel | `citadel` | Stone keep, gate passages, corner towers and ramparts |
| Skybridge | `skybridge` | Elevated walkways, a central pavilion and satellite platforms |
| Dune Outpost | `dune-outpost` | Desert courtyards, roof terraces, canopies and a watchtower |

These are **original layouts made for Terminal Craft**, not downloaded maps.
Foundry takes inspiration from industrial arena settings; it is **not a remake
of MW2 Rust**. No Call of Duty geometry, textures, branding, or community-build
assets are bundled. The pack retains the repository's all-rights-reserved license.

## Play and save

```sh
terminal-craft --maps
terminal-craft --map foundry
terminal-craft --here --map citadel
terminal-craft --terminal --map skybridge
```

All five start in creative mode with a placed player spawn. **C** switches the
resource-mode rules; **G** enables creative flight. These are sandbox worlds for
exploring, mining and building—not shooter levels with combat or multiplayer.

For the normal `world.tcrf` save, the editable map copies live under:

```text
~/.local/share/terminal-craft/world.tcrf.prebuilt/
  foundry.tcrf
  switchyard.tcrf
  citadel.tcrf
  skybridge.tcrf
  dune-outpost.tcrf
```

The path is derived from the resolved base save: `<base-save-path>.prebuilt/<id>.tcrf`.
`TERMINAL_CRAFT_SAVE` and `XDG_DATA_HOME` therefore also isolate the map collection.
The existing base world is not replaced by selecting a prebuilt world. In the
native host, **Save and main menu** lets you select another map. **Continue** still
means the normal base world; reopen a prebuilt through the picker to resume it.

A new app version does not reset your map edits. A corrupt existing map save is
reported as an error, not silently replaced with a fresh template.

For a pristine extra copy, export to a **new, nonexistent** destination:

```sh
terminal-craft --export-map foundry /tmp/foundry-fresh.tcrf
TERMINAL_CRAFT_SAVE=/tmp/foundry-fresh.tcrf terminal-craft
```

Then select **Continue**. Export refuses to overwrite an existing file. This does
not reset the picker save. There is no destructive reset button in this release.

## Layouts and provenance

Every template is a fixed **96×96 footprint, 40-block-high** authored world, not a
new random seed standing in for a map. The source definitions are in
[`scripts/build_maps.py`](../scripts/build_maps.py); the ordinary v2 TCRF assets,
SHA-256 hashes, spawns and checked landmarks are in
[`assets/maps/catalog.json`](../assets/maps/catalog.json).

```sh
python3 scripts/build_maps.py --check  # verify exact reproducibility; no writes
python3 scripts/build_maps.py          # developer: rebuild bundled templates
```

Rebuilding templates changes future fresh copies, not existing player saves.
The builder checks two-block standing clearance and connected, single-block-step
routes from the spawn to the listed checkpoints. That geometric reachability
check is not an exhaustive player-physics simulation of every stair and roof.

## Verification and release status

This is a **source candidate**, not an assertion that a 0.10.0 GitHub release,
notarized app, or downloadable map archive exists. See the
[repair status](prebuilt-repair-status.md) for source-bound checks and remaining
gates. Hosted CI applies only to its recorded commit, not a dirty checkout.

The asset tests check deterministic geometry, spawn clearance and catalog hashes.
CLI tests exercise the actual save parser, byte-exact exports and refusal to
replace existing files. Private PTY tests select, move, save and directly resume
all five maps without replacing the base world. The Rust tests cover isolated
map saves and error handling.

Native QA exists in `tests/test_prebuilt_native.py` and
`tests/test_native_window.py`, but requires explicit exclusive GUI approval.
It captures the pointer and exercises focus/fullscreen. A retained September 5,
2026 full-suite run failed the installed app's default-theme native motion check;
passing map-specific reports do not supersede that failure. No current all-pass
native/installed or visual claim is made. Screenshots will be added only after
real, source-bound capture and review; missing placeholder links were removed.

Export uses an atomic hard link and refuses existing destinations, including
symlinks. The destination's parent must exist and its filesystem must support
hard links. Unsupported filesystems return an error; they do not fall back to
unsafe overwrite. Existing corrupt map saves are retained and reported, never
silently reset. No exhaustive physics, long-play or cross-filesystem guarantee
is made. Social material remains draft-only.
