---
title: MCP Tools Overview
description: 28 MCP tools for AI agents — create, validate, score, manage artifacts
---

## What is MCP?

Model Context Protocol (MCP) lets AI agents (Claude Code, GPT, Cursor) interact with Forgeplan directly. 28 tools available.

## Setup

Add to your project's `.mcp.json`:

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve"]
    }
  }
}
```

## Tool Categories

### Create
| Tool | Description |
|------|-------------|
| `artifact_create` | Create artifact from structured input |
| `artifact_from_description` | AI generates artifact from natural language |
| `evidence_create` | Create evidence pack |

### Analyze
| Tool | Description |
|------|-------------|
| `validate` | Validate artifact against schema rules |
| `score` | Compute R_eff quality score |
| `health` | Project health dashboard |
| `blindspots` | Find decisions without evidence |

### Navigate
| Tool | Description |
|------|-------------|
| `search` | Smart search (keyword + semantic) |
| `graph` | Dependency graph |
| `blocked` | Show blocked artifacts |
| `context` | Full reasoning context for artifact |

### Decide
| Tool | Description |
|------|-------------|
| `route` | Suggest depth + pipeline |
| `reason` | ADI reasoning cycle |
| `decompose` | Break PRD into RFC tasks |
| `capture` | Capture decision from conversation |

## Example

AI agent conversation:
```
User: "Add payment processing"

Agent calls: route("Add payment processing")
→ Depth: Deep, Pipeline: PRD → Spec → RFC → ADR

Agent calls: artifact_create(kind: "prd", title: "Payment Processing")
→ Created: PRD-026

Agent calls: validate("PRD-026")
→ PASS ✓
```
