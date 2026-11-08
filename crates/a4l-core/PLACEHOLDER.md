# a4l-core — PLACEHOLDER (Milestone 1, build FIRST)

**Purpose:** the deterministic engine. State machine, role pipeline, transition guards, L1–L4 gate runner, circuit breaker. Shared by CLI and desktop. No UI, no LLM-specific code here.

**Spec:** GOAL.md §1, §2.2–§2.8.

## What to build
1. `state` module: types for `.a4l/tasks.json` exactly as GOAL §2.3.
   - Todo status is tri-state: `pending -> done -> verified`. Never a plain bool.
   - Gates carry `cmd`, `exit`, `log`, `ran_at` for all four tiers (L1–L4).
   - `errors[]` with `{hash, count, last_seen}`; `circuit_breaker` object; `version`; `context_bundle`.
   - One status vocabulary only (use the §2.3 names; role `human`, not `hitl`).
2. Atomic persistence: write temp file + rename, behind a file lock (CLI and desktop may both write).
3. Transition guards (§2.4) as pure functions: `can_advance(task, to) -> Result<(), GuardFailure>`. The model only *proposes*; only this crate writes `status` / `role`.
4. Gate runner: execute L1–L4 commands, capture exit code + log, store results on the task. Only the harness observed exit `0` counts.
5. Error hashing: `sha256(normalized stderr + compilation unit)`; strip paths and line numbers before hashing.
6. Circuit breaker (§2.6): duplicate hash x2, diff oscillation within N=3 snapshots, or 3rd Maker attempt ending red (`max_attempts=3`, configurable). On trip: stash to `wip/<task-id>-attempt-<n>`, reset tree, purge bundle, write `.a4l/failures/<task-id>.json`, route to Planner with `attempt+1`.
7. `LlmProvider` trait + a **mock provider** so the whole role loop is testable offline. The ONLY built-in real provider is OpenAI-compatible Chat Completions (GOAL §2.9): `providers.yaml`, quirk flags, stream-usage fallback with local token estimates, tolerant SSE parsing, malformed tool-call retry. Native providers (Anthropic, Gemini) are extensions, not core.
8. Freeze JSON Schemas for tasks.json, failure bundle, checker verdict (§2.8).
9. `assembler` module (GOAL §11.2): fixed layer order, cache-stable prefix (kernel, tool schemas, profile text byte-identical across handoffs), per-layer budgets, per-model rendering. Appends the `[A4L STATE]` block; the kernel has no placeholders.
10. `handoff` module (GOAL §11.3–11.5): rolling packet at `.a4l/handoffs/<task-id>.latest.json`. Facts come only from harness state; the model writes only the capped note; facts win on conflict. Implement the model-switch / mode-switch / resume procedure.
11. `ledger` module (GOAL §11.7): per-call token counts by layer to `.a4l/history/ledger.jsonl`, plus CI budget tests.
12a. `tools` module (GOAL §2.10): `read`, `write`, `edit`, `bash`, `todo`, `ask_user`, each routed through `a4l-policy`. One-sentence descriptions. `edit` returns a diff only; `bash` returns exit code + first failing error + log pointer; `todo` is pipeline-aware (model sets `done` only) with a freeform fallback at `.a4l/session/todos.json`; `ask_user` goes through the `Frontend` interface and degrades to "no interactive channel" when headless.
12. Model profiles loader (`.a4l/profiles/*.yaml`): `instruction_depth`, `pipeline_depth` (lite/standard/strict), `tail_turns`, `note_max_tokens`, `tool_output_limit`.

## Do NOT
- Let any code path set `completed` from model output.
- Depend on a4l-cli, desktop, or extensions.
- Move the assembler or handoff into an extension or MCP server; they are core by design (GOAL §11.1).

## Done when
- The §11.8 handoff tests pass: resume equivalence, crash safety, model switch, facts integrity, cap enforcement.
- Every guard in §2.4 has a unit test (pass and fail case).
- A task with a failing gate cannot reach Checker; a deliberately unfixable task trips the breaker after 3 attempts and lands in Planner with a failure bundle.

## Open decisions
- Final status/role names (pick once; they land on users' disks).
- Crate for YAML: `serde_yaml` is archived; use a maintained one (e.g. `serde-saphyr`, `serde_norway`).
