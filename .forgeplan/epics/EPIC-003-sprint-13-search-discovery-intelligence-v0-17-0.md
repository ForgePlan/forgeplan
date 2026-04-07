---
depth: deep
id: EPIC-003
kind: epic
status: draft
title: Sprint 13 — Search, Discovery, Intelligence (v0.17.0)
---

---
id: EPIC-003
title: "Sprint 13 — Search, Discovery, Intelligence (v0.17.0)"
status: Draft
author: gogocat
created: 2026-04-07
updated: 2026-04-07
priority: P0
depth: deep
---

# EPIC-003: Sprint 13 — Search, Discovery, Intelligence (v0.17.0)

## Progress

```
PRD-039  ░░░░░░░░░░░░░░░░░░░░░░░░  0/3   Smart Search v2
PRD-035  ░░░░░░░░░░░░░░░░░░░░░░░░  0/13  Brownfield Discovery
PRD-040  ░░░░░░░░░░░░░░░░░░░░░░░░  0/2   Scoring Intelligence
PRD-041  ░░░░░░░░░░░░░░░░░░░░░░░░  0/4   FPF Rules CLI/MCP
PRD-042  ░░░░░░░░░░░░░░░░░░░░░░░░  0/3   KB Vector Search
─────────────────────────────────────────────────
TOTAL                              0/25  ( 0%)
```

---

## Vision

Sprint 13 — единый release v0.17.0 объединяющий **5 независимых улучшений** Forgeplan: умный поиск (BM25 + filters + graph), brownfield discovery protocol для AI-агентов, адаптивный routing/scoring, FPF rules CLI/MCP surface, и vector search в FPF knowledge base. Все портированы или вдохновлены анализом RuVector и закрытием deferred задач из Sprint 12.

## Problem

После v0.16.0 (Sprint 12 — FPF Engine v2 Phase 2 + Rule Engine) накопилась группа связанных improvement'ов:

1. **Поиск примитивен** (PRD-039) — substring grep, бинарный score, 2-поля фильтр, нет graph context
2. **Brownfield onboarding ломается** (PRD-035 / PROB-022) — агент идёт в docs/, игнорирует код, нет discovery protocol
3. **Scoring/routing статичны** (PRD-040) — router не учится, R_eff без confidence interval
4. **FPF rule engine невидим** (PRD-041 / RFC-001 Ph3) — есть код, нет CLI/MCP surface
5. **KB search keyword-only** (PRD-042) — semantic search в FPF knowledge.rs deferred с Sprint 12

Каждая проблема small-medium, но вместе формируют **единый release "Search, Discovery, Intelligence"**.

## Goals

| ID | Goal | Метрика |
|----|------|---------|
| G-1 | Smart Search v2 в production | BM25 + 5+ filter fields + 1-hop graph expansion |
| G-2 | Brownfield Discovery v1 готов | discover CLI + 3 MCP tools + tags + tier mapping |
| G-3 | Scoring/Routing адаптивно | Skills Memory + R_eff CI |
| G-4 | FPF Rules через CLI/MCP | rules + check команды + MCP tools |
| G-5 | KB Vector Search работает | EmbedDriver wired в knowledge.rs |
| G-6 | Release v0.17.0 published | cargo-dist 5 platforms + GH release |

## Non-Goals

- Не интегрируем RuVector как зависимость — портируем только паттерны
- Не реализуем full RL pipeline (Q-Learning, DQN)
- Не делаем hyperedges — Links остаются binary
- Не переписываем LanceDB layer
- Не делаем multi-pass discovery (Phase 2 → Sprint 14)

## Target Users

| Персона | Что получает |
|---------|-------------|
| AI-агент (MCP) | Умный поиск, discovery protocol, FPF rules через MCP |
| Разработчик (CLI) | Те же фичи + UX (filters, CI display, tags) |
| Maintainer | Закрыты 5 deferred items, готов v0.17.0 |

## Children PRDs

| PRD | Title | Sprint | LOC |
|-----|-------|--------|-----|
| PRD-039 | Smart Search v2 | 13.1 | ~430 |
| PRD-035 | Brownfield Discovery | 13.4a + 13.4b | ~750 |
| PRD-040 | Scoring & Routing Intelligence | 13.5 | ~130 |
| PRD-041 | FPF Rules CLI/MCP | 13.2 | ~150 |
| PRD-042 | FPF KB Vector Search | 13.3 | ~150 |

## Phases

### Phase 0: Security Hotfix (Sprint 13.0)
- Vite + lru dependency bumps (4 CVE fixes)
- Branch: `feat/sprint-13.0-security`
- No artifact (Tactical hotfix)
- Time: 1-2h

### Phase 1: Smart Search v2 (Sprint 13.1)
- PRD-039: BM25 + Composable Filters + Graph Expansion
- Branch: `feat/sprint-13.1-prd-039-search`
- TeamCreate: 4 waves, ~430 LOC
- Time: 1-1.5d

