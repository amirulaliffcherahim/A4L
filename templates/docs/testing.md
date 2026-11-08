# A4L Testing Ladder (L1–L4)

A task cannot advance to Review or Checker unless all configured verification gates return exit code 0.
Model claims or ticked checkboxes are meaningless without harness-observed proof.

## Gates
- **L1 (Static Check):** Linters, AST checks, typechecking (`cargo check`, `tsc --noEmit`, `eslint`).
- **L2 (Unit / Dynamic):** Deterministic unit and module tests tied directly to acceptance criteria.
- **L3 (Smoke Test):** Booting service/binary, container health, zero startup crashes (`.a4l/init.sh smoke`).
- **L4 (Behavioral / E2E):** Real execution proof (live API endpoint curls, e2e CLI flows).

## Maker Rules
1. `tests/**` is strictly read-only for Maker. Any edit attempt is denied by policy.
2. Only harness-executed commands with exit `0` satisfy verification.
