# Terminal Craft — original map pack

Five original v2 TCRF worlds for Terminal Craft v0.10.0:

- `foundry.tcrf` — industrial service yard and tower
- `switchyard.tcrf` — freight yard and gantry
- `citadel.tcrf` — stone fort and keep
- `skybridge.tcrf` — elevated platforms and walkways
- `dune-outpost.tcrf` — desert compound and rooftops

These are not Minecraft schematics or downloaded MW2 maps. Foundry is an
original industrial arena, not an exact Rust recreation. All geometry is authored
for this project; no third-party map assets are included.

The app embeds these templates and provides a **Prebuilt worlds** picker. It
creates separate editable copies instead of modifying the templates or replacing
the normal world. For a downloaded `.tcrf`, set `TERMINAL_CRAFT_SAVE` to that file
before launch and choose **Continue**.

See [prebuilt-worlds.md](../../docs/prebuilt-worlds.md) in the same source
checkout for installation, save locations, controls and verification limits.
The 0.10.0 pack is a source candidate; no downloadable release is asserted.

`catalog.json` records SHA-256 hashes, spawn positions and geometry checks.
`scripts/build_maps.py` in the source repository reproduces the templates using
Python's standard library. The repository's **all-rights-reserved** license applies;
source availability does not grant permission to redistribute or relicense it.
See [LICENSE](../../LICENSE).
