# A4L — Agent Handoff

Read `GOAL.md` first (the spec). Then pick up the folder you were assigned: each has a `PLACEHOLDER.md` saying what to build, what not to do, and what "done" means. Delete a folder's `PLACEHOLDER.md` once real code replaces it.

## Build order
1. `crates/a4l-core` — state machine, guards, gate runner, breaker, mock provider, assembler, handoff, token ledger (M1)
2. `crates/a4l-policy` — allow/ask/deny, test fence (M1 minimum)
3. `templates/` (+ `docs`, `roles`, `policies`) — assets for `a4l init` (M1)
4. `crates/a4l-cli` — `init`, `doctor`, `run`, `status` (M1)
5. `crates/a4l-ext-api`, `crates/a4l-ext`, `crates/a4l-mcp`, `crates/a4l-server`, `templates/skills` (M2); `sdk/a4l-ts-sdk` is deferred
6. `crates/a4l-memory`, `crates/a4l-pty` (M3)
7. `crates/a4l-desktop` (M4)

## Rules for every agent
- Six built-in tools live in core (GOAL §2.10): read, write, edit, bash, todo, ask_user. All go through `a4l-policy`. `todo` marks pipeline items `done` only; the harness alone sets `verified`. `ask_user` is information, not the policy `ask`.
- The base ships ONE provider: OpenAI-compatible Chat Completions (GOAL §2.9). Do not add Anthropic, Gemini, or others to core; they are extensions.
- Lean base (GOAL §1.7): the base has the engine, CLI + TUI, desktop, headless server, MCP, skills, and the extension manager. It never carries a specific vendor or chat channel; those are extensions.
- Headless/remote safety rules in GOAL §5.1 are mandatory for any channel work. Remote control of an agent must never auto-approve.
- Handoff and the assembler are the core feature and live in `a4l-core` (GOAL §11). Never move them into an extension or MCP server.
- The kernel prompt is static. Role, task, skills index, and the handoff packet go in the `[A4L STATE]` block appended after the cached prefix.
- The model never writes `status` or `role`; only a4l-core does, after guards pass.
- Only a harness-observed exit `0` counts as verified.
- Scaffold only the crates you are building now; do not generate empty stubs for the rest.
- Use a maintained YAML crate (`serde_yaml` is archived).

## Decided
MCP client, skills loader, extension manager, CLI + TUI, and desktop app are in the base. Headless mode and a local server API are in the base (M2). Telegram and other channels are extensions. The base ships six built-in tools (`read`, `write`, `edit`, `bash`, `todo`, `ask_user`); web search is cancelled. iOS/Android apps are far-future, not planned.

Extensions are Rust crates compiled into the user's own binary; the base stays installed and `a4l --no-extensions` runs it clean (GOAL §3). Skills, modes, policies, and `mcp.json` stay hot-reloaded files.

## Still open
- Are `a4l-memory` and `a4l-pty` base or opt-in? (GOAL §1.7 table.)
- Mode (user-chosen persona) vs role (pipeline stage) split, and a pipeline-free chat path. (`lite` pipeline depth in GOAL §11.6 partly covers the latter.)
- Optional TypeScript authoring via an embedded JS engine (deferred).
