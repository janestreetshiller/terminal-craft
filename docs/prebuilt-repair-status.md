# 0.10.0 source candidate — repair status

This page separates source repairs from publication and installed-app approval.
It is not a release announcement. The repository's all-rights-reserved terms
remain unchanged; `LICENSE` now records them explicitly. Import hashes and the
reproducible map generator preserve provenance, not a new third-party rights grant.

## Source repairs

- Five original templates, catalog, generator, both host pickers and regression
  tests are included together; no missing `include_bytes!` assets.
- Missing screenshot/report links and unsupported full-suite success claims
  were removed instead of replaced with fabricated evidence.
- CI runs all portable Python tests, including map and documentation contracts;
  native and installed tests require explicit opt-in and are reported as skipped.
- `scripts/verify.py` uses a separate source snapshot, offline dependency cache,
  target and test data. It records source/binary hashes and before/after source
  preservation. Native QA uses an exclusive owner lock; it is never run by default.
- Map regressions cover corrupt-save preservation, symlink/overwrite refusal,
  concurrent exports and temporary-file cleanup, in addition to asset and PTY QA.
- `scripts/repair_cli_links.py` repairs relocated aliases with timestamped
  adjacent backups, preflights ordinary files, and does not build or touch the app.
- Future installer runs embed `Resources/build-manifest.json` with relative
  source hashes and packaged executable/launcher hashes. This does not invent
  provenance for an already-installed binary.
- Public documentation's absolute user/workspace paths were redacted in the
  current tree without changing historical test results or imported file hashes.
  Git history was not rewritten; a fresh secret/rights review remains a release gate.

## Source-only verification — September 6, 2026 UTC

The [source-bound result summary](verification/prebuilt-worlds/source-report.json)
records a successful isolated offline build, formatting and Clippy checks,
**46 passing Rust tests** (one hardware benchmark ignored), **21 passing Python
tests** (three native/installed checks explicitly skipped), and four additional
slowed PTY regression passes. Source and snapshot input hashes stayed unchanged
during that run. Evidence documentation was added afterward and its link/path
contracts were rechecked separately. This is not a full GUI release pass.

An initial repair test exposed macOS's `/var` versus `/private/var` path alias;
the assertion now compares canonical paths, and the fresh full source run passed.
The three CLI aliases were repaired with timestamped backups and each passed
`--help` under a minimal shell environment. The installed binary was not changed.

## Native failure diagnosis and remaining gates

The retained September 5, 2026 full-suite run failed installed default-theme
native motion: no walking/turning/jump and no pointer lock. Its SDL event trace
records **FocusLost at frame 1**, before the scripted game selection at frame 6;
focus was not regained until frame 53, after the movement checks. This explains
why the early actions had no effect, but does not identify which external app
or operator took focus. Do not weaken focus-loss safety or count that run as a
pass. Coordinate exclusive GUI ownership, rerun the native and installed suites,
and review actual gameplay/picker screenshots before claiming closure.

Outstanding owner gates:

1. Exclusive desktop/app time for native QA and visual review; no automated
   pointer capture until approved. `python3 scripts/verify.py --gui` provides a
   fresh evidence directory and refuses an existing native-QA owner lock.
2. Installed build/source binding: the retained 0.10.0 binary has no complete
   contemporaneous source manifest. Approve a tested install and verify its new
   manifest/signature before asserting it corresponds to the repaired source.
3. Owner confirmation of imported-work/map rights and desired release scope.
   All rights reserved is not an open-source grant; dependencies keep their terms.
4. Explicit push permission, then successful hosted CI for the exact pushed
   commit. No 0.10.0 tag, GitHub Release, downloadable app/map archive, notarization
   or social posting is asserted or authorized by a local commit alone.

A successful source-only verifier report is useful evidence, not completion of
these gates. Hard-link export requires a supporting destination filesystem;
unsupported filesystems fail safely instead of using an overwrite fallback.
