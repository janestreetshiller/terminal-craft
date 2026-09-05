# Hosted CI timing regression

The [first public Actions run](https://github.com/janestreetshiller/terminal-craft/actions/runs/33947818109)
passed the Ubuntu job and all macOS Rust/build gates, but failed the macOS PTY test:
the first mined block was still present (`2 != 0`); the legacy session also observed
the controls overlay too early.

## Diagnosis and regression

The PTY harness sent mining input, waited a fixed 420 ms, then paused. The game loop
caps simulation `dt` at 50 ms; wall-clock time is not guaranteed simulation progress
when a hosted runner is descheduled. Other fixed waits also raced rendered output
and atomic saves.

The unchanged release binary was locally descheduled for 90 ms, then resumed for
35 ms, repeatedly. **Both original PTY tests failed the same `2 != 0` assertion.**
The same throttled run passed both tests after the harness was changed to wait for
observable state with eight-second deadlines.

- [Failure before the fix](verification/ci-timing/slow-pty-before.log)
- [Passing reproduction after the fix](verification/ci-timing/slow-pty-after.log)
- [Complete local reverification](verification/ci-timing/report.json)

No gameplay source, mining rates, inventory rules or binary changed. All original
block/tool/inventory/movement/save assertions remain. Enhanced input uses one held
press and explicit release; legacy input supplies repeats as a real terminal would.
The permanent regression command is:

```sh
cargo build --release --locked
python3 scripts/stress_terminal_session.py
```

This command only signals private child processes created for disposable PTY
worlds; it does not signal other game processes or touch normal saves. Both normal
and deliberately descheduled PTY sessions now run in the macOS/Ubuntu CI matrix.

The complete local verifier passed again after the fix: 40 Rust tests, 6 Python
integration tests, lint, format, release build, and native-window QA for release
and packaged binaries. The installed/release SHA-256 remains
`8c112f84af001c9cba0937ca2d6218df9165e98c434616c3b76a3b95f13e14ea`.

See the repository's Actions page for the hosted result for the current commit;
this document does not predeclare an in-flight CI run successful.
