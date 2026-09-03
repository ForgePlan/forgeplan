---
title: Installation
description: Install Forgeplan - CLI, AI Skill, or MCP Server
---

## Read this first: three things installing the binary does not do

Installing Forgeplan gives you routing, artifacts, validation, scoring, the
link graph and keyword search. Three further capabilities each need one
command, and **each one fails quietly if you skip it**. Nothing crashes. You
get a worse answer and no indication that you got one — which is exactly why
this block is at the top of the page rather than buried below.

| Step | Command | What you lose by skipping it |
|---|---|---|
| **1. Embedding model** (~2.1 GB) | `forgeplan setup` | `search --semantic` degrades to keyword matching |
| **2. FPF knowledge base** | install the `fpf` plugin, then `forgeplan fpf ingest` | `fpf search` answers "no matches" and `reason --fpf` loses its grounding |
| **3. Agent harness** | install 5 marketplace plugins | `/smith`, `/forge-cycle`, `/audit` and the specialist agents do not exist |

Steps 1 and 2 apply to everyone. Step 3 applies if you drive Forgeplan
through an AI agent — which is the intended way to use it.

### 1. Fetch the embedding model

```bash
forgeplan setup
```

Every released binary — Homebrew, `install.sh`, GitHub Release archives —
carries the semantic search engine. It does **not** carry the 2.1 GB of model
weights, because shipping them in the binary would make every download 2.1 GB
whether you use search or not.

**Why it matters.** Without the model, `forgeplan search --semantic` still
returns results. It falls back to BM25 keyword matching and says so — but a
line of fallback text is easy to miss, and keyword search cannot find "how do
we handle auth failures" in a document that says "retry policy for rejected
credentials". You get plausible results that quietly miss the thing you were
looking for.

**If it did not download.** `forgeplan setup` is idempotent — run it again.
`forgeplan init -y` deliberately never downloads (agents and CI must not pull
gigabytes by accident), so a scripted setup needs `forgeplan init --with-model`.
To check what you actually have, run `forgeplan embed`: it starts loading the
model, or refuses and tells you why. `forgeplan --version` does not report this.

Details, cache locations and overrides: [First run](#first-run-fetch-the-embedding-model).

### 2. Install FPF and ingest it

```bash
/plugin install fpf@ForgePlan-marketplace   # inside Claude Code
forgeplan fpf ingest                        # then, in your shell
forgeplan fpf search "trust calculus"       # verify — should return B.3
```

The First Principles Framework spec is a 204-section corpus that ships as a
**separate skill**, not inside the binary. `fpf ingest` parses it, embeds it,
and writes it into your workspace knowledge base.

**Why it matters.** FPF is what `forgeplan reason` uses for ADI reasoning —
the step that makes an artifact generate genuine alternatives instead of
restating your first idea. It is required for Standard-depth work and above.

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
