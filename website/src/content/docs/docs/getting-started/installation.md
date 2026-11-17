---
title: Installation
description: Install Forgeplan - CLI, AI Skill, or MCP Server
---

## Read this first: three things installing the binary does not do

Forgeplan 0.38.0 supports artifacts, validation, scoring, links and keyword
search without an LLM or marketplace plugins. Choose the additional setup for
the capabilities you need:

| Capability | Setup | Requirement |
|---|---|---|
| Semantic search | `forgeplan setup`, then `forgeplan embed` in each workspace | Model weights (~2.1 GB) and artifact embeddings |
| FPF context for ADI | Install `fpf`, then `forgeplan fpf ingest` | Corpus in the workspace; `reason --fpf` also needs an LLM |
| Agent orchestration | Install marketplace plugins | Needed for `/smith`, `/forge-cycle`, `/audit` and specialist agents, not for CLI-only work |

These are separate capabilities, not three mandatory installation steps.
`forgeplan reason` requires an LLM provider in `.forgeplan/config.yaml`;
installing a skill does not configure one. See [Configuration](/docs/getting-started/configuration/).

### 1. Fetch the embedding model

```bash
forgeplan setup
```

Every released binary: Homebrew, `install.sh`, and GitHub Release archives, all carry the semantic search engine. It does **not** carry the 2.1 GB of model weights, because shipping them in the binary would make every download 2.1 GB whether you use search or not.

**Why it matters.** The model downloads on first use if it isn't cached.
`forgeplan setup` makes that download explicit. The default smart `search`
can fall back to keyword scoring when the semantic channel is unavailable.
Explicit `search --semantic` does not provide that fallback: model or
embedding failures can return an error.

**If it did not download.** `forgeplan setup` is idempotent: run it again.
`forgeplan init -y` does not download unless you explicitly add `--with-model`.
After creating or changing artifacts, run `forgeplan embed` to index them.
Downloading the model alone does not create artifact vectors.

