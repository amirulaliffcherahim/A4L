# a4l-mcp — PLACEHOLDER (Milestone 2)

**Purpose:** native Model Context Protocol client so users can mount any standard MCP server.

**Spec:** GOAL.md §4 (gemini-cli pattern), §6 (layer 4).

## What to build
1. Read `.a4l/mcp.json`; support `stdio` and SSE/HTTP transports.
2. Discover each server's tools and expose them to the agent through the same tool registry as built-ins.
3. Route every MCP tool call through `a4l-policy` (default `ask` for unknown servers/tools).
4. Server lifecycle: start lazily, restart on crash, clean shutdown, per-call timeout.
5. `a4l doctor` integration: report which servers start and which fail.

## Do NOT
- Bypass policy because a tool came from a "trusted" server.

## Done when
- A sample stdio MCP server's tool is listed, policy-checked, and callable in a task.
