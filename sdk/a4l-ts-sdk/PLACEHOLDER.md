# sdk/a4l-ts-sdk — PLACEHOLDER (DEFERRED, not v1)

**Status:** deferred. Native extensions are Rust (`a4l-ext-api`). Do not build this now.

**Only if TypeScript authoring is wanted later:** embed a small JS engine (e.g. QuickJS) in the base, bundle TS extensions at `a4l ext add` time, and expose the same registry through `@a4l/sdk` types (`defineTool`, hook types, capability `ctx` routed through `a4l-policy`). Decide this after M2 ships.
