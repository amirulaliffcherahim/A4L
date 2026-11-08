---
name: setup-mcp
description: How to configure and verify an MCP server in .a4l/mcp.json.
when_to_load: user asks to add or configure an MCP server
---

# Setting Up Model Context Protocol (MCP) Servers

MCP servers provide custom tools via JSON-RPC.

## Configuration File: `.a4l/mcp.json`
```json
{
  "mcpServers": {
    "sqlite": {
      "command": "uvx",
      "args": ["mcp-server-sqlite", "--db-path", "test.db"],
      "env": {}
    }
  }
}
```

## Procedure
1. Add entry under `mcpServers` in `.a4l/mcp.json`.
2. Run `a4l doctor` to verify server connectivity and discovered tools.
3. Call tools through A4L. Note: all MCP calls are subject to `a4l-policy`.
