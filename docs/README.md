# Documentation Index

[English](README.md) · [Русский](README.ru.md)

Contributor documentation for the Forgeplan project. Task-oriented user guides
and CLI/MCP references live on [the website](https://forgeplan.dev/docs/).

- [Constellation user guide](https://forgeplan.dev/docs/guides/constellation/)
- [Constellation interface reference](https://forgeplan.dev/docs/reference/constellation/)
- [Portable pilot and rollback runbook (RU)](operations/CONSTELLATION-PILOT.ru.md)

Constellation documentation targets the scoped `0.39.0+constellation.s1.1`
build, not the published `0.38.0` release. It is not a full S1-G/S2 acceptance claim.

> **Local notes** (research, planning, sessions, raw source materials) live in `.local/` (gitignored) — not part of this tree.

## Structure

```
docs/
├── README.md          ← this file — navigation index
├── ROADMAP.md         ← gap analysis + priority matrix (Architecture, UX, Distribution, Docs)
├── methodology/       ← how the Forgeplan methodology works (for humans)
├── architecture/      ← system architecture references (C4, Constellation)
├── operations/        ← agent hooks, enforcement, repo protection (devops)
└── schemas/           ← artifact conventions and implemented validation rules
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
| [GLOSSARY.md](methodology/GLOSSARY.md) | Core terms and lifecycle reference table |
| [LESSONS.ru.md](methodology/LESSONS.ru.md) | Lessons learned — dependent sprint base verification, audit incidents, process improvements |
| [agent-protocol.md](methodology/agent-protocol.md) | **Hint contract (PRD-071)** — 5 markers (Next/Or/Wait/Done/Fix), good/bad examples, agent reading protocol |
| [release-workflow.md](methodology/release-workflow.md) | End-to-end release recipe — dependabot triage gate, version bump, release/* PR strategy, post-release sync (CLAUDE.md red lines #9 + #10), hotfix flow, anti-patterns |

## Operations

Setup, hooks, and repository protection.

| Document | Purpose |
|---|---|
| [AGENT-ENFORCEMENT.md](operations/AGENT-ENFORCEMENT.md) | Rules and guardrails for AI agents working in this project |
| [AGENT-HOOKS.md](operations/AGENT-HOOKS.md) | PreToolUse / PostToolUse hooks (safety, formatting, tests) |
| [LLM-PROVIDERS.md](operations/LLM-PROVIDERS.md) | LLM configuration: HTTP/Ollama plus explicit subscription CLI profiles (`claude-code`, `codex-cli`, `devin-cli`, `gemini-cli`, `cursor-agent`), safety modes, login recovery, and personal/local-only limits |
| [MULTI-AGENT.md](operations/MULTI-AGENT.md) | **v0.24.0+ multi-agent dispatch** — `forgeplan_dispatch/claim/release/claims` MCP tools, file-overlap detection, skill routing |
| [REPO-PROTECTION-GUIDE.md](operations/REPO-PROTECTION-GUIDE.md) | Branch protection, PR rules, destructive-action prevention |
| [GIT-WORKFLOW.ru.md](operations/GIT-WORKFLOW.ru.md) | Full Git rules — branching lifecycle, PR pipeline, release process, worktrees |
| [RELEASE-PROTOCOL.md](operations/RELEASE-PROTOCOL.md) ([ru](operations/RELEASE-PROTOCOL.ru.md)) | **Canonical release procedure** — the 10 steps for any `release/vX.Y.Z` cut, pre-conditions, and the mandatory post-merge sync-PR (RED LINE #9) |
| `dependabot-triage-YYYY-MM-DD.md` | Per-release dependency-alert triage (RED LINE #10) — every open alert labelled addressed / scheduled / accepted-with-justification. One file per release window |
| [SOURCE-PORTING.ru.md](operations/SOURCE-PORTING.ru.md) | Reference Code map — what was ported from `sources/{quint-code,git-adr,BMAD,OpenSpec,ccpm}` to our crates |
| [PLAYBOOK-AUTHORING.ru.md](operations/PLAYBOOK-AUTHORING.ru.md) | **v0.26.0+ playbook authoring** — declarative YAML workflows, 5 delegation types, fallback hints, DAG ordering. **v0.27.0+ Subprocess lifecycle** section (real dispatchers, kill_on_drop, timeout policy, security model) per ADR-010/PRD-072 |
| [INGEST-MAPPINGS.ru.md](operations/INGEST-MAPPINGS.ru.md) | **v0.26.0+ ingest mapping authoring** — translate plugin output to forge artifacts with hallucination-proof `## Sources` invariant (PRD-066/SPEC-004) |
| [QUALITY-GATES.ru.md](operations/QUALITY-GATES.ru.md) | **v0.28.0+ CI quality gates** — all CI gates (fmt, clippy, test, health, validate, drift detector) with local run commands and fix guides. Note: `docs/methodology/QUALITY-GATES.md` covers methodology gates (R_eff, Verification Gate). |
| [CONSTELLATION-PILOT.ru.md](operations/CONSTELLATION-PILOT.ru.md) | Portable sandbox pilot: version gate, backups, embedding prerequisites, single writer and rollback |
| [DOCUMENTATION-AUDIT-2026-10-23.ru.md](operations/DOCUMENTATION-AUDIT-2026-10-23.ru.md) | Measured documentation audit: source boundaries, Constellation coverage, resolved drift, verification and remaining gaps |

## Architecture

| Document | Purpose |
|---|---|
| [constellation.md](architecture/constellation.md) | **Constellation** — versioned generations, pinned reads, federation registry, section-level semantic index, MCP read profile, invariants and known edges |
| [c4-context.md](architecture/c4-context.md) | C4 context view of the system |

## Schemas

Authoring conventions with implemented validation rules called out separately.
Recommendations are not promises of automated semantic enforcement.

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
- **Constellation** publishes derived generations through `.forgeplan/lance/CURRENT.json`; this layout is not interchangeable with the legacy index.
- **Config** `.forgeplan/config.yaml` = workspace settings, including LLM and embedding configuration; not an index or an artifact body. In this repository it is git-tracked and uses environment-variable references rather than embedded secrets. Other projects may keep their configuration local.

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
| `.forgeplan/solutions/` | SolutionPortfolios — 2-3+ variants with weakest-link scoring |
| [`.forgeplan/notes/`](../.forgeplan/notes/) | Notes — small decisions; expiry is not guaranteed merely by their kind |
| `.forgeplan/refresh/` | RefreshReports — re-evaluation of stale artifacts |
| [`.forgeplan/memory/`](../.forgeplan/memory/) | Decision memory |

`solutions/` and `refresh/` are paths, not links to available data: this checkout
does not contain those directories. Create artifacts through the supported
interface rather than adding empty directories just to satisfy navigation.

**Managing artifacts:** use supported CLI/MCP operations rather than hand-editing
YAML frontmatter. CLI `validate` works on legacy and migrated Constellation
layouts; full MCP `forgeplan_validate` was also exercised successfully.
See the [pilot compatibility notes (RU)](operations/CONSTELLATION-PILOT.ru.md)
for the tested scope; 76 registered full-profile tools do not mean every behavior was verified.

```bash
forgeplan new prd "Title"        # create new artifact
forgeplan list -t adr            # list all ADRs
forgeplan get ADR-003            # read one
forgeplan validate PRD-024       # check artifact schema
forgeplan score PRD-024          # compute R_eff
forgeplan index                  # build/reconcile the Constellation index
forgeplan reindex                # legacy rebuild; delegates to index on migrated layout
```

**New directory versus existing clone:**

- In a genuinely new directory outside any existing workspace, `forgeplan init -y`
  creates the workspace. Discovery walks up the directory tree; check the reported
  path before any writes.
- A clone with tracked `.forgeplan/` is already discoverable: `init -y` may return
  early without creating a local config or index. If `.forgeplan/config.yaml`
  already exists, use `forgeplan index` for Constellation or `forgeplan reindex`
  for the legacy layout, then inspect `forgeplan list` and the expected artifacts.
- If local config is missing, back up the entire `.forgeplan/` and MCP config,
  then initialize config deliberately with `forgeplan init --force -y`. This keeps
  artifact bodies but activates default config; custom settings, including
  `embedding`, survive only in the backup until restored. Afterwards, use `index`
  for Constellation or `reindex` for the legacy layout.
  This is not the normal index-recovery command.

## See also

- [`CLAUDE.md`](../CLAUDE.md) — project instructions for Claude Code
- [`AGENTS.md`](../AGENTS.md) — standard instructions for other AI agents (Aider, Cursor, etc.)
- [`README.md`](../README.md) — project README for humans
- [`templates/`](../templates/) — markdown templates for each artifact kind
- `.local/` (gitignored) — local research, planning, sessions, raw source materials

## Conventions

- **Source paths in prose are relative to repository root unless qualified.** Markdown links resolve relative to the containing document.
- **Artifact files in `.forgeplan/` are managed through supported CLI/MCP operations.** For index recovery, use `index` for Constellation or `reindex` for the legacy layout, not `scan-import`.
- **Methodology docs define authoring conventions.** Schema pages distinguish those recommendations from implemented validation and separate activation gates.
- **Preserve active decisions by convention** — replace them via `forgeplan supersede ID --by NEW-ID`, rather than rewriting the decision history. Activation is not filesystem write protection or a blanket runtime ban on body updates.
