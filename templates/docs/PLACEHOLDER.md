# templates/docs — PLACEHOLDER (Milestone 1)

**Purpose:** conditional domain docs loaded only when a role/task needs them (progressive disclosure, GOAL.md §2.1).

## What to build
- `testing.md`: the L1–L4 ladder rules shown to the Maker/Checker when running or verifying code.
  - L1 static (`cargo check`, `tsc --noEmit`, `eslint`), L2 unit tests tied to acceptance criteria, L3 smoke (boots, zero startup crashes), L4 behavioral (live API curl, Playwright, e2e CLI).
  - State plainly: only a harness-observed exit `0` counts; a ticked checkbox does not; `tests/**` is read-only for Maker.
  - Keep it short; the model reads it every time.

Note: root `.gitignore` must use `/docs/` (not `docs/`) so this folder is tracked.
