# GOAL.md — A4L (Agentic 4 All)

> **Vision:** A maximally free, self-customizing, Human-in-the-Loop (HITL) agentic harness written in Rust. A4L runs a single unified core across both a fast CLI (`a4l-cli`) and a native Desktop GUI (`a4l-desktop`), giving users total authority to shape their agent's tools, skills, personas, MCP servers, and verification policies — guided by the harness itself.

---

## 0. End Goal

**Ship two products from one Rust core:**

1. **`a4l` (CLI/TUI)** — fast terminal harness, plus a headless mode for CI and background loops.
2. **`a4l-desktop` (Tauri v2)** — native desktop app with a visual pipeline board and a one-click HITL approval station.
3. **Headless mode** — the same core runs as a background service so the user can talk to it from chat channels such as Telegram (§5.1).

*Far future, keep in view only (not planned): iOS and Android companion apps.*

**What A4L is:** *Agentic 4 All* — an agentic harness where a human stays in the loop and the user has maximum freedom. The harness itself helps the user customize it: authoring extensions, skills, MCP connections, modes/personas, and policies through conversation.

**Non-negotiable pillars:**

| Pillar | Meaning |
| --- | --- |
| Minimal system prompt | A ~60-line kernel prompt (`A4L.md`) only steers the model to follow A4L rules and load detail on demand. |
| Lean base | The base ships the engine, both front ends, and the ways to extend it (MCP, skills, extensions), with one provider (OpenAI-compatible). Vendors, channels, and optional stores are added by the user (§1.7). |
| Handoff-first context | A compact, harness-built handoff packet moves a task across roles, modes, **models**, and sessions without replaying history (§11). |
| Rust core | Single deterministic engine shared by CLI and desktop. |
| Open extensibility | Rust extensions built into the user's own binary (base always kept, `--no-extensions` for clean), plus standard MCP servers, Markdown skills, modes/personas, and policies as hot-reloaded files in `.a4l/`. |
| Self-customizing | The built-in Meta-Architect scaffolds and validates new extensions, skills, MCP configs, modes, and policies. File-based layers hot-reload; code extensions rebuild and resume the session. |
| Human in the loop | Policies decide `allow` / `ask` / `deny`; the final merge gate is always human. |

**Definition of done for the project:** a user installs either front end, runs `init`, describes a task, watches it pass Planner → Maker → Checker with real gate proofs, approves the merge, and can ask the harness to build a new tool/skill/persona/policy for them without restarting.

---

## 1. Core Architectural Philosophy

1. **The Model Is Smart, the Harness Makes It Reliable:** LLMs are treated as stateless, untrusted CPUs. A4L provides the external RAM (`.a4l/tasks.json`, `.a4l/progress.md`), the progressive instruction bus (`A4L.md`), and the deterministic hardware interrupts (L1–L4 verification & HITL gates).
2. **Freedom to the Max (User-Owned Runtime):** Zero locked-in vendor workflows. Every persona, mode, verification policy, hook, and tool is declarative, inspectable, and hot-swappable.
3. **Self-Extending Meta-Harness:** A4L ships bootstrap skills that let the agent help the user author new extensions, wire up MCP servers, draft `.md` skills, or configure new safety policies directly through conversation.
4. **Strict Context Partitioning & Role Dissection:** Work is dissected across isolated roles (**Planner → Maker → Checker → Human**), where each role receives only the exact context slice handed to it — never a whole decaying session.
5. **Minimal Kernel Prompt:** The base system prompt (`A4L.md`) is capped at ~50–80 lines. It acts solely as a protocol router instructing the model how to read A4L state files, obey role boundaries, and follow conditional pointers to domain `.md` files.
6. **Deterministic State, Never Promised State:** The model never writes `status` or `role_state`. It can only *propose* a transition; `a4l-core` validates the guard conditions deterministically and performs the write (temp file + atomic rename, file-locked).
7. **Lean Base, Opt-In Everything Else:** The install carries no bloat the user did not choose. The base contains the engine, both front ends, and the ways to extend it; specific vendors, channels, and optional stores are installed deliberately.

   | In the base | Opt-in (extension or file) |
   | --- | --- |
   | `a4l-core` engine, assembler, handoff, ledger | native providers (Anthropic Messages API, Gemini, ...) |
   | **One provider: OpenAI-compatible** (§2.9) | channel adapters (Telegram and others) |
   | **CLI + TUI** and **desktop app** | `a4l-memory` (FTS5), `a4l-pty` *(decision: base or opt-in?)* |
   | **Headless mode + local server API** (§5.1) | community extensions, extra skills, modes, profiles |
   | **MCP client, skills loader, extension manager** | iOS / Android apps *(far future, not planned)* |
   | **Six built-in tools: read, write, edit, bash, todo, ask_user** (§2.10); policy engine; `--no-extensions` launcher | |

   Rule for membership: the base carries the engine, both front ends, and the ways to extend it (MCP, skills, extensions). It never carries a specific vendor, channel, or optional store. Proposed: `install.sh --no-desktop` lets headless servers skip the GUI.

