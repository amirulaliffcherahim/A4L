# a4l-desktop — PLACEHOLDER (Milestone 4)

**Purpose:** native desktop app (Tauri v2) over the same `a4l-core`. Do not start before the CLI loop works end to end.

**Spec:** GOAL.md §5 (surface 2), §10 (M4).

## What to build
1. Tauri v2 shell; Rust commands call `a4l-core` via async Tokio bridges. No duplicated logic in the frontend.
2. **Pipeline board:** live view of `tasks.json` moving Planner -> Maker -> Checker -> Human.
3. **HITL diff & proof station:** side-by-side git diff, green/red L1–L4 badges, buttons *Approve & Commit* and *Send Back with Notes*.
4. **A4L Studio:** toggle `.md` skills, edit `.ts` tools, inspect MCP servers, edit mode/persona/policy YAML (validated against the schemas).
5. Safe concurrent access to `tasks.json` with the CLI (file lock from a4l-core).

## Done when
- A full task (plan -> make -> check -> approve) completes from the GUI without touching the CLI.