Details, cache locations and overrides: [First run](#first-run-fetch-the-embedding-model).

### 2. Install FPF and ingest it

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install fpf@ForgePlan-marketplace   # inside Claude Code
forgeplan init -y                          # in your project shell, if not initialized
forgeplan fpf ingest                       # imports the corpus into this workspace
forgeplan fpf status
forgeplan fpf search "trust calculus"
```

The First Principles Framework corpus ships as a **separate plugin**, not
inside the binary. `fpf ingest` parses its sections and stores them in your
workspace knowledge base. In the released semantic-search build it also
loads the embedding model and computes section vectors.

**When it is required.** The published harness uses FPF for Standard+ work.
The CLI does not require an FPF installation for all commands:
`forgeplan reason` requires an LLM; `--fpf` additionally requests corpus
context. Reasoning generates alternatives for review; it does not verify
their correctness or replace independent evidence.

**Why this step is easy to get wrong.** Skipping it produces a closed loop
that looks like a working system: `fpf search` reports "no matches, run
ingest", and before v0.35.0 `ingest` pointed at a skill name that no longer
existed. An empty corpus and a missing corpus give the identical answer. If
`fpf search` returns nothing, run `forgeplan fpf status`. It distinguishes
the two.

Details: [`forgeplan fpf ingest`](/docs/cli/fpf-ingest/).

### 3. Install the agent harness

Everything above works from a plain shell. But Forgeplan is designed to be
driven by an agent, and the commands that do the driving: `/smith`, `/forge-cycle`, `/audit`, `/sprint`. They live in marketplace plugins, not in the binary.

The published `/smith-bootstrap` checks five required plugins; `fpf` and
`fpl-hsmem` are recommendations at that bootstrap gate. That is a skill-level
contract, not a requirement of the CLI binary. See
[Full Harness Bundle](#full-harness-bundle-claude-code) below.

`/smith` recommends a methodology and agents. `/forge-cycle` coordinates an
artifact pipeline. Independent reviewers assess the generated work; the
main orchestrator handles activation. Installing plugins alone does not run
the pipeline or prove that an artifact is ready.

---

## AI Skill (recommended for AI agents)

With the CLI installed, add its bundled `/forge` skill to Claude Code:

```bash
forgeplan setup-skill   # writes ~/.claude/skills/forge/SKILL.md, no network
```

After installation, use in chat:
```
/forge "Add OAuth2 authentication"
```

This command writes one embedded skill file to
`~/.claude/skills/forge/SKILL.md` without network access. It does not install
marketplace plugins, specialist agents, MCP configuration or an LLM provider.
It does not install the skill into Cursor, Codex or Gemini.

See [`forgeplan setup-skill`](/docs/cli/setup-skill/) for details.

**Discover more plugins**: [Marketplace Overview](/docs/marketplace/overview/).

## Full harness bundle (Claude Code)

The full harness is separate from the bundled `/forge` skill. The list below
matches the five-plugin gate in the public `fpl-skills` 1.75.1
`/smith-bootstrap` skill. It provides routing, artifact orchestration and
specialists; add the methodology-specific plugins for BMAD or enforced TDD
when those workflows apply.

Run these inside a Claude Code session:

```bash
# One-time: add the marketplace
/plugin marketplace add ForgePlan/marketplace

# Required by /smith-bootstrap, not by the CLI
/plugin install fpl-skills@ForgePlan-marketplace          # /smith, /audit, /sprint, /methodology-check
/plugin install agents-pro@ForgePlan-marketplace          # brief-intake, adr-architect, guardian, research-analyst
/plugin install agents-sparc@ForgePlan-marketplace        # specification and architecture specialists; /sparc
/plugin install agents-core@ForgePlan-marketplace         # coder, code-reviewer, tester
/plugin install forgeplan-workflow@ForgePlan-marketplace  # /forge-cycle and /forge-audit

# Recommended by the bootstrap skill
/plugin install fpf@ForgePlan-marketplace                 # FPF corpus and reasoning skills
/plugin install fpl-hsmem@ForgePlan-marketplace           # optional memory; separate server/project setup required

# Reload to activate
/reload-plugins
```

After reload, use `/smith-bootstrap` for a fresh repo or `/smith` for
next-action recommendations. A populated existing codebase needs discovery,
not greenfield bootstrap. Hindsight 3.7.0 additionally requires a compatible
server and an explicit `.hindsight.json` with `url`, `banks` and `defaultBank`;
the selected bank must already exist. Memory hooks default off. Plugin
installation alone does not enable memory.

### What each plugin gives you

| Plugin | What you can do with it |
|---|---|
| `fpl-skills` | `/smith` routes methodology; `/audit` and `/sprint` coordinate review and execution; `/forgeplan-cookbook` helps choose tools. `/smith-plan` recommends a plan rather than executing it. |
| `agents-pro` | Creators such as `brief-intake` and `adr-architect`, independent reviewers and the `guardian` gatekeeper. Creators do not self-activate; guardian returns a verdict, not an activation. |
| `agents-sparc` | Specification and architecture specialists, plus `/sparc` for one well-scoped feature in an **existing active system**. Bootstrap reuses its `specification` agent; that does not make SPARC the greenfield methodology. |
| `agents-core` | `coder` changes source; `code-reviewer` and `tester` independently assess the result and record evidence. Keep generator and verifier in separate contexts. |
| `forgeplan-workflow` | `/forge-cycle` coordinates artifact stages and `/forge-audit` coordinates review. These are marketplace orchestration, not additional CLI commands or a universal server-side lock. |

### Optional plugins

| Plugin | When to add |
|---|---|
| `agents-bmad` | `/bmad` for a brand-new product/service, idea to delivery; run `/bmad-init` first to enable its no-code-before-plan gate |
| `agents-tdd` | `/tdd` for test-first implementation with an active parent PRD and an active SPEC containing `#### Scenario` blocks; run `/tdd-init` first |
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
cargo install forgeplan --version 0.38.0 --locked --features semantic-search
```

### GitHub Releases

Download pre-built binaries from [GitHub Releases](https://github.com/ForgePlan/forgeplan/releases).

### First run: fetch the embedding model

Semantic search is compiled into the published 0.38.0 release binaries:
Homebrew, the install script and GitHub Release archives. The engine is
[`tract`](https://github.com/sonos/tract), a pure-Rust inference engine.
Source builds need the `semantic-search` feature shown above.

What the binary does *not* carry is the model itself. Run once per machine:

```bash
forgeplan setup
```

It prepares the following:

- downloads the **embedding model** up front, so your first semantic search
  does not stall for minutes with no explanation
- creates the **`fpl` alias** when you installed via `cargo install`. brew and
  `install.sh` get it from cargo-dist's `bin-aliases`, but cargo has no
  post-install hook, so a source install has `forgeplan` and no `fpl`
- installs managed **`post-merge` and `post-checkout` git hooks** in the current
  repository to run `forgeplan git-sync` after pulls and branch changes.
  Existing user hooks are not overwritten. These hooks update the derived
  index; they do not compute embeddings or enforce agent permissions.

The steps are idempotent; `--skip-model`, `--skip-alias` and `--skip-hooks`
opt out individually. An existing alias at the installation path is not
overwritten.

Run `forgeplan embed` in each workspace to create vectors for its artifacts.
Default smart search can use keyword scoring without vectors; explicit
`--semantic` requires working embedding support.

`forgeplan init` also offers the download interactively. Under `-y`, it
downloads only if you explicitly add `--with-model`.

The model is **~2.1 GB**, downloaded once per machine with a progress bar, and
cached outside your projects, in the platform cache directory:

| Platform | Cache location |
|---|---|
| macOS | `~/Library/Caches/forgeplan/models` |
| Linux | `~/.cache/forgeplan/models` |
| Windows | `%LOCALAPPDATA%\forgeplan\models` |

Override with `FORGEPLAN_MODEL_CACHE`. The 0.38.0 `tract` download path uses
this resolved cache directly; it does not use `HF_HOME` to override it.

To check which kind of build you have, run `forgeplan embed`: a build without
the feature refuses immediately and prints the install command above, while a
build with it starts loading the model. `forgeplan --version` does not report
features.

## MCP Server (for AI agents)

From your project directory, prefer the installer so it records the detected
binary's absolute path:

```bash
forgeplan mcp install --client claude --scope project
```

For manual configuration, add this to `.mcp.json` and ensure the client's
environment can resolve `forgeplan`:

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
