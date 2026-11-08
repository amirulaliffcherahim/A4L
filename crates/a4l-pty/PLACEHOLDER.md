# a4l-pty — PLACEHOLDER (Milestone 3)

**Purpose:** pseudo-terminal manager with rolling buffers so command output never floods the model's context.

**Spec:** GOAL.md §4 (opencode pattern), §10 (M3).

## What to build
1. Spawn commands in a PTY; capture output into a fixed-size rolling buffer (head + tail, middle elided).
2. Full raw output saved to a log file; the model receives only the trimmed view plus a log reference.
3. Timeouts, kill on cancel, and exit-code capture (the gate runner in a4l-core consumes this).
4. Cross-platform behavior (Linux, macOS, Windows) where practical.

## Done when
- A command printing 100 MB of output returns a bounded summary and a full log path.
