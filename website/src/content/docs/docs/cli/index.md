---
title: CLI Reference
description: "Complete reference for all 61 Forgeplan CLI commands."
---

Forgeplan ships with **61 top-level commands** covering the full Shape→Validate→ADI→Code→Evidence→Activate lifecycle.

All commands are listed below grouped by purpose. Click any command for full usage, arguments and examples.

### Workspace & setup

| Command | Description |
|---|---|
| [`forgeplan init`](/docs/cli/init/) | Initialize a new .forgeplan/ workspace |
| [`forgeplan setup-skill`](/docs/cli/setup-skill/) | Install /forge skill for Claude Code |
| [`forgeplan migrate`](/docs/cli/migrate/) | Run schema migrations on existing workspace |
| [`forgeplan import`](/docs/cli/import/) | Import artifacts from JSON file |
| [`forgeplan export`](/docs/cli/export/) | Export all artifacts to JSON file |

### Creating artifacts

| Command | Description |
|---|---|
| [`forgeplan new`](/docs/cli/new/) | Create a new artifact from template |
| [`forgeplan generate`](/docs/cli/generate/) | Generate an artifact using AI from a natural language description |
| [`forgeplan capture`](/docs/cli/capture/) | Capture a decision from conversation into a Note or ADR artifact |
| [`forgeplan promote`](/docs/cli/promote/) | Promote a memory to a full artifact (e.g., forgeplan promote mem-xxx --kind prd) |

### Reading artifacts

| Command | Description |
|---|---|
| [`forgeplan list`](/docs/cli/list/) | List artifacts |
| [`forgeplan get`](/docs/cli/get/) | Read a full artifact by ID |
| [`forgeplan tree`](/docs/cli/tree/) | Show artifact hierarchy as ASCII tree |
| [`forgeplan search`](/docs/cli/search/) | Search artifacts (smart by default: keyword + semantic + boosters) |
| [`forgeplan recall`](/docs/cli/recall/) | Recall memories — search, filter, list |
| [`forgeplan log`](/docs/cli/log/) | Show change log — audit trail of artifact mutations |
| [`forgeplan journal`](/docs/cli/journal/) | Show decision journal — chronological timeline with R_eff scores |
| [`forgeplan session`](/docs/cli/session/) | Show methodology session state (current phase, active artifact) |
| [`forgeplan progress`](/docs/cli/progress/) | Show checkbox progress for artifacts |
| [`forgeplan graph`](/docs/cli/graph/) | Generate mermaid dependency graph of linked artifacts |
| [`forgeplan order`](/docs/cli/order/) | Show artifacts in topological order (dependency order) |

### Editing artifacts

| Command | Description |
|---|---|
| [`forgeplan update`](/docs/cli/update/) | Update artifact metadata or body |
| [`forgeplan delete`](/docs/cli/delete/) | Delete an artifact |
| [`forgeplan tag`](/docs/cli/tag/) | Add tags to an artifact |
| [`forgeplan untag`](/docs/cli/untag/) | Remove tags from an artifact |
| [`forgeplan link`](/docs/cli/link/) | Link two artifacts with a typed relationship |
| [`forgeplan unlink`](/docs/cli/unlink/) | Remove a relation between two artifacts |

### Quality & validation

| Command | Description |
|---|---|
| [`forgeplan validate`](/docs/cli/validate/) | Validate artifact completeness against schema rules |
| [`forgeplan score`](/docs/cli/score/) | Compute R_eff quality score for decisions with evidence |
| [`forgeplan fgr`](/docs/cli/fgr/) | Show F-G-R quality scores (Formality, Granularity, Reliability) |
| [`forgeplan review`](/docs/cli/review/) | Review an artifact — run validation and show lifecycle checklist |
| [`forgeplan estimate`](/docs/cli/estimate/) | Estimate effort for an artifact based on FR and Phase items |
| [`forgeplan calibrate`](/docs/cli/calibrate/) | Suggest depth level (Tactical/Standard/Deep/Critical) based on artifact content |
| [`forgeplan calibrate-estimate`](/docs/cli/calibrate-estimate/) | Compare estimated vs actual hours — calibrate estimation accuracy |
| [`forgeplan decay`](/docs/cli/decay/) | Show evidence decay impact on R_eff scores |
| [`forgeplan stale`](/docs/cli/stale/) | Detect stale artifacts with expired valid_until |

