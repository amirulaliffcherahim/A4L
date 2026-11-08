# a4l-ext — PLACEHOLDER (Milestone 2)

**Purpose:** extension manager. Builds user-installed extensions into the user's own `a4l` binary while the base stays untouched.

**Spec:** GOAL.md §3 (locked decision: build-into-binary).

## What to build
1. Registry `~/.a4l/extensions.toml`: source, pinned commit, content hash per extension.
2. `a4l ext add <git-url|path> | remove | list | rebuild`.
3. Install flow: fetch + pin -> HITL review (show source, Cargo deps, `build.rs`/proc-macro use) -> build in temp dir with a cached target dir -> `--selftest` smoke -> atomic swap. Any failure leaves the previous binary working.
4. Launcher: `a4l` runs the custom build if present, else base. `a4l --no-extensions` execs `a4l-base`. Two failed startups of the custom build = offer base.
5. `a4l doctor` checks for `rustc`/`cargo`; no toolchain means extensions unavailable, base unaffected.
6. After a rebuild, restart and resume from `.a4l/` state.
7. Build speed: shared target dir, tuned release profile (high codegen-units, `lld`/`mold` if present). Measure and report build time.

## Do NOT
- Modify `a4l-base` ever.
- Claim in-process extensions are sandboxed; they are reviewed-then-trusted.

## Done when
- A sample extension installs, its tool runs after resume, `--no-extensions` runs clean, and a deliberately broken extension fails the build without breaking the install.
