# GOAL.md — A4L (Agentic 4 All)

> **Vision:** A maximally free, self-customizing, Human-in-the-Loop (HITL) agentic harness written in Rust. A4L runs a single unified core across both a fast CLI (`a4l-cli`) and a native Desktop GUI (`a4l-desktop`), giving users total authority to shape their agent's tools, skills, personas, MCP servers, and verification policies — guided by the harness itself.

---

## 1. Core Architectural Philosophy

1. **The Model Is Smart, the Harness Makes It Reliable:** LLMs are treated as stateless, untrusted CPUs. A4L provides the external RAM (`.a4l/tasks.json`, `.a4l/progress.md`), the progressive instruction bus (`A4L.md`), and the deterministic hardware interrupts (L1–L4 verification & HITL gates).
2. **Freedom to the Max (User-Owned Runtime):** Zero locked-in vendor workflows. Every persona, mode, verification policy, hook, and tool is declarative, inspectable, and hot-swappable.
3. **Self-Extending Meta-Harness:** A4L ships bootstrap skills that let the agent help the user author new TypeScript extensions, wire up MCP servers, draft `.md` skills, or configure new safety policies directly through conversation.
4. **Strict Context Partitioning & Role Dissection:** Work is dissected across isolated roles (**Planner → Maker → Checker → Human**), where each role receives only the exact context slice handed to it — never a whole decaying session.
5. **Minimal Kernel Prompt:** The base system prompt (`A4L.md`) is capped at ~50–80 lines. It acts solely as a protocol router instructing the model how to read A4L state files, obey role boundaries, and follow conditional pointers to domain `.md` files.
6. **Deterministic State, Never Promised State:** The model never writes `status` or `role_state`. It can only *propose* a transition; `a4l-core` validates the guard conditions deterministically and performs the write (temp file + atomic rename, file-locked).

---

## 2. The Core A4L Execution Engine

### 2.1 Progressive Instruction Disclosure (Layer 1 to Layer 3)

To prevent token waste and instruction drift:

* **Layer 1 (`A4L.md` Entry Point):** Ultra-lean root file. Contains universal commands, active Role/Mode metadata, and conditional pointers ("If touching X → read `docs/X.md`").
* **Layer 2 (Conditional Domain Docs & Role MDs):** Dedicated files (`docs/testing.md`, `.a4l/roles/maker.md`, `.a4l/skills/*.md`) loaded *only* when triggered by the current role or task domain.
* **Layer 3 (Atomic Task Context):** Injected dynamically from `tasks.json` for the active task only.

### 2.2 The 4-Role Pipeline & Checklist-Driven Transitions

State transitions trigger automatically when `.a4l/tasks.json` is created or updated, progressing through a strict checklist handoff:

```text
[tasks.json Created / Updated]
         |
         v
[0. Pre-Flight (init.sh)] --> Checks DB, binaries, git tree; emits .a4l/env_state.json
         |
         v
[1. PLANNER] --------------> Reads architecture & backlog; writes the context bundle
         |                   (micro-spec), TODO checklist, & acceptance criteria;
         |                   writes & locks L2/L4 tests; passes ONLY the bundle to Maker.
         v
[2. MAKER] ----------------> Receives ONLY the Planner's scoped bundle; implements;
      ^    |                 checks off TODO items; runs L1 -> L2 -> L3 -> L4 gates.
      |    |                 (tests/** is strictly read-only / git-locked.)
      |    |
      |    +-- [Circuit Breaker: Repetitive Error / Diff Ping-Pong]
      |         Routes back to PLANNER: purge context, rebase git tree, re-plan.
      v
[3. CHECKER] ---------------> Adversarial audit of raw git diff + L1-L4 test logs
         |                    against acceptance criteria. Detects test tampering
         |                    or swallowed errors. Has NO authority to mark complete.
         v
[4. HUMAN (HITL)] ----------> Final executive gate. Human inspects diff & L1-L4 proofs
                               in CLI/Desktop UI -> Approve & Commit, or Send Back
                               to Planner with notes.
```

### 2.3 tasks.json State Schema

The single source of truth for the task queue. The harness is the only writer of `state`; the model proposes, `a4l-core` disposes.

