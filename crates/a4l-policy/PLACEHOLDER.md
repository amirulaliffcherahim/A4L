# a4l-policy — PLACEHOLDER (Milestone 1 minimum, expanded in M2)

**Purpose:** gatekeeper evaluated before EVERY tool call: `allow` / `ask` / `deny`. File-lock ACLs, HITL gates, stop-hook verifier.

**Spec:** GOAL.md §7, §2.5, §6 (policies).

## What to build
1. YAML policy loader (`.a4l/policies/*.yaml`) with rules per command pattern, file path glob, and git operation. Use a maintained YAML crate (not `serde_yaml`).
2. Rule engine: `evaluate(request) -> Allow | Ask(reason) | Deny(reason)`. Default for unknown = `ask`.
3. Role/mode ACLs: Planner read-only; Maker writes `src/**` but `tests/**` is read-only; Checker read-only.
4. Immutable test fence: while role is Maker, deny writes to `tests/**`; verify with `git diff --exit-code tests/` against the Planner's locked commit; unauthorized diff = auto-revert + failed gate.
5. Stop-hook verifier: refuse `completed` unless all L1–L4 gates have harness-observed exit `0`.
6. Deny edits to `.a4l/policies/` during untrusted runs.

## Do NOT
- Trust extension or model claims about what a call will do; evaluate the actual request.

## Done when
- Table-driven tests cover allow/ask/deny for commands, paths, and git ops, plus a test-tamper case that fails the gate.
