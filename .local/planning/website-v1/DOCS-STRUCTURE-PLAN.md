# Documentation Structure Plan

## Website Docs Architecture

```
website/src/content/docs/
├── getting-started/
│   ├── installation.md       ← FORGEPLAN-GUIDE.md
│   ├── quick-start.md        ← FORGEPLAN-GUIDE.md
│   └── configuration.md      ← CLAUDE.md
│
├── methodology/
│   ├── overview.md            ← METHODOLOGY-COURSE.md ch1
│   ├── routing.md             ← DEPTH-CALIBRATION.md
│   ├── lifecycle.md           ← ARTIFACT-MODEL.md
│   ├── evidence.md            ← QUALITY-GATES.md
│   └── adi.md                 ← METHODOLOGY-COURSE.md ch8
│
├── artifacts/
│   ├── types.md               ← ARTIFACT-MODEL.md (10 types)
│   ├── prd.md                 ← PRD-SCHEMA.md + example
│   ├── rfc.md                 ← RFC template + example
│   ├── adr.md                 ← ADR template + example
│   └── evidence.md            ← EvidencePack guide
│
├── cli/
│   ├── overview.md            ← auto-gen from --help
│   ├── health.md              ← command reference
│   ├── route.md
│   ├── new.md
│   ├── validate.md
│   ├── score.md
│   ├── ... (33 commands)
│
├── mcp/
│   ├── overview.md            ← MCP server guide
│   ├── tools.md               ← 28 tool reference
│   └── integration.md         ← Claude Code + GPT setup
│
├── marketplace/
│   ├── overview.md            ← marketplace intro
│   ├── forgeplan-workflow.md  ← plugin guide
│   ├── dev-toolkit.md         ← plugin guide
│   ├── fpf.md                 ← FPF plugin guide
│   └── installation.md        ← how to install plugins
│
├── guides/
│   ├── ten-rules.md           ← HOW-TO-USE.md
│   ├── git-workflow.md        ← CLAUDE.md git section
│   ├── agent-hooks.md         ← AGENT-HOOKS.md
│   └── by-role.md             ← USAGE-BY-ROLE.md
│
└── reference/
    ├── glossary.md            ← GLOSSARY.md
    ├── decision-tree.md       ← PRD-RFC-ADR-FLOW.md
    └── schemas.md             ← schemas overview
```

## Migration Priority

P0: getting-started/ + methodology/ (core onboarding)
P1: cli/ + artifacts/ (reference)
P2: mcp/ + marketplace/ + guides/ (integration)
P3: reference/ + examples (lookup)

## Sources

All content from: docs/guides/*.md, docs/schemas/*.md, CLAUDE.md
Marketplace: https://github.com/ForgePlan/marketplace

*Created: 2026-04-05*
