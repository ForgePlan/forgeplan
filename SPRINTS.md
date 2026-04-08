# Sprint Roadmap — ForgePlan

> Бери задачу из текущего спринта в новый чат. Следуй forge-cycle для каждой.
> Обновляй чекбоксы по мере завершения.

---

## Sprint 11: Vision + Architecture Foundation

**Цель:** установить стратегический north star (EPIC-002) и архитектурную основу (RFC-001) для v2.0.
**Depth:** Deep (архитектурные решения, необратимо)
**Ветка из:** `dev`

### Tasks

- [x] **11.1** Shape EPIC-002 "ForgePlan v2.0 — Knowledge OS for AI Agent Teams"
  - Заполнить Vision, Goals, Children (PRD list), Success Criteria
  - `forgeplan get EPIC-002` → fill MUST sections → `forgeplan validate EPIC-002`
  - Branch: `docs/epic-002-shape` — PR #128 merged

- [x] **11.2** Shape + implement RFC-001 "FPF Engine core module architecture"
  - RFC-001 shaped: 3 options, ADI confirmed Option C (Layered Core+Ext)
  - fpf/core/ module: config.rs, trust.rs, adi.rs, model.rs (34 tests)
  - FpfConfig wired into CLI: score, fgr, context, dashboard read from config.yaml
  - Audit: 3 agents, 3 HIGH + 1 MEDIUM fixed, NaN validation added
  - R_eff=1.00, F-G-R=0.94 (A), EVID-055 linked
  - Remaining: 1.5 (migrate reason to auto-save AdiRecord) → Sprint 12
  - Branch: `feat/rfc-001-fpf-engine` — PR #131

- [x] **11.3** CI/CD Architecture Linter (PRD-034)
  - PRD-034 shaped + validated + activated (R_eff=0.90)
  - `forgeplan health --ci --fail-on "orphans=10,blind_spots=5"` — exit 1 on threshold breach
  - `forgeplan validate --ci` — exit 1 on MUST errors in active+stale artifacts
  - GitHub Actions: `.github/workflows/forgeplan-health.yml`
  - 2-agent audit: 2 HIGH fixed (stale filter, scan-import)
  - Branch: `feat/ci-linter` — PR #132

- [x] **11.4** Housekeeping
  - Link 12 orphans → EPIC-002 / PRD-026
  - Удалены 3 untracked PNG из корня
  - `forgeplan health` → 0 orphans ✓

### Definition of Done
- [x] EPIC-002 active, filled, validated
- [x] RFC-001 active, R_eff=1.00, Phase 1 implemented (6/7)
- [x] CI workflow `forgeplan-health.yml` in PR #132
- [x] 0 orphans in `forgeplan health`

---

## Sprint 12: Graph Intelligence + Agent Memory

**Цель:** расширить возможности graph queries и заложить foundation для agent memory.
**Depth:** Deep
**Ветка из:** `dev`

### Tasks

- [ ] **12.1** Shape + implement RFC-002 "Graph Intelligence"
  - petgraph traversal, smart search, gap detection, tree UI
  - `forgeplan get RFC-002` → fill → validate → reason (ADI)
  - Код: `crates/forgeplan-core/src/graph/` — расширение
  - Branch: `feat/rfc-002-graph-intelligence`

- [ ] **12.2** Shape NOTE-025 → PRD "Agent Memory Engine"
  - Превратить идею в PRD: forgeplan как structured memory backend для AI agents
  - `forgeplan new prd "Agent Memory Engine"` из NOTE-025
  - Fill MUST sections, validate, ADI
  - Branch: `docs/prd-agent-memory`

- [ ] **12.3** Website deploy to forgeplan.dev
  - GitHub Pages или Vercel
  - CI: auto-deploy on merge to main
  - Branch: `feat/website-deploy`

- [ ] **12.4** Version bump: Cargo.toml 0.14.0 → 0.16.0
  - Синхронизировать workspace version с git tags
  - Branch: `chore/version-bump`

### Definition of Done
- [ ] RFC-002 active, R_eff > 0, graph queries enhanced
- [ ] PRD for Agent Memory shaped + validated
- [ ] Website live at forgeplan.dev
- [ ] Cargo.toml version matches release tags

---

## Sprint 13: v0.17.0 Release Series (EPIC-003)

**Цель:** Ship v0.17.0 — Search, Discovery, Intelligence.
**Depth:** Deep
**Branch:** `release/v0.17.0` (integration) ← `feat/sprint-13.x-*` feature branches

### Tasks (sequential execution pattern)

