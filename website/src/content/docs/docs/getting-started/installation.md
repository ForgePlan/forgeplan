---
title: Installation
description: Install Forgeplan — CLI, AI Skill, or MCP Server
---

## AI Skill (recommended for AI agents)

Install the `/forge` skill for Claude Code, Cursor, Codex, Gemini and 40+ AI agents:

```bash
forgeplan setup-skill   # writes ~/.claude/skills/forge/SKILL.md, no network
```

After installation, use in chat:
```
/forge "Add OAuth2 authentication"
```

**Alternative**: if you already have the CLI installed, use the built-in command instead -- it embeds the skill file directly, no network required:

```bash
forgeplan setup-skill
```

See [`forgeplan setup-skill`](/docs/cli/setup-skill/) for details.

**Discover more plugins**: [Marketplace Overview](/docs/marketplace/overview/).

## Full Harness Bundle (Claude Code)

The `/forge` skill above is enough to start. For the **complete Forgeplan harness** — `/smith` master orchestrator, `/forge-cycle` reactive enforcer, `/audit`, `/sprint`, `/methodology-check`, `/forgeplan-cookbook`, guardian + Profile A/B/C/D canonical agents, FPF ADI reasoning, Hindsight cross-session memory — install the recommended plugin set. This is what `/smith-bootstrap` Step 0a expects on a greenfield project.

Run these inside a Claude Code session:

```bash
# One-time: add the marketplace
/plugin marketplace add ForgePlan/marketplace

# 5 MUST plugins — full pipeline depends on all five
/plugin install fpl-skills@ForgePlan-marketplace          # 34 skills: smith, forge-cycle, forgeplan-cookbook, audit, sprint, methodology-check, ...
/plugin install agents-pro@ForgePlan-marketplace          # 28 agents: smith, guardian, brief-intake, adr-architect, research-analyst, ...
/plugin install agents-sparc@ForgePlan-marketplace        # 5 SPARC phase agents — first PRD is dispatched to specification
/plugin install agents-core@ForgePlan-marketplace         # 11 baseline agents: coder, code-reviewer, tester
/plugin install forgeplan-workflow@ForgePlan-marketplace  # /forge-cycle + /forge-audit + guardian gate enforcement

# 2 SHOULD plugins — strongly recommended
/plugin install fpf@ForgePlan-marketplace                 # FPF ADI reasoning — mandatory for Standard+ artifacts
/plugin install fpl-hsmem@ForgePlan-marketplace           # Hindsight cross-session memory (per-project bank)

# Reload to activate
/reload-plugins
```

After reload, `/smith-bootstrap` for a fresh repo or `/smith` for next-action recommendations are ready to use.

### Why all five MUST plugins are needed

| Plugin | Provides | Without it |
|---|---|---|
| `fpl-skills` | `/smith`, `/forge-cycle`, `/audit`, `/sprint`, `/forgeplan-cookbook`, 34 skills total | No orchestrator, no methodology routing |
| `agents-pro` | smith agent body, guardian, brief-intake, adr-architect, research-analyst (28 agents) | No Profile A creators, no guardian gate |
| `agents-sparc` | specification, architecture, pseudocode, refinement, sparc-orchestrator | First PRD silently falls back to generic Profile A, misses SPARC contract |
| `agents-core` | coder, code-reviewer, tester (11 agents) | No Profile C-coder for actual code work, no canonical reviewers |
| `forgeplan-workflow` | `/forge-cycle` (reactive enforcer), `/forge-audit`, guardian gate enforcement | No 4-layer pipeline driver, no `/forge` command, no audit |

### Optional plugins

| Plugin | When to add |
|---|---|
| `laws-of-ux` | Frontend / UX code review with 30 Laws of UX |
| `agents-domain` | Domain-specific agents: TypeScript, Go, Python, Next.js, React, Rust, etc. |
| `agents-github` | GitHub workflow agents: PR, issues, releases, projects, workflows |
| `forgeplan-brownfield-pack` | Onboarding existing codebases via the 7-phase Discover protocol |
| `forgeplan-orchestra` | Sync with Orchestra task management |

See [Marketplace Overview](/docs/marketplace/overview/) for the full plugin catalog.

## CLI Binary

### macOS (Homebrew)

```bash
brew install forgeplan/tap/forgeplan
```

### From source (Rust)

```bash
cargo install forgeplan
```

### GitHub Releases

Download pre-built binaries from [GitHub Releases](https://github.com/ForgePlan/forgeplan/releases).

## MCP Server (for AI agents)

Add to your project's `.mcp.json`:

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve"],
      "env": {}
    }
  }
}
```

## Initialize Workspace

```bash
forgeplan init -y
```

This creates `.forgeplan/` directory with config and LanceDB storage.

## Verify Installation

```bash
forgeplan --version
forgeplan health
```

:::note
AI agents should always use `forgeplan init -y` (non-interactive mode).
:::
