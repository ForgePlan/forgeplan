---
title: Installation
description: Install Forgeplan - CLI, AI Skill, or MCP Server
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

### What each plugin gives you

| Plugin | What you can do with it |
|---|---|
| `fpl-skills` | Type `/smith` and it figures out what kind of task you have and which methodology fits. Drives the day-to-day commands: `/forge-cycle` to walk a task to completion, `/audit` for multi-expert code review, `/sprint` for wave-based execution, `/forgeplan-cookbook` to look up the right forgeplan tool. The skills brain of the system. |
| `agents-pro` | Dispatch named specialists when you need them: `brief-intake` turns a vague idea into a structured Brief, `adr-architect` produces architecture decisions with three considered alternatives, `research-analyst` gathers prior art before you commit to a direction, `guardian` runs the last check before activation. |
| `agents-sparc` | Use the SPARC five-phase flow for any new feature: Specification → Pseudocode → Architecture → Refinement → Completion. Without it, the first PRD on a fresh project lands without the SPARC structure and you have to redo the spec phase by hand. |
| `agents-core` | Actually write, review, and test code. The `coder` agent edits files in an isolated worktree, `code-reviewer` produces structured findings against the spec, `tester` runs the suite and reports coverage delta. |
| `forgeplan-workflow` | Run the `/forge-cycle` command — the reactive enforcer that walks an artifact through validate → ADI → review → activate one step at a time. Plus `/forge-audit` for multi-expert code audit and the guardian gate enforcement that decides whether a PRD is ready to ship. |

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

:::note[Homebrew 6.0+ requires trusting the tap]
Homebrew 6.0 made third-party taps **untrusted by default**. If you see
`Error: Refusing to load formula forgeplan/tap/forgeplan from untrusted tap`,
trust the tap once, then re-run the install:

```bash
brew trust forgeplan/tap
brew install forgeplan/tap/forgeplan
```

This is a one-time, per-machine confirmation that you trust the ForgePlan tap.
On Homebrew < 6.0 the plain `brew install` above works without this step.
:::

### From source (Rust)

```bash
cargo install forgeplan
```

### GitHub Releases

Download pre-built binaries from [GitHub Releases](https://github.com/ForgePlan/forgeplan/releases).

### First run: fetch the embedding model

Semantic search is compiled into every released binary — Homebrew, the install
script and the GitHub Release archives alike. The engine is [`tract`](https://github.com/sonos/tract),
pure Rust, so there is no platform where the feature exists in the source but
not in the build.

What the binary does *not* carry is the model itself. Run once per machine:

```bash
forgeplan setup
```

It does two things:

- downloads the **embedding model** up front, so your first semantic search
  does not stall for minutes with no explanation
- creates the **`fpl` alias** when you installed via `cargo install` — brew and
  `install.sh` get it from cargo-dist's `bin-aliases`, but cargo has no
  post-install hook, so a source install has `forgeplan` and no `fpl`

Both steps are idempotent; `--skip-model` and `--skip-alias` opt out of either.
An existing `fpl` on your PATH is never overwritten.

Until the model is present, `forgeplan search --semantic` falls back to BM25
keyword search and says so; everything else — routing, artifacts, validation,
scoring, the graph — is unaffected.

`forgeplan init` also offers the download when run interactively. It never
downloads under `-y` — agents and CI runners must not pull gigabytes by
accident — so pass `--with-model` when a scripted install genuinely wants it.

The model is **~2.1 GB**, downloaded once per machine with a progress bar, and
cached outside your projects, in the platform cache directory:

| Platform | Cache location |
|---|---|
| macOS | `~/Library/Caches/forgeplan/models` |
| Linux | `~/.cache/forgeplan/models` |
| Windows | `%LOCALAPPDATA%\forgeplan\models` |

Override with `FORGEPLAN_MODEL_CACHE`. If you already have `HF_HOME` set, it
takes precedence over both — that is fastembed's behaviour and we do not
override it, so a shared HuggingFace cache stays authoritative.

To check which kind of build you have, run `forgeplan embed`: a build without
the feature refuses immediately and prints the install command above, while a
build with it starts loading the model. `forgeplan --version` does not report
features.

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
