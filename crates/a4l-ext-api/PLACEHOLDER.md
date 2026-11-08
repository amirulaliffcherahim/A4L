# a4l-ext-api — PLACEHOLDER (Milestone 2, design early)

**Purpose:** the small public crate extensions compile against. Keep it tiny and stable; it is the contract with every third-party extension.

**Spec:** GOAL.md §3, §6 (layer 3).

## What to build
1. `Extension` trait with `register(&self, registry: &mut Registry)`.
2. `Registry` to add: tools (name, description, JSON schema, handler), lifecycle hooks (`beforeToolCall`, `onStop`, `onRoleTransition`, `onCircuitBreakerTrip`, `onHandoffBuild`), L4 runners, and **providers** (the `LlmProvider` trait). Native providers such as Anthropic's Messages API arrive this way because the base ships only OpenAI-compatible.
2b. `Channel` trait for chat bridges (Telegram etc.): receive messages, send status cards and approval requests, talk to the `a4l-server` API. Channels connect outbound only; enforce allowlist and request-bound approvals per GOAL §5.1.
3. Host handle given to extensions for fs/net/exec so every action goes through `a4l-policy`. Extensions are expected to use it; document that Rust cannot enforce this in-process.
4. Semver the API; the extension manager must refuse to build an extension targeting an incompatible version.
5. An extension template (`cargo generate`-style) the Meta-Architect scaffolds from.

## Do NOT
- Expose `a4l-core` internals. Anything not in this crate is private.