---

## 2. The Core A4L Execution Engine

### 2.1 Progressive Instruction Disclosure (Layer 1 to Layer 3)

To prevent token waste and instruction drift:

* **Layer 1 (`A4L.md` Entry Point):** Ultra-lean root file. Contains universal commands and conditional pointers; active role/task metadata arrives separately in the `[A4L STATE]` block (§11.2). Conditional pointers ("If touching X → read `docs/X.md`").
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
| Breaker trip (any time in Maker) | Duplicate hash ×2, diff hashes repeat within N=3 snapshots, or the Maker's 3rd attempt ends without all gates green (`max_attempts = 3`, configurable; attempts are counted from 1) → stash dirty tree to quarantine branch, reset to clean commit, purge context bundle, write `.a4l/failures/T-0001.json`, route to Planner with `attempt + 1` |
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

### 2.7 Pre-Flight Contract (`.a4l/init.sh`)

* `init.sh` is a user-owned shell script scaffolded by `a4l init`; the harness only calls it, never edits it.
* Subcommands: `init.sh check` (pre-flight) and `init.sh smoke` (the L3 gate).
* `check` exits `0` when the environment is usable, prints one JSON object to stdout, and the harness stores it as `.a4l/env_state.json` (`{ "binaries": {...}, "services": {...}, "git_clean": true }`). Non-zero exit blocks the task before the Planner starts.
* `smoke` exits `0` only if the service/binary boots with zero startup crashes.

### 2.8 Artifact Schemas (to be frozen as JSON Schema in M1)

* **Failure bundle** `.a4l/failures/<task-id>.json`: `{ task_id, attempt, trip_reason, error_hashes[], last_gate_logs[], quarantine_branch, diff_ref }`.
* **Checker verdict** `.a4l/verdicts/<task-id>-<attempt>.json`: `{ verdict: "pass" | "fail", findings[], evidence: { diff_ref, gate_logs[] } }`. The verdict cannot set `completed`.
* **Test fence:** enforced by `git diff --exit-code tests/` against the commit the Planner locked, plus an `a4l-policy` write-deny on `tests/**` while the role is Maker.

---

### 2.9 Provider Layer — Base Ships One: OpenAI-Compatible

**Decision:** the base has exactly one built-in provider, the OpenAI-compatible **Chat Completions** API (`/v1/chat/completions`, with tools and SSE streaming). A user can start with any endpoint that speaks it (OpenAI, OpenRouter, Ollama, vLLM, llama.cpp, LM Studio, LiteLLM, and similar) and add anything else later as an extension.

Configuration lives in `.a4l/providers.yaml`: named endpoints with `base_url`, `api_key_env` (optional for local servers), default model, extra headers, and quirk flags. Model profiles (§11.6) point at a named endpoint.

Quirks the base must handle, because "compatible" is loose in practice:

* **Stream usage.** Token usage in streams usually needs `stream_options.include_usage`; some servers omit it and a few reject the option. Retry without it, then estimate tokens locally so the ledger (§11.7) still works, flagged as estimated.
* **Stream framing.** Tolerate a missing `[DONE]` sentinel and unusual `finish_reason` values.
* **Tool calls.** Streamed tool-call deltas arrive in pieces and vary by server; validate JSON arguments and retry malformed calls with a one-line error (§11, weak-model path).
* **Parameter names.** `max_tokens` vs `max_completion_tokens` is a per-endpoint flag.
* **Capabilities.** Each endpoint declares what it supports (tools, streaming, usage, caching); the assembler adapts rather than assuming.

