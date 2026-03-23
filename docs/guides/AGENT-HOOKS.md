# Agent Hooks — Auto-integrate Forgeplan with AI Agents

Forgeplan can automatically provide project context to AI agents through hooks.
This guide covers integration with Claude Code, but the same principles apply to any agent framework.

## SessionStart Hook (Claude Code)

Add to `.claude/settings.json`:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "matcher": "",
        "command": "forgeplan health --compact --json 2>/dev/null || true"
      }
    ]
  }
}
```

This runs `forgeplan health` at the start of every prompt, giving the agent project context automatically.

### What the agent sees

`forgeplan health --compact --json` returns a compact JSON payload (< 500 tokens):

```json
{
  "total": 27,
  "active": 11,
  "draft": 16,
  "blind_spots": 0,
  "at_risk": 2,
  "stale": 1,
  "next_action": "Review EVID-003 — evidence expires in 3 days"
}
```

This tells the agent:
- How many artifacts exist and their status breakdown
- Whether there are blind spots (decisions without evidence)
- What the most urgent next action is

## PostToolUse Hook

Remind the agent to capture decisions after significant file changes:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "command": "echo '[Forgeplan] Consider: does this change represent a decision worth capturing? Use forgeplan capture if so.'"
      }
    ]
  }
}
```

This is intentionally lightweight — a text reminder, not an automated action.

## Route-Before-Work Hook

Auto-determine depth before starting work on a task:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "matcher": "implement|build|create|add feature",
        "command": "forgeplan route \"$PROMPT\" --json 2>/dev/null || true"
      }
    ]
  }
}
```

The agent receives routing guidance:
```json
{
  "depth": "Standard",
  "pipeline": ["PRD", "RFC"],
  "confidence": 85
}
```

## MCP Server (Recommended)

For deeper integration, run Forgeplan as an MCP server:

```bash
forgeplan serve
```

This exposes all 26+ tools via MCP stdio transport, giving the agent full CRUD access to artifacts, validation, scoring, and search without needing CLI hooks.

### Claude Code MCP config

Add to `.claude/settings.json`:

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve"],
      "cwd": "/path/to/your/project"
    }
  }
}
```

## Best Practices

1. **Start with health** — the SessionStart hook gives the agent situational awareness
2. **MCP > hooks** — MCP provides structured tool access; hooks are text-only
3. **Keep hooks lightweight** — `2>/dev/null || true` prevents hook failures from blocking the agent
4. **Don't over-automate** — the PostToolUse hook should suggest, not force artifact creation
5. **Route before work** — helps the agent decide whether to create artifacts or just code