```jsonc
{
  "version": 1,
  "queue": [
    {
      "id": "T-0001",
      "title": "Add metrics endpoint",
      "created_at": "2025-11-07T09:00:00Z",
      "updated_at": "2025-11-07T09:41:00Z",
      "state": {
        "status": "in_progress",   // planned -> in_progress -> review -> verifying
                                    //   -> awaiting_hitl -> completed | failed | sent_back
        "role": "maker",           // planner | maker | checker | human
        "attempt": 1,
        "context_bundle": ".a4l/contexts/T-0001-attempt1.md" // the ONLY thing Maker sees
      },
      "todos": [
        // pending -> done -> verified
        // "done" is the model's claim; "verified" means the harness ran a gate
        // and observed exit 0. Only "verified" items satisfy transition guards.
        { "id": "T-0001.1", "text": "Route handler", "status": "verified", "verified_at": "..." }
      ],
      "acceptance_criteria": [
        { "id": "AC-1", "check": "GET /metrics returns 200 schema-valid", "status": "unverified" }
      ],
      "gates": {
        "l1": { "cmd": "cargo check",         "exit": null, "log": ".a4l/history/T-0001.l1.jsonl", "ran_at": null },
        "l2": { "cmd": "cargo test -q",       "exit": null, "log": null, "ran_at": null },
        "l3": { "cmd": ".a4l/init.sh smoke",  "exit": null, "log": null, "ran_at": null },
        "l4": { "cmd": "e2e/metrics.sh",      "exit": null, "log": null, "ran_at": null }
      },
      "errors": [
        // hash = sha256(normalized stderr + compilation unit). Paths and line
        // numbers are normalized before hashing, else every error looks
        // "different" and the x2 duplicate rule never trips.
        { "hash": "sha256(...)", "count": 2, "last_seen": "..." }
      ],
      "circuit_breaker": {
        "status": "armed",          // armed | tripped
        "reason": null,             // duplicate_error | diff_oscillation | max_attempts
        "quarantine_branch": null  // wip/T-0001-attempt-1
      }
    }
  ]
}
```

Writes are atomic (temp file + rename) behind a file lock, because both `a4l-cli` and `a4l-desktop` may touch the same file.

### 2.4 Transition Guards (Deterministic, Enforced in Rust)

| Transition | Guard — the model's word is never sufficient |
| --- | --- |
| *(file created/updated)* → Planner | File-lock write detected; Planner gets full architecture + backlog context |
| Planner → Maker | Context bundle exists; todos non-empty; every AC has an L2/L4 command; tests committed and tree-lock on `tests/**` active |
| Maker → Checker | Every todo `verified`; all four gates `exit == 0`; `git diff --exit-code tests/` empty; no error hash at `count >= 2`; breaker not tripped |
| Breaker trip (any time in Maker) | Duplicate hash ×2, diff hashes repeat within N=3 snapshots, or `attempt > 3` → stash dirty tree to quarantine branch, reset to clean commit, purge context bundle, write `.a4l/failures/T-0001.json`, route to Planner with `attempt + 1` |
| Checker → HITL | Checker verdict object written (verdict + evidence: raw diff ref + gate logs); Checker cannot set `completed` |
| HITL → completed / sent_back | Human input only; `sent_back` re-enters Planner with reviewer notes appended to the context bundle |

### 2.5 The 4-Tier Verification Ladder (L1–L4) & Immutable Test Fence

A task cannot transition from **Maker** to **Checker** on LLM promises. `docs/testing.md` defines four mandatory tiers:

* **L1 (Static Check):** Linters, formatting, AST validation, static typechecking (`cargo check`, `tsc --noEmit`, `eslint`).
* **L2 (Dynamic Verification):** Deterministic unit and module tests tied directly to the feature's acceptance criteria.
* **L3 (Smoke Test):** Booting the service/binary, checking container health, verifying zero startup crashes.
* **L4 (Actual Behavioral Test):** Real-world execution proof — curling live API endpoints, running Playwright browser flows, or executing end-to-end CLI behavioral tests.
* **Immutable Test Fence:** While in Maker mode, `tests/**` is locked by `a4l-policy`. Any unauthorized diff in `tests/**` triggers an automatic revert and fails the gate.

### 2.6 Repetitive Error Circuit Breaker & Planner Rebase

When the Maker gets trapped in a local repair loop:

**Trigger conditions:**

1. **Identical Error Hash:** Same (normalized) stack trace or failure output occurs 2× consecutively.
2. **Oscillating Diff:** Code reverts to a state seen N−2 iterations ago.
3. **Attempt Ceiling:** Exceeds max allowed iterations per task (default: 3).

**The Planner Intercept (Rebase Action):**

