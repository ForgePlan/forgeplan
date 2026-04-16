---
title: forgeplan setup-skill
description: "Install the /forge skill into Claude Code so AI agents can drive the Forgeplan workflow"
---

`forgeplan setup-skill` installs the `/forge` Claude Code skill **user-globally** by writing a single markdown file to `~/.claude/skills/forge/SKILL.md`. Once installed, Claude Code can invoke the full Forgeplan workflow (route → shape → validate → ADI → code → evidence → activate) through a single `/forge` slash command in **any** project, with all methodology rules loaded as context via the bundled skill definition.

The skill file is embedded directly into the `forgeplan` binary at compile time — the command performs **no network access**, does **not** fetch anything from the marketplace, and does **not** modify the current project's `.forgeplan/` or `.claude/` directories.

## When to use

- First time you connect Claude Code (or another Claude Agent SDK client) to Forgeplan.
- Once per machine — the skill is user-global and works across **all** projects afterwards.
- After upgrading the Forgeplan binary if the bundled skill definition changed (re-run to pick up updates).
- When onboarding a teammate who already has Claude Code but has never run Forgeplan locally.

## When NOT to use

- If you don't use Claude Code — the skill is Claude-Code-specific and has no effect on plain CLI usage.
- If you've customized `~/.claude/skills/forge/SKILL.md` manually and don't want to overwrite it (the command re-installs the bundled version).
- As a substitute for `forgeplan init` — this only installs the skill file, it does not create a workspace.
- If you want marketplace plugins (`/audit`, `/sprint`, `/fpf`, etc.) — those are installed separately via `npx skills add ForgePlan/marketplace --plugin <name>`. See the [Marketplace Overview](/docs/marketplace/overview/).

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

Typical bootstrap sequence: create the workspace, then install the Claude Code skill so `/forge` is immediately available in the next chat session.

### Example 2: Refresh the skill after upgrading

```bash
cargo install forgeplan  # or brew upgrade forgeplan
forgeplan setup-skill
```

Re-runs the installer so the skill markdown matches the new binary version. Safe to run repeatedly — it overwrites `~/.claude/skills/forge/SKILL.md`.

### Example 3: Verify installation

```bash
forgeplan setup-skill
ls ~/.claude/skills/forge/
# -> SKILL.md
```

Confirms the single skill file landed in the user-global Claude Code skills directory.

## How it fits the workflow

This is a one-shot setup command, not part of the artifact lifecycle. It belongs in the initial project bootstrap alongside:

1. `forgeplan init -y` — create workspace
2. `forgeplan setup-skill` — install Claude Code integration
3. Configure `.forgeplan/config.yaml` — LLM provider + API key
4. `forgeplan health` — sanity check
5. Open Claude Code and call `/forge` — methodology-aware workflow kicks in

Inside Claude Code, `/forge` gives AI agents the same routing, validation, and evidence discipline described in [the methodology guide](/docs/methodology/overview/).

## Safety notes

- The command writes to `~/.claude/skills/forge/SKILL.md` in your **home directory** (user-global). If you have local edits to that file, back them up first.
- Because it is user-global (not project-local), there is nothing to gitignore — the file lives outside any repository.
- No workspace data is touched; this command does not modify `.forgeplan/`.

## See also

- [`forgeplan init`](/docs/cli/init/) — create the workspace before installing the skill
- [`forgeplan health`](/docs/cli/health/) — verify session readiness
- [Methodology overview](/docs/methodology/overview/) — what the `/forge` skill actually enforces
- [Configuration](/docs/getting-started/configuration/) — LLM provider setup required for `/forge` to reason
- [Marketplace Overview](/docs/marketplace/overview/) — full plugin catalog for additional skills
- [Forgeplan Workflow Plugin](/docs/marketplace/forgeplan-workflow/) — details on what `/forge` provides
