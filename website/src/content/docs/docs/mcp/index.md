---
title: MCP Tools
description: "Reference for all 63 Model Context Protocol tools exposed by `forgeplan serve`."
---

Forgeplan ships with **71 MCP tools** that an AI agent can call over the Model Context Protocol (stdio transport).

Start the MCP server:

```bash
forgeplan serve
```

Configure your agent (Claude Code, Cursor, etc.) to connect, then invoke any of the tools listed below.

### Workspace & Data

| Tool | Description |
|---|---|
| [`forgeplan_export`](/docs/mcp/forgeplan_export/) | Export all artifacts and relations to a JSON bundle. Returns the exported data directly for programmatic use, or writes to a file path. |
| [`forgeplan_import`](/docs/mcp/forgeplan_import/) | Import artifacts and relations from a JSON export bundle. Set force=true to overwrite existing artifacts. |
| [`forgeplan_init`](/docs/mcp/forgeplan_init/) | Initialize a new .forgeplan/ workspace. Creates LanceDB tables, config, and artifact subdirectories. |

### Creating Artifacts

| Tool | Description |
|---|---|
| [`forgeplan_capture`](/docs/mcp/forgeplan_capture/) | Capture a decision from conversation into a Note or ADR artifact. Auto-detects type: simple decisions become Notes, architectural decisions become ADRs. Requires LLM provider. |
| [`forgeplan_generate`](/docs/mcp/forgeplan_generate/) | Generate an artifact using AI from a natural language description. Requires LLM provider configured in .forgeplan/config.yaml. Supports OpenAI, Claude, Gemini, Ollama, and any OpenAI-compatible endpoint. |
| [`forgeplan_new`](/docs/mcp/forgeplan_new/) | Create a new artifact from template. Generates a sequential ID (e.g., PRD-001), renders the template, stores in LanceDB, and writes a markdown projection. |

### Reading Artifacts

| Tool | Description |
|---|---|
| [`forgeplan_blocked`](/docs/mcp/forgeplan_blocked/) | Show blocked artifacts and their unmet dependencies. Only draft artifacts block — deprecated and superseded are considered resolved. Uses structural relations only (based_on, refines, supersedes, contradicts). |
| [`forgeplan_get`](/docs/mcp/forgeplan_get/) | Read a full artifact by ID. Returns all metadata and body content. |
| [`forgeplan_graph`](/docs/mcp/forgeplan_graph/) | Generate a mermaid dependency graph of all linked artifacts. Includes explicit links and parent_epic belongs_to edges. |
| [`forgeplan_journal`](/docs/mcp/forgeplan_journal/) | Show decision journal — chronological timeline of ADR, Note, Problem, Solution artifacts with R_eff scores and evidence status. |
| [`forgeplan_list`](/docs/mcp/forgeplan_list/) | List artifacts with optional kind/status filters. Returns ID, kind, status, and title for each artifact. |
| [`forgeplan_order`](/docs/mcp/forgeplan_order/) | Show artifacts in topological order (dependency order). Returns ordered list, ready/blocked classification, and cycle detection. Uses structural relations only. |
| [`forgeplan_progress`](/docs/mcp/forgeplan_progress/) | Show checkbox progress for artifacts. Parses markdown checkboxes (- [ ] / - [x]) and computes completion percentages. |
| [`forgeplan_search`](/docs/mcp/forgeplan_search/) | Smart search across artifacts: BM25 keyword + optional semantic + graph expansion. Supports filters by kind/status/depth/evidence/since and graph expansion toggle. |
| [`forgeplan_session`](/docs/mcp/forgeplan_session/) | Show current methodology session state — phase (idle/routing/shaping/coding/evidence/pr), active artifact, depth, enforcement status. Use this to know where in the workflow you are. |

### Editing Artifacts

| Tool | Description |
|---|---|
| [`forgeplan_delete`](/docs/mcp/forgeplan_delete/) | Delete an artifact from LanceDB and remove its markdown projection file. |
| [`forgeplan_link`](/docs/mcp/forgeplan_link/) | Link two artifacts with a typed relationship. Valid types: informs, based_on, supersedes, contradicts, refines. |
| [`forgeplan_update`](/docs/mcp/forgeplan_update/) | Update artifact metadata (status, title) and/or body. Re-renders markdown projection after update. |

### Quality & Validation

| Tool | Description |
|---|---|
| [`forgeplan_calibrate`](/docs/mcp/forgeplan_calibrate/) | Suggest depth level (Tactical/Standard/Deep/Critical) for artifacts based on content analysis. Detects security sections, breaking changes, link count, body complexity. |
| [`forgeplan_coverage`](/docs/mcp/forgeplan_coverage/) | Show decision coverage per code module — which modules have architectural decisions and which are blind spots. |
| [`forgeplan_decay`](/docs/mcp/forgeplan_decay/) | Show evidence decay impact on R_eff scores. Lists artifacts where expired evidence has degraded quality scores, with current vs fresh R_eff comparison. |
| [`forgeplan_drift`](/docs/mcp/forgeplan_drift/) | Check for drifted decisions — affected files that changed after ADR/RFC was created. |
| [`forgeplan_estimate`](/docs/mcp/forgeplan_estimate/) | Estimate effort for an artifact based on FR and Phase items. Returns multi-grade breakdown (Junior/Middle/Senior/Principal/AI) with confidence scoring. |
| [`forgeplan_guard`](/docs/mcp/forgeplan_guard/) | Check if a methodology phase transition is allowed. Use before performing actions to avoid blocked operations. Example: can I go from 'shaping' to 'coding'? Returns allowed=true/false with reason. |
| [`forgeplan_review`](/docs/mcp/forgeplan_review/) | Review an artifact — run validation and show lifecycle checklist. Shows MUST/SHOULD findings and whether artifact can be activated. |
| [`forgeplan_score`](/docs/mcp/forgeplan_score/) | Compute R_eff quality score for an artifact based on linked evidence. R_eff uses the weakest-link principle: score = min(evidence_scores). |
| [`forgeplan_stale`](/docs/mcp/forgeplan_stale/) | Detect stale artifacts with expired valid_until dates. Returns the list of expired artifacts with days since expiry. |
| [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) | Validate artifact completeness against schema rules. Checks required sections per artifact kind and depth level. Returns structured findings with severity (MUST/SHOULD/COULD). |

