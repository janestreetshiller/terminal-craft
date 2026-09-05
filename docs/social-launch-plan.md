# Terminal Craft — social launch plan

Owner/voice: **janestreetshiller** (independent builder).
Status: **drafts only — not posted, not scheduled**. Day 1 means the day the owner chooses to post after public-repository verification. No Jane Street affiliation or endorsement is implied.

Repository: https://github.com/janestreetshiller/terminal-craft

## Positioning and claim boundaries

Native Rust voxel sandbox, v0.9.0: an SDL window with pointer-locked mouse look, plus optional Kitty graphics/ANSI terminal rendering, sharing one game engine and save format. Mine, place blocks, switch tools, fly in creative mode, and save worlds. No browser or Node runtime.

Local verification is macOS arm64. Do not claim Windows support, subjective control quality, measured display FPS, multiplayer, mobs/combat, or browser-game parity. CPU timings are not display FPS. This is a public source repository with the existing **all-rights-reserved** license, not an open-source release. Installation is from source; do not advertise a binary download.

## X launch post

```text
Terminal Craft v0.9.0: a native Rust voxel sandbox. Mine, build, fly, save. SDL pointer-locked mouse look, plus optional Kitty/ANSI terminal rendering. No browser or Node runtime.

Source: https://github.com/janestreetshiller/terminal-craft
```

Attach `screenshots/native-polish-live.png` as a historical gameplay screenshot, not a fresh performance benchmark. Alt text: “First-person voxel terrain with a sand-colored block held above a textured hotbar.”

## X technical thread

### Post 1

```text
1/3 Terminal Craft has two hosts: a native SDL window with pointer-locked mouse look, and optional Kitty/ANSI terminal rendering. One Rust game engine and save format underneath. No browser or Node runtime.
```

### Post 2

```text
2/3 Final macOS checks: 40 Rust tests and 6 Python tests passed. Scripted native-window QA exercised source and packaged builds; terminal PTY tests covered mining, placement and saves. Automated checks are not a substitute for play feel.
```

### Post 3

```text
3/3 I'd like feedback on movement: turning, stopping, jumping and terrain traversal. If something feels off, open an issue with your OS, native/terminal mode and a short repro.

https://github.com/janestreetshiller/terminal-craft
```

Post these as one reply chain, not three unrelated daily posts.

## LinkedIn post

```text
I've published the source for Terminal Craft v0.9.0, a native Rust voxel sandbox.

Mine, place blocks, switch between a pickaxe, axe and shovel, fly in creative mode, and save your world. It opens in a native SDL window with pointer-locked mouse look. Optional Kitty/ANSI terminal rendering shares the same engine and save format. No browser or Node runtime is required.

The final local macOS checks passed 40 Rust tests and 6 Python integration tests, including scripted native-window and terminal sessions. That verifies specific behaviors—not how the controls feel to everyone.

I'd especially like feedback on turning, stopping, jumping and terrain traversal. Source-build instructions and verification details are in the README. The source is public with the existing all-rights-reserved license.

https://github.com/janestreetshiller/terminal-craft
```

## Seven-day schedule

| Relative day | Channel/action | Asset or output |
|---|---|---|
| Day 1 | Publish X launch post after confirming the public URL and build instructions | Launch copy + gameplay still |
| Day 2 | Publish the full three-post X thread | Technical thread above; link verification report in a follow-up if asked |
| Day 3 | Record a short gameplay clip in a disposable world | Shot list below; review footage before posting |
| Day 4 | Publish LinkedIn post | LinkedIn copy + reviewed gameplay still or newly recorded clip |
| Day 5 | Ask for movement feedback on X | “Tried Terminal Craft? I'd like a short repro for any awkward turning, stopping or jumping. Include your OS and native/terminal mode. https://github.com/janestreetshiller/terminal-craft” |
| Day 6 | Triage actual replies and GitHub issues | Reproduce reports; no invented engagement or testimonials |
| Day 7 | Post a factual follow-up only if there are real results | Actual fixes/known issues; otherwise a simple invitation to try the source build |

## Planned clip — not yet recorded

Target: 20–30 seconds of actual native-window gameplay, cropped to the game only.

1. Establish terrain and show continuous mouse turning.
2. Walk and stop; jump over a small terrain change.
3. Equip the pickaxe, mine a block, then place a held block.
4. Save; show pause releasing the pointer.
5. End card: project name and repository URL.

Use a disposable `TERMINAL_CRAFT_SAVE`; never expose personal desktop content. No synthetic gameplay or FPS overlays implying benchmark results. Add captions and descriptive alt text where supported.

## Posting checklist

- Verify the public GitHub URL and use source-build instructions, not a nonexistent download link.
- Use reviewed game-only imagery. Existing screenshots are historical; don't present them as new captures.
- Keep “native window” and “terminal mode” distinct.
- Keep the all-rights-reserved license explicit when describing source availability.
- Confirm wording in the platform composer before posting; no automated posting is configured.
- Log actual post URLs and responses here only after publication and readback.

Drafting assistance: local Gemma CLI; edited and fact-checked against the repository and real verification output.
