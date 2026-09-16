---
title: CLI Reference
description: "Complete reference for all 82 Forgeplan CLI commands."
---

Forgeplan ships with **82 top-level commands** covering the full Shape→Validate→ADI→Code→Evidence→Activate lifecycle.

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
| [`forgeplan recall`](/docs/cli/recall/) | Recall memories - search, filter, list |
| [`forgeplan log`](/docs/cli/log/) | Show change log - audit trail of artifact mutations |
| [`forgeplan journal`](/docs/cli/journal/) | Show decision journal - chronological timeline with R_eff scores |
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
| [`forgeplan review`](/docs/cli/review/) | Review an artifact - run validation and show lifecycle checklist |
| [`forgeplan estimate`](/docs/cli/estimate/) | Estimate effort for an artifact based on FR and Phase items |
| [`forgeplan calibrate`](/docs/cli/calibrate/) | Suggest depth level (Tactical/Standard/Deep/Critical) based on artifact content |
| [`forgeplan calibrate-estimate`](/docs/cli/calibrate-estimate/) | Compare estimated vs actual hours - calibrate estimation accuracy |
| [`forgeplan decay`](/docs/cli/decay/) | Show evidence decay impact on R_eff scores |
| [`forgeplan stale`](/docs/cli/stale/) | Detect stale artifacts with expired valid_until |

### Lifecycle transitions

| Command | Description |
|---|---|
| [`forgeplan activate`](/docs/cli/activate/) | Activate an artifact (draft → active) with validation gate |
| [`forgeplan supersede`](/docs/cli/supersede/) | Supersede an artifact (active → superseded) with replacement link |
| [`forgeplan deprecate`](/docs/cli/deprecate/) | Deprecate an artifact (active/stale → deprecated) with reason |
| [`forgeplan renew`](/docs/cli/renew/) | Renew a stale artifact (stale → active) with extended validity |
| [`forgeplan reopen`](/docs/cli/reopen/) | Reopen an artifact - creates a NEW draft artifact, deprecates the old one |

### Reasoning & AI

| Command | Description |
|---|---|
| [`forgeplan reason`](/docs/cli/reason/) | Analyze an artifact using FPF ADI reasoning cycle (Abduction→Deduction→Induction) |
| [`forgeplan decompose`](/docs/cli/decompose/) | Decompose a PRD into RFC tasks using AI |
| [`forgeplan context`](/docs/cli/context/) | Single-call reasoning context - artifact + graph + validation + scoring |
| [`forgeplan route`](/docs/cli/route/) | Suggest depth level and artifact pipeline for a task description |

### Dashboards & health

| Command | Description |
|---|---|
| [`forgeplan health`](/docs/cli/health/) | Show project health dashboard - gaps, risks, blind spots, next actions |
| [`forgeplan status`](/docs/cli/status/) | Show project status dashboard |
| [`forgeplan gaps`](/docs/cli/gaps/) | Show pipeline compliance gaps by depth |
| [`forgeplan blocked`](/docs/cli/blocked/) | Show blocked artifacts and their dependencies |
| [`forgeplan blindspots`](/docs/cli/blindspots/) | Show blind spots - decisions without evidence, orphan artifacts |
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
| [`forgeplan discover`](/docs/cli/discover/) | Start brownfield discovery - creates session, prints protocol for agent |

### FPF knowledge base

| Command | Description |
|---|---|
| [`forgeplan fpf`](/docs/cli/fpf/) | FPF Knowledge Base - dashboard, ingest, search, sections |

### MCP server

| Command | Description |
|---|---|
| [`forgeplan serve`](/docs/cli/serve/) | Start MCP server (stdio transport) for AI agent integration |

### Other

