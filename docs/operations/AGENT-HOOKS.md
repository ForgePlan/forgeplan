[English](AGENT-HOOKS.md) · [Русский](AGENT-HOOKS.ru.md)

# Agent Hooks — Auto-integrate Forgeplan with AI Agents

Hooks belong to the agent host or a marketplace plugin, not to the Forgeplan MCP protocol. This guide describes Forgeplan **0.38.0** and the public marketplace snapshot **1.187.1** (`fpl-skills` 1.75.1). Installing the CLI or starting `forgeplan serve` does not install these host hooks.

## SessionStart Hook (Claude Code)

For a project-local, advisory context hook, merge this into `.claude/settings.json`:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "forgeplan health --compact",
            "timeout": 5
          }
        ]
      }
    ]
  }
}
```

Prerequisites: `forgeplan` on the host's `PATH`, an initialized workspace, and the hook running in that workspace. `SessionStart` runs on session start; `UserPromptSubmit` runs on each submitted prompt. Do not label one as the other.

This is a custom example, not the marketplace's installed hook. For advisory hooks only, a project may deliberately append `|| true` to avoid blocking startup; this also hides command failures, so do not use it for a required gate.

### What the agent sees

`forgeplan health --compact` produces a short text summary plus `Next: ...` or `Done.`. Use `forgeplan health --json` for structured data. Combining `--compact --json` selects JSON, not a special compact JSON schema or a guaranteed token budget.

The JSON report includes arrays such as `blind_spots`, `orphans`, `at_risk`, counts such as `stale_count`, a `verdict`, and `_next_action`. These are workspace findings, not proof that implementation or tests passed.

## PostToolUse Hook

A local reminder can suggest capturing a decision without creating an artifact:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "echo '[Forgeplan] Does this change need a decision record? Review before creating an artifact.'"
          }
        ]
      }
    ]
  }
}
```

`capture` uses a configured LLM provider and can create an artifact. Do not invoke it automatically for every edit or treat a reminder as authorization to write.

## Route-Before-Work Hook

The public `fpl-skills` plugin registers `hooks/scripts/prompt-router.sh` on `UserPromptSubmit`. It matches keywords and emits a non-blocking recommendation. It **never invokes a skill**, calls MCP, or runs an LLM. It reads `CLAUDE_USER_PROMPT`; if the host does not supply that variable, it exits silently.

For explicit offline routing, use `forgeplan route "task description" --level 0`. `--level 1` requests LLM classification; without an explicit level the implementation can try the configured LLM and fall back to keywords. The CLI has no `--json` flag in 0.38.0. A `UserPromptSubmit` matcher is not a portable regex filter over prompt content; do not rely on `$PROMPT` or a matcher such as `implement|build` to supply the task.

## MCP Server (Recommended)

Run the server in the intended project directory:

```bash
forgeplan serve
```

Forgeplan 0.38.0 exposes **73 tools** over stdio. MCP gives structured access to supported operations, not enforcement of every project rule. For Claude Code, merge the server entry into project `.mcp.json` and approve it in the host:

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

The server's launch directory sets the default workspace. Tools exposing a `workspace` argument can target another worktree explicitly; do not assume changing a worker's shell directory redirects the server. Some tools do not expose that override.

## Safety Hook (Forge Mode)

The public plugins register their own scripts:

| Plugin | Event / matcher | Script and effect |
|---|---|---|
| `fpl-skills` | `PreToolUse` / `Bash` | `hooks/scripts/safety-hook.sh`: checks selected command patterns, exits 2 on a match |
| `forgeplan-workflow` | `PreToolUse` / `Bash` | `hooks/scripts/forge-safety-hook.sh`: skips its fallback if it finds the expected dev-toolkit hook path |
| `forgeplan-workflow` | `PreToolUse` / `Write` | `hooks/scripts/pre-code-check.sh`: cached active-PRD warning, always exits 0 |

The safety scripts contain checks for force-push, hard reset, forced clean, selected recursive deletions and SQL `DROP` in SQL-client contexts. **They do not check `cargo publish` or every dangerous command.** Their parser reads a top-level `.command` from stdin; a host sending only `.tool_input.command` will leave that value empty and the script allows the call. Missing both `jq` and `python3` blocks; empty/unparsed commands otherwise allow. Validate the actual host payload before relying on this layer.

The pre-code warning is not an artifact-coverage check or a gate requiring an active PRD before coding. It queries `forgeplan list --status active --type prd` and caches the check for five minutes in `/tmp/.forgeplan-prd-cache`, shared across projects. It does not inspect which decision covers the file being written.

### Three trust zones (FPF B.3)

Permission zones can be a useful **project policy**, but plugin pattern checks are not a sandbox. Broad allow rules such as `Bash(cargo:*)` include write and publish operations. The host's permissions, tool grants and user approvals remain independent controls; an `acceptEdits` setting does not prove a safety hook received or understood a command.

## Methodology Hook (Skill Activation)

The public `fpl-skills` hooks also register:

- `SessionStart`: session greeting, report initialization, decay reminder and insight watchdog. The session script's health detail is opt-in via `FPL_SHOW_HEALTH=1`.
- `PostToolUse` / `Write|Edit|MultiEdit`: `test-hint.sh`, an advisory hint based on added public declarations in `git diff HEAD`. It does not run tests.
- `PostToolUse` / `.*`: report counter.
- `UserPromptSubmit`: the keyword router described above.

There is no registered `skill-activation-hook.sh` or automatic `pre-commit-fmt` gate in these two plugins' manifests. Repository-local hooks and templates are separate from public plugin registration. Other hosts need their own adapters; copying Claude Code event JSON is not a cross-host installation procedure.

## Best Practices

1. Use reminders for context and explicit failure exits for required gates; never suppress gate failures with `|| true`.
2. Inspect installed `hooks.json`, script input shape and host event support together.
3. Keep generation and verification separate: a skill hint, successful tool call or report counter is not test evidence.
4. Give parallel workers separate worktrees and exact owned files. Claims are advisory, not file locks; see [Multi-Agent Workflow](MULTI-AGENT.md).
5. Keep task tracker operations separate from artifact lifecycle operations. Neither these hooks nor Orchestra automatically activates a Forgeplan artifact.

## Release sources

- [Core health output](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/commands/health.rs#L113-L170).
- [Public fpl-skills hooks](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/fpl-skills/hooks/hooks.json), [safety input](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/fpl-skills/hooks/scripts/safety-hook.sh#L7-L55), [prompt router](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/fpl-skills/hooks/scripts/prompt-router.sh#L1-L25).
- [Public workflow hooks](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-workflow/hooks/hooks.json), [pre-code warning](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-workflow/hooks/scripts/pre-code-check.sh#L9-L38).
