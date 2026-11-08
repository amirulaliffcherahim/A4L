# templates/ — PLACEHOLDER (Milestone 1)

**Purpose:** default assets that `a4l init` copies into a user's project as `.a4l/` (and `docs/`). Embed them in the binary (e.g. `include_dir`) so `init` works offline.

## What to build (top level)
- `A4L.md`: the kernel prompt, ~60 lines max. Blueprint is in GOAL.md §9. It is STATIC (no placeholders); the assembler appends the `[A4L STATE]` block.
- `init.sh`: pre-flight script with `check` (emits env_state JSON) and `smoke` (L3 gate). See GOAL.md §2.7. User-owned after init.
- `providers.yaml`: one commented example endpoint for an OpenAI-compatible server (`base_url`, `api_key_env`, `model`, quirk flags). No other providers.
- `tasks.json`: empty valid queue `{ "version": 1, "queue": [] }`.
- `progress.md`: empty append-only ledger with a one-line header.

Subfolders each have their own PLACEHOLDER.md: `docs/`, `roles/`, `policies/`, `profiles/`, `skills/`.
