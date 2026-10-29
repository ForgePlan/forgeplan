---
title: "Справочник CLI"
description: "Команды и вложенные команды выбранной сборки Forgeplan."
---

Снимок интерфейса: `forgeplan 0.39.0+constellation.s1.1`. **84 корневых команд; 120 путей команд всего**.

Это инвентарь CLI help, не заявление о полной runtime-приёмке. Ограничения новой раскладки описаны в [Constellation reference](/ru/docs/reference/constellation/).

## Корневые команды

| Команда | Назначение |
|---|---|
| [`forgeplan activity`](/ru/docs/cli/activity/) | Query the activity log — append-only JSONL record of every MCP tool invocation at .forgeplan/logs/tools-YYYY-MM-DD.jsonl. Use this to reconstruct what the agent did over a time window, attribute LLM-token spend, or audit destructive operations |
| [`forgeplan activity-stats`](/ru/docs/cli/activity-stats/) | Aggregate statistics from the activity log grouped by tool name: count, error count, p50/p95 duration, total time. Use to attribute LLM-token spend and identify slow tools |
| [`forgeplan restore`](/ru/docs/cli/restore/) | Restore a soft-deleted artifact from the most recent non-consumed receipt in `.forgeplan/trash/`. Works for delete (recreates row + moves projection back), supersede (resets status + drops link), and deprecate (resets status). Refuses if a different artifact with the same ID currently exists. TTL default: 30 days from the destructive op |
| [`forgeplan undo-last`](/ru/docs/cli/undo-last/) | Reverse the most recent destructive operation (delete, supersede, or deprecate) by reading the soft-delete trash and applying restore to the most recently written non-consumed receipt. If no matching receipt is found, returns an error with guidance; the tool never guesses |
| [`forgeplan init`](/ru/docs/cli/init/) | Initialize a new .forgeplan/ workspace |
| [`forgeplan new`](/ru/docs/cli/new/) | Create a new artifact from template |
| [`forgeplan list`](/ru/docs/cli/list/) | List artifacts |
| [`forgeplan status`](/ru/docs/cli/status/) | Show project status dashboard |
| [`forgeplan tag`](/ru/docs/cli/tag/) | Add tags to an artifact |
| [`forgeplan untag`](/ru/docs/cli/untag/) | Remove tags from an artifact |
| [`forgeplan discover`](/ru/docs/cli/discover/) | Start brownfield discovery — creates session, prints protocol for agent |
| [`forgeplan validate`](/ru/docs/cli/validate/) | Validate artifact completeness against schema rules |
| [`forgeplan score`](/ru/docs/cli/score/) | Compute R_eff quality score for decisions with evidence |
| [`forgeplan estimate`](/ru/docs/cli/estimate/) | Estimate effort for an artifact based on FR and Phase items |
| [`forgeplan link`](/ru/docs/cli/link/) | Link two artifacts with a typed relationship |
| [`forgeplan unlink`](/ru/docs/cli/unlink/) | Remove a relation between two artifacts |
| [`forgeplan graph`](/ru/docs/cli/graph/) | Generate mermaid dependency graph of linked artifacts |
| [`forgeplan search`](/ru/docs/cli/search/) | Search artifacts (smart by default: keyword + semantic + boosters) |
| [`forgeplan stale`](/ru/docs/cli/stale/) | Detect stale artifacts with expired valid_until |
| [`forgeplan session`](/ru/docs/cli/session/) | Show methodology session state (current phase, active artifact) |
| [`forgeplan progress`](/ru/docs/cli/progress/) | Show checkbox progress for artifacts |
| [`forgeplan claim`](/ru/docs/cli/claim/) | Claim an artifact (PRD-057 multi-agent coordination — soft signal "I'm working on this") |
| [`forgeplan claims`](/ru/docs/cli/claims/) | List active claims (sorted by expiry, soonest first) |
| [`forgeplan decay`](/ru/docs/cli/decay/) | Show evidence decay impact on R_eff scores |
| [`forgeplan calibrate-estimate`](/ru/docs/cli/calibrate-estimate/) | Compare estimated vs actual hours — calibrate estimation accuracy |
| [`forgeplan calibrate`](/ru/docs/cli/calibrate/) | Suggest depth level (Tactical/Standard/Deep/Critical) based on artifact content |
| [`forgeplan promote`](/ru/docs/cli/promote/) | Promote a memory to a full artifact (e.g., forgeplan promote mem-xxx --kind prd) |
| [`forgeplan generate`](/ru/docs/cli/generate/) | Generate an artifact using AI from a natural language description |
| [`forgeplan reason`](/ru/docs/cli/reason/) | Analyze an artifact using FPF ADI reasoning cycle (Abduction→Deduction→Induction) |
| [`forgeplan decompose`](/ru/docs/cli/decompose/) | Decompose a PRD into RFC tasks using AI |
| [`forgeplan context`](/ru/docs/cli/context/) | Single-call reasoning context — artifact + graph + validation + scoring |
| [`forgeplan get`](/ru/docs/cli/get/) | Read a full artifact by ID |
| [`forgeplan update`](/ru/docs/cli/update/) | Update artifact metadata or body |
| [`forgeplan delete`](/ru/docs/cli/delete/) | Delete an artifact |
| [`forgeplan route`](/ru/docs/cli/route/) | Suggest depth level and artifact pipeline for a task description |
| [`forgeplan review`](/ru/docs/cli/review/) | Review an artifact — run validation and show lifecycle checklist |
| [`forgeplan activate`](/ru/docs/cli/activate/) | Activate an artifact (draft → active) with validation gate |
| [`forgeplan supersede`](/ru/docs/cli/supersede/) | Supersede an artifact (active → superseded) with replacement link |
| [`forgeplan deprecate`](/ru/docs/cli/deprecate/) | Deprecate an artifact (active/stale → deprecated) with reason |
| [`forgeplan release`](/ru/docs/cli/release/) | Release a claim (PRD-057). Idempotent — missing claim = success |
| [`forgeplan release-notes`](/ru/docs/cli/release-notes/) | Generate Keep-a-Changelog–shaped release notes from artifacts that changed between two git refs. Walks `git log` over `.forgeplan/{prds,problems,evidence,rfcs,adrs,specs,epics,solutions}/` and categorises each touched artifact: PRD→Added, PROB→Fixed, EVID-on-security→Security, RFC/ADR→Changed. Quality gate: only artifacts with `status==active` or `r_eff_score &gt; 0` are emitted (override with `--draft`) |
| [`forgeplan renew`](/ru/docs/cli/renew/) | Renew a stale artifact (stale → active) with extended validity |
| [`forgeplan reopen`](/ru/docs/cli/reopen/) | Reopen an artifact — creates a NEW draft artifact, deprecates the old one |
| [`forgeplan setup-skill`](/ru/docs/cli/setup-skill/) | Install /forge skill for Claude Code |
| [`forgeplan setup`](/ru/docs/cli/setup/) | One-time per-machine preparation: `fpl` alias + embedding model |
| [`forgeplan fpf`](/ru/docs/cli/fpf/) | FPF Knowledge Base — dashboard, ingest, search, sections |
| [`forgeplan gaps`](/ru/docs/cli/gaps/) | Show pipeline compliance gaps by depth |
| [`forgeplan fgr`](/ru/docs/cli/fgr/) | Show F-G-R quality scores (Formality, Granularity, Reliability) |
| [`forgeplan scan`](/ru/docs/cli/scan/) | Scan codebase for source modules |
| [`forgeplan coverage`](/ru/docs/cli/coverage/) | Show decision coverage per code module |
| [`forgeplan dispatch`](/ru/docs/cli/dispatch/) | Compute a parallel-safe work plan for N sub-agents (PRD-057 dispatcher) |
| [`forgeplan drift`](/ru/docs/cli/drift/) | Check for drifted decisions (affected files changed after decision) |
| [`forgeplan blocked`](/ru/docs/cli/blocked/) | Show blocked artifacts and their dependencies |
| [`forgeplan blindspots`](/ru/docs/cli/blindspots/) | Show blind spots — decisions without evidence, orphan artifacts |
| [`forgeplan anomalies`](/ru/docs/cli/anomalies/) | Issue #289 — detect pipeline anomalies (stuck drafts, orphan links, mis-typed based_on, etc.). Each entry carries severity + tier + suggested_resolution so orchestrators can dispatch fixes |
| [`forgeplan journal`](/ru/docs/cli/journal/) | Show decision journal — chronological timeline with R_eff scores |
| [`forgeplan health`](/ru/docs/cli/health/) | Show project health dashboard — gaps, risks, blind spots, next actions |
| [`forgeplan capture`](/ru/docs/cli/capture/) | Capture a decision from conversation into a Note or ADR artifact |
| [`forgeplan export`](/ru/docs/cli/export/) | Export all artifacts to JSON file |
| [`forgeplan import`](/ru/docs/cli/import/) | Import artifacts from JSON file |
| [`forgeplan scan-import`](/ru/docs/cli/scan-import/) | Scan for existing docs and import as artifacts |
| [`forgeplan tree`](/ru/docs/cli/tree/) | Show artifact hierarchy as ASCII tree |
| [`forgeplan order`](/ru/docs/cli/order/) | Show artifacts in topological order (dependency order) |
| [`forgeplan phase`](/ru/docs/cli/phase/) | Read advisory phase state for an artifact. Returns current_phase, workflow_type, timestamps, and the full append-only transition history from `.forgeplan/state/&lt;id&gt;.yaml`. If no state file exists yet (pre-PRD-056 artifact or phase tracking was disabled), returns `current_phase: unknown` -- never an error. Phase tracking is advisory and never blocks other tools |
| [`forgeplan phase-advance`](/ru/docs/cli/phase-advance/) | Manually advance (or set) the advisory phase marker for an artifact. Appends a transition to the history. Does NOT validate phase ordering -- advisory layer allows out-of-order jumps (e.g. direct `done` override). Full phase enforcement lands in a later PRD under EPIC-005. Use when auto-advancement missed a transition or when reclassifying workflow state |
| [`forgeplan migrate`](/ru/docs/cli/migrate/) | Run schema migrations on existing workspace |
| [`forgeplan ci-assign-id`](/ru/docs/cli/ci-assign-id/) | PROB-060 Phase 0b — atomic CI assigner of `assigned_number`. Walks `.forgeplan/**/*.md` in `--head`, finds candidates with `assigned_number: null`, computes `next = max(assigned_number)+1` per kind from `--base` git ref, rewrites frontmatter (no file rename — Phase 2.1). LanceDB-free per ADR-003. Wrapped in production by `.github/workflows/assign-id.yml` `concurrency: forgeplan-id-assign` |
| [`forgeplan migrate-dry-run`](/ru/docs/cli/migrate-dry-run/) | PROB-060 Phase 0b — EVID-C migration dry-run. Scans all artifacts in `.forgeplan/`, computes the slug each would receive under SPEC-005 rules, and detects per-kind collisions before Phase 4 migration. Read-only — never mutates `.md` files |
| [`forgeplan migrate-secrets`](/ru/docs/cli/migrate-secrets/) | PRD-077 FR-023 Part B — import known API-key env vars (GEMINI_API_KEY, OPENAI_API_KEY, ANTHROPIC_API_KEY) from the process environment into `.forgeplan/secrets.env` so keys live project-local instead of in your shell rc. Default is dry-run; pass `--apply` to write |
| [`forgeplan reconcile-ids`](/ru/docs/cli/reconcile-ids/) | PROB-060 Phase 2.4 (W2.C) — manual cleanup tool for post-merge identity drift. Scans `.forgeplan/&lt;kind&gt;/*.md`, detects four drift categories (filename mismatch, missing `predicted_number`, body-links drift, duplicate `assigned_number`), and either reports (`--check-only`) or auto-fixes the safe categories. LanceDB is never touched; run `forgeplan scan-import` afterwards if the index needs to be rebuilt |
| [`forgeplan reindex`](/ru/docs/cli/reindex/) | Rebuild LanceDB index from .md files (files-first sync) |
| [`forgeplan index`](/ru/docs/cli/index-command/) | Run one Constellation index maintenance pass (capture → reconcile → publish, verified), optionally draining the embedding queue |
| [`forgeplan embed`](/ru/docs/cli/embed/) | Generate embeddings for all artifacts (semantic search) |
| [`forgeplan log`](/ru/docs/cli/log/) | Show change log — audit trail of artifact mutations |
| [`forgeplan remember`](/ru/docs/cli/remember/) | Save a memory (fact, convention, procedure) for later recall |
| [`forgeplan recall`](/ru/docs/cli/recall/) | Recall memories — search, filter, list |
| [`forgeplan watch`](/ru/docs/cli/watch/) | Watch .forgeplan/ files and sync changes to LanceDB in real time |
| [`forgeplan git-sync`](/ru/docs/cli/git-sync/) | Sync artifact changes from git operations (pull/merge) into LanceDB |
| [`forgeplan serve`](/ru/docs/cli/serve/) | Start MCP server (stdio transport) for AI agent integration |
| [`forgeplan mcp`](/ru/docs/cli/mcp/) | MCP integration helpers (install config for AI agent clients) |
| [`forgeplan playbook`](/ru/docs/cli/playbook/) | Playbook runtime (PRD-065 / SPEC-003) — declarative YAML orchestration with delegations to external plugins |
| [`forgeplan ingest`](/ru/docs/cli/ingest/) | Ingest engine (PRD-066 / SPEC-004) — apply mapping YAML to plugin outputs, generate forge artifacts with file:line source refs |
| [`forgeplan plugins`](/ru/docs/cli/plugins/) | Plugin detection + recommendations (PRD-067) — list installed plugins, doctor health-check, info on specific plugin |
| [`forgeplan constellation`](/ru/docs/cli/constellation/) | Constellation registry — named sets of `.forgeplan` sources that `search`/`get` fan out over. The registry lives in `~/.forgeplan/constellations.yaml`: per-user, shared by every ForgePlan instance on the machine |

