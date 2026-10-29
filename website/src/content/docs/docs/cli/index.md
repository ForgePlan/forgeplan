---
title: "CLI Reference"
description: "Commands and nested commands in the selected Forgeplan build."
---

Interface snapshot: `forgeplan 0.39.0+constellation.s1.1`. **84 top-level commands; 120 command paths in total**.

This is an inventory of CLI help, not a full runtime acceptance claim. New-layout limitations are documented in the [Constellation reference](/docs/reference/constellation/).

## Top-level commands

| Command | Purpose |
|---|---|
| [`forgeplan activity`](/docs/cli/activity/) | Query the activity log — append-only JSONL record of every MCP tool invocation at .forgeplan/logs/tools-YYYY-MM-DD.jsonl. Use this to reconstruct what the agent did over a time window, attribute LLM-token spend, or audit destructive operations |
| [`forgeplan activity-stats`](/docs/cli/activity-stats/) | Aggregate statistics from the activity log grouped by tool name: count, error count, p50/p95 duration, total time. Use to attribute LLM-token spend and identify slow tools |
| [`forgeplan restore`](/docs/cli/restore/) | Restore a soft-deleted artifact from the most recent non-consumed receipt in `.forgeplan/trash/`. Works for delete (recreates row + moves projection back), supersede (resets status + drops link), and deprecate (resets status). Refuses if a different artifact with the same ID currently exists. TTL default: 30 days from the destructive op |
| [`forgeplan undo-last`](/docs/cli/undo-last/) | Reverse the most recent destructive operation (delete, supersede, or deprecate) by reading the soft-delete trash and applying restore to the most recently written non-consumed receipt. If no matching receipt is found, returns an error with guidance; the tool never guesses |
| [`forgeplan init`](/docs/cli/init/) | Initialize a new .forgeplan/ workspace |
| [`forgeplan new`](/docs/cli/new/) | Create a new artifact from template |
| [`forgeplan list`](/docs/cli/list/) | List artifacts |
| [`forgeplan status`](/docs/cli/status/) | Show project status dashboard |
| [`forgeplan tag`](/docs/cli/tag/) | Add tags to an artifact |
| [`forgeplan untag`](/docs/cli/untag/) | Remove tags from an artifact |
| [`forgeplan discover`](/docs/cli/discover/) | Start brownfield discovery — creates session, prints protocol for agent |
| [`forgeplan validate`](/docs/cli/validate/) | Validate artifact completeness against schema rules |
| [`forgeplan score`](/docs/cli/score/) | Compute R_eff quality score for decisions with evidence |
| [`forgeplan estimate`](/docs/cli/estimate/) | Estimate effort for an artifact based on FR and Phase items |
| [`forgeplan link`](/docs/cli/link/) | Link two artifacts with a typed relationship |
| [`forgeplan unlink`](/docs/cli/unlink/) | Remove a relation between two artifacts |
| [`forgeplan graph`](/docs/cli/graph/) | Generate mermaid dependency graph of linked artifacts |
| [`forgeplan search`](/docs/cli/search/) | Search artifacts (smart by default: keyword + semantic + boosters) |
| [`forgeplan stale`](/docs/cli/stale/) | Detect stale artifacts with expired valid_until |
| [`forgeplan session`](/docs/cli/session/) | Show methodology session state (current phase, active artifact) |
| [`forgeplan progress`](/docs/cli/progress/) | Show checkbox progress for artifacts |
| [`forgeplan claim`](/docs/cli/claim/) | Claim an artifact (PRD-057 multi-agent coordination — soft signal "I'm working on this") |
| [`forgeplan claims`](/docs/cli/claims/) | List active claims (sorted by expiry, soonest first) |
| [`forgeplan decay`](/docs/cli/decay/) | Show evidence decay impact on R_eff scores |
| [`forgeplan calibrate-estimate`](/docs/cli/calibrate-estimate/) | Compare estimated vs actual hours — calibrate estimation accuracy |
| [`forgeplan calibrate`](/docs/cli/calibrate/) | Suggest depth level (Tactical/Standard/Deep/Critical) based on artifact content |
| [`forgeplan promote`](/docs/cli/promote/) | Promote a memory to a full artifact (e.g., forgeplan promote mem-xxx --kind prd) |
| [`forgeplan generate`](/docs/cli/generate/) | Generate an artifact using AI from a natural language description |
| [`forgeplan reason`](/docs/cli/reason/) | Analyze an artifact using FPF ADI reasoning cycle (Abduction→Deduction→Induction) |
| [`forgeplan decompose`](/docs/cli/decompose/) | Decompose a PRD into RFC tasks using AI |
| [`forgeplan context`](/docs/cli/context/) | Single-call reasoning context — artifact + graph + validation + scoring |
| [`forgeplan get`](/docs/cli/get/) | Read a full artifact by ID |
| [`forgeplan update`](/docs/cli/update/) | Update artifact metadata or body |
| [`forgeplan delete`](/docs/cli/delete/) | Delete an artifact |
| [`forgeplan route`](/docs/cli/route/) | Suggest depth level and artifact pipeline for a task description |
| [`forgeplan review`](/docs/cli/review/) | Review an artifact — run validation and show lifecycle checklist |
| [`forgeplan activate`](/docs/cli/activate/) | Activate an artifact (draft → active) with validation gate |
| [`forgeplan supersede`](/docs/cli/supersede/) | Supersede an artifact (active → superseded) with replacement link |
| [`forgeplan deprecate`](/docs/cli/deprecate/) | Deprecate an artifact (active/stale → deprecated) with reason |
| [`forgeplan release`](/docs/cli/release/) | Release a claim (PRD-057). Idempotent — missing claim = success |
| [`forgeplan release-notes`](/docs/cli/release-notes/) | Generate Keep-a-Changelog–shaped release notes from artifacts that changed between two git refs. Walks `git log` over `.forgeplan/{prds,problems,evidence,rfcs,adrs,specs,epics,solutions}/` and categorises each touched artifact: PRD→Added, PROB→Fixed, EVID-on-security→Security, RFC/ADR→Changed. Quality gate: only artifacts with `status==active` or `r_eff_score &gt; 0` are emitted (override with `--draft`) |
| [`forgeplan renew`](/docs/cli/renew/) | Renew a stale artifact (stale → active) with extended validity |
| [`forgeplan reopen`](/docs/cli/reopen/) | Reopen an artifact — creates a NEW draft artifact, deprecates the old one |
| [`forgeplan setup-skill`](/docs/cli/setup-skill/) | Install /forge skill for Claude Code |
| [`forgeplan setup`](/docs/cli/setup/) | One-time per-machine preparation: `fpl` alias + embedding model |
| [`forgeplan fpf`](/docs/cli/fpf/) | FPF Knowledge Base — dashboard, ingest, search, sections |
| [`forgeplan gaps`](/docs/cli/gaps/) | Show pipeline compliance gaps by depth |
| [`forgeplan fgr`](/docs/cli/fgr/) | Show F-G-R quality scores (Formality, Granularity, Reliability) |
| [`forgeplan scan`](/docs/cli/scan/) | Scan codebase for source modules |
| [`forgeplan coverage`](/docs/cli/coverage/) | Show decision coverage per code module |
| [`forgeplan dispatch`](/docs/cli/dispatch/) | Compute a parallel-safe work plan for N sub-agents (PRD-057 dispatcher) |
| [`forgeplan drift`](/docs/cli/drift/) | Check for drifted decisions (affected files changed after decision) |
| [`forgeplan blocked`](/docs/cli/blocked/) | Show blocked artifacts and their dependencies |
| [`forgeplan blindspots`](/docs/cli/blindspots/) | Show blind spots — decisions without evidence, orphan artifacts |
| [`forgeplan anomalies`](/docs/cli/anomalies/) | Issue #289 — detect pipeline anomalies (stuck drafts, orphan links, mis-typed based_on, etc.). Each entry carries severity + tier + suggested_resolution so orchestrators can dispatch fixes |
| [`forgeplan journal`](/docs/cli/journal/) | Show decision journal — chronological timeline with R_eff scores |
| [`forgeplan health`](/docs/cli/health/) | Show project health dashboard — gaps, risks, blind spots, next actions |
| [`forgeplan capture`](/docs/cli/capture/) | Capture a decision from conversation into a Note or ADR artifact |
| [`forgeplan export`](/docs/cli/export/) | Export all artifacts to JSON file |
| [`forgeplan import`](/docs/cli/import/) | Import artifacts from JSON file |
| [`forgeplan scan-import`](/docs/cli/scan-import/) | Scan for existing docs and import as artifacts |
| [`forgeplan tree`](/docs/cli/tree/) | Show artifact hierarchy as ASCII tree |
| [`forgeplan order`](/docs/cli/order/) | Show artifacts in topological order (dependency order) |
| [`forgeplan phase`](/docs/cli/phase/) | Read advisory phase state for an artifact. Returns current_phase, workflow_type, timestamps, and the full append-only transition history from `.forgeplan/state/&lt;id&gt;.yaml`. If no state file exists yet (pre-PRD-056 artifact or phase tracking was disabled), returns `current_phase: unknown` -- never an error. Phase tracking is advisory and never blocks other tools |
| [`forgeplan phase-advance`](/docs/cli/phase-advance/) | Manually advance (or set) the advisory phase marker for an artifact. Appends a transition to the history. Does NOT validate phase ordering -- advisory layer allows out-of-order jumps (e.g. direct `done` override). Full phase enforcement lands in a later PRD under EPIC-005. Use when auto-advancement missed a transition or when reclassifying workflow state |
| [`forgeplan migrate`](/docs/cli/migrate/) | Run schema migrations on existing workspace |
| [`forgeplan ci-assign-id`](/docs/cli/ci-assign-id/) | PROB-060 Phase 0b — atomic CI assigner of `assigned_number`. Walks `.forgeplan/**/*.md` in `--head`, finds candidates with `assigned_number: null`, computes `next = max(assigned_number)+1` per kind from `--base` git ref, rewrites frontmatter (no file rename — Phase 2.1). LanceDB-free per ADR-003. Wrapped in production by `.github/workflows/assign-id.yml` `concurrency: forgeplan-id-assign` |
| [`forgeplan migrate-dry-run`](/docs/cli/migrate-dry-run/) | PROB-060 Phase 0b — EVID-C migration dry-run. Scans all artifacts in `.forgeplan/`, computes the slug each would receive under SPEC-005 rules, and detects per-kind collisions before Phase 4 migration. Read-only — never mutates `.md` files |
| [`forgeplan migrate-secrets`](/docs/cli/migrate-secrets/) | PRD-077 FR-023 Part B — import known API-key env vars (GEMINI_API_KEY, OPENAI_API_KEY, ANTHROPIC_API_KEY) from the process environment into `.forgeplan/secrets.env` so keys live project-local instead of in your shell rc. Default is dry-run; pass `--apply` to write |
| [`forgeplan reconcile-ids`](/docs/cli/reconcile-ids/) | PROB-060 Phase 2.4 (W2.C) — manual cleanup tool for post-merge identity drift. Scans `.forgeplan/&lt;kind&gt;/*.md`, detects four drift categories (filename mismatch, missing `predicted_number`, body-links drift, duplicate `assigned_number`), and either reports (`--check-only`) or auto-fixes the safe categories. LanceDB is never touched; run `forgeplan scan-import` afterwards if the index needs to be rebuilt |
| [`forgeplan reindex`](/docs/cli/reindex/) | Rebuild LanceDB index from .md files (files-first sync) |
| [`forgeplan index`](/docs/cli/index-command/) | Run one Constellation index maintenance pass (capture → reconcile → publish, verified), optionally draining the embedding queue |
| [`forgeplan embed`](/docs/cli/embed/) | Generate embeddings for all artifacts (semantic search) |
| [`forgeplan log`](/docs/cli/log/) | Show change log — audit trail of artifact mutations |
| [`forgeplan remember`](/docs/cli/remember/) | Save a memory (fact, convention, procedure) for later recall |
| [`forgeplan recall`](/docs/cli/recall/) | Recall memories — search, filter, list |
| [`forgeplan watch`](/docs/cli/watch/) | Watch .forgeplan/ files and sync changes to LanceDB in real time |
| [`forgeplan git-sync`](/docs/cli/git-sync/) | Sync artifact changes from git operations (pull/merge) into LanceDB |
| [`forgeplan serve`](/docs/cli/serve/) | Start MCP server (stdio transport) for AI agent integration |
| [`forgeplan mcp`](/docs/cli/mcp/) | MCP integration helpers (install config for AI agent clients) |
| [`forgeplan playbook`](/docs/cli/playbook/) | Playbook runtime (PRD-065 / SPEC-003) — declarative YAML orchestration with delegations to external plugins |
| [`forgeplan ingest`](/docs/cli/ingest/) | Ingest engine (PRD-066 / SPEC-004) — apply mapping YAML to plugin outputs, generate forge artifacts with file:line source refs |
| [`forgeplan plugins`](/docs/cli/plugins/) | Plugin detection + recommendations (PRD-067) — list installed plugins, doctor health-check, info on specific plugin |
| [`forgeplan constellation`](/docs/cli/constellation/) | Constellation registry — named sets of `.forgeplan` sources that `search`/`get` fan out over. The registry lives in `~/.forgeplan/constellations.yaml`: per-user, shared by every ForgePlan instance on the machine |

