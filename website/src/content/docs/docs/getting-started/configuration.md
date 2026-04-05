---
title: Configuration
description: Configure Forgeplan workspace and LLM providers
---

## Workspace Structure

After `forgeplan init -y`, the `.forgeplan/` directory is created:

```
.forgeplan/
├── config.yaml    ← workspace config
├── lance/         ← LanceDB storage (gitignore)
├── prds/          ← markdown projections (git-tracked)
├── rfcs/
├── adrs/
├── epics/
├── specs/
├── problems/
├── solutions/
├── evidence/
├── notes/
└── refresh/
```

## Config File

`.forgeplan/config.yaml`:

```yaml
# LLM provider for generate, reason, route (Level 1+)
llm:
  provider: gemini          # gemini | openai | anthropic
  model: gemini-2.0-flash   # model name
  api_key_env: GEMINI_API_KEY  # env var with API key

# Embedding model for semantic search
embed:
  model: BGE-M3             # default, built-in
  enabled: true

# Project metadata
project:
  name: my-project
  default_depth: standard
```

## LLM Providers

| Provider | Env Variable | Models |
|----------|-------------|--------|
| Gemini | `GEMINI_API_KEY` | gemini-2.0-flash, gemini-1.5-pro |
| OpenAI | `OPENAI_API_KEY` | gpt-4o, gpt-4o-mini |
| Anthropic | `ANTHROPIC_API_KEY` | claude-sonnet-4-20250514 |

## Important Notes

:::caution
- `.forgeplan/` is in `.gitignore` — workspace data is NOT tracked
- Config is lost on `forgeplan init -y` reinit
- Before reinit: `forgeplan export --output backup.json`
- After reinit: `forgeplan import backup.json`
:::

:::note
AI agents should always use `forgeplan init -y` (non-interactive).
:::
