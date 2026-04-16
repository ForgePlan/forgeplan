---
title: Plugin Development
description: How to create plugins and skills for Claude Code marketplace
---

## Plugin Structure

```
plugin-name/
├── .claude-plugin/plugin.json    # Required: name, version, description
├── commands/                     # Slash commands (/command-name)
│   └── command.md               # Frontmatter: name, description
├── agents/                       # Specialized sub-agents
│   └── agent.md                 # Frontmatter: name, description, model
├── skills/                       # Knowledge bases
│   └── skill-name/
│       ├── SKILL.md             # Router with navigation table
│       └── sections/            # Content files (agentic RAG)
├── hooks/                        # Automation triggers
│   └── hooks.json               # PostToolUse, PreToolUse events
└── README.md
```

## Agentic RAG Pattern

Skills use **agentic RAG** -- intelligent retrieval that loads only ~300 lines at a time, not the entire knowledge base. For a real-world example of this pattern in action, see the [Forgeplan Workflow plugin](/docs/marketplace/forgeplan-workflow/) which uses a `SKILL.md` router to serve methodology sections on demand.

### How it works:

1. **SKILL.md** = router — maps user needs to sections via table
2. **sections/_index.md** = section index — lists files with descriptions
3. **sections/topic.md** = content — ~30-50 lines each

```markdown
<!-- SKILL.md -->
| What you need | Start here |
|---|---|
| Decompose a system | sections/decomposition/ |
| Evaluate options | sections/evaluation/ |
```

Claude reads SKILL.md → picks the right section → reads _index.md → loads specific file. Context stays focused.

## Standalone Skills (npx)

For distribution via `npx skills add`:

```
skill-name/
├── SKILL.md              # Router
├── sections/
│   ├── 01-intro/_index.md
│   ├── 01-intro/overview.md
│   └── 02-usage/_index.md
└── README.md
```

Install: `npx skills add ForgePlan/skill-name -g`

## Publishing to Marketplace

```bash
# 1. Copy plugin to marketplace
cp -R my-plugin forgeplan-marketplace/plugins/

# 2. Add to marketplace.json catalog
# Edit .claude-plugin/marketplace.json → plugins[]

# 3. Validate
./scripts/validate-all-plugins.sh my-plugin

# 4. Create PR
git add -A && git commit -m "feat: add my-plugin v1.0.0"
gh pr create --base main
```

## Example Plugins

Browse existing plugins in the [ForgePlan/marketplace](https://github.com/ForgePlan/marketplace) repository for reference implementations. The `forgeplan-workflow` and `dev-toolkit` plugins demonstrate the full structure including commands, agents, skills, and hooks.

## Contribution Guidelines

See [CONTRIBUTING.md](https://github.com/ForgePlan/marketplace/blob/main/CONTRIBUTING.md) for full details.

### PR Requirements:
- Plugin structure validated
- Version bumped in plugin.json + marketplace.json
- README with install commands
- No secrets or credentials
