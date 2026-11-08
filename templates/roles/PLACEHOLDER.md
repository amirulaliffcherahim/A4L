# templates/roles — PLACEHOLDER (Milestone 1)

**Purpose:** role specs the kernel prompt points to. Each role sees only its own context slice.

## What to build (one short `.md` each)
- `planner.md`: reads architecture + backlog; writes the micro-spec context bundle, TODO checklist, acceptance criteria; writes and locks L2/L4 tests; hands ONLY the bundle to Maker. Read-only on source.
- `maker.md`: implements from the bundle only; runs L1 -> L4; `tests/**` read-only; may only *request* a transition; on breaker trip, expects a fresh re-plan.
- `checker.md`: adversarial audit of raw git diff + gate logs vs acceptance criteria; looks for test tampering and swallowed errors; read-only; writes a verdict object; cannot mark anything complete.
- `architect.md`: the Meta-Architect; scaffolds skills, TS tools, MCP configs, roles/modes, policies against the schemas, validates, hot-reloads.
- Each file: duties, allowed tools/paths, transition request rules, what NOT to do.
- Also add `modes/*.yaml` examples (path ACLs and tool grants per role) if modes are shipped in M1.
