# A4L — Agentic 4 All

```
   ___   _  _____ _  _
  / _ \ / \|_   _| || |     A deterministic, self-customizing,
 / /_\// _ \ | | | __ |     human-in-the-loop agentic harness
/ /_\/ /_\ \| | | |_|| |    written in Rust.
\____/_/  \_\_|  \__ /_/
```

A4L turns an LLM into a reliable engineering collaborator by treating it as a
smart but stateless, untrusted CPU — and giving it deterministic hardware
around it: external RAM on disk, hard verification gates, circuit breakers,
and a human executive who signs off on every merge.

One Rust core. Two front ends:

* **`a4l`** — a fast CLI with an interactive TUI (and a headless mode for CI).
* **`a4l-desktop`** — a native desktop app (Tauri v2) with a visual pipeline
  board, diff/proof inspector, and a one-click HITL approval station.

No locked-in workflows. Every skill, tool, persona, role, policy, and MCP
server is a declarative file in `.a4l/` that you (or the agent itself, via
the built-in Meta-Architect) can author, inspect, and hot-swap.

> **Status: pre-alpha — specification phase.** The architecture is fully
> specified in [GOAL.md](GOAL.md); Milestone 1 (the Rust state engine and
> role loop) is the active implementation target. The commands below describe
> the design surface, not a shipping binary yet.

---

## Why A4L Is Different

| Principle | What it means in practice |
| --- | --- |
| **The model is smart; the harness makes it reliable** | State lives in `.a4l/tasks.json` and `.a4l/progress.md`, never in chat memory. A kernel prompt capped at ~60 lines routes the model to load only what the current step needs. |
| **Roles, not one decaying session** | Work is dissected across **Planner → Maker → Checker → Human**. Each role receives only the context slice handed to it. |
| **Lean base** | The install ships one provider (any OpenAI-compatible endpoint) and six built-in tools (`read`, `write`, `edit`, `bash`, `todo`, `ask_user`). MCP, skills, and extensions are built in; native providers, chat channels, and optional stores are added only if you need them. |
| **Handoff-first** | A compact packet (mostly harness-built exact facts + a short capped model note) moves work across roles, modes, and even a mid-session model switch without re-pushing history. |
| **Proof over promises** | A task cannot advance unless L1–L4 verification commands return exit code `0` — verified by the Rust core, not the model's word. A `done` checkbox the model ticks is worthless until the harness observes a passing gate. |
| **Circuit breakers, not doom loops** | The same error twice, an oscillating diff, or too many attempts trips a breaker: dirty state is quarantined to a `wip/` branch, the tree resets, and the Planner re-plans with a structured failure bundle. |
| **Human-in-the-Loop authority** | Policies declare what runs autonomously (`allow`), what pauses for sign-off (`ask`), and what is hard-blocked (`deny`). The final merge gate is always human. |
| **Freedom to the max** | Markdown skills, roles/modes, MCP servers, and policies are hot-reloaded plain files; code extensions are Rust crates built into your own binary (base always kept). |
| **Self-extending** | Ask A4L to build a Postgres-querying tool or a Security Reviewer persona, and the Meta-Architect scaffolds it, validates it against internal schemas, and hot-reloads — no restart. |

---

## The Execution Pipeline

```text
[tasks.json updated]
        |
        v
   [Pre-Flight]     init.sh checks DBs, binaries, git tree -> .a4l/env_state.json
        |
        v
   [PLANNER]        micro-spec + TODO checklist + acceptance criteria;
        |           writes and LOCKS the L2/L4 tests
        v
   [MAKER]          implements from the scoped bundle only; runs L1 -> L2 -> L3 -> L4
   |  ^  |          (tests/** is read-only and git-locked; tamper = auto-revert)
   |  |  |          Circuit breakers route stuck Makers back to Planner:
   |  +--+           quarantine branch -> clean reset -> re-plan -> fresh Maker
        v
   [CHECKER]        adversarial audit of raw git diff + gate logs vs criteria;
        |           cannot mark anything complete
        v
   [HUMAN]          final executive gate: Approve & Commit, or Send Back with notes
```

The verification ladder (`docs/testing.md` after `a4l init`):

| Tier | Check | Examples |
| --- | --- | --- |
| **L1** | Static | `cargo check`, `tsc --noEmit`, `eslint` |
| **L2** | Unit / module tests tied to acceptance criteria | `cargo test -q` |
| **L3** | Smoke | service boots, container health, zero startup crashes |
| **L4** | Behavioral | live API curls, Playwright flows, end-to-end CLI tests |

---

## Quick Start (design surface)

```bash
# Scaffold a project workspace (.a4l/ + templates/A4L.md, roles, policies, skills)
a4l init

# Check environment: providers, Rust toolchain (for extensions), MCP servers
a4l doctor

# Interactive TUI session (role pipeline, live L1-L4 gate logs, HITL approvals)
a4l

# Headless autonomous run watching the task queue (CI / background loops)
a4l run --watch tasks.json

# Run a single task through the pipeline
a4l run --task T-0001

# Run as a background service for chat channels / remote clients (Milestone 2)
a4l serve
```