**Consequence for token savings:** prompt caching depends on the endpoint. Endpoints that cache by prefix reward the byte-stable layers 1–3 (§11.2); others give no cache benefit, which makes the handoff packet more valuable, not less. The ledger reports cache hits only when the server returns them. Anthropic's own OpenAI-compatibility layer, for example, does not support prompt caching, does not return thinking, and is documented as a testing aid rather than a production path, so users who want cache savings on Claude should install a native-provider extension (Anthropic Messages API with `cache_control`). Native providers implement the provider trait from `a4l-ext-api`.

### 2.10 Built-in Tools: Six

The base ships exactly six tools: Pi's four defaults plus two A4L adds. They are core, not extensions, so the pipeline and `--no-extensions` mode always have them. Every call goes through `a4l-policy` and the active role/mode ACLs, and each tool description is one sentence (tool schemas are paid on every turn; the six together are covered by the CI budget test in §11.7).

| Tool | Behavior in A4L |
| --- | --- |
| `read` | Read a file with offset/limit. Output is capped per the model profile and ends with a continuation hint. |
| `write` | Create or overwrite a file. Subject to path ACLs (for example `tests/**` is denied to the Maker). |
| `edit` | Exact-string replacement; fails if the match is not unique. Returns only a small diff, not the file. |
| `bash` | Run a command with a timeout and a rolling output buffer. The model receives the exit code, the first failing error, and a log pointer; the full log stays in `.a4l/history/`. The gate runner uses the same executor. Rules in policy decide `allow` / `ask` / `deny`. |
| `todo` | One tool, two behaviors. In a pipeline task it edits that task's todos in `tasks.json`: the model may set only `pending` or `done` (a claim), and only the harness sets `verified` (§2.3). With no task active it keeps a freeform checklist at `.a4l/session/todos.json`. Actions: list, add, update, remove. Output is compact (ids and statuses). A `/todos` command shows it in every front end. Todos appear in the handoff facts (§11.3). |
| `ask_user` | Ask the user for information: 1–4 questions, each with 2–4 options plus free text, optionally multi-select. Rendered natively by each front end through `question_request` / `answer` events (§5.1). |

`ask_user` rules:

* It is not the policy `ask`, which is a permission request. Keep the names and UI distinct.
* Headless with no front end: return "no interactive channel" instead of blocking. On a channel: wait with a timeout, then return "no answer"; the model is told not to guess consequential choices. Unanswered questions are added to the handoff `open_questions` by the harness; answers are logged to `progress.md`.
* The description steers the model to a sensible default for reversible choices and to ask only when blocked.

Extensions can add more tools and wrap these through hooks, but cannot remove the policy check on them.

---

## 3. Extension Model (`a4l-ext`) — Locked Decision: Build-Into-Binary

**Decision: code extensions are compiled into the user's own binary at install time. The base app is never modified and always remains available.**

| Piece | Behaviour |
| --- | --- |
| Base | `install.sh` installs `a4l-base` (prebuilt, untouched) under `~/.a4l/bin/`. |
| Custom build | `a4l ext add <git-url\|path>` builds a custom `a4l` that includes the base plus all installed extensions. Fast startup: no runtime loading, no sidecar. |
| Clean mode | `a4l --no-extensions` execs `a4l-base` with user extensions disabled: the six built-in tools (§2.10) only. |
| Registry | `~/.a4l/extensions.toml` lists each extension with source, pinned commit, and content hash. |

**What needs a rebuild and what does not:**

* Rebuild required: **code extensions** (custom tools, hooks, L4 runners, providers), written as Rust crates against `a4l-ext-api`.
* Still hot-reloaded as plain files, no rebuild: Markdown skills, `mcp.json`, `policies/*.yaml`, `modes/*.yaml`, `roles/*.md`.

**Build and install flow (`a4l ext add`):**

1. Fetch the source; pin the commit and hash it.
2. HITL review (always `ask`): show source, `Cargo.toml` dependencies, and `build.rs` / proc-macro usage before anything compiles.
3. Build into a temp location using a cached target dir (`~/.a4l/build/target`).
4. Smoke-test the new binary (`--selftest`, extension registration check).
5. Atomically swap it in. On any failure the previous binary stays; a failed build never breaks a working install.

**Standing rules:**