- [x] **13.0** Security hotfix — vite CVEs #4-6 (PR #144)
- [x] **13.1** PRD-043 Methodology Integrity — duplicate guard, stub detection, health (PR #145)
- [x] **13.1.5** Hardening — 7 audit findings fix + IntegrityConfig (PR #146)
- [x] **13.1.6** Audit followup — 5 H/M fixes + lesson learned in CLAUDE.md (PR #147)
- [x] **13.1.7** IntegrityConfig wiring fix — open_store validates config (PR #148)
- [x] **13.2** PRD-039 Smart Search v2 — BM25 + Filter DSL + Graph Expansion (PR #149)
- [x] **13.3** PRD-035 p1 — Tags system + Source Tier (PR #150)
  - FR-001..003: tags in frontmatter + CLI tag/untag/list --tag
  - FR-008: SourceTier → CL mapping (T1→CL3, T2→CL2, T3→CL1)
  - Multi-agent audit (4 auditors + 7 fixers) — 2 CRITICAL + 5 HIGH resolved
  - Release-blocker fixed: migration v3→v4 via NewColumnTransform::AllNulls
  - EVID-060 linked, R_eff 0.90, PRD-035 activated
  - PROB-026 (deferred M/L) + PROB-027 (reindex-from-zero) tracked
- [ ] **13.4** PRD-035 p2 — Discover MCP tools + CLI command (FR-004..007)
  - Depends on: 13.3 merge ✓
  - Files: mcp/server.rs (3 new tools), cli/commands/discover.rs (NEW)
- [ ] **13.5** PRD-040 — Scoring Intelligence (adaptive routing, R_eff CI)
- [ ] **13.6** PRD-041 — FPF Rules CLI/MCP (rules list, check)
- [ ] **13.7** PRD-042 — FPF KB Vector Search via EmbedDriver
- [ ] **Final** `/forge-cycle` audit on release/v0.17.0 → merge to main → tag v0.17.0

### Deferred from original Sprint 13 plan

The original Sprint 13 plan (CLI Polish + Agent Memory + generate-docs + Brownfield Discovery) was reorganized around EPIC-003 v0.17.0. Original items moved:

- CLI UX Polish (NOTE-029) → future sprint
- Agent Memory Engine → future sprint
- generate-docs command → future sprint
- Brownfield Discovery → **split into PRD-035 p1 (13.3) + p2 (13.4)** ✓

### Definition of Done
- [ ] `forgeplan links`, `forgeplan doctor` working
- [ ] Agent Memory Phase 1: store + recall via MCP
- [ ] `forgeplan generate-docs` produces useful output

---

## Sprint 14: Advanced Features + v2.0 RC

**Цель:** продвинутые фичи из NOTE-030, подготовка к v2.0 release.
**Depth:** Standard → Deep
**Ветка из:** `dev`

### Tasks

- [ ] **14.1** Watch v2 — file watcher for auto-index
  - `forgeplan watch` — monitor `.forgeplan/` and auto-rebuild LanceDB
  - Based on RFC-004 (files-first architecture)
  - Branch: `feat/watch-v2`

- [ ] **14.2** Diff command
  - `forgeplan diff PRD-001` — show changes since last activation
  - Git-aware: compare with last commit
  - Branch: `feat/diff-command`

- [ ] **14.3** Dashboard (TUI)
  - `forgeplan dashboard` — terminal UI with health, artifacts, graph
  - Consider: ratatui or similar
  - Branch: `feat/dashboard-tui`

- [ ] **14.4** Nx Monorepo evaluation (PRD-025)
  - Re-evaluate: нужен ли Nx к этому моменту?
  - Если да — implement. Если нет — deprecate PRD-025
  - Branch: `feat/nx-monorepo` or `docs/prd-025-deprecate`

- [ ] **14.5** Release v2.0.0
  - EPIC-002 fully activated
  - All Sprint 11-14 evidence created
  - Release branch: `release/v2.0.0`

### Definition of Done
- [ ] v2.0.0 released
- [ ] EPIC-002 activated with full evidence
- [ ] All new commands documented (EN + RU)

---

## Методология checklist (что ещё не доделано по /forge)

Эти пункты нужно закрыть в рамках спринтов выше:

| # | Что | Когда | Статус |
|---|---|---|---|
| 1 | EPIC-002 заполнить + активировать | Sprint 11 | [x] PR #128 |
| 2 | RFC-001 Phase 1 реализовать | Sprint 11 | [x] PR #131 |
| 2b | RFC-001 Phase 2 (rule engine, KB search, contexts) | Sprint 12 | [ ] |
| 3 | RFC-002 реализовать | Sprint 12 | [ ] |
| 4 | NOTE-025 → PRD (Agent Memory) | Sprint 12 | [ ] |
| 5 | NOTE-026 → PRD + implement (CI Linter) | Sprint 11 | [x] PR #132 |
| 5b | PROB-022 → PRD (Brownfield Discover) | Sprint 13 | [ ] |
| 6 | NOTE-029 → implement (CLI UX) | Sprint 13 | [ ] |
| 7 | NOTE-030 → partial implement (generate-docs, watch, diff, dashboard) | Sprint 13-14 | [ ] |
| 8 | PRD-025 (Nx) → evaluate or deprecate | Sprint 14 | [ ] |
| 9 | Version sync (Cargo.toml vs tags) | Sprint 12 | [ ] |
| 10 | Website deploy | Sprint 12 | [ ] |
| 11 | 0 orphans в health | Sprint 11 | [x] PR #129-130 |
| 12 | Dependabot #3 (lru) — monitor for upstream fix | Ongoing | [ ] |

---

## Как работать со спринтами

```bash
# 1. Новый чат → восстанови контекст
memory_recall("ForgePlan")
forgeplan health
cat SPRINTS.md | head -60   # текущий спринт

# 2. Выбери задачу из текущего спринта
forgeplan route "описание задачи"

# 3. Следуй forge-cycle
# Route → Shape → Validate → ADI → Code → Test → Audit → Evidence → Activate → PR

# 4. Отметь в SPRINTS.md: [ ] → [x]
# 5. memory_retain("Sprint 11: task X done, ...")
```
