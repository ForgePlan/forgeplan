---
title: Installation
description: Install Forgeplan - CLI, AI Skill, or MCP Server
---

## Read this first: three things installing the binary does not do

The binary, model files, and workspace vectors are separate prerequisites.
Install only the capabilities your workflow needs; none of the steps below
is evidence that semantic coverage or the full methodology pipeline is ready.

| Capability | Preparation | Without it |
|---|---|---|
| Semantic search | `semantic-search` build, `forgeplan setup`, then index/embed | No complete semantic answer; forced semantic search on a migrated workspace does not promise keyword fallback |
| FPF knowledge base | Install `fpf`, then `forgeplan fpf ingest` | FPF corpus is unavailable to its search/reasoning paths |
| Agent harness | Install the relevant client plugins | Plugin commands and specialist agents are unavailable |

The Constellation instructions here describe local
`0.39.0+constellation.s1.1`, not the published 0.38.0. They do not assert
S1-G/S2 readiness. See [Constellation](/docs/guides/constellation/).

### 1. Fetch the embedding model

```bash
forgeplan setup
```

The model is not bundled with the binary. The current engine is tract and
supports only BGE-M3 (roughly 2.1 GB). A source install needs the feature
explicitly; do not infer enabled features from the version string.

`forgeplan setup` deliberately prepares the model. `init -y` does not download
it; a new scripted workspace can opt in with `init -y --with-model`.
An existing-workspace early return can bypass that download.

`forgeplan embed` computes workspace vectors; fetching weights alone does
not populate the index. Queries and background maintenance do not fetch
missing snapshots. Check coverage instead of assuming a semantic result.

Details, cache locations and overrides: [First run](#first-run-fetch-the-embedding-model).

### 2. Install FPF and ingest it

```bash
/plugin install fpf@ForgePlan-marketplace   # inside Claude Code
forgeplan fpf ingest                        # then, in your shell
forgeplan fpf search "trust calculus"       # verify — should return B.3
```

The First Principles Framework corpus depends on the installed edition and ships as a
**separate skill**, not inside the binary. `fpf ingest` parses it, embeds it,
and writes it into your workspace knowledge base.

**Why it matters.** `forgeplan reason` uses a configured LLM for ADI.
`--fpf` optionally adds retrieved FPF context; the corpus is not a prerequisite
for running ADI itself. The methodology recommends ADI at Standard depth and
requires it for Deep/Critical; a project's stricter workflow still applies.
On migrated Constellation workspaces this CLI path currently proceeds without
FPF context rather than opening the retired FPF table.

**Why this step is easy to get wrong.** Skipping it produces a closed loop
that looks like a working system: `fpf search` reports "no matches, run
ingest", and before v0.35.0 `ingest` pointed at a skill name that no longer
existed. An empty corpus and a missing corpus give the identical answer. If
`fpf search` returns nothing, run `forgeplan fpf status` — it distinguishes
the two.

Details: [`forgeplan fpf ingest`](/docs/cli/fpf-ingest/).

### 3. Install the agent harness

Everything above works from a plain shell. But Forgeplan is designed to be
driven by an agent, and the commands that do the driving — `/smith`,
`/forge-cycle`, `/audit`, `/sprint` — live in marketplace plugins, not in the
binary.

Five plugins are required for the full pipeline; two more are strongly
recommended. The exact list, what each one gives you, and the optional extras
are in [Full Harness Bundle](#full-harness-bundle-claude-code) below.

**Why it matters.** Without them you have a well-structured filing system and
no one to run it. With them, `/smith` reads your project state and tells you
what to do next; `/forge-cycle` walks an artifact from draft to activated one
gate at a time; `/audit` dispatches independent reviewers who must find real
issues rather than rubber-stamp.

---

## AI Skill (recommended for AI agents)

`forgeplan setup-skill` installs the bundled `/forge` skill for Claude Code:

```bash
forgeplan setup-skill   # writes ~/.claude/skills/forge/SKILL.md, no network
```

After installation, use in chat:
```
/forge "Add OAuth2 authentication"
```

This is an offline skill installation, not a plugin or cross-client installer:

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
/plugin install fpl-skills@ForgePlan-marketplace          # smith, forgeplan-cookbook, audit, sprint, methodology-check, ...
/plugin install agents-pro@ForgePlan-marketplace          # guardian, brief-intake, adr-architect, research-analyst, ...
/plugin install agents-sparc@ForgePlan-marketplace        # one feature in an existing system: 4 phase specialists + orchestrator
/plugin install agents-core@ForgePlan-marketplace         # coder, code-reviewer, tester, ...
/plugin install forgeplan-workflow@ForgePlan-marketplace  # /forge-cycle + /forge-audit + guardian gate enforcement

# 2 SHOULD plugins — strongly recommended
/plugin install fpf@ForgePlan-marketplace                 # FPF reasoning and optional knowledge-base context
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
| `agents-sparc` | Use Specification → Pseudocode → Architecture → Refinement → Completion for one feature in an existing, active system. It is not a mandatory first-PRD workflow for every greenfield project; choose the methodology for the task. |
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

The local Constellation build uses [`tract`](https://github.com/sonos/tract),
not the earlier fastembed runtime. Source installation must enable the
feature explicitly:

```bash
cargo install forgeplan --features semantic-search
```

The engine, cached BGE-M3 snapshot, and vectors in your workspace index are
separate. To deliberately prepare the model:

```bash
forgeplan setup
```

It prepares the model and alias, and also attempts to install Git hooks:

- downloads the **embedding model** up front, so your first semantic search
  does not stall for minutes with no explanation
- creates the **`fpl` alias** when you installed via `cargo install` — brew and
  `install.sh` get it from cargo-dist's `bin-aliases`, but cargo has no
  post-install hook, so a source install has `forgeplan` and no `fpl`
- installs `post-merge` and `post-checkout` index-sync hooks in the current
  Git repository when applicable; existing foreign hooks are left intact.

`--skip-model`, `--skip-alias` and `--skip-hooks` opt out of those actions.
The alias is created beside the running executable; an occupied alias path
is not overwritten.

Queries and background maintenance use cached weights; they do not download
missing snapshots. `init -y --with-model` is an explicit download opt-in for
a new workspace, but an already-initialized workspace can return before it.

The roughly **2.1 GB** model is shared across projects. Current cache priority:

1. Nonempty `FORGEPLAN_MODEL_CACHE` (cache root, not snapshot directory).
2. `~/.forgeplan/models` on macOS, Linux and Windows.
3. Platform cache, then legacy relative fallback, only if no home is available.

`HF_HOME` does **not** override the tract path. Earlier platform caches stay
in place; select one explicitly with `FORGEPLAN_MODEL_CACHE` if needed.
Do not delete caches for diagnosis.

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

For a new project this creates `.forgeplan/` with config and a derived index.
Repeated `init -y` is a no-op when a workspace is found, including in a parent
directory. On a clone that already has `.forgeplan/` but no config, inspect
the root and use `init -y --force` only if you intend to activate defaults;
it preserves Markdown and backs up the old local state. Then run `reindex`.
See [Configuration](/docs/getting-started/configuration/).

## Verify Installation

```bash
forgeplan --version
forgeplan health
```

:::note
AI agents should always use `forgeplan init -y` (non-interactive mode).
:::
