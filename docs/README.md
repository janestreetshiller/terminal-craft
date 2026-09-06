# Terminal Craft documentation

## Play and configure

- [Main README](../README.md): requirements, source build, installation, controls,
  save locations and configuration.
- [Five prebuilt worlds](prebuilt-worlds.md): map catalog, picker/direct launch,
  separate editable saves, pristine exports and deterministic asset generation.
- [Gilded UI](gilded-ui.md): default skin, Classic selection and live switching.
- [Native motion](motion.md): pointer lock, movement and focus-loss behavior.

## Verification and provenance

- [Current repair and test status](prebuilt-repair-status.md): what passed,
  historical native results, and the distinction between source verification
  and installed-app or release certification.
- [Source verification results](verification/prebuilt-worlds/source-report.json):
  source/binary hashes, 46 Rust passes, 21 Python passes, three explicit skips,
  and four slowed PTY regression passes.
- [Historical native map results](verification/prebuilt-worlds/historical-native-report.json):
  prior successful five-map checks on both release and installed binaries,
  alongside the unsuccessful overall suite result. These are retained evidence,
  not new runs or a source manifest for the repaired commit.
- [Source import record](source-import.json): retained original-file hashes.
- [Existing license notice](../LICENSE): unchanged; the owner has deferred any
  licensing-policy change. Documentation work does not depend on changing it.

### Reuse evidence before running tests again

The linked reports document completed work. Updating documentation does not
require reinstalling the app or rerunning pointer-capturing GUI tests.

For later code changes, `python3 scripts/verify.py` runs source checks in a fresh
snapshot with separate offline dependency cache, build output and disposable
saves. Dependencies must already be cached. Its JSON report identifies the
source inputs, binary and scope; native/installed skips are not counted as passes.

Only if new native/installed verification is needed, coordinate exclusive desktop
use first and run `python3 scripts/verify.py --gui`. This is an opt-in operation
that captures the pointer and changes window state; it is not a documentation step.
Neither verifier mode publishes anything or installs a new app.

## Troubleshooting

| Symptom | Check / action |
|---|---|
| CLI command stopped working after moving the checkout | Run `python3 scripts/repair_cli_links.py` from the checkout. It repairs the three aliases with adjacent timestamped backups and refuses to replace ordinary files. |
| `--maps` is unavailable | Check `terminal-craft --version`. The launcher can use an older retained binary; editing source alone does not rebuild or reinstall it. |
| A map reopens with previous edits | Expected: picker saves are separate and resumable. Use `--export-map ID NEW_PATH` for a pristine copy rather than deleting a save. |
| Export fails | Use a nonexistent destination under an existing directory on a filesystem supporting hard links. Existing files and symlinks are deliberately refused. |
| Existing map save is corrupt | The game reports an error and retains the file. Preserve a backup; exporting a pristine copy to a different path does not recover old edits. |
| Movement or pointer lock pauses after switching windows | Expected safety behavior: focus loss clears held input and pauses. Return to the game and explicitly resume. |
| A test report says “skipped” | That behavior was not exercised in that run. Consult the recorded scope and historical results; do not count a skip as a pass. |

## Historical work and publication drafts

- [0.9.0 public-launch verification](public-launch-verification.md)
- [Hosted CI timing regression](ci-timing-verification.md)
- [Native polish](polish.md)
- [Five-world social drafts](social-prebuilt-worlds.md)
- [Historical launch drafts and optional schedule](social-launch-plan.md)

Historical reports apply to their recorded builds, not automatically to the
current checkout. Social documents remain drafts. A local commit or passing
test does not assert that a GitHub tag, Release, app download or post exists.