1. The harness halts the Maker and saves dirty state to a quarantine branch (`wip/<task-id>-attempt-<n>`).
2. Working tree resets to the last clean commit; the Maker's bloated context bundle is purged.
3. Control routes back to the **Planner** with a structured failure bundle (`.a4l/failures/<task-id>.json`).
4. The Planner diagnoses the systemic flaw, rebases the approach, updates the TODO checklist, and hands a clean, re-scoped bundle to a fresh **Maker** instance.

---

## 3. TypeScript Extension Runtime (`a4l-runtime-ts`) — Locked Decision

**Decision: subprocess IPC sidecar (Bun preferred, Node fallback) as the only v1 host**, behind an abstraction that allows an embedded host (`deno_core`) as a later opt-in compile feature. No hybrid at launch — a hybrid means two hosts, two capability stacks, and two bug surfaces during the phase we can least afford it.

| Criterion | Embedded V8 (`deno_core`) | Sidecar (Bun/Node over stdio) |
| --- | --- | --- |
| npm ecosystem | Partial; native addons shaky | Full, effortless — the biggest win for "freedom to the max" |
| Fault isolation | None — a V8/native crash takes down `a4l-core` mid-session | Crash kills the sidecar; harness restarts it, session survives |
| Binary size / build | ~70MB, heavy dep tree, hurts cross-compile for Tauri targets | ~15MB; TS host iterates independently of Rust releases |
| User prerequisite | None | Needs a runtime; mitigated via `a4l doctor` + optional pinned-Bun fetch |

Rationale: user-authored extensions are the same threat class as the LLM itself (untrusted code). A process boundary is the only sandbox boundary consistent with the "untrusted CPU" philosophy — losing a 45-minute agent session because a user's Postgres extension segfaulted is unacceptable.

Host abstraction:

```rust
// a4l-runtime-ts
pub trait ExtensionHost: Send + Sync {
    fn capabilities(&self) -> Vec<HostCapability>; // tools, hooks, l4_runners
    async fn call_tool(&self, name: &str, args: Value) -> Result<Value>;
    async fn dispatch_hook(&self, ev: HookEvent) -> HookOutcome; // beforeToolCall, onStop, onRoleTransition
}

/// NDJSON-RPC over stdio.
/// Rust -> sidecar: tool/hook dispatch.
/// Sidecar -> Rust: capability requests (fs, net) routed through a4l-policy,
/// so an extension can never bypass the gatekeeper.
pub struct SidecarHost { /* managed child proc, hot-reload watch on .a4l/extensions */ }
```

Standing rules:

1. Extensions reach the outside world only via capability grants dispatched back through Rust and checked by `a4l-policy` — no raw fs/net in the sidecar.
2. If no runtime is found (`a4l doctor`), TS extensions are flagged unavailable and the harness degrades gracefully to core tools + MCP without blocking the session.
3. `EmbeddedHost` remains a documented compile-time feature for a later milestone, not a v1 promise.

---

## 4. Patterns Adopted from Open Harnesses

| Source Harness | Pattern Adopted | How A4L Implements It |
| --- | --- | --- |
| `claude-code` | Deterministic stop hooks & sidechain logs | `onStop` / `onPostTool` hooks block task completion unless L1–L4 exit `0`. Sub-agent tool noise is isolated in `.a4l/history/*.jsonl`. |
| `opencode` | Strict mode permissions & rolling PTY buffers | Structural ACLs between Plan (read-only), Maker (src write, tests read-only), Checker (read-only) roles. Rolling PTY buffers prevent stdout explosions. |
| `deepseek-harness` | Plugin microkernel & programmatic tool calling (PTC) | Every capability (LLM, tools, memory, UI) mounts over an event bus. PTC ("Code Mode") lets the agent write a TS script to batch-execute tools in a single turn. |
| `hermes-agent` | Self-authored procedural skills & dual memory | `skill_manage` / `extension_manage` so A4L can write its own reusable `.md` skills and `.ts` tools, backed by bounded `MEMORY.md` + SQLite FTS5 episodic search. |
| `gemini-cli` | Native MCP bus & workspace checkpointing | First-class Model Context Protocol (`.a4l/mcp.json`) support for external tools, paired with full workspace/session checkpointing and provider token caching. |

---

## 5. Target Deliverables & Surfaces

A unified Rust Cargo workspace compiling into two applications sharing `a4l-core`:

1. **`a4l-cli` (Terminal Interface):**
   * Headless autonomous runner (`a4l run --watch tasks.json`) for background loops and CI.
   * Interactive TUI (`ratatui`) for terminal workflows, live L1–L4 gate logs, role-transition visualization, and rapid keyboard HITL approvals.