### Lifecycle

| Tool | Description |
|---|---|
| [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) | Activate an artifact (draft → active). Requires all MUST validation rules to pass. |
| [`forgeplan_deprecate`](/docs/mcp/forgeplan_deprecate/) | Deprecate an artifact (active → deprecated) with a reason. |
| [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) | Supersede an artifact (active → superseded). Creates link to replacement and notifies dependents. |

### Reasoning & AI

| Tool | Description |
|---|---|
| [`forgeplan_decompose`](/docs/mcp/forgeplan_decompose/) | Decompose a PRD into RFC tasks using AI. Analyzes functional requirements and suggests 3-7 RFCs with titles, descriptions, scope, and dependencies. Requires LLM provider. |
| [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) | Analyze an artifact using FPF ADI reasoning cycle: Abduction (3+ hypotheses) → Deduction (evaluate each) → Induction (synthesize recommendation). Requires LLM provider. |
| [`forgeplan_route`](/docs/mcp/forgeplan_route/) | Suggest depth level (Tactical/Standard/Deep/Critical) and artifact pipeline for a task description. Uses LLM classification (Level 1) when API key is configured, falls back to rule-based keywords (Level 0). |

### Dashboards

| Tool | Description |
|---|---|
| [`forgeplan_blindspots`](/docs/mcp/forgeplan_blindspots/) | Show blind spots — decisions (PRD/RFC/ADR/Epic) without linked evidence, and orphan artifacts with no connections. |
| [`forgeplan_health`](/docs/mcp/forgeplan_health/) | Show project health dashboard — gaps, risks, blind spots, orphans, stale evidence, and recommended next actions. No LLM needed. |
| [`forgeplan_status`](/docs/mcp/forgeplan_status/) | Show project status dashboard — total artifacts, counts by kind and status. |

### FPF Knowledge Base

| Tool | Description |
|---|---|
| [`forgeplan_fpf_check`](/docs/mcp/forgeplan_fpf_check/) | Check which FPF rules match a given artifact, showing all matched rules, the winning rule (first in priority order, same as runtime), and rules that did not match. Use this to understand FPF engine behavior for a specific artifact before acting on it. |
| [`forgeplan_fpf_list`](/docs/mcp/forgeplan_fpf_list/) | List all available FPF (First Principles Framework) sections in the knowledge base. |
| [`forgeplan_fpf_rules`](/docs/mcp/forgeplan_fpf_rules/) | List active FPF rules from the workspace. By default returns all rules with full condition trees and messages. Parameters allow filtering: `action` (EXPLORE/INVESTIGATE/EXPLOIT) to show only rules for that action category; `name` to fetch a single rule by name; `summary: true` to return only name/priority/action without condition details (useful for quick overviews); `source` (config/default) for debugging which rule source is active. If workspace has user-defined rules in .forgeplan/config.yaml under fpf.rules, those take precedence; otherwise built-in defaults are returned. |
| [`forgeplan_fpf_search`](/docs/mcp/forgeplan_fpf_search/) | Search FPF (First Principles Framework) knowledge base. Default is keyword search. Pass `semantic: true` for vector similarity search via BGE-M3 embeddings (requires the `semantic-search` build feature). When `semantic: true` but the feature is not compiled in, the query gracefully falls back to keyword search and the response includes a `warning` field. Note: the first invocation with `semantic: true` may take 10–30 seconds if the BGE-M3 model needs to be downloaded (~150MB). Params: query (required, 1..=8192 chars), limit (default 5, max 50), semantic (default false). |
| [`forgeplan_fpf_section`](/docs/mcp/forgeplan_fpf_section/) | Get full content of a specific FPF section by ID (e.g. 'B.3', 'C.2.2', 'A.1'). |

### Brownfield Discovery

| Tool | Description |
|---|---|
| [`forgeplan_discover_complete`](/docs/mcp/forgeplan_discover_complete/) | Complete a discovery session. Generates a summary report with findings per phase/tier, runs forgeplan health, and marks the session as completed. |
| [`forgeplan_discover_finding`](/docs/mcp/forgeplan_discover_finding/) | Report a discovery finding. The agent calls this after analyzing a file/module/git-log during a phase. ForgePlan creates an artifact (note/prd/rfc/problem/evidence) with the finding content, tags it with the source tier, and links it to the discovery session. |
| [`forgeplan_discover_start`](/docs/mcp/forgeplan_discover_start/) | Start a brownfield discovery session. Returns a structured protocol (7 phases: detect/structure/code/git/tests/docs/synthesize) that the AI agent follows to map an existing codebase. ForgePlan provides the protocol; the agent parses code and reports findings via forgeplan_discover_finding. |

