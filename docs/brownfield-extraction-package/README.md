# Brownfield Business Logic Extraction Package

> **Purpose**: Design package for a reverse-engineering workflow intended to produce **standalone business documentation**. RAG-ready, rewrite-ready and interview-ready output are design goals, not verified results of this package.

## Implementation status (2026-10-23)

This directory preserves the original extraction design, example data, templates and prospective task/verification lists. It is **not the current CLI/MCP reference**. Checked headings, sample `PASS` fields, dates, percentages and fixture descriptions are not evidence that those checks ran.

The reviewed target is `c852f40c` (`0.39.0+constellation.s1.1`). It registers all six brownfield kinds, native source/verification-plan validation, hypothesis promotion, and limited coverage/contradiction/orphan reports. Its schemas differ from the richer frontmatter, semantic relations and state machines proposed here. Native templates/validators remain authoritative; package templates are authoring proposals, not drop-in runtime defaults.

Registration and creation interfaces differ: the captured `forgeplan_new`/`forgeplan_generate` MCP enums and `new` help still list ten older kinds. Do not infer acceptance of all six brownfield kinds on every surface from the core kind registry.

`forgeplan discover start` creates a discovery session and prints a seven-phase protocol; an **agent** must inspect code and report findings. It does not automatically run this twelve-skill design. The core interview draft/ingest tools are explicitly experimental stubs returning `status: not_implemented`; they do not generate packets, ingest answers, promote hypotheses or launch discovery. Their plugin hint is not proof of plugin availability or installation. Canonical rendering, RAG export and reproducibility-check tools proposed here are absent from the captured full catalog.

