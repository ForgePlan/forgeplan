---
title: forgeplan setup-skill
description: "Install the /forge skill into Claude Code so AI agents can drive the Forgeplan workflow"
---

`forgeplan setup-skill` writes the embedded `/forge` instructions to `~/.claude/skills/forge/SKILL.md` for the current OS user. A compatible Claude Code client can load those instructions across projects; writing the file alone does not register an MCP server, initialize a workspace, or prove that an agent follows every rule.

The skill file is embedded directly into the `forgeplan` binary at compile time - the command performs **no network access**, does **not** fetch anything from the marketplace, and does **not** modify the current project's `.forgeplan/` or `.claude/` directories.

## When to use

- Install instructions for a local Claude Code user; other clients must explicitly support this skill location.
- The file is user-global, not shared with every machine, account or remote runtime.
- After upgrading the Forgeplan binary if the bundled skill definition changed (re-run to pick up updates).
- When onboarding a teammate who already has Claude Code but has never run Forgeplan locally.

## When NOT to use

- If you don't use Claude Code - the skill is Claude-Code-specific and has no effect on plain CLI usage.
- If you've customized `~/.claude/skills/forge/SKILL.md` manually and don't want to overwrite it (the command re-installs the bundled version).
- As a substitute for `forgeplan init` - this only installs the skill file, it does not create a workspace.
- If you want marketplace plugins (`/audit`, `/sprint`, `/fpf`, etc.) - those are installed separately via `/plugin install <name>@ForgePlan-marketplace` (after `/plugin marketplace add ForgePlan/marketplace`). See the [Marketplace Overview](/docs/marketplace/overview/).

## Usage

```text
forgeplan setup-skill
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Example 1: New project onboarding

```bash
forgeplan init -y
forgeplan setup-skill
```

Creates a workspace, then writes the skill file. Confirm discovery in the actual client; availability in its next chat is a client behavior, not an installer handshake.

### Example 2: Refresh the skill after upgrading

```bash
cargo install forgeplan  # or brew upgrade forgeplan
forgeplan setup-skill
```

Reinstalls the definition embedded in the invoked binary. It overwrites the existing file; back up any personal edits before repeating the command.

### Example 3: Verify installation

```bash
forgeplan setup-skill
ls ~/.claude/skills/forge/
# -> SKILL.md
```

Confirms the single skill file landed in the user-global Claude Code skills directory.

## How it fits the workflow

This is a one-shot setup command, not part of the artifact lifecycle. It belongs in the initial project bootstrap alongside:

1. `forgeplan init -y` - create workspace
2. `forgeplan setup-skill` - install Claude Code integration
3. For LLM-powered commands, configure a provider: HTTP uses an API-key environment variable; explicitly opted-in personal local CLI profiles use their existing login.
4. `forgeplan health` - sanity check
5. Open Claude Code and call `/forge` - methodology-aware workflow kicks in

The skill supplies workflow instructions from [the methodology guide](/docs/methodology/overview/). Instructions in context are not runtime proof of enforcement or completed gates.

## Safety notes

- The command writes to `~/.claude/skills/forge/SKILL.md` in your **home directory** (user-global). If you have local edits to that file, back them up first.
- Because it is user-global (not project-local), there is nothing to gitignore - the file lives outside any repository.
- No workspace data is touched; this command does not modify `.forgeplan/`.

## See also

- [`forgeplan init`](/docs/cli/init/) - create the workspace before installing the skill
- [`forgeplan health`](/docs/cli/health/) - verify session readiness
- [Methodology overview](/docs/methodology/overview/) - workflow instructions and gates
- [Configuration](/docs/getting-started/configuration/) - LLM provider setup required for `/forge` to reason
- [Marketplace Overview](/docs/marketplace/overview/) - full plugin catalog for additional skills
- [Forgeplan Workflow Plugin](/docs/marketplace/forgeplan-workflow/) - details on what `/forge` provides

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan setup-skill --help`.

```text
Install /forge skill for Claude Code

Usage: forgeplan setup-skill

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