2. **`a4l-desktop` (Native Desktop App via Tauri v2):**
   * High-performance Rust backend (`a4l-core`) bridged via async Tokio commands to a reactive frontend.
   * **Visual Pipeline Board:** Real-time Kanban/graph view of `tasks.json` moving through Planner → Maker → Checker → HITL.
   * **HITL Diff & Proof Station:** Side-by-side git diff viewer paired with green/red L1–L4 execution badges and one-click *Approve & Commit* or *Send Back to Planner with Notes*.
   * **A4L Studio:** Visual manager to toggle `.md` skills, edit `.ts` custom tools, inspect MCP servers, and tweak mode/persona YAML rules.

---

## 6. Dynamic Extensibility Engine ("Freedom to the Max")

Users — or the A4L Meta-Architect agent itself — can customize five hot-reloadable layers inside `.a4l/`:

1. **Role & Persona Definitions (`.a4l/roles/*.md` & `.a4l/modes/*.yaml`):** Defines which tools, file glob permissions (`allow_write: ["src/**"]`, `deny_write: ["tests/**"]`), and verification tiers apply to each role, plus when to transition.
2. **Markdown Skill Sets (`.a4l/skills/*.md`):** Progressive disclosure playbooks with YAML frontmatter triggers (`name`, `description`, `when_to_load`).
3. **TypeScript Extensions & Custom Tools (`.a4l/extensions/*.ts`):** Powered by `@a4l/sdk`. Users write standard TypeScript to register new LLM tools, custom L4 test runners, or lifecycle interceptors (`beforeToolCall`, `onCircuitBreakerTrip`, `onRoleTransition`).
4. **Native MCP Connections (`.a4l/mcp.json`):** Mount any standard Model Context Protocol server over `stdio` or `SSE/HTTP`.
5. **Declarative Policies (`.a4l/policies/*.yaml`):** Fine-grained `allow | ask | deny` rules per command, file path, or git operation.

The **Meta-Harness Assistant ("A4L Architect")** is a built-in capability letting users say things like *"Help me build a TypeScript tool that queries my local Postgres schema"* or *"Create a Security Reviewer persona."* A4L scaffolds the exact `.ts`, `.md`, or `.yaml` file against internal JSON schemas, validates it, and hot-reloads the harness without restarting the session.

---

## 7. Policy & HITL Gatekeeper (`a4l-policy`)

Rule engine evaluating every tool call before execution:

* **`allow`:** Auto-execute (reading files, running unit tests).
* **`ask` (HITL Gate):** Pause the execution loop and prompt via CLI/Desktop UI (destructive shell commands, modifying locked directories, finalizing a task merge).
* **`deny`:** Hard block (modifying `.a4l/policies/` during an untrusted run, touching `tests/**` in Maker mode).

**Deterministic stop hooks:** the agent cannot mark a task `completed` unless the configured L1–L4 verification scripts all return exit code `0`.

---

## 8. Proposed Cargo Workspace & Repository Layout

```text
a4l/
├── Cargo.toml                  # Rust workspace root
├── GOAL.md                     # Master architecture & vision specification (this file)
├── README.md                   # Project overview & quick start
├── crates/
│   ├── a4l-core/               # State machine, Role Pipeline (Planner/Maker/Checker),
│   │                           #   circuit breakers, deterministic transition guards
│   ├── a4l-runtime-ts/         # TS extension host: sidecar IPC + ExtensionHost trait
│   ├── a4l-mcp/                # Native MCP client implementation (stdio / SSE)
│   ├── a4l-policy/             # ACL file locks, HITL gates, L1-L4 stop-hook verifier
│   ├── a4l-memory/             # Bounded MEMORY.md manager + SQLite FTS5 episodic store
│   ├── a4l-pty/                # Rolling-buffer pseudo-terminal manager
│   ├── a4l-cli/                # CLI + ratatui TUI binary target
│   └── a4l-desktop/            # Tauri v2 desktop binary target (src-tauri)
├── sdk/
│   └── a4l-ts-sdk/             # TypeScript definitions (@a4l/sdk) for custom tools & hooks
└── templates/                  # Default bootstrap assets for `a4l init`
    ├── A4L.md                  # Minimal ~60-line system prompt router (kernel)
    ├── docs/
    │   └── testing.md          # L1 (Static), L2 (Unit), L3 (Smoke), L4 (Behavioral) rules
    ├── roles/                  # Role specs (planner.md, maker.md, checker.md, architect.md)
    ├── policies/               # Default HITL & immutable test-lock policies
    └── skills/                 # Meta-skills (create-skill.md, create-ts-tool.md, setup-mcp.md)
```

