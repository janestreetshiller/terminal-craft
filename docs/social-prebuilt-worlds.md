# 0.10.0 — five-world candidate drafts (not approved for posting)

**Drafts only. Nothing posted or scheduled.** Owner: janestreetshiller; no Jane
Street or Activision affiliation is implied. The repository is public source under
all-rights-reserved terms, installed from source on macOS. No 0.10.0 release or
map archive is asserted to exist. Native/installed QA remains a release gate. Use the existing seven-day schedule only after choosing a
posting day and checking the release URL.

## X post

```text
Terminal Craft v0.10.0 adds 5 original worlds: Foundry, Switchyard, Citadel, Skybridge and Dune Outpost. Pick a map, build, save, come back. Embedded for offline play. Gilded UI by default.

Source: https://github.com/janestreetshiller/terminal-craft
```

Before posting, capture and review actual installed-app screenshots from the
approved build. No five-world screenshot attachment is available yet.
Alt text: “Five first-person voxel worlds: an industrial yard, freight tracks,
a stone fort entrance, elevated platforms and a desert courtyard.”

## Three-post technical thread

```text
1/3 Terminal Craft v0.10.0: five original sandbox maps in one Rust/SDL app, with optional Kitty/ANSI terminal mode. Foundry is an industrial arena, not an MW2 Rust remake. No downloaded community maps or external map loader.
```

```text
2/3 Each map has a separate editable save. Reopening it keeps your changes; your normal world stays separate. The templates are embedded for offline play. Export a fresh copy with --export-map; it refuses to overwrite existing files.
```

```text
3/3 Each template is reproducible from the included Python generator. Source tests cover exports, map saves and terminal sessions. Native and installed-app release QA still needs approval and completion. Public source, all rights reserved—not open source.
```

Reply to the thread with the repository and `docs/prebuilt-worlds.md` links.
Do not advertise combat, multiplayer, a notarized app download or measured FPS.

## Five-shot clip — not recorded yet

Record actual game-only footage in disposable map saves. About five seconds per
map: Foundry's tower and tanks; Switchyard's rail lanes; Citadel's gate and keep;
Skybridge's elevated paths; placing a block in Dune Outpost. Finish by returning
to the map picker. Review the recording before posting; do not substitute a
synthetic gameplay clip or an unverified performance overlay.

## Publishing checklist

- Close the gates in [candidate status](prebuilt-repair-status.md), then obtain
  explicit release and posting approval. Confirm the actual release/tag and CI
  status before changing this copy to announce availability.
- Use the current five-map screenshots, not the older terrain-only launch still.
- Say **original maps**, not “MW2 Rust included” or “downloaded maps.”
- Link source-build instructions; distinguish a map archive from an app binary.
- Keep the existing seven-day schedule as guidance, not a live scheduled campaign.
- Record real post URLs only after posting and readback.

Drafting assistance: local Gemma CLI; wording edited and checked against actual
repository behavior and verification output.
