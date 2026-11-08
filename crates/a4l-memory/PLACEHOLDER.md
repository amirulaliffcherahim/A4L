# a4l-memory — PLACEHOLDER (Milestone 3)

**Purpose:** keep context small over long sessions. Bounded `MEMORY.md` plus SQLite FTS5 episodic search.

**Spec:** GOAL.md §4 (hermes-agent pattern), §10 (M3).

## What to build
1. Bounded `MEMORY.md` manager: hard size cap, consolidate instead of append forever.
2. SQLite FTS5 episodic store: index failure bundles, progress entries, and past task summaries.
3. Query API: retrieve prior failure bundles by symptom (error text / hash).
4. Sidechain writer: sub-task chatter and tool dumps go to `.a4l/history/*.jsonl`, never back into the main context.

## Done when
- Long sessions show no context growth from tool noise; searching a symptom returns the matching earlier failure bundle.
