# templates/skills — PLACEHOLDER (Milestone 2)

**Purpose:** bootstrap meta-skills: Markdown playbooks with YAML frontmatter (`name`, `description`, `when_to_load`) that teach A4L to customize itself. Indexed (name + one line each) into the `[A4L STATE]` block.

## What to build
- `meta-architect.md`: master skill. Loaded FIRST when the user asks to create/edit a skill, TS tool, MCP server, role/persona, or policy. Says which sub-skill to use and to validate before reload.
- `create-skill.md`: how to author a skill file correctly (frontmatter, scope, when_to_load, keep it short).
- `create-ts-tool.md`: how to write an `@a4l/sdk` extension (depends on the runtime decision).
- `setup-mcp.md`: how to add and test an entry in `.a4l/mcp.json`.
- Optional: `create-role.md`, `create-policy.md`.
- Each skill ends with a validation checklist the Architect must run before hot-reload.

## Done when
- The agent uses `meta-architect.md` to create a working custom tool and calls it on the next turn.
