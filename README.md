<div align="center">

# ForgePlan

**Forge your plan — from raw idea to proven decision.**

ForgePlan is an **engineering decision framework** — a methodology plus CLI for managing structured
artifacts (PRD, RFC, ADR, Epic, Spec) with quality scoring, evidence tracking, semantic search,
and native AI-agent integration.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/ForgePlan/forgeplan?include_prereleases)](https://github.com/ForgePlan/forgeplan/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/ForgePlan/forgeplan/ci.yml?branch=main)](https://github.com/ForgePlan/forgeplan/actions)

[English](README.md) · [Русский](README.ru.md) · [Documentation](docs/README.md) · [Methodology](docs/methodology/FORGEPLAN-GUIDE.md) · [Releases](https://github.com/ForgePlan/forgeplan/releases)

</div>

---

## What is ForgePlan?

ForgePlan turns ad-hoc engineering work into a **disciplined decision pipeline**:

```
Observe → Route → Shape → Build → Prove → Ship
```

Every non-trivial task becomes a traceable chain of artifacts — PRDs capture *what* and *why*, RFCs describe *how*, ADRs record *decisions*, and EvidencePacks provide *proof*. Quality is scored automatically via **R_eff** (effective reliability) and **F-G-R** (Formality–Granularity–Reliability). Stale decisions surface on their own. Nothing rots in the dark.

It's built for **teams working with AI agents** — Claude Code, Cursor, Aider — where the methodology needs to be machine-readable as well as human-readable.

## Why?

| Problem | How ForgePlan solves it |
|---|---|
| Decisions get lost in Slack/Linear/email | Every decision is a git-tracked markdown artifact with structured fields |
| No way to tell if a decision is still valid | Evidence packs with expiration + `R_eff` scoring flag stale artifacts |
| "Why did we choose X?" has no answer six months later | ADRs with required *Context → Decision → Consequences* structure |
| AI agents produce plausible-but-shallow work | Depth calibration (Tactical → Standard → Deep → Critical) enforces rigor per task |
| Artifacts become obsolete or drift from code | File watcher + `forgeplan scan-import` keep markdown and index in sync |
| Research never makes it into the process | SolutionPortfolio with weakest-link scoring forces alternatives |

## Features

- **Markdown-first storage** — all artifacts live in `.forgeplan/` as plain markdown, version-controlled in git. LanceDB is a derived index, not a cache of truth.
- **Quality scoring** — `R_eff` (weakest-link evidence trust) and `F-G-R` (epistemic quality) are computed automatically.
- **Smart routing** — `forgeplan route "task"` analyzes the request and suggests the right artifact pipeline and depth.
- **ADI reasoning** — *Abduction → Deduction → Induction*. Forces 3+ hypotheses before decisions.
- **MCP server** — 37+ tools for AI agents. Works natively with Claude Code, Cursor, Continue.
- **Semantic search** — local fastembed (BGE-M3, 1024 dims). No network, no API keys.
- **Graph queries** — topological sort, blocked artifacts, dependency traversal (petgraph).
- **Depth calibration** — task complexity determines how many artifacts you *must* create. Don't over-document a typo fix.
- **Evidence decay** — artifacts with expired `valid_until` are marked stale. Trust deteriorates honestly.
- **Lifecycle v2** — `draft → active → superseded/deprecated/stale → renew/reopen`. Terminal states are terminal.

## Install

### Homebrew (macOS, Linux)

```bash
brew install ForgePlan/tap/forgeplan
```

### Install script (Linux, macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/ForgePlan/forgeplan/main/install.sh | sh
```

### From source

```bash
git clone https://github.com/ForgePlan/forgeplan.git
cd forgeplan
cargo install --path crates/forgeplan-cli
```

### Binary releases

Download the latest binary for your platform from [Releases](https://github.com/ForgePlan/forgeplan/releases).

## Quick Start

```bash
# 1. Initialize workspace in your project
cd my-project
forgeplan init -y

# 2. Check project health
forgeplan health

# 3. Route a task to the correct depth and pipeline
forgeplan route "Add OAuth2 authentication"
#   → Depth: Standard
#   → Pipeline: PRD → RFC
#   → Confidence: 92%

# 4. Create an artifact
forgeplan new prd "OAuth2 Authentication"

# 5. Fill MUST sections (Problem, Goals, Non-Goals, Target Users, FR), then validate
forgeplan validate PRD-001

# 6. Reason through alternatives (ADI cycle — 3+ hypotheses)
forgeplan reason PRD-001

# 7. Implement, then capture evidence
forgeplan new evidence "OAuth2: 15 tests pass, Google login benchmarked 180ms p95"
forgeplan link EVID-001 PRD-001 --relation informs
forgeplan score PRD-001
#   → R_eff = 1.00

# 8. Review and activate
forgeplan review PRD-001
forgeplan activate PRD-001
```

Full tutorial: **[docs/methodology/FORGEPLAN-GUIDE.md](docs/methodology/FORGEPLAN-GUIDE.md)**

## Architecture

ForgePlan ships as three components:

| Component | Role |
|---|---|
| `forgeplan-core` | Storage, validation, scoring, routing, search, graph, FPF reasoning engine |
| `forgeplan-cli` | The `forgeplan` binary — 33 commands |
| `forgeplan-mcp` | MCP server for AI agents — 37 tools over stdio transport |

**Storage model ([ADR-003](.forgeplan/adrs/ADR-003-markdown-files-as-source-of-truth-lancedb-as-index-layer.md)):**

- Markdown files in `.forgeplan/` = **source of truth** (git-tracked)
- LanceDB in `.forgeplan/lance/` = derived index (gitignored, rebuildable via `forgeplan scan-import`)

## Documentation

- **[docs/README.md](docs/README.md)** — Documentation index
- **[docs/methodology/](docs/methodology/)** — Methodology guides (10 documents)
  - [FORGEPLAN-GUIDE.md](docs/methodology/FORGEPLAN-GUIDE.md) — Full reference (**start here**)
  - [HOW-TO-USE.md](docs/methodology/HOW-TO-USE.md) — 10 rules with examples
  - [DEPTH-CALIBRATION.md](docs/methodology/DEPTH-CALIBRATION.md) — Tactical → Critical
  - [QUALITY-GATES.md](docs/methodology/QUALITY-GATES.md) — R_eff, adversarial review
  - [PRD-RFC-ADR-FLOW.md](docs/methodology/PRD-RFC-ADR-FLOW.md) — Which artifact for which task
  - [UNIFIED-WORKFLOW.md](docs/methodology/UNIFIED-WORKFLOW.md) — ForgePlan × Orchestra × Hindsight
- **[docs/operations/](docs/operations/)** — Agent hooks, enforcement, repo protection
- **[docs/schemas/](docs/schemas/)** — Formal artifact schemas (PRD, EPIC, SPEC)
- **[CLAUDE.md](CLAUDE.md)** — Project instructions for Claude Code
- **[AGENTS.md](AGENTS.md)** — Standard instructions for AI agents (Aider, Cursor, Continue)

## Project artifacts

This repository dogfoods ForgePlan — the project is managed with itself.

- **[.forgeplan/adrs/](.forgeplan/adrs/)** — Architecture Decision Records (5)
- **[.forgeplan/rfcs/](.forgeplan/rfcs/)** — Architectural proposals (6)
- **[.forgeplan/prds/](.forgeplan/prds/)** — Product Requirements (24+)
- **[.forgeplan/epics/](.forgeplan/epics/)** — Epics (2)
- **[.forgeplan/evidence/](.forgeplan/evidence/)** — Evidence packs (50+)

Use the `forgeplan` CLI to browse: `forgeplan list`, `forgeplan get ADR-003`, `forgeplan health`.

## Status

- **Current release:** [v0.15.1](https://github.com/ForgePlan/forgeplan/releases/tag/v0.15.1)
- **Tests:** 728+ passing
- **Commands:** 33 CLI commands, 37 MCP tools
- **Dogfood:** This repo manages itself — 138 tracked markdown artifacts

See [.forgeplan/prds/](.forgeplan/prds/) and the current [CHANGELOG](https://github.com/ForgePlan/forgeplan/releases) for the roadmap.

## Contributing

See **[CLAUDE.md](CLAUDE.md)** for the full contribution guide: branching strategy, commit conventions, PR pipeline, and methodology requirements.

Short version:

1. Branch from `dev`: `git checkout dev && git pull && git checkout -b feat/my-feature`
2. Follow the full cycle: **Route → Shape → Validate → Build → Evidence → Activate**
3. `cargo fmt` + `cargo test` before every commit
4. PR → `dev` (feature/fix/docs branches); PR → `main` only via `release/vX.Y.Z` branches

## Related

- [`website/`](website/) — Official website (Astro + Starlight + React + GSAP)
- [`marketplace/`](marketplace/) — Plugin marketplace (ForgePlan methodology + FPF + dev toolkit)
- [`templates/`](templates/) — Markdown templates for each artifact kind

## License

MIT License — see [LICENSE](LICENSE) for details.

## Acknowledgements

ForgePlan stands on the shoulders of:

- **[Quint-code](https://github.com/quint-code)** — R_eff scoring, data model inspiration
- **[BMAD Method](https://github.com/bmadcode/BMAD-METHOD)** — PRD workflow, 13-step validation
- **[OpenSpec](https://openspec.ai/)** — Artifact DAG, delta-specs
- **[First Principles Framework](https://github.com/ForgePlan/marketplace/tree/main/plugins/fpf)** — Reasoning architecture, ADI cycle, trust calculus
- **[adr-tools](https://github.com/npryce/adr-tools)** — ADR pattern (Michael Nygard)
- **[LanceDB](https://lancedb.com/)** — Embedded vector database
- **[fastembed](https://github.com/qdrant/fastembed)** — Local embeddings (BGE-M3)

---

<div align="center">

**Forge your plan. Structure. Evidence. Trust.**

[Documentation](docs/README.md) · [Releases](https://github.com/ForgePlan/forgeplan/releases) · [Marketplace](marketplace/) · [Русский](README.ru.md)

</div>
