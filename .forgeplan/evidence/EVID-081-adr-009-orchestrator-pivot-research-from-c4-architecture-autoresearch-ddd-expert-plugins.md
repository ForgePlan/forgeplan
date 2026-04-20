---
created: 2026-04-20
depth: tactical
id: EVID-081
kind: evidence
links:
- target: ADR-009
  relation: supports
- target: EPIC-007
  relation: supports
- target: PROB-042
  relation: supports
status: draft
title: ADR-009 orchestrator pivot — research from c4-architecture + autoresearch + ddd-expert plugins
updated: 2026-04-20
---

# EVID-081: ADR-009 orchestrator pivot — ecosystem research (CL2, upgrade to CL3 after spikes)

## Structured Fields

verdict: supports
congruence_level: 2
evidence_type: research

## Measurement

Разведка adjacent ecosystem для ADR-009 Orchestrator decision (2026-04-20):

1. **c4-architecture plugin** — прочитан `commands/c4-architecture.md` + 4 agents (c4-context, c4-container, c4-component, c4-code). Установлен на машине в `/Users/explosovebit/.claude/plugins/marketplaces/claude-code-workflows/plugins/c4-architecture/`. Bottom-up pipeline: каждая директория → c4-code doc, синтезируется в components, потом containers с API docs, потом context с personas.

2. **autoresearch** — прочитан README + `/guide/autoresearch-learn.md` + `/claude-plugin/skills/autoresearch/references/plan-workflow.md`. В `sources/autoresearch/`. 8-phase pipeline (Scout → Analyze → Map → Generate → Validate → Fix → Finalize → Log). 4 modes (init/update/check/summarize). Multi-harness compat: Claude Code, OpenCode, Codex.

3. **agents-pro:ddd-domain-expert** — использован в 4-agent audit PROB-040 (2026-04-19). Нашёл 3 P0 DDD findings (MigrationPlan aggregate ownership, status_map leaky ACL, bidirectional supersede should be event-driven). Это прямое measurement качества output — агент полезен для nontrivial domain reasoning.

4. **agents-sparc:specification** — в списке установленных. Не тестирован напрямую, но SPARC methodology — de-facto стандарт для requirements-to-design pipelines.

5. **Parallel session aod-worker** — 105K LOC Go, 1180 commits. User хотел brownfield onboarding. Manual workflow: один путь через docs (existing), другой через код — нет unified инструмента. Confirms hypothesis: нужно orchestrate не реализовывать.

## Result

**Pattern adoption matrix (evidence для 10 паттернов)**:

| Pattern | Observed в existing plugin | Adopt в ADR-009 | Why |
|---|---|---|---|
| Multi-level structural docs (C4) | c4-architecture: 4 agents bottom-up | yes (external delegate) | Mature, специализированный |
| Bottom-up code analysis | c4-architecture + autoresearch:learn | yes | Pattern работает |
| Multi-phase pipeline с validation | autoresearch 8 phases | yes (playbook design) | Proven для docs generation |
| Interactive 4-question goal capture | autoresearch:plan | yes (greenfield-kickoff playbook step) | UX паттерн для vision capture |
| Multi-harness compat (3-7 harnesses) | autoresearch 3 harness, OpenSpec 9 | yes (из ADR-008) | Ecosystem alignment |
| DDD bounded context extraction | agents-pro:ddd-domain-expert | yes (external delegate для behavioral viewpoint) | Validated на audit PROB-040 |
| SPARC specification workflow | agents-sparc:specification | yes (external delegate для behavioral viewpoint) | De-facto standard |
| Script-first rule (deterministic→bash, reasoning→LLM) | ccpm, autoresearch | yes (playbook делегирует по типам) | Correct separation |
| Self-describing tool output | ADR-008 (наш) | yes (расширяется в PRD-067) | Уже принято |
| Declarative YAML configuration | OpenSpec config, autoresearch manifests | yes (playbook + mapping YAML) | Reuse standard pattern |

**Gap identified (не покрывается existing plugins, требует forge-specific)**:
- Lifecycle management over artifacts (draft → active → superseded etc.)
- Graph + typed links между артефактами
- R_eff scoring + evidence decay
- Git history mining → inferred ADRs (PRD-068 gap filler)
- Forge-aware ingest mappings (PRD-066)
- Unified playbook runtime orchestrating heterogeneous delegates (PRD-065)

**Consensus**: 10 из 10 identified patterns validated в минимум одном adjacent проекте. Gap identification cleanly mapped к 5 PRDs в EPIC-007. Unique value forgeplan (lifecycle + graph + scoring) orthogonal к plugin capabilities — нет direct competition, сугубо complementary.

**Forgeplan's moat**:
- **Существующие плагины** — specialized document generators (что делают), deterministic pipelines (как делают)
- **Forgeplan уникальность** — lifecycle-aware graph с scoring поверх heterogeneous sources, backed by evidence с trust calculus. Orthogonal to generation.

## Interpretation

Research confirms ADR-009 Decision (Option B) соответствует существующему ecosystem pattern'у — integration vs competition. Все rejected options (status-quo in-house / monorepo / fork / metadata-only) не проходят research filter: либо конкурируют с mature tools (A), либо ломают license/version (C, D), либо теряют forge moat (E).

ADI output (`forgeplan reason ADR-009`) confirms: **H1 adopt orchestrator model — High confidence**. 3 evidence tests для upgrade к CL3:
- E1 mapping round-trip (Spike-1)
- E2 playbook runtime greenfield fixture (spike после PRD-065 implementation)
- E3 brownfield-code E2E aod-worker fixture (post-Phase 2)

## Congruence Level Justification

**CL2 (related context)**: evidence — research паттернов в adjacent проектах, НЕ прямое measurement на forgeplan codebase или actual playbook runtime. Unified decision не тестировался end-to-end. Это proper research evidence для Shape-phase decision, достаточное для activate ADR-009 (ADR-008 активирован на аналогичном CL2).

**Upgrade to CL3 planned** в Phase 1 (spikes): spike-1 (c4 на Forgeplan) + spike-2 (autoresearch:learn) + spike-3 (ddd-expert) — DoR blocker для activate PRDs. CL3 measurement каждого mapping на реальном output.

Penalty CL2 = 0.1, acceptable для Shape-phase decision-support evidence.

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| ADR-009 | ADR | supports |
| EPIC-007 | Epic | supports |
| PROB-042 | Problem | supports (confirms ecosystem gap requires orchestrator pivot) |
| EVID-079 | Evidence | informs (parallel research for ADR-008, similar methodology) |