Everything the agent knows and is allowed to do is inspectable:

```text
your-project/
└── .a4l/
    ├── A4L.md            # kernel prompt router (~60 lines, yours to edit)
    ├── tasks.json        # task queue: todos, gates, breaker state (harness-owned)
    ├── progress.md       # append-only execution ledger
    ├── roles/*.md        # planner, maker, checker, architect role specs
    ├── modes/*.yaml      # declarative mode presets (path ACLs, tool grants)
    ├── skills/*.md       # progressive-disclosure skill playbooks
    ├── policies/*.yaml   # allow | ask | deny rules per command/path/git op
    ├── providers.yaml    # OpenAI-compatible endpoints (base_url, key env, quirks)
    ├── profiles/*.yaml   # per-model instruction depth & pipeline depth
    ├── mcp.json          # mounted MCP servers (stdio / SSE)
    ├── handoffs/         # rolling compact handoff packets (latest.json per task)
    └── history/*.jsonl   # sidechain logs: sub-agent chatter & tool dumps
```

### Adding an extension

Code extensions are Rust crates built into **your own** copy of A4L, so they
load instantly with no runtime or sidecar. The base install stays untouched.

```bash
a4l ext add github.com/someone/a4l-postgres   # review source + deps, then build
a4l ext list
a4l ext remove a4l-postgres

a4l --no-extensions                            # run the clean base harness
```

`a4l ext add` shows you the source and its dependencies for approval, builds
into a temp location, smoke-tests the result, and swaps it in. A failed build
never breaks a working install. Rebuilding restarts the process and the session
resumes from `.a4l/` state. Skills, modes, policies, and `mcp.json` are plain
files and still hot-reload with no rebuild. Building needs a Rust toolchain;
without one, A4L runs as the base with extensions unavailable.

---

## Architecture

Single Cargo workspace, two binary targets sharing the core:

| Crate | Responsibility |
| --- | --- |
| `a4l-core` | State machine, role pipeline (Planner/Maker/Checker), deterministic transition guards, circuit breakers |
| `a4l-policy` | `allow/ask/deny` rule engine, file-lock ACLs, HITL gates, L1–L4 stop hooks |
| `a4l-ext` | extension manager: `a4l ext add`, build, atomic swap, `--no-extensions` launcher |
| `a4l-ext-api` | public API crate that extensions compile against |
| `a4l-mcp` | Model Context Protocol client (stdio / SSE-HTTP transports) |
| `a4l-memory` | bounded `MEMORY.md` + SQLite FTS5 episodic search |
| `a4l-pty` | rolling-buffer pseudo-terminal manager (no stdout explosions) |
| `a4l-server` | headless daemon + local API so chat channels (Telegram etc.) and future clients can drive A4L |
| `a4l-cli` | CLI + ratatui TUI |
| `a4l-desktop` | Tauri v2 desktop app (pipeline board, diff/proof station, A4L Studio) |
| `sdk/a4l-ts-sdk` | deferred: optional TypeScript authoring (not v1) |

```text
            +---------------------------+
            |         a4l-core          |
            |  tasks.json state machine |
            |  Planner/Maker/Checker    |
            |  L1-L4 gates, breakers    |
            +------------+--------------+
      +------------------+------------------+
      v                  v                  v
 a4l-policy          a4l-ext            a4l-mcp
 (HITL gates,      (build-in extensions, (Model Context
  file ACLs)        --no-extensions)      Protocol client)
      |                  |                  |
      +--------+---------+---------+--------+
               v                   v
          a4l-cli (TUI)      a4l-desktop (Tauri v2)
```

Full specification, schemas, and locked design decisions: **[GOAL.md](GOAL.md)**.

---

## Roadmap

| Milestone | Scope | Status |
| --- | --- | --- |
| 1 | Rust state engine & role loop, L1–L4 gates, circuit breaker + Planner rebase, CLI MVP | **Active — specification locked** |
| 2 | Extension build/install flow, `a4l-ext-api`, MCP client, meta-architect skills | Planned |
| 3 | Memory & context hygiene (SQLite FTS5, sidechain logs, PTY buffers) | Planned |
| 4 | Desktop control center (Tauri v2 pipeline board, HITL diff station) | Planned |

Acceptance criteria for each milestone are defined in
[GOAL.md §10](GOAL.md#10-implementation-roadmap).

---

## Design Credits

A4L synthesizes the strongest seams from open harnesses studied in `ref/`:
deterministic stop hooks and sidechain logging (`claude-code`), strict mode
permissions (`opencode`), plugin microkernel + programmatic tool calling
(`deepseek-harness`), self-authored procedural skills (`hermes-agent`), and
the native MCP bus with workspace checkpointing (`gemini-cli`).
