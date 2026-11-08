# templates/profiles — PLACEHOLDER (Milestone 1)

**Purpose:** model profiles loaded by the assembler. Frontier and low-end models need different amounts of instruction, so each profile sets how much text and how many model calls to use.

**Spec:** GOAL.md §11.6.

## What to build (YAML files)
- `frontier.yaml`: `instruction_depth: lite`, `pipeline_depth: lite`, small `tail_turns`, tight `note_max_tokens`, `examples: false`.
- `standard.yaml`: middle values for most models.
- `small.yaml`: `instruction_depth: strict`, `pipeline_depth: strict`, more explicit role docs, `examples: true`.
- A mapping from model id (or glob) to profile, with a safe default.
- Comment every field so users can tune it. Gates, test fence, breaker, and human sign-off are NOT configurable by profile.
- Starting values are guesses: tune them from the token ledger and the eval set, not by feel.
