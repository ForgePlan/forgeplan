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

- [ ] **11.2** Shape + implement RFC-001 "FPF Engine core module architecture"
  - Определить: bounded contexts FPF модулей, interfaces, data flow
  - ADI reasoning обязателен (Deep depth)
  - `forgeplan get RFC-001` → fill → `forgeplan validate` → `forgeplan reason`
  - Код: `crates/forgeplan-core/src/fpf/` — refactor по RFC
  - Branch: `feat/rfc-001-fpf-engine`

- [ ] **11.3** Quick win: CI/CD Architecture Linter (NOTE-026)
  - Превратить NOTE-026 → PRD → implement
  - `forgeplan health` и `forgeplan validate` как CI pipeline gates
  - Script: `scripts/ci-forgeplan-check.sh`
  - GitHub Actions: `.github/workflows/forgeplan-health.yml`
  - Branch: `feat/ci-linter`

- [x] **11.4** Housekeeping
  - Link 12 orphans → EPIC-002 / PRD-026
  - Удалены 3 untracked PNG из корня
  - `forgeplan health` → 0 orphans ✓

### Definition of Done
- [x] EPIC-002 active, filled, validated
- [ ] RFC-001 active, R_eff > 0, code implemented
- [ ] CI workflow `forgeplan-health.yml` merged
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

## Sprint 13: CLI Polish + Agent Memory Implementation

**Цель:** CLI UX доводка и начало реализации Agent Memory.
**Depth:** Standard
**Ветка из:** `dev`

### Tasks

- [ ] **13.1** CLI UX Polish (NOTE-029)
  - `forgeplan links` — show all artifact connections
  - `forgeplan doctor` — diagnose workspace issues
  - `--ci` mode — machine-readable output for pipelines
  - Error message consistency across all 33 commands
  - Branch: `feat/cli-ux-polish`

- [ ] **13.2** Agent Memory Engine — Phase 1 implementation
  - Based on PRD from Sprint 12
  - Memory storage, recall, retention for AI agents
  - MCP tools: `forgeplan_memory_*`
  - Branch: `feat/agent-memory-p1`

- [ ] **13.3** generate-docs command (NOTE-030 partial)
  - `forgeplan generate-docs` — auto-generate documentation from artifacts
  - Markdown report: all artifacts, their status, R_eff, links
  - Branch: `feat/generate-docs`

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
| 1 | EPIC-002 заполнить + активировать | Sprint 11 | [ ] |
| 2 | RFC-001 реализовать | Sprint 11 | [ ] |
| 3 | RFC-002 реализовать | Sprint 12 | [ ] |
| 4 | NOTE-025 → PRD (Agent Memory) | Sprint 12 | [ ] |
| 5 | NOTE-026 → PRD + implement (CI Linter) | Sprint 11 | [ ] |
| 6 | NOTE-029 → implement (CLI UX) | Sprint 13 | [ ] |
| 7 | NOTE-030 → partial implement (generate-docs, watch, diff, dashboard) | Sprint 13-14 | [ ] |
| 8 | PRD-025 (Nx) → evaluate or deprecate | Sprint 14 | [ ] |
| 9 | Version sync (Cargo.toml vs tags) | Sprint 12 | [ ] |
| 10 | Website deploy | Sprint 12 | [ ] |
| 11 | 0 orphans в health | Sprint 11 | [ ] |
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