1. **Threat model change.** Extensions now run in-process with the user's privileges, and Rust cannot sandbox in-process code. Extensions are therefore *reviewed-then-trusted* code, not untrusted code. The mitigations are the install-time HITL review, pinned hashes, `#![forbid(unsafe_code)]` enforcement where possible, and routing all harness-level actions through the `a4l-ext-api` host API (which calls `a4l-policy`). This is policy discipline, not a hard sandbox; document that plainly.
2. **Crash fallback.** If the custom binary fails at startup twice, the launcher offers to run `a4l-base`.
3. **Toolchain.** Building needs `rustc`/`cargo`. `install.sh` offers a minimal `rustup` install; `a4l doctor` checks it. With no toolchain, extensions are unavailable and the base works normally.
4. **Session continuity.** Rebuilding restarts the process. Because state lives in `.a4l/tasks.json` and `progress.md`, the session resumes where it left off.
5. **Meta-Architect.** It can scaffold an extension crate from a template and run the same build flow. Compile errors feed the L1 gate. The install step is still `ask`.
6. **Platform.** `install.sh` targets Linux/macOS first; Windows is a later decision (PowerShell installer or WSL).

**Open question: TypeScript authoring.** Native extensions are Rust. If TS authoring is still wanted, the option is to embed a small JS engine (for example QuickJS) in the base and bundle TS extensions at install time. This is deferred and not a v1 promise.

Considered and rejected for v1: Bun/Node sidecar (needs a runtime, extra IPC surface) and WASM components (sandboxed and toolchain-free, but a new extension ABI to maintain). Either can be added later behind the same registry.

---

## 4. Patterns Adopted from Open Harnesses

| Source Harness | Pattern Adopted | How A4L Implements It |
| --- | --- | --- |
| `claude-code` | Deterministic stop hooks & sidechain logs | `onStop` / `onPostTool` hooks block task completion unless L1–L4 exit `0`. Sub-agent tool noise is isolated in `.a4l/history/*.jsonl`. |
| `opencode` | Strict mode permissions & rolling PTY buffers | Structural ACLs between Plan (read-only), Maker (src write, tests read-only), Checker (read-only) roles. Rolling PTY buffers prevent stdout explosions. |
| `deepseek-harness` | Plugin microkernel & programmatic tool calling (PTC) | Every capability (LLM, tools, memory, UI) mounts over an event bus. PTC ("Code Mode") lets the agent write a TS script to batch-execute tools in a single turn. |
| `hermes-agent` | Self-authored procedural skills & dual memory | `skill_manage` / `extension_manage` so A4L can write its own reusable `.md` skills and extension crates, backed by bounded `MEMORY.md` + SQLite FTS5 episodic search. |
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
   * **A4L Studio:** Visual manager to toggle `.md` skills, manage installed extensions, inspect MCP servers, and tweak mode/persona YAML rules.

### 5.1 Headless Mode & Remote Channels

* **Headless today:** `a4l run --watch` (M1). **Service mode:** `a4l serve` (M2) runs the core as a background service with a local API: a versioned, transport-agnostic stream of events (task state, gate results, handoff packets, approval requests, user questions) and commands (submit task, approve, answer a question, send back, switch model, pause). Bound to localhost with token auth by default.
* **One front-end interface.** The CLI and desktop embed the core directly; the server exposes the same events and commands remotely. A chat channel is just another front end, so nothing in the core changes when channels are added.
* **Channels are extensions.** A `Channel` trait in `a4l-ext-api` lets an extension bridge a chat service to the server API. The base ships none; a Telegram reference extension proves the trait (M2).
* **Handoff doubles as the remote card.** The L1 packet (§11.3) is already a compact, exact status: it is what a chat or phone screen shows. The full diff is sent only on request, via the packet's pointer.

Safety rules (remote control of a coding agent is high-risk):

1. A4L opens no inbound port for channels. Channel adapters connect outbound (for example bot polling); remote reach beyond that is the user's own tunnel.
2. Per-channel allowlist of user/chat IDs; messages from anyone else are ignored.
3. `deny` rules can never be overridden remotely. `ask` requests may be answered remotely, but the final merge approval is off remotely by default (`allow_remote_merge: false`) *(proposed)*.
4. Each approval is bound to one request id and expires; there is no blanket "approve all"; a timeout is a denial.
5. Headless `ask` pauses and notifies; it never auto-approves.
6. Channels receive trimmed output only, never secrets.