## Nested commands

| Command | Purpose |
|---|---|
| [`forgeplan discover start`](/docs/cli/discover-start/) | Start a new discovery session — prints protocol for AI agent |
| [`forgeplan discover list`](/docs/cli/discover-list/) | List all discovery sessions in the workspace |
| [`forgeplan discover show`](/docs/cli/discover-show/) | Show status of a discovery session |
| [`forgeplan discover complete`](/docs/cli/discover-complete/) | Mark a discovery session as completed |
| [`forgeplan fpf dashboard`](/docs/cli/fpf-dashboard/) | Show FPF dashboard — bounded contexts, quality scores, explore-exploit actions |
| [`forgeplan fpf ingest`](/docs/cli/fpf-ingest/) | Ingest FPF spec into knowledge base |
| [`forgeplan fpf search`](/docs/cli/fpf-search/) | Search FPF knowledge base |
| [`forgeplan fpf section`](/docs/cli/fpf-section/) | Show a specific FPF section |
| [`forgeplan fpf list`](/docs/cli/fpf-list/) | List all FPF sections |
| [`forgeplan fpf status`](/docs/cli/fpf-status/) | Show FPF knowledge base status — source, ingested count, staleness |
| [`forgeplan fpf rules`](/docs/cli/fpf-rules/) | List active FPF rules grouped by action (EXPLORE/INVESTIGATE/EXPLOIT) |
| [`forgeplan fpf check`](/docs/cli/fpf-check/) | Check which FPF rules match a given artifact |
| [`forgeplan mcp serve`](/docs/cli/mcp-serve/) | Start MCP server (alias for `forgeplan serve`) |
| [`forgeplan mcp install`](/docs/cli/mcp-install/) | Install forgeplan MCP config into a client (Claude / Cursor / Windsurf) |
| [`forgeplan playbook list`](/docs/cli/playbook-list/) | List available playbooks (built-in + installed packs) with applicable recommendations based on project signals |
| [`forgeplan playbook show`](/docs/cli/playbook-show/) | Show playbook structure (steps, delegations, requires) |
| [`forgeplan playbook validate`](/docs/cli/playbook-validate/) | Validate playbook YAML against SPEC-003 schema |
| [`forgeplan playbook run`](/docs/cli/playbook-run/) | Run a playbook — sequential step execution with delegations |
| [`forgeplan plugins list`](/docs/cli/plugins-list/) | List installed plugins (Claude / agent-skills / Cursor / Forgeplan) |
| [`forgeplan plugins doctor`](/docs/cli/plugins-doctor/) | Health check across known plugins — reports missing, outdated, OK |
| [`forgeplan plugins info`](/docs/cli/plugins-info/) | Show details for a specific plugin (path, version, description) |
| [`forgeplan constellation create`](/docs/cli/constellation-create/) | Create a named constellation. The first one created becomes active |
| [`forgeplan constellation add`](/docs/cli/constellation-add/) | Add a `.forgeplan` directory as a source. Validates the path at registration time — a symlink or a non-workspace fails here, not at every later read |
| [`forgeplan constellation remove`](/docs/cli/constellation-remove/) | Remove a source from a constellation |
| [`forgeplan constellation rebind`](/docs/cli/constellation-rebind/) | Rebind a source after its checkout moved. Source identity is path-derived, so a moved checkout is a new source: this rewrites the registry path and, by default, the index's path-bound marker so `forgeplan index` answers at the new location. References qualified by the old `src:` id stop resolving — that is the contract |
| [`forgeplan constellation list`](/docs/cli/constellation-list/) | List all constellations and which is active |
| [`forgeplan constellation use`](/docs/cli/constellation-use/) | Select the active constellation for `search`/`get` |
| [`forgeplan constellation sources`](/docs/cli/constellation-sources/) | List the sources of a constellation (default: active) |
| [`forgeplan constellation doctor`](/docs/cli/constellation-doctor/) | Health-check every registered source: reachable, looks like a workspace, has a published index |
| [`forgeplan constellation snapshot`](/docs/cli/constellation-snapshot/) | Pin every source of the active constellation at its current Git HEAD and write the lockfile to `~/.forgeplan/snapshots/`. Sources that are not git checkouts are reported, not pinned |
| [`forgeplan constellation search`](/docs/cli/constellation-search/) | Search across the active constellation's sources |
| [`forgeplan constellation get`](/docs/cli/constellation-get/) | Fetch one artifact by id across the active constellation. When two sources claim the same id the answer is `ambiguous` and names them |
| [`forgeplan constellation pack`](/docs/cli/constellation-pack/) | Stage 6 — build a budgeted ContextPack for a task: lexical ranking, evidence units with roles, omissions and a spend receipt |
| [`forgeplan constellation graph`](/docs/cli/constellation-graph/) | The relation graph of the active constellation (or a recorded snapshot): every artifact as a source-qualified node, every declared link as a typed edge with its provenance |
| [`forgeplan constellation impact`](/docs/cli/constellation-impact/) | Stage 7 — explainable impact: who depends on this artifact, by which relation path, and where the bounded walk stopped |
| [`forgeplan constellation export`](/docs/cli/constellation-export/) | Stage 9 — portable export: every document and edge of the active constellation (or a recorded snapshot) as JSONL or SCIP-shaped records, with provenance and no absolute local paths |
