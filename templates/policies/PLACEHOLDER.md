# templates/policies — PLACEHOLDER (Milestone 1)

**Purpose:** default `allow | ask | deny` rules shipped by `a4l init`.

## What to build (YAML files)
- `default.yaml`:
  - allow: reading files, running unit tests/linters.
  - ask: destructive shell commands (`rm -rf`, force push), writes outside the project, modifying locked dirs, final merge.
  - deny: editing `.a4l/policies/` during untrusted runs.
- `test-lock.yaml`: while role is Maker, deny writes to `tests/**`; trigger auto-revert on diff.
- Comment every rule so users can edit with confidence. Schema must match what a4l-policy loads.