**Far future, keep in view only (not planned):** iOS and Android companion apps as further remote front ends over the same API. They would need a secure remote-access design (relay or tunnel) and push notifications. For now the only obligation is to keep the server API versioned and transport-agnostic; nothing is built.

---

## 6. Dynamic Extensibility Engine ("Freedom to the Max")

Users — or the A4L Meta-Architect agent itself — can customize five layers inside `.a4l/` (four hot-reload as files; code extensions rebuild per §3):

1. **Role & Persona Definitions (`.a4l/roles/*.md` & `.a4l/modes/*.yaml`):** Defines which tools, file glob permissions (`allow_write: ["src/**"]`, `deny_write: ["tests/**"]`), and verification tiers apply to each role, plus when to transition.
2. **Markdown Skill Sets (`.a4l/skills/*.md`):** Progressive disclosure playbooks with YAML frontmatter triggers (`name`, `description`, `when_to_load`).
3. **Code Extensions & Custom Tools (Rust crates against `a4l-ext-api`, installed via `a4l ext add`):** Register new LLM tools, custom L4 test runners, or lifecycle interceptors (`beforeToolCall`, `onCircuitBreakerTrip`, `onRoleTransition`). Compiled into the user's own binary; the base stays available via `a4l --no-extensions`.
4. **Native MCP Connections (`.a4l/mcp.json`):** Mount any standard Model Context Protocol server over `stdio` or `SSE/HTTP`.
5. **Declarative Policies (`.a4l/policies/*.yaml`):** Fine-grained `allow | ask | deny` rules per command, file path, or git operation.

The **Meta-Harness Assistant ("A4L Architect")** is a built-in capability letting users say things like *"Help me build an extension that queries my local Postgres schema"* or *"Create a Security Reviewer persona."* A4L scaffolds the exact extension crate, `.md`, or `.yaml` file against internal schemas and validates it. File-based changes hot-reload; a code extension goes through the §3 build flow and the session resumes from `.a4l/` state.

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
│   ├── a4l-ext/                # Extension manager: `a4l ext add`, build, swap, --no-extensions launcher
│   ├── a4l-ext-api/            # Public API crate extensions compile against (Extension trait, registry)
│   ├── a4l-mcp/                # Native MCP client implementation (stdio / SSE)
│   ├── a4l-policy/             # ACL file locks, HITL gates, L1-L4 stop-hook verifier
│   ├── a4l-memory/             # Bounded MEMORY.md manager + SQLite FTS5 episodic store
│   ├── a4l-pty/                # Rolling-buffer pseudo-terminal manager
│   ├── a4l-server/             # Headless daemon + local API (events/commands) for channels and future clients
│   ├── a4l-cli/                # CLI + ratatui TUI binary target
│   └── a4l-desktop/            # Tauri v2 desktop binary target (src-tauri)
├── sdk/
│   └── a4l-ts-sdk/             # DEFERRED: optional TS authoring (embedded JS engine); not v1
└── templates/                  # Default bootstrap assets for `a4l init`
    ├── A4L.md                  # Minimal ~60-line system prompt router (kernel)
    ├── docs/
    │   └── testing.md          # L1 (Static), L2 (Unit), L3 (Smoke), L4 (Behavioral) rules
    ├── roles/                  # Role specs (planner.md, maker.md, checker.md, architect.md)
    ├── policies/               # Default HITL & immutable test-lock policies
    ├── profiles/               # Model profiles: instruction depth, pipeline depth, handoff caps
    └── skills/                 # Meta-skills (create-skill.md, create-ts-tool.md, setup-mcp.md)
```

---

## 9. Minimal System Prompt Blueprint (`templates/A4L.md`)

The kernel is **static**: no placeholders. Anything that changes (role, task, skills index, handoff) is appended by the assembler (§11.2) in a `[A4L STATE]` block after the cached prefix, so role changes and handoffs never invalidate the cache.

```markdown
# A4L Kernel Protocol

You are operating inside **A4L (Agentic 4 All)**, a deterministic,
human-in-the-loop engineering harness. This text never changes in a session.

## 1. Where Your Current State Is
- Your role, task, skills index, and latest handoff packet are in the
  `[A4L STATE]` block at the end of your context. Trust it over memory.
