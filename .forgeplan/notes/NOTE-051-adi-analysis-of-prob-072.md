---
depth: tactical
id: NOTE-051
kind: note
links:
- target: PROB-072
  relation: informs
status: draft
title: ADI analysis of PROB-072
---

```json
{
  "hypotheses": [
    {
      "id": "H1",
      "description": "Introduce an optional `worktree` parameter to all mutating tools and manage a `HashMap<PathBuf, Workspace>` in the MCP server to handle multiple concurrent worktrees.",
      "assumptions": [
        "The agent framework can instruct subagents to pass their specific worktree path via the tool parameter.",
        "Embedded LanceDB supports multiple concurrent connections to different database directories without locking conflicts."
      ],
      "confidence": "High — directly satisfies AC (a) and the per-workspace lock requirement, preserves backward compatibility (optional param), and avoids process-level environment variable limitations."
    },
    {
      "id":