| Command | Description |
|---|---|
| [`forgeplan activity`](/docs/cli/activity/) | Query the activity log - append-only JSONL record of every MCP tool invocation at .forgeplan/logs/tools-YYYY-MM-DD.jsonl. Use this to reconstruct what the agent did over a time window, attribute LLM-token spend, or audit destructive operations |
| [`forgeplan activity-stats`](/docs/cli/activity-stats/) | Aggregate statistics from the activity log grouped by tool name: count, error count, p50/p95 duration, total time. Use to attribute LLM-token spend and identify slow tools |
| [`forgeplan restore`](/docs/cli/restore/) | Restore a soft-deleted artifact from the most recent non-consumed receipt in `.forgeplan/trash/`. Works for delete (recreates row + moves projection back), supersede (resets status + drops link), and deprecate (resets status). Refuses if a different artifact with the same ID currently exists. TTL default: 30 days from the destructive op |
| [`forgeplan undo-last`](/docs/cli/undo-last/) | Reverse the most recent destructive operation (delete, supersede, or deprecate) by reading the soft-delete trash and applying restore to the most recently written non-consumed receipt. If no matching receipt is found, returns an error with guidance; the tool never guesses |
| [`forgeplan claim`](/docs/cli/claim/) | Claim an artifact (PRD-057 multi-agent coordination - soft signal "I'm working on this") |
| [`forgeplan claims`](/docs/cli/claims/) | List active claims (sorted by expiry, soonest first) |
| [`forgeplan release`](/docs/cli/release/) | Release a claim (PRD-057). Idempotent - missing claim = success |
| [`forgeplan release-notes`](/docs/cli/release-notes/) | Generate Keep-a-Changelog-shaped release notes from artifacts that changed between two git refs. Walks `git log` over `.forgeplan/{prds,problems,evidence,rfcs,adrs,specs,epics,solutions}/` and categorises each touched artifact: PRD→Added, PROB→Fixed, EVID-on-security→Security, RFC/ADR→Changed. Quality gate: only artifacts with `status==active` or `r_eff_score > 0` are emitted (override with `--draft`) |
| [`forgeplan setup`](/docs/cli/setup/) | One-time per-machine preparation: `fpl` alias + embedding model |
| [`forgeplan dispatch`](/docs/cli/dispatch/) | Compute a parallel-safe work plan for N sub-agents (PRD-057 dispatcher) |
| [`forgeplan anomalies`](/docs/cli/anomalies/) | Issue #289 - detect pipeline anomalies (stuck drafts, orphan links, mis-typed based_on, etc.). Each entry carries severity + tier + suggested_resolution so orchestrators can dispatch fixes |
| [`forgeplan phase`](/docs/cli/phase/) | Read advisory phase state for an artifact. Returns current_phase, workflow_type, timestamps, and the full append-only transition history from `.forgeplan/state/<id>.yaml`. If no state file exists yet (pre-PRD-056 artifact or phase tracking was disabled), returns `current_phase: unknown` -- never an error. Phase tracking is advisory and never blocks other tools |
| [`forgeplan phase-advance`](/docs/cli/phase-advance/) | Manually advance (or set) the advisory phase marker for an artifact. Appends a transition to the history. Does NOT validate phase ordering -- advisory layer allows out-of-order jumps (e.g. direct `done` override). Full phase enforcement lands in a later PRD under EPIC-005. Use when auto-advancement missed a transition or when reclassifying workflow state |
| [`forgeplan ci-assign-id`](/docs/cli/ci-assign-id/) | PROB-060 Phase 0b - atomic CI assigner of `assigned_number`. Walks `.forgeplan/**/*.md` in `--head`, finds candidates with `assigned_number: null`, computes `next = max(assigned_number)+1` per kind from `--base` git ref, rewrites frontmatter (no file rename - Phase 2.1). LanceDB-free per ADR-003. Wrapped in production by `.github/workflows/assign-id.yml` `concurrency: forgeplan-id-assign` |
| [`forgeplan migrate-dry-run`](/docs/cli/migrate-dry-run/) | PROB-060 Phase 0b - EVID-C migration dry-run. Scans all artifacts in `.forgeplan/`, computes the slug each would receive under SPEC-005 rules, and detects per-kind collisions before Phase 4 migration. Read-only - never mutates `.md` files |
| [`forgeplan migrate-secrets`](/docs/cli/migrate-secrets/) | PRD-077 FR-023 Part B - import known API-key env vars (GEMINI_API_KEY, OPENAI_API_KEY, ANTHROPIC_API_KEY) from the process environment into `.forgeplan/secrets.env` so keys live project-local instead of in your shell rc. Default is dry-run; pass `--apply` to write |
| [`forgeplan reconcile-ids`](/docs/cli/reconcile-ids/) | PROB-060 Phase 2.4 (W2.C) - manual cleanup tool for post-merge identity drift. Scans `.forgeplan/<kind>/*.md`, detects four drift categories (filename mismatch, missing `predicted_number`, body-links drift, duplicate `assigned_number`), and either reports (`--check-only`) or auto-fixes the safe categories. LanceDB is never touched; run `forgeplan scan-import` afterwards if the index needs to be rebuilt |
| [`forgeplan mcp`](/docs/cli/mcp/) | MCP integration helpers (install config for AI agent clients) |
| [`forgeplan playbook`](/docs/cli/playbook/) | Playbook runtime (PRD-065 / SPEC-003) - declarative YAML orchestration with delegations to external plugins |
| [`forgeplan ingest`](/docs/cli/ingest/) | Ingest engine (PRD-066 / SPEC-004) - apply mapping YAML to plugin outputs, generate forge artifacts with file:line source refs |
| [`forgeplan plugins`](/docs/cli/plugins/) | Plugin detection + recommendations (PRD-067) - list installed plugins, doctor health-check, info on specific plugin |

