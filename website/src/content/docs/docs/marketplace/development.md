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

Skills can use **agentic RAG** -- a `SKILL.md` router directs the host to
relevant files rather than loading the entire corpus. The [Forgeplan Workflow
plugin](/docs/marketplace/forgeplan-workflow/) illustrates this pattern.
About 300 lines per read is a writing convention, not a loader-enforced cap.

### How it works:

1. **SKILL.md** = router - maps user needs to sections via table
2. **sections/_index.md** = section index - lists files with descriptions
3. **sections/topic.md** = content - ~30-50 lines each

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
# In a separate, reviewed marketplace checkout, place plugins/my-plugin/.
# Register it in .claude-plugin/marketplace.json and bump catalog metadata.version.
node scripts/gen-omp-catalog.js
./scripts/validate-all-plugins.sh my-plugin

# 4. Create PR
git add -A && git commit -m "feat: add my-plugin v1.0.0"
gh pr create --base main
```

This is an illustrative contribution sequence, not a command to run in a
Forgeplan project. Review and stage only intended files; `git add -A`
would also include unrelated changes. Follow the marketplace's current
CONTRIBUTING instructions and required checks before committing or opening a PR.

## Example Plugins

Browse existing plugins in the [ForgePlan/marketplace](https://github.com/ForgePlan/marketplace) repository for reference implementations. The `forgeplan-workflow` and `dev-toolkit` plugins demonstrate the full structure including commands, agents, skills, and hooks.

## Contribution Guidelines

See [CONTRIBUTING.md](https://github.com/ForgePlan/marketplace/blob/main/CONTRIBUTING.md) for full details.

### PR Requirements:
- Plugin structure validated
- Version bumped in plugin.json + marketplace.json
- Catalog `metadata.version` bumped and `.omp-plugin/marketplace.json` regenerated when the catalog changes
- README with install commands
- No secrets or credentials