### Phase 2: Brownfield Discovery Phase 1 (Sprint 13.4a)
- PRD-035 part 1: Tags system + Source tier mapping (FR-001..003 + FR-008)
- Branch: `feat/sprint-13.4a-prd-035-tags`
- TeamCreate: 4 waves, ~250 LOC
- Time: 1.5d

### Phase 3: Brownfield Discovery Phase 2 (Sprint 13.4b)
- PRD-035 part 2: MCP discover tools + CLI command (FR-004..007)
- Branch: `feat/sprint-13.4b-prd-035-discover`
- Depends on Phase 2 merged
- TeamCreate: 4 waves, ~500 LOC
- Time: 2d

### Phase 4: Scoring & Routing Intelligence (Sprint 13.5)
- PRD-040: Skills Memory + R_eff Confidence Intervals
- Branch: `feat/sprint-13.5-prd-040-scoring`
- TeamCreate: 3 waves, ~130 LOC
- Time: 0.5-1d

### Phase 5: FPF Rules CLI/MCP (Sprint 13.2)
- PRD-041: forgeplan fpf rules + check + MCP tools
- Branch: `feat/sprint-13.2-prd-041-fpf-rules`
- TeamCreate: 3 waves, ~150 LOC
- Time: 1d

### Phase 6: FPF KB Vector Search (Sprint 13.3)
- PRD-042: knowledge.rs vector search via EmbedDriver
- Branch: `feat/sprint-13.3-prd-042-kb-search`
- TeamCreate: 2 waves, ~150 LOC
- Time: 0.5-1d

### Phase 7: Final Release Audit (release/v0.17.0)
- Full /forge-cycle audit on release branch
- 4-agent /audit (code, Rust, architect, test coverage)
- Fix all HIGH/CRITICAL findings
- PR release/v0.17.0 → main (merge commit)
- Tag v0.17.0, sync main → dev, cargo-dist release
- Time: 0.5d

## Success Criteria

| ID | Criterion | Metric |
|----|-----------|--------|
| SC-1 | All 5 PRDs activated | R_eff > 0 each |
| SC-2 | Release v0.17.0 published | git tag + 5 platforms |
| SC-3 | Health checks pass | 0 blind spots, 0 stale |
| SC-4 | Tests maintained | 830+ tests, 0 failures |
| SC-5 | Zero new dependencies | Cargo.toml unchanged |

## Branch Strategy

```
release/v0.17.0 (integration branch from dev)
├── feat/sprint-13.0-security
├── feat/sprint-13.1-prd-039-search
├── feat/sprint-13.4a-prd-035-tags
├── feat/sprint-13.4b-prd-035-discover
├── feat/sprint-13.5-prd-040-scoring
├── feat/sprint-13.2-prd-041-fpf-rules
└── feat/sprint-13.3-prd-042-kb-search
       │
       ▼
release/v0.17.0 → main (after final /forge-cycle audit)
main → dev (sync back) + tag v0.17.0
```

Каждый sprint = отдельная feature branch ИЗ release/v0.17.0. После всех — финальный audit на release branch (независимо от per-sprint проверок), потом merge в main.

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation |
|----|------|-------------|--------|------------|
| R-1 | Sprint 13.4a/b большие, не влезут в 1 chat | High | Med | Continuation prompts через /sprint |
| R-2 | Конфликты в search/scoring модулях | Med | Med | File ownership table per sprint |
| R-3 | Final audit находит regressions | Low | High | Per-sprint audit + smoke test |
| R-4 | Discovery protocol design нестабилен | Med | Med | Phase 1 = MVP, итерация в Sprint 14 |

## Affected Files

- `crates/forgeplan-core/src/search/**` — PRD-039
- `crates/forgeplan-core/src/scan/**`, `crates/forgeplan-core/src/discover/**` (NEW) — PRD-035
- `crates/forgeplan-core/src/scoring/**`, `crates/forgeplan-core/src/routing/**` — PRD-040
- `crates/forgeplan-core/src/fpf/**` — PRD-041, PRD-042
- `crates/forgeplan-cli/src/commands/**` — все 5 PRDs
- `crates/forgeplan-mcp/src/server.rs` — все 5 PRDs (MCP tools)

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| PRD-039 | child (Sprint 13.1) | draft |
| PRD-035 | child (Sprint 13.4a/b) | draft |
| PRD-040 | child (Sprint 13.5) | draft |
| PRD-041 | child (Sprint 13.2) | draft |
| PRD-042 | child (Sprint 13.3) | draft |
| RFC-001 | parent context (FPF Engine) | active |
| PROB-022 | source problem (brownfield) | draft |
| EPIC-002 | parent epic (v2.0 vision) | active |
| sources/RuVector | pattern source | external |
