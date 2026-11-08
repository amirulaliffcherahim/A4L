# a4l-server — PLACEHOLDER (Milestone 2)

**Purpose:** run `a4l-core` as a background service so chat channels (Telegram etc.) and future clients can drive A4L. Base feature; channel adapters themselves are extensions.

**Spec:** GOAL.md §5.1.

## What to build
1. Local API over WebSocket/HTTP, bound to localhost with token auth by default. Version it from day one and keep it transport-agnostic.
2. Event stream out: task state changes, gate results, handoff L1 packets, approval requests, user questions (`ask_user`), errors.
3. Commands in: submit task, approve / send back / answer a question (bound to a request id), switch model, pause, status.
4. Same `Frontend` interface the CLI and desktop use, so the core does not change per client.
5. Approval handling: per-request id, expiry, timeout = deny, no blanket approve, `deny` rules never overridable, `allow_remote_merge` off by default.
6. Never send secrets; send trimmed output and the handoff packet, with the full diff only on request.

## Do NOT
- Open an inbound port for channels or expose the API beyond localhost by default.
- Auto-approve anything while headless.
- Build mobile app support. That is far-future and not planned; just keep the API clean.

## Done when
- A task submitted from a test channel runs, pauses on `ask`, is approved by id, and completes; an unknown sender is ignored; an unanswered `ask` expires as a denial.