### Lifecycle transitions

| Command | Description |
|---|---|
| [`forgeplan activate`](/docs/cli/activate/) | Activate an artifact (draft → active) with validation gate |
| [`forgeplan supersede`](/docs/cli/supersede/) | Supersede an artifact (active → superseded) with replacement link |
| [`forgeplan deprecate`](/docs/cli/deprecate/) | Deprecate an artifact (active/stale → deprecated) with reason |
| [`forgeplan renew`](/docs/cli/renew/) | Renew a stale artifact (stale → active) with extended validity |
| [`forgeplan reopen`](/docs/cli/reopen/) | Reopen an artifact — creates a NEW draft artifact, deprecates the old one |

### Reasoning & AI

| Command | Description |
|---|---|
| [`forgeplan reason`](/docs/cli/reason/) | Analyze an artifact using FPF ADI reasoning cycle (Abduction→Deduction→Induction) |
| [`forgeplan decompose`](/docs/cli/decompose/) | Decompose a PRD into RFC tasks using AI |
| [`forgeplan context`](/docs/cli/context/) | Single-call reasoning context — artifact + graph + validation + scoring |
| [`forgeplan route`](/docs/cli/route/) | Suggest depth level and artifact pipeline for a task description |

### Dashboards & health

| Command | Description |
|---|---|
| [`forgeplan health`](/docs/cli/health/) | Show project health dashboard — gaps, risks, blind spots, next actions |
| [`forgeplan status`](/docs/cli/status/) | Show project status dashboard |
| [`forgeplan gaps`](/docs/cli/gaps/) | Show pipeline compliance gaps by depth |
| [`forgeplan blocked`](/docs/cli/blocked/) | Show blocked artifacts and their dependencies |
| [`forgeplan blindspots`](/docs/cli/blindspots/) | Show blind spots — decisions without evidence, orphan artifacts |
| [`forgeplan drift`](/docs/cli/drift/) | Check for drifted decisions (affected files changed after decision) |
| [`forgeplan coverage`](/docs/cli/coverage/) | Show decision coverage per code module |

### Indexing & sync

| Command | Description |
|---|---|
| [`forgeplan scan`](/docs/cli/scan/) | Scan codebase for source modules |
| [`forgeplan scan-import`](/docs/cli/scan-import/) | Scan for existing docs and import as artifacts |
| [`forgeplan reindex`](/docs/cli/reindex/) | Rebuild LanceDB index from .md files (files-first sync) |
| [`forgeplan embed`](/docs/cli/embed/) | Generate embeddings for all artifacts (semantic search) |
| [`forgeplan watch`](/docs/cli/watch/) | Watch .forgeplan/ files and sync changes to LanceDB in real time |
| [`forgeplan git-sync`](/docs/cli/git-sync/) | Sync artifact changes from git operations (pull/merge) into LanceDB |

### Memory

| Command | Description |
|---|---|
| [`forgeplan remember`](/docs/cli/remember/) | Save a memory (fact, convention, procedure) for later recall |
| [`forgeplan discover`](/docs/cli/discover/) | Start brownfield discovery — creates session, prints protocol for agent |

### FPF knowledge base

| Command | Description |
|---|---|
| [`forgeplan fpf`](/docs/cli/fpf/) | FPF Knowledge Base — dashboard, ingest, search, sections |

### MCP server

| Command | Description |
|---|---|
| [`forgeplan serve`](/docs/cli/serve/) | Start MCP server (stdio transport) for AI agent integration |

## Ecosystem & Plugins

Beyond the built-in CLI, Forgeplan integrates with AI coding agents via the `/forge` skill and related marketplace plugins:

- [**Forgeplan Workflow**](/docs/marketplace/forgeplan-workflow/) — `/forge`, `/forge-cycle`, `/forge-audit` slash commands
- [**Dev Toolkit**](/docs/marketplace/dev-toolkit/) — `/sprint`, `/audit`, `/recall`, `/research`, `/build`
- [**Marketplace Overview**](/docs/marketplace/overview/) — full plugin catalog

Install the core skill with `forgeplan setup-skill` or `npx skills add ForgePlan/marketplace --skill forge`. Additional plugins are available via `npx skills add ForgePlan/marketplace --plugin <name>`.

