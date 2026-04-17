# Documentation Index

[English](README.md) · [Русский](README.ru.md)

Production documentation for the Forgeplan project.

> **Local notes** (research, planning, sessions, raw source materials) live in `.local/` (gitignored) — not part of this tree.

## Structure

```
docs/
├── README.md          ← this file — navigation index
├── ROADMAP.md         ← gap analysis + priority matrix (Architecture, UX, Distribution, Docs)
├── methodology/       ← how the Forgeplan methodology works (for humans)
├── operations/        ← agent hooks, enforcement, repo protection (devops)
└── schemas/           ← formal artifact schemas (contracts for the validator)
```

**Artifacts** (PRDs, RFCs, ADRs, Epics, Specs, Evidence, Problems, Notes) live in the Forgeplan workspace at `.forgeplan/` — see [Artifacts](#artifacts) below.

## Methodology — start here

Full methodology reference. Canonical source for humans learning to use Forgeplan.

| Document | Purpose |
|---|---|
| [FORGEPLAN-GUIDE.md](methodology/FORGEPLAN-GUIDE.md) | **Start here** — full guide: methodology + CLI + evidence + lifecycle |
| [HOW-TO-USE.md](methodology/HOW-TO-USE.md) | 10 methodology rules with practical examples |
| [ARTIFACT-MODEL.md](methodology/ARTIFACT-MODEL.md) | Artifact hierarchy: Epic → PRD → Spec → RFC → ADR + lifecycle |
| [PRD-RFC-ADR-FLOW.md](methodology/PRD-RFC-ADR-FLOW.md) | Decision tree: which artifact type to create |
| [DEPTH-CALIBRATION.md](methodology/DEPTH-CALIBRATION.md) | Tactical → Standard → Deep → Critical, with auto-escalation |
| [QUALITY-GATES.md](methodology/QUALITY-GATES.md) | Verification Gate + Adversarial Review + R_eff scoring |
| [UNIFIED-WORKFLOW.md](methodology/UNIFIED-WORKFLOW.md) | Forgeplan × Orchestra × Hindsight integration |
| [USAGE-BY-ROLE.md](methodology/USAGE-BY-ROLE.md) | How to use Forgeplan based on your role |
| [METHODOLOGY-COURSE.md](methodology/METHODOLOGY-COURSE.md) | Full learning path (course format) |
| [GLOSSARY.md](methodology/GLOSSARY.md) | 31 terms + lifecycle reference table |
| [LESSONS.ru.md](methodology/LESSONS.ru.md) | Lessons learned — dependent sprint base verification, audit incidents, process improvements |

## Operations

Setup, hooks, and repository protection.

| Document | Purpose |
|---|---|
| [AGENT-ENFORCEMENT.md](operations/AGENT-ENFORCEMENT.md) | Rules and guardrails for AI agents working in this project |
| [AGENT-HOOKS.md](operations/AGENT-HOOKS.md) | PreToolUse / PostToolUse hooks (safety, formatting, tests) |
| [REPO-PROTECTION-GUIDE.md](operations/REPO-PROTECTION-GUIDE.md) | Branch protection, PR rules, destructive-action prevention |
| [GIT-WORKFLOW.ru.md](operations/GIT-WORKFLOW.ru.md) | Full Git rules — branching lifecycle, PR pipeline, release process, worktrees |
| [SOURCE-PORTING.ru.md](operations/SOURCE-PORTING.ru.md) | Reference Code map — what was ported from `sources/{quint-code,git-adr,BMAD,OpenSpec,ccpm}` to our crates |

## Schemas

Formal specifications that the validator enforces.

| Document | Purpose |
|---|---|
| [PRD-SCHEMA.md](schemas/PRD-SCHEMA.md) | PRD: MUST sections, depth calibration, validation rules |
| [EPIC-SCHEMA.md](schemas/EPIC-SCHEMA.md) | Epic: aggregated progress, children rules |
| [SPEC-SCHEMA.md](schemas/SPEC-SCHEMA.md) | Spec: API contracts, data models, versioning |

## Artifacts

**Location:** `.forgeplan/` in the repository root.

**Storage model (per [ADR-003](../.forgeplan/adrs/ADR-003-markdown-files-as-source-of-truth-lancedb-as-index-layer.md)):**
- **Markdown files** in `.forgeplan/{adrs,rfcs,prds,epics,specs,evidence,problems,solutions,notes,refresh,memory}/` = **source of truth** (git-tracked)
- **LanceDB** in `.forgeplan/lance/` = derived index layer (git-ignored, rebuildable)
- **Config** `.forgeplan/config.yaml` = local LLM keys (git-ignored)

**Directories:**

| Directory | Contents |
|---|---|
| [`.forgeplan/epics/`](../.forgeplan/epics/) | Epics — strategic groupings |
| [`.forgeplan/prds/`](../.forgeplan/prds/) | Product Requirements Documents |
| [`.forgeplan/rfcs/`](../.forgeplan/rfcs/) | RFCs — architectural proposals with implementation phases |
| [`.forgeplan/adrs/`](../.forgeplan/adrs/) | Architecture Decision Records |
| [`.forgeplan/specs/`](../.forgeplan/specs/) | Formal specifications (API contracts, data models) |
| [`.forgeplan/evidence/`](../.forgeplan/evidence/) | EvidencePacks — tests, benchmarks, measurements |
| [`.forgeplan/problems/`](../.forgeplan/problems/) | ProblemCards — problem framing with anti-Goodhart indicators |
| [`.forgeplan/solutions/`](../.forgeplan/solutions/) | SolutionPortfolios — 2-3+ variants with weakest-link scoring |
| [`.forgeplan/notes/`](../.forgeplan/notes/) | Micro-decisions (auto-expire 90 days) |
| [`.forgeplan/refresh/`](../.forgeplan/refresh/) | RefreshReports — re-evaluation of stale artifacts |
| [`.forgeplan/memory/`](../.forgeplan/memory/) | Decision memory |

**Managing artifacts:** always use `forgeplan` CLI — do not hand-edit YAML frontmatter.

```bash
forgeplan new prd "Title"        # create new artifact
forgeplan list -t adr            # list all ADRs
forgeplan get ADR-003            # read one
forgeplan validate PRD-024       # check quality
forgeplan score PRD-024          # compute R_eff
forgeplan scan-import            # rebuild LanceDB index from markdown
```

**Fresh clone workflow:**

```bash
git clone <repo> && cd forgeplan
forgeplan init -y                # creates .forgeplan/lance/ locally (empty)
forgeplan scan-import            # indexes tracked markdown into LanceDB
forgeplan list                   # verify — should see all artifacts
```

## See also

- [`CLAUDE.md`](../CLAUDE.md) — project instructions for Claude Code
- [`AGENTS.md`](../AGENTS.md) — standard instructions for other AI agents (Aider, Cursor, etc.)
- [`README.md`](../README.md) — project README for humans
- [`templates/`](../templates/) — markdown templates for each artifact kind
- `.local/` (gitignored) — local research, planning, sessions, raw source materials

## Conventions

- **All paths in documents are relative to repository root.**
- **Artifact files in `.forgeplan/` are managed by the `forgeplan` CLI** — hand-editing works but may cause drift with the LanceDB index until `scan-import` runs.
- **Methodology docs here are authoritative** — if a tutorial and a schema disagree, the schema wins.
- **Activated artifacts are immutable** — supersede via `forgeplan supersede`, do not rewrite history.
