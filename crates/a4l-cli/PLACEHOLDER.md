# a4l-cli — PLACEHOLDER (Milestone 1 MVP, TUI later)

**Purpose:** the `a4l` binary. Thin front end over `a4l-core`. Also the headless runner for CI.

**Spec:** GOAL.md §5, §10 (M1).

## What to build
1. `clap` command surface: `a4l init`, `a4l doctor`, `a4l run --task <id>`, `a4l run --watch tasks.json`, plain-text `a4l status`.
2. `a4l init`: scaffold `.a4l/` in the current project from `templates/` (A4L.md, init.sh, tasks.json, progress.md, roles, policies, skills, docs/testing.md). Never overwrite existing files without asking.
3. `a4l doctor`: check providers/API keys, Bun/Node (for TS extensions), git, MCP servers. Missing runtime = warning, not failure.
4. HITL prompt in the terminal: show diff + L1–L4 proof, then Approve & Commit / Send Back with notes.
5. Headless mode: no prompts; `ask` actions pause and exit with a clear code (never auto-approve).
5a. Render `ask_user` questions in plain text (and later the TUI); render `/todos`. Distinguish them from permission `ask` prompts.
5b. M2: `a4l serve` starts the headless daemon from `a4l-server` (GOAL §5.1).
6. LATER (not M1): `ratatui` TUI with live gate logs and role visualization.

## Do NOT
- Put business logic here. If it decides a transition, it belongs in a4l-core.

## Done when
- `a4l init && a4l run --task T-0001` runs a sample task through the full loop with the mock provider.
