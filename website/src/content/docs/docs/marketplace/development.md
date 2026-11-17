---
title: Plugin Development
description: Authoring, registering, validating, and publishing ForgePlan marketplace plugins
---

This guide follows the public marketplace [commit `b4e8a1b`](https://github.com/ForgePlan/marketplace/tree/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7), catalog **1.187.1**. Work in a clone of [ForgePlan/marketplace](https://github.com/ForgePlan/marketplace); paths below are relative to its root. A plugin is not part of the core Forgeplan CLI release.

## Plugin Structure

```text
plugins/my-plugin/
├── .claude-plugin/plugin.json    # name, version, description
├── commands/                     # Optional Claude Code command files
│   └── my-command.md             # YAML name, description
├── agents/                       # Optional Claude Code subagent definitions
│   └── my-agent.md               # YAML name, description, model, color
├── skills/
│   └── my-skill/
│       ├── SKILL.md              # YAML name, description; instructions or router
│       └── sections/             # Optional topic files and indexes
├── .agents/skills/               # Relative interop symlinks, not copied skills
│   └── my-skill -> ../../skills/my-skill
├── hooks/
│   └── hooks.json                # Optional host-native automation
└── README.md
```

Only add the components the plugin needs. Keep prose documentation outside `agents/`: the runtime treats Markdown there as agent definitions. Frontmatter names must match their skill directory or command/agent filename. Quote descriptions containing YAML-sensitive text such as `: `.

A minimal plugin manifest is:

```json
{
  "name": "my-plugin",
  "version": "1.0.0",
  "description": "A focused description of the plugin.",
  "author": { "name": "Your Name" },
  "license": "MIT"
}
```

Claude Code discovers the standard component directories; do not add an invented `components` registration field. ForgePlan-specific fields such as `cost`, `stability`, `targets`, `requires` and `supersedes` belong in the **catalog entry**, not the plugin manifest. Plugin names use lowercase letters, digits, dots and hyphens, with no leading/trailing separator and a maximum of 64 characters; OMP can reject a catalog containing an invalid name.

### Commands, skills and agents

A command file and a chat-invocable skill are different assets. For example, `fpl-skills` publishes 46 skills and **no** `commands/` files, yet Claude Code can expose entrypoints such as `/fpl-skills:shape` from skill frontmatter. `disable-model-invocation: true` makes a skill explicit-only, not inaccessible from chat. See [Commands Reference](/docs/marketplace/commands/) for the 29-file inventory and migration names.

Agent `model: opus|sonnet|haiku` values are Claude Code bindings, not portable model requirements. Follow the [agent-authoring guide](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/fpl-skills/AGENT-AUTHORING-GUIDE.md) for roles, mutation boundaries, descriptions and model tiers. Another runtime needs its own tier-appropriate binding and permission enforcement; copying `disallowedTools` does not establish that boundary there.

Claude Code [loads agent files recursively under `agents/`](https://code.claude.com/docs/en/plugins/components#agents), without explicit manifest registration for that default directory. Subfolder names remain in the scoped name: the published `forgeplan-brownfield-pack/agents/discover/discover.md` is `forgeplan-brownfield-pack:discover:discover`. Include nested files when counting definitions: this public release has **97 agents**, not just its 96 direct `agents/*.md` files.

## Agentic RAG Pattern

For a large knowledge base, **agentic RAG** means the agent follows a small router to relevant files instead of loading everything. It is an authoring pattern, not a runtime-enforced 300-line limit. Small skills may keep their instructions in one `SKILL.md`; many orchestration skills are longer.

### How it works:

1. `SKILL.md` maps tasks to topic directories.
2. `sections/<topic>/_index.md` lists the topic's files and their purpose.
3. The agent reads the relevant content files as needed.

```markdown
| What you need | Start here |
|---|---|
| Choose a decomposition | sections/decomposition/_index.md |
| Compare alternatives | sections/evaluation/_index.md |
```

The `agentic-rag` plugin recommends 30–50 lines per content file (70 maximum), 15–30 per index (40 maximum), and 60–100 for a router (150 maximum). These are that pattern's authoring budgets, **not a universal size or loading guarantee for every marketplace skill**. See its [file-budget guidance](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agentic-rag/skills/agentic-rag/sections/structure/file-budget.md) and the [Forgeplan Workflow plugin](/docs/marketplace/forgeplan-workflow/) for a different, instruction-oriented skill.

## Standalone Skills (npx)

A standalone installation distributes **skills**, not the plugin's commands, agents or hooks. Use a real published source, for example:

```bash
npx skills add ForgePlan/fpf -g
npx skills add ForgePlan/loux -g
```

Those repositories provide `fpf-knowledge` and `ux-laws`, respectively. There is no general rule that a skill named `X` has a repository `ForgePlan/X`. The authoritative mirror list is [`scripts/standalone-mirrors.json`](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/scripts/standalone-mirrors.json); not every plugin or skill has a standalone mirror. A new mirror also needs the repository's sync workflow and mirror checks updated, rather than a made-up install URL.

## Publishing to Marketplace

Run the following from the marketplace clone, not a consuming project:

1. Add the plugin under `plugins/my-plugin/`, including its manifest, components and README.
2. Register it in `.claude-plugin/marketplace.json` → `plugins[]` with `"source": "./plugins/my-plugin"`, matching name, description and version. Declare installation requirements and any replacement metadata there. Update a named profile only when the plugin belongs in that set.
3. For an update, bump the plugin version in **both** manifests. Bump the catalog's `metadata.version` as well: it is the marketplace update cache key.
4. Regenerate the OMP catalog and skill discovery aliases:

   ```bash
   node scripts/gen-omp-catalog.js
   node scripts/gen-interop-skills.js
   ```

   Commit the resulting `.omp-plugin/marketplace.json` and relative `.agents/skills/` symlinks with the source change. The OMP catalog has a lowercase marketplace name; it is not a separately maintained registry. Resolve any real directory blocking an alias manually; the alias generator leaves it untouched.

5. Validate using the interfaces below, then submit a PR following [CONTRIBUTING.md](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/CONTRIBUTING.md). A local change or PR is not a published installation; the merged catalog and plugin files must reach the public marketplace.

### Validation commands and scope

```bash
# A plugin basename, NOT plugins/my-plugin or a manifest path
./scripts/validate-all-plugins.sh my-plugin

# Also make legacy agent-pattern warnings fatal
./scripts/validate-all-plugins.sh my-plugin --strict-agents

# All plugins
./scripts/validate-all-plugins.sh

# Claude Code's own validator takes a plugin DIRECTORY
claude plugin validate plugins/my-plugin/ --strict

# The catalog validator takes the catalog FILE, without --strict
claude plugin validate .claude-plugin/marketplace.json
```

The repository validator accepts `[plugin-name] [--strict-agents]`. The plugin argument narrows structural and agent-pattern checks; it **does not isolate the entire run**. Command-collision checks, repository-wide CI gates, the official runtime-validation wrapper and every `plugins/*/tests/test-*.sh` suite still run. This is not a cheap single-plugin lint command.

The script needs Bash, Python 3 with PyYAML for agent linting, Node.js for CI gates, and any prerequisites of the plugin suites. The official validation wrapper reports **SKIPPED**, not passed, if `claude` is missing. Plugin manifests are validated strictly; the catalog is deliberately non-strict because it carries ForgePlan metadata Claude Code does not recognize. Validation asserts the current files; it does not regenerate stale catalogs or aliases. Inspect its actual output rather than treating the existence of a file as proof it loads.

## Runtime-Specific Hooks and Interop

Use Claude Code's `hooks/hooks.json` schema for Claude automation, with `type: "command"` and paths rooted at `${CLAUDE_PLUGIN_ROOT}`. Repository policy bans `type: "prompt"` hooks. Events are plugin-specific: the public surface includes `PreToolUse`, `PostToolUse`, `SessionStart`, `UserPromptSubmit`, `Stop` and `SessionEnd`; do not assume each plugin implements each event.

Portable skill bodies do not translate host APIs. The producer-side `plugins/<plugin>/.agents/skills/` aliases make one canonical skill body discoverable; they neither register Claude commands nor port subagents, tool permissions, hooks or MCP credentials. In a consuming project, the `cross-runtime` skill's separate `scripts/cross-runtime.mjs` reports discovery gaps and can create project links with `--fix`; it does not replace producer registration or set up every host automatically.

Most hooks remain Claude-native. `slopocop` **2.0.0** is an explicit exception with Claude hooks, trusted Codex hooks, an OMP extension and a host-neutral startup-instruction installer. Its editorial commands and reviewer agent still use Claude-native formats. See the published [cross-CLI contract](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/docs/CROSS-CLI.md) for host-specific limits.

For memory integration, `fpl-hsmem` **3.7.0** requires project opt-in through `.hindsight.json` with explicit `url`, `banks` and `defaultBank`; the selected banks must already exist on a compatible Hindsight API server (`>=0.10.2`, `<0.11.0`). Hooks default off. Environment variables alone are not a project configuration, and BankGate is client safety, not server authorization. See [fpl-hsmem](/docs/marketplace/fpl-hsmem/) rather than reusing older env-only setup recipes.

## Example Plugins

Use [forgeplan-workflow](/docs/marketplace/forgeplan-workflow/) for commands, a skill, an agent and a hook; [fpl-skills](/docs/marketplace/fpl-skills/) for skills-only entrypoints; and [slopocop](/docs/marketplace/slopocop/) for explicitly supported host adapters. `dev-toolkit` remains published but deprecated; it is not the default template for a new plugin.

## Contribution Guidelines

See [CONTRIBUTING.md](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/CONTRIBUTING.md) and the source scripts at the pinned public revision.

### PR Requirements:

- Valid plugin manifest, component frontmatter and README with real installation instructions.
- Matching plugin/catalog versions and a bumped catalog `metadata.version` for updates.
- Regenerated OMP catalog and interop aliases when their sources change.
- Validation output that distinguishes pass, failure and skipped runtime checks.
- No secrets, personal paths or credentials; follow each host's permission and configuration model.