## Вложенные команды

| Команда | Назначение |
|---|---|
| [`forgeplan discover start`](/ru/docs/cli/discover-start/) | Start a new discovery session — prints protocol for AI agent |
| [`forgeplan discover list`](/ru/docs/cli/discover-list/) | List all discovery sessions in the workspace |
| [`forgeplan discover show`](/ru/docs/cli/discover-show/) | Show status of a discovery session |
| [`forgeplan discover complete`](/ru/docs/cli/discover-complete/) | Mark a discovery session as completed |
| [`forgeplan fpf dashboard`](/ru/docs/cli/fpf-dashboard/) | Show FPF dashboard — bounded contexts, quality scores, explore-exploit actions |
| [`forgeplan fpf ingest`](/ru/docs/cli/fpf-ingest/) | Ingest FPF spec into knowledge base |
| [`forgeplan fpf search`](/ru/docs/cli/fpf-search/) | Search FPF knowledge base |
| [`forgeplan fpf section`](/ru/docs/cli/fpf-section/) | Show a specific FPF section |
| [`forgeplan fpf list`](/ru/docs/cli/fpf-list/) | List all FPF sections |
| [`forgeplan fpf status`](/ru/docs/cli/fpf-status/) | Show FPF knowledge base status — source, ingested count, staleness |
| [`forgeplan fpf rules`](/ru/docs/cli/fpf-rules/) | List active FPF rules grouped by action (EXPLORE/INVESTIGATE/EXPLOIT) |
| [`forgeplan fpf check`](/ru/docs/cli/fpf-check/) | Check which FPF rules match a given artifact |
| [`forgeplan mcp serve`](/ru/docs/cli/mcp-serve/) | Start MCP server (alias for `forgeplan serve`) |
| [`forgeplan mcp install`](/ru/docs/cli/mcp-install/) | Install forgeplan MCP config into a client (Claude / Cursor / Windsurf) |
| [`forgeplan playbook list`](/ru/docs/cli/playbook-list/) | List available playbooks (built-in + installed packs) with applicable recommendations based on project signals |
| [`forgeplan playbook show`](/ru/docs/cli/playbook-show/) | Show playbook structure (steps, delegations, requires) |
| [`forgeplan playbook validate`](/ru/docs/cli/playbook-validate/) | Validate playbook YAML against SPEC-003 schema |
| [`forgeplan playbook run`](/ru/docs/cli/playbook-run/) | Run a playbook — sequential step execution with delegations |
| [`forgeplan plugins list`](/ru/docs/cli/plugins-list/) | List installed plugins (Claude / agent-skills / Cursor / Forgeplan) |
| [`forgeplan plugins doctor`](/ru/docs/cli/plugins-doctor/) | Health check across known plugins — reports missing, outdated, OK |
| [`forgeplan plugins info`](/ru/docs/cli/plugins-info/) | Show details for a specific plugin (path, version, description) |
| [`forgeplan constellation create`](/ru/docs/cli/constellation-create/) | Create a named constellation. The first one created becomes active |
| [`forgeplan constellation add`](/ru/docs/cli/constellation-add/) | Add a `.forgeplan` directory as a source. Validates the path at registration time — a symlink or a non-workspace fails here, not at every later read |
| [`forgeplan constellation remove`](/ru/docs/cli/constellation-remove/) | Remove a source from a constellation |
| [`forgeplan constellation rebind`](/ru/docs/cli/constellation-rebind/) | Rebind a source after its checkout moved. Source identity is path-derived, so a moved checkout is a new source: this rewrites the registry path and, by default, the index's path-bound marker so `forgeplan index` answers at the new location. References qualified by the old `src:` id stop resolving — that is the contract |
| [`forgeplan constellation list`](/ru/docs/cli/constellation-list/) | List all constellations and which is active |
| [`forgeplan constellation use`](/ru/docs/cli/constellation-use/) | Select the active constellation for `search`/`get` |
| [`forgeplan constellation sources`](/ru/docs/cli/constellation-sources/) | List the sources of a constellation (default: active) |
| [`forgeplan constellation doctor`](/ru/docs/cli/constellation-doctor/) | Health-check every registered source: reachable, looks like a workspace, has a published index |
| [`forgeplan constellation snapshot`](/ru/docs/cli/constellation-snapshot/) | Pin every source of the active constellation at its current Git HEAD and write the lockfile to `~/.forgeplan/snapshots/`. Sources that are not git checkouts are reported, not pinned |
| [`forgeplan constellation search`](/ru/docs/cli/constellation-search/) | Search across the active constellation's sources |
| [`forgeplan constellation get`](/ru/docs/cli/constellation-get/) | Fetch one artifact by id across the active constellation. When two sources claim the same id the answer is `ambiguous` and names them |
| [`forgeplan constellation pack`](/ru/docs/cli/constellation-pack/) | Stage 6 — build a budgeted ContextPack for a task: lexical ranking, evidence units with roles, omissions and a spend receipt |
| [`forgeplan constellation graph`](/ru/docs/cli/constellation-graph/) | The relation graph of the active constellation (or a recorded snapshot): every artifact as a source-qualified node, every declared link as a typed edge with its provenance |
| [`forgeplan constellation impact`](/ru/docs/cli/constellation-impact/) | Stage 7 — explainable impact: who depends on this artifact, by which relation path, and where the bounded walk stopped |
| [`forgeplan constellation export`](/ru/docs/cli/constellation-export/) | Stage 9 — portable export: every document and edge of the active constellation (or a recorded snapshot) as JSONL or SCIP-shaped records, with provenance and no absolute local paths |