---

## 9. Minimal System Prompt Blueprint (`templates/A4L.md`)

```markdown
# A4L Kernel Protocol

You are operating inside **A4L (Agentic 4 All)**, a deterministic,
human-in-the-loop engineering harness.

## 1. Active Role & Context Boundary
- **Current Role:** `{{ACTIVE_ROLE}}` (Read `.a4l/roles/{{ACTIVE_ROLE}}.md`
  for your exact duties and transition rules).
- **Current Task:** `{{ACTIVE_TASK_ID}}` (Loaded from `.a4l/tasks.json`).
- **Strict Scope:** Operate ONLY on the context and TODO checklist provided
  for your current role. Do not assume unverified state.

## 2. Operating Laws
- **External RAM:** Track all state in `.a4l/tasks.json` and log every
  milestone/command exit code to `.a4l/progress.md`.
- **Progressive Disclosure:** Never read all docs upfront:
  - Running or verifying code? -> Read `docs/testing.md` (Enforce L1 Static,
    L2 Unit, L3 Smoke, L4 Behavioral).
  - Matching a domain trigger below? -> Read the linked `.a4l/skills/*.md` first.
- **Role Transition:** You may only request a transition to the next role
  when every item in your current role's TODO checklist is checked off and
  verified with exit code `0`.
- **HITL Authority:** If an action triggers a `HITL_PAUSE` or reaches the
  final merge gate, halt immediately and await human approval.

## 3. Progressive Skills & Self-Customization Index
- **Available Skills:** {{SKILLS_INDEX}}
- **Meta-Harness Customization:** If the user asks to create/edit a skill,
  TypeScript tool, MCP server, role/persona, or policy, load
  `.a4l/skills/meta-architect.md` FIRST.
```

---

## 10. Implementation Roadmap

### Milestone 1 — Rust State Engine & Role Loop (`a4l-core` + `a4l-cli`)
* [ ] Scaffold `.a4l/` workspace (`init.sh`, `tasks.json`, `progress.md`, `A4L.md`, `docs/testing.md`).
* [ ] Implement the Planner → Maker → Checker → HITL state machine driven by `tasks.json` TODO completion, with the §2.4 transition guards.
* [ ] Implement the L1–L4 verification runner, immutable `tests/**` git-diff guard, and identical-error circuit breaker with automatic Planner Rebase.
* [ ] LLM provider router (OpenAI / Anthropic / Gemini / local OpenAI-compatible endpoints).
* **Acceptance:** `a4l` runs a sample task that must pass a locked shell test script before exiting; a task with a failing gate cannot be marked complete by the model.
* **Acceptance (breaker):** a sample task with a deliberately unfixable error trips the breaker after 3 attempts, stashes to `wip/<task-id>-attempt-<n>`, and re-enters Planner with a failure bundle.

### Milestone 2 — Polyglot Extensibility (`a4l-runtime-ts` + `a4l-mcp` + Meta-Skills)
* [ ] Ship `@a4l/sdk` plus the `SidecarHost` (NDJSON-RPC over stdio) per §3.
* [ ] Integrate MCP client (`.a4l/mcp.json`) and the progressive `.md` skill loader (`{{SKILLS_INDEX}}`).
* [ ] Ship the `meta-architect` skill so A4L can build and hot-reload extensions.
* **Acceptance:** the agent successfully creates a custom `.ts` tool using `meta-architect.md` and executes it in the next turn.

### Milestone 3 — Memory & Context Hygiene (`a4l-memory` + `a4l-pty`)
* [ ] Sidechain `.jsonl` logging for sub-tasks, rolling PTY buffers, SQLite FTS5 episodic search, bounded `MEMORY.md`.
* **Acceptance:** long sessions show no context growth from tool noise; episodic search retrieves prior failure bundles by symptom.

### Milestone 4 — Desktop Control Center (`a4l-desktop`)
* [ ] Tauri v2 GUI: visual Role Pipeline board, L1–L4 proof inspector, HITL diff sign-off station, and visual Extension/Persona studio.
* **Acceptance:** a full task lifecycle (plan → make → check → approve) is completable end-to-end from the GUI without touching the CLI.

---

## Appendix A — Reference Harnesses

The design distills patterns from five open harnesses; reference checkouts live in `ref/`:

`claude-code`, `deepseek-harness`, `hermes-agent`, `pi` (Mistral), `zeroclaw`, `gemini-cli`, `opencode`.
