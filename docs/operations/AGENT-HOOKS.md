[English](AGENT-HOOKS.md) · [Русский](AGENT-HOOKS.ru.md)

# Agent Hooks — Auto-integrate Forgeplan with AI Agents

Forgeplan can automatically provide project context to AI agents through hooks.
This guide covers integration with Claude Code, but the same principles apply to any agent framework.

## UserPromptSubmit Hook (Claude Code)

Add to `.claude/settings.json`:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {"type": "command", "command": "forgeplan health --compact"}
        ]
      }
    ]
  }
}
```

This runs `forgeplan health` at the start of every prompt, giving the agent project context automatically.

### What the agent sees

`forgeplan health --compact` emits the compact text form. For structured data,
use `forgeplan health --json` or MCP `forgeplan_health`.

`--json` takes precedence over `--compact`: combining them returns the full JSON
dashboard, not a payload bounded to 500 tokens. Relevant JSON fields include
`total`, `by_status`, array-valued `blind_spots`/`at_risk`, `stale_count` and the
CLI hint `_next_action`. The size grows with workspace data; inspect actual output.
This hook is advisory context, not proof that validation or release gates passed.

## PostToolUse Hook

Remind the agent to capture decisions after significant file changes:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {"type": "command", "command": "echo '[Forgeplan] Consider: does this change represent a decision worth capturing? Use forgeplan capture if so.'"}
        ]
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
        "hooks": [
          {"type": "command", "command": "forgeplan route \"$(jq -r '.prompt // empty')\" --level 0"}
        ]
      }
    ]
  }
}
```

This example runs on every prompt. `UserPromptSubmit` does not match prompt
keywords through `matcher`; its JSON stdin supplies `.prompt`, not a `$PROMPT`
environment variable. `jq` is a prerequisite. These are opt-in examples, not an
installation or a change to this repository's native settings.

The agent receives human-readable CLI routing guidance, not JSON. `route` has
no `--json` flag; `--level 0` explicitly avoids LLM routing. Keep stderr and the
exit status visible so a failed hook is not mistaken for a successful recommendation.
For a structured response, use the MCP `forgeplan_route` contract instead.

## MCP Server (Recommended)

For deeper integration, run Forgeplan as an MCP server:

```bash
forgeplan serve
```

The scoped `0.39.0+constellation.s1.1` build registers 76 full-profile tools and
an additional 9-tool `constellation-read` profile. Writable `full` remains the
default; registration alone does not verify every tool's behavior.
CLI `validate` works on legacy and migrated Constellation layouts; full MCP
`forgeplan_validate` was also exercised successfully on migrated artifacts.
See the [pilot compatibility notes (RU)](CONSTELLATION-PILOT.ru.md).

### Claude Code MCP config

Add to the project's `.mcp.json` (adapt the executable and workspace paths):

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve", "--workspace", "/path/to/your/project"]
    }
  }
}
```

For a Constellation pilot, use an explicit versioned executable and
`serve --profile full --auto-index --workspace <project-root>`. Startup
maintenance scope prefers `FORGEPLAN_WORKSPACE` over the flag; request selection
prefers an explicit request workspace over server defaults. Check or unset the
environment when pinning a project. Automatic embedding also needs active `config.embedding`,
a complete cached BGE-M3 snapshot and a semantic-search build. See the
[portable runbook (RU)](CONSTELLATION-PILOT.ru.md); a read-only server is not a writer.

## Safety Hook (Forge Mode)

With this hook installed and enabled in Claude Code, matching literal Bash
command strings can be refused before execution:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/forge-safety-hook.sh",
            "timeout": 3
          }
        ]
      }
    ]
  }
}
```

`forge-safety-hook.sh` checks the blacklist:
- `git push --force` / `git push -f`
- `git reset --hard`
- `rm -rf /` / `rm -rf ~`
- `cargo publish`
- `DROP TABLE`

On a literal blacklist match in a `Bash` call, the script exits 2 and reports
`BLOCKED by forge-safety-hook`. It is not a shell parser or sandbox, does not
inspect other tools and is not a guarantee against equivalent command spellings.
Repository prohibitions apply even when a hook does not detect a command.

### Three trust zones (local teaching analogy, not FPF B.3's algorithm)

| Zone | Controlled by | Mechanism |
|------|---------------|-----------|
| **Green** (safe) | `settings.local.json` allow | Wildcard whitelist: `Bash(cargo:*)` |
| **Yellow** (reversible) | Claude Code acceptEdits | File operations auto-approved |
| **Red** (irreversible) | `forge-safety-hook.sh` | PreToolUse blacklist blocks |

## Methodology Hook (Skill Activation)

Reminds about available methodology commands:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/skill-activation-hook.sh"
          }
        ]
      }
    ]
  }
}
```

Outputs: available skills (/forge, /fpf-simple), methodology rules (Shape -> Validate -> Code -> Evidence -> Activate), Rust skills reminder.

## Best Practices

1. **Start with health** — `UserPromptSubmit` runs once per submitted prompt; use `SessionStart` only when session-start timing is intended
2. **MCP and hooks have different jobs** — MCP exposes tools; these command-hook examples print context or reminders
3. **Keep failures visible** — `2>/dev/null || true` suppresses diagnostics and failure status; if deliberately used for optional context, do not treat its result as a passed gate
4. **Don't over-automate** — the PostToolUse hook should suggest, not force artifact creation
5. **Route before work** — helps the agent decide whether to create artifacts or just code
