---
title: Installation
description: Install Forgeplan — CLI, AI Skill, or MCP Server
---

## AI Skill (recommended for AI agents)

Install the `/forge` skill for Claude Code, Cursor, Codex, Gemini and 40+ AI agents:

```bash
npx skills add ForgePlan/marketplace --skill forge
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