See [the current/proposed surface comparison](04-FORGEPLAN-EXTENSIONS.md#current-implementation-versus-this-proposal). Authorities are the exact target CLI help/full MCP catalog and `crates/forgeplan-core/src/{artifact/types.rs,validation/rules.rs,hypothesis.rs,brownfield.rs}` plus `crates/forgeplan-mcp/src/server.rs`.

TripSales excerpts and dated run/coverage claims are historical or illustrative; their original source revision and execution receipts are not shipped here. Missing IDs denote examples from that external graph, not artifacts in this repository. The autoresearch baseline is likewise unversioned here. Nothing in this package changes or waives native G1/G2/G3 gates.

## Target audience

This package is consumed by:
1. The **forgeplan maintainer agent** — it reads these docs and generates:
   - New forgeplan artifact kinds (migrations, schema additions).
   - New skills (`.claude/skills/<name>/SKILL.md`).
   - Meta-commands (`.claude/commands/<name>.md`).
   - Integration glue with autoresearch.
2. Future AI agents that run the extraction workflow on real brownfield projects.

## What problem this solves

Current forgeplan + autoresearch produce **code-map documentation**: references to files and line numbers. This is not self-contained — if the code is deleted, moved, or copied elsewhere, the docs become dangling references.

We need docs that answer:
- *Why* does this system exist? What business problem does it solve?
- *What* are the business rules, invariants, and user journeys?
- *Standalone* DDL/SDL/pseudo-code, not file:line pointers.

## Package structure

```
brownfield-extraction-package/
├── README.md                      ← you are here
├── 00-CONTEXT.md                  ← real-world problem that triggered this design
├── 01-PROBLEM-STATEMENT.md        ← detailed gap analysis (code-map vs business docs)
├── 02-METHODOLOGY.md              ← two-tier: Factum vs Intent (FPF-derived)
├── 03-ARCHITECTURE.md             ← 12 bounded contexts + data flow
├── 04-FORGEPLAN-EXTENSIONS.md     ← what to add to forgeplan itself
├── 05-AUTORESEARCH-INTEGRATION.md ← how to glue with existing autoresearch
├── 06-SKILLS-INVENTORY.md         ← the 12 skills at a glance
├── ROADMAP.md                     ← phased implementation plan (5 waves)
├── TASKS.md                       ← concrete task list for the agent
├── VERIFICATION.md                ← acceptance criteria / how to know it's done
├── GLOSSARY.md                    ← meta-terms used in this package
│
├── skills/                        ← design doc per skill (12 files)
│   ├── 01-ubiquitous-language.md
│   ├── 02-use-case-miner.md
│   ├── 03-intent-inferrer.md
│   ├── 04-invariant-detector.md
│   ├── 05-causal-linker.md
│   ├── 06-hypothesis-triangulator.md
│   ├── 07-interview-packager.md
│   ├── 08-scenario-writer.md
│   ├── 09-kg-curator.md
│   ├── 10-canonical-reproducer.md
│   ├── 11-reproducibility-validator.md
│   └── 12-rag-packager.md
│
├── artifact-kinds/                ← new forgeplan kinds (6 files)
│   ├── glossary.md
│   ├── use-case.md
│   ├── invariant.md
│   ├── scenario.md
│   ├── hypothesis.md
│   └── domain-model.md
│
├── templates/                     ← proposed authoring templates, not runtime defaults
│   ├── glossary.template.md
│   ├── use-case.template.md
│   ├── invariant.template.md
│   ├── scenario.template.md
│   ├── hypothesis.template.md
│   └── domain-model.template.md
│
├── orchestration/
│   ├── extract-business-logic.md  ← design of the meta-command
│   └── phase-transitions.md       ← rules between phases 1→5
│
├── integration/
│   ├── autoresearch-hooks.md      ← how skills hook into /autoresearch:*
│   ├── forgeplan-mcp-additions.md ← new MCP tools to expose
│   └── rag-export-format.md       ← JSON/chunks schema for RAG ingestion
│
└── examples/
    ├── tripsales-glossary-sample.md
    ├── tripsales-use-case-sample.md
    ├── tripsales-scenario-sample.md
    ├── tripsales-canonical-orders-sample.md
    └── interview-packet-orders-sample.md
```

## Reading order (for the forgeplan agent)

1. **Start here**: `00-CONTEXT.md` → `01-PROBLEM-STATEMENT.md` → `02-METHODOLOGY.md`
2. **Understand architecture**: `03-ARCHITECTURE.md` → `06-SKILLS-INVENTORY.md`
3. **See what to modify in forgeplan**: `04-FORGEPLAN-EXTENSIONS.md`
4. **Build skill-by-skill**: start with `skills/01-ubiquitous-language.md` (foundation), then follow the dependency order in `ROADMAP.md`.
5. **For each skill**: consult the corresponding `artifact-kinds/<kind>.md` + `templates/<kind>.template.md`.
6. **Final check**: `VERIFICATION.md` — is everything wired correctly?

## Key design principles

1. **Factum ≠ Intent** (see `02-METHODOLOGY.md`). Documents MUST separate what code does from why it exists.
2. **Every assertion has a confidence score** (`verified | inferred | speculation`).
3. **Hypotheses are first-class artifacts** with their own lifecycle.
4. **Domain Owner is a resource, not a blocker** — the proposed workflow packages unresolved hypotheses for interviews; core interview stubs do not do this automatically.
5. **Output is self-contained** — no dangling file:line refs in final docs.
6. **Outputs are RAG-ready** — stable ids, markdown chunks, embeddings-compatible metadata.

## What's out of scope for this package

- Actual execution on TripSales. This package is the **design spec**; running it requires the forgeplan agent to first implement the skills/kinds described here.
- UI / dashboards for the extracted knowledge graph.
- External system integration (Confluence, Notion, etc.) — only file-based markdown output.

## License / reuse

The package is intended to live inside the `forgeplan` project as a contribution. Once the forgeplan agent has consumed it, these files can be archived or deleted — the resulting skills/kinds become the canonical source of truth.

## Contact points

- Methodology questions → `02-METHODOLOGY.md`
- "Why this skill?" questions → `skills/<NN>-*.md` → `Why this skill exists` section
- "How does it integrate?" → `integration/` directory
- Implementation order → `ROADMAP.md`