- **Strict Scope:** Operate ONLY on the context and TODO checklist given for
  your current role. Do not assume unverified state.

## 2. Operating Laws
- **External RAM:** State lives in `.a4l/tasks.json`; log milestones and
  command exit codes to `.a4l/progress.md`.
- **Progressive Disclosure:** Never read all docs upfront:
  - Running or verifying code? -> Read `docs/testing.md` (L1 Static,
    L2 Unit, L3 Smoke, L4 Behavioral).
  - Matching a skill in the index? -> Read that `.a4l/skills/*.md` first.
- **Role Transition:** You may only request a transition when every item in
  your role's TODO checklist is verified with exit code `0`.
- **Handoff:** When asked, write the short handoff note (goal, decisions and
  why, next step, pitfalls). Do not restate facts the harness already holds.
- **HITL Authority:** On a `HITL_PAUSE` or the final merge gate, halt and
  await human approval.

## 3. Self-Customization
- If the user asks to create/edit a skill, extension, MCP server, role/mode,
  or policy, load `.a4l/skills/meta-architect.md` FIRST.
```

---

## 10. Implementation Roadmap

### Milestone 1 — Rust State Engine & Role Loop (`a4l-core` + `a4l-cli`)
* [ ] Scaffold `.a4l/` workspace (`init.sh`, `tasks.json`, `progress.md`, `A4L.md`, `docs/testing.md`).
* [ ] Implement the Planner → Maker → Checker → HITL state machine driven by `tasks.json` TODO completion, with the §2.4 transition guards.
* [ ] Implement the L1–L4 verification runner, immutable `tests/**` git-diff guard, and identical-error circuit breaker with automatic Planner Rebase.
* [ ] LLM provider trait with ONE built-in implementation: OpenAI-compatible Chat Completions (§2.9), including the quirk handling and `providers.yaml`. Native providers are extensions.
* [ ] Freeze the §2.8 JSON Schemas; ship a stub/mock provider so the full role loop is testable offline.
* [ ] CLI MVP: `a4l init`, `a4l doctor`, `a4l run --task`, plain-text status. The ratatui TUI follows once the headless path works.
* [ ] The six built-in tools (§2.10) behind the policy engine: `read`, `write`, `edit`, `bash`, `todo` (pipeline and freeform), and `ask_user` (plain-text prompt first).
* [ ] Assembler with the §11.2 cache-stable layer order; static kernel with no placeholders (§9).
* [ ] Handoff builder with rolling facts + capped model note, `latest.json` schema, and the §11.5 model/mode-switch procedure.
* [ ] Token ledger (`ledger.jsonl`) and minimal tool-output trimming (head + tail + log pointer). The full `a4l-pty` stays in M3.
* [ ] Model profiles (`lite` / `standard` / `strict`) and the §11.8 handoff acceptance tests.
* **Acceptance:** `a4l` runs a sample task that must pass a locked shell test script before exiting; a task with a failing gate cannot be marked complete by the model.
* **Acceptance (breaker):** a sample task with a deliberately unfixable error trips the breaker after 3 attempts, stashes to `wip/<task-id>-attempt-<n>`, and re-enters Planner with a failure bundle.

### Milestone 2 — Extensibility & Headless (`a4l-ext` + `a4l-ext-api` + `a4l-mcp` + `a4l-server` + Meta-Skills)
* [ ] Ship `a4l-ext-api` plus the `a4l ext add/remove/list/rebuild` flow, `install.sh`, and the `--no-extensions` base launcher per §3.
* [ ] Reference native-provider extension (Anthropic Messages API with `cache_control`) built against `a4l-ext-api`, proving the provider trait.
* [ ] `a4l serve` headless daemon + local API (`a4l-server`) and the `Channel` trait in `a4l-ext-api`; reference Telegram channel extension with allowlist and request-bound approvals (§5.1).
* [ ] Integrate MCP client (`.a4l/mcp.json`) and the progressive `.md` skill loader (skills index delivered in the `[A4L STATE]` block).
* [ ] Ship the `meta-architect` skill so A4L can scaffold, build, and install extensions.
* **Acceptance:** the agent creates a custom extension using `meta-architect.md`, it builds and installs after HITL review, and the tool runs in the resumed session; `a4l --no-extensions` runs clean without it; a deliberately broken extension fails the build and leaves the previous binary working.

### Milestone 3 — Memory & Context Hygiene (`a4l-memory` + `a4l-pty`)
* [ ] Sidechain `.jsonl` logging for sub-tasks, rolling PTY buffers, SQLite FTS5 episodic search, bounded `MEMORY.md`.
* **Acceptance:** long sessions show no context growth from tool noise; episodic search retrieves prior failure bundles by symptom.

### Milestone 4 — Desktop Control Center (`a4l-desktop`)
* [ ] Tauri v2 GUI: visual Role Pipeline board, L1–L4 proof inspector, HITL diff sign-off station, and visual Extension/Persona studio.
* **Acceptance:** a full task lifecycle (plan → make → check → approve) is completable end-to-end from the GUI without touching the CLI.

### Future, keep in view only — iOS / Android companion apps
Not planned and not scheduled. Remote front ends over the §5.1 server API; revisit after Milestone 4.

---

## 11. Handoff & Context Engineering — A4L's Core Feature

### 11.1 Principle

Context is the scarce resource. A **handoff** is a compact, structured packet that lets a different role, mode, **model**, or session continue a task without replaying history. One packet format serves every case: role → role, mode switch, model switch mid-session, crash recovery, and resume.

Handoff and the assembler are **built into `a4l-core`**, not delivered as MCP tools, skills, or extensions:

1. They must run when the model cannot be asked (switch, crash, resume), so they cannot be model-invoked tools.
2. Facts must be exact. The harness reads gate results, todo state, and breaker state straight from `tasks.json`; a model summary could misstate them.
3. Cache stability is a byte-level property; exactly one component owns the prompt layout.
4. No process hop or per-turn tool-schema cost for the feature everything depends on.
5. A third party must not be able to alter what the model sees or drop gate results from a packet.

Extensions and MCP can add to it, never own it: an `onHandoffBuild` hook may append a namespaced section (counted against budget, unable to modify harness facts); MCP servers may contribute pointers only.

### 11.2 The Assembler

The assembler builds the exact bytes sent on every model call, in this fixed order:

| # | Layer | Stability |
| --- | --- | --- |
| 1 | Kernel (`A4L.md`, no placeholders) | static, cached |
| 2 | Tool schemas for the active mode | static per mode, cached |
| 3 | Model-profile instructions (§11.6) | static per profile, cached |
| 4 | Role / mode doc | changes at handoff |
| 5 | `[A4L STATE]` block: role, task id, skills index | changes at handoff |
| 6 | Handoff packet (L1) | changes at handoff |
| 7 | Tail: last 2–4 turns verbatim | rolling |
| 8 | New input | per turn |

Rules:

* Layers 1–3 are byte-identical within a session and across role handoffs, so the cache survives role changes. A mode switch changes layer 2 (different tool set) and a model switch changes everything; both are expected cache misses, which is exactly why the packet exists.
* Every layer has a token budget; the assembler enforces it, trimming from the tail first, then the note, never the facts.
* Tool and gate output enters context as exit code + first failing error + a log pointer; full output stays in `.a4l/history/`.
* Rendering is per model: tool-call format, tokenizer, context window.
* Every call's per-layer token counts are written to the token ledger (§11.7).

### 11.3 The Handoff Packet

Three levels, loaded only as needed:

* **L0 pointer:** one line (task, state, next action); enough to resume a paused task.
* **L1 packet:** the default for any receiver (below).
* **L2 artifacts:** full diff, gate logs, failure bundles, history. Stay on disk; fetched by path on demand, never pasted.

Who writes what:

| Part | Author | Source |
| --- | --- | --- |
| task id, role, status, attempt, trigger | harness | `tasks.json` |
| todo states (`pending` / `done` / `verified`) | harness | `tasks.json` |
| gate results: exit code + first failing error | harness | gate runner |
| changed files, diff stat | harness | `git diff --stat` |
| breaker state, failed-approach hashes | harness | `errors[]`, failure bundles |
| goal (one line), decisions + why, next step, pitfalls, open questions | model | the **note**, hard token cap |
| pointers to L2 artifacts | harness | paths only |

Stored at `.a4l/handoffs/<task-id>.latest.json`; schema frozen with the §2.8 schemas:

```jsonc
{
  "v": 1, "task_id": "T-0001", "seq": 7, "created_at": "...",
  "trigger": "todo_verified", // role_transition | mode_switch | model_switch | session_end | breaker_trip | manual
  "from": { "role": "maker", "mode": "build", "model": "..." },
  "facts": { "status": "...", "todos": [], "gates": {}, "diff_stat": {}, "breaker": {} },
  "note": { "goal": "", "decisions": [{ "what": "", "why": "" }], "next": "", "pitfalls": [], "open_questions": [] },
  "note_stale": false,
  "refs": { "diff": "", "gate_logs": [], "failure_bundle": null, "history": "" }
}
```

Conflict rule: if a note field contradicts `facts`, facts win. The cap is enforced by the harness: an over-long note is rejected once with a request to shorten, then truncated by field priority (`next`, `pitfalls`, `decisions`, `goal`). Default cap values are set after measuring real packets in Milestone 1.

### 11.4 Rolling Handoff

The packet is kept current, not built at the last second:

* **Facts** refresh automatically at every boundary (todo verified, gate run, role transition, breaker trip). This costs no model tokens.
* **Note** refreshes at role transitions, at batches of verified todos, before a mode or model switch, and at session end.
* After a crash the facts are current and the note may lag; the packet is marked `note_stale` and the next model is told.

### 11.5 Model Switch, Mode Switch, Resume

Switching models mid-session must not re-push the whole transcript into the new model (a new model has no cache, and a transcript is the most expensive thing to resend). Procedure:

1. Freeze the current turn; refresh facts.
2. If the note is stale and the outgoing model is reachable, request a fresh note from it (its cache is still warm, so this is cheap). Otherwise ship facts only, flagged.
3. The assembler re-renders for the incoming model: its profile, tool-call format, tokenizer, and window.
4. The new model starts with layers 1–6 plus the tail. History stays on disk.
5. If the new window is smaller, cut the tail first, then the note; never the facts.

Mode switches and session resume use the same procedure. The cost is lossiness (the new model lacks conversational nuance); mitigations are the verbatim tail and pointer access to `.a4l/history/`.

### 11.6 Model Profiles (`.a4l/profiles/*.yaml`)

Frontier and low-end models need different amounts of instruction. A profile per model (or model class) sets:

| Field | Purpose |
| --- | --- |
| `instruction_depth` | `lite` / `standard` / `strict` text in layer 3 and the role docs |
| `pipeline_depth` | how many model calls per task (below) |
| `tail_turns` | verbatim turns kept in handoffs |
| `note_max_tokens` | cap for the model-written note |
| `tool_output_limit` | head/tail limits for tool output |
| `examples` | include worked examples (usually for weak models only) |

| Pipeline depth | Flow | Typical use |
| --- | --- | --- |
| `lite` | one agent plans and makes → gates → human | frontier models, small tasks |
| `standard` | Planner → Maker → gates → human | most work |
| `strict` | Planner → Maker → Checker → human | weak models, risky changes |

Gates (L1–L4), the test fence, the circuit breaker, and the human sign-off are enforced in Rust at every depth; only the number of model calls changes.

### 11.7 Token Ledger and Budgets

* Each model call records tokens per layer (§11.2), cache hits, and tool-output bytes trimmed, written to `.a4l/history/ledger.jsonl`.
* CI budget tests fail the build if the kernel + tool schemas, the L1 packet, or the cache-hit rate regress beyond set ceilings. Ceilings are chosen from measured Milestone 1 data, not guessed in advance.

### 11.8 Handoff Acceptance Tests

* **Resume equivalence:** a task resumed from the packet alone completes as reliably as one with the full transcript (small eval set run on a frontier and a small model).
* **Crash safety:** `kill -9` mid-task, then resume from the latest packet.
* **Model switch:** a task started on model A finishes on model B; tokens sent at the switch are only layers 1–6 plus the tail.
* **Facts integrity (property test):** no model-written field can change `facts`; on conflict facts win.
* **Cap enforcement:** an over-cap note is rejected, then truncated by priority.

---

## Appendix A — Reference Harnesses

The design distills patterns from five open harnesses; reference checkouts live in `ref/`:

`claude-code`, `deepseek-harness`, `hermes-agent`, `pi` (Mario Zechner's minimal coding-agent harness, now maintained under Earendil), `zeroclaw`, `gemini-cli`, `opencode`.
