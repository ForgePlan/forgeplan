---
depth: standard
id: PRD-041
kind: prd
links:
- target: EPIC-003
  relation: refines
status: active
title: FPF Rules — CLI list/check + MCP tools (RFC-001 Phase 3)
---

---
id: PRD-041
title: "FPF Rules — CLI list/check + MCP tools (RFC-001 Phase 3)"
status: Draft
author: gogocat
created: 2026-04-07
updated: 2026-04-07
priority: P1
depth: standard
parent_epic: EPIC-003
---

# PRD-041: FPF Rules — CLI list/check + MCP tools (RFC-001 Phase 3)

## Progress

```
FR-001  ████████████████████████  1/1  forgeplan fpf rules    ✓ Sprint 13.6
FR-002  ████████████████████████  1/1  forgeplan fpf check    ✓ Sprint 13.6
FR-003  ████████████████████████  1/1  MCP fpf_rules tool     ✓ Sprint 13.6
FR-004  ████████████████████████  1/1  MCP fpf_check tool     ✓ Sprint 13.6
─────────────────────────────────────────────────
TOTAL                              4/4  (100%) — COMPLETE
```

## Implementation map (FR → file:line → test)

| FR | Surface | Implementation | Tests |
|---|---|---|---|
| FR-001 | `forgeplan fpf rules [--flat] [--json]` | `crates/forgeplan-cli/src/commands/fpf.rs::run_rules()` + `crates/forgeplan-cli/src/main.rs::FpfCommands::Rules` | `cli_fpf_rules_shows_default_source`, `cli_fpf_rules_json_valid`, `cli_fpf_rules_flat_has_priorities` (tests/fpf_rules_check.rs) + `summarize_condition` unit tests |
| FR-002 | `forgeplan fpf check <id> [--verbose] [--json]` | `crates/forgeplan-cli/src/commands/fpf.rs::run_check()` + `crates/forgeplan-cli/src/main.rs::FpfCommands::Check` | `cli_fpf_check_missing_artifact_errors`, `cli_fpf_check_existing_artifact`, `cli_fpf_check_verbose_shows_unmatched`, `cli_fpf_check_json_has_required_fields` |
| FR-003 | MCP `forgeplan_fpf_rules` (params: action/name/summary/source) | `crates/forgeplan-mcp/src/server.rs::forgeplan_fpf_rules()` | `fpf_param_validation_tests` (rules bounds) + core `active_rules` tests in `forgeplan-core/src/fpf/mod.rs::prd041_tests` |
| FR-004 | MCP `forgeplan_fpf_check` (param: id) | `crates/forgeplan-mcp/src/server.rs::forgeplan_fpf_check()` | `fpf_param_validation_tests` (id bound) + core `check_artifact_against_rules` tests (missing id, custom config, canonical serialize, summary_line variants) |

### Core API (shared by CLI + MCP)
- `forgeplan_core::fpf::RuleSource` — Config | Default
- `forgeplan_core::fpf::active_rules(fpf_config) -> (Vec<Rule>, RuleSource)`
- `forgeplan_core::fpf::RuleCheckResult { artifact_id, artifact_kind, artifact_status, matched, unmatched, winning }` + `Serialize` (canonical JSON: `kind`/`status`) + `summary_line()` method
- `forgeplan_core::fpf::check_artifact_against_rules(store, id, config) -> Result<Option<RuleCheckResult>>`
- `forgeplan_core::fpf::ext::rules::Condition::summarize()` — human-readable one-liner for rules surface
- Private helpers: `build_lookup_maps`, `enrich_one` (shared between `build_rule_actions` and `check_artifact_against_rules`, O(N+R))

**Sprint 13.6 delivered:** All 4 FRs implemented, audited (0 CRITICAL, 0 HIGH remaining after fixes), re-audited READY TO MERGE. 1075 tests pass, E2E regression 12/12 pass on release binary.

See EVID-063 for full evidence.

---

## Executive Summary

### Vision

FPF Rule Engine (реализован в Sprint 12, PR #133+#135) становится доступен через CLI и MCP — пользователь и AI-агент могут увидеть активные правила и проверить артефакт против них без чтения config.yaml.

### Problem

В Sprint 12 реализован полноценный rule engine: 5 default rules, configurable через config.yaml, graph-aware, time-aware, ~600 LOC, 38 тестов. Но **нет surface для interaction**:

- Нельзя посмотреть активные правила (`forgeplan fpf rules` не существует)
- Нельзя проверить артефакт против rules без полного `forgeplan score` или `dashboard`
- AI-агент через MCP не имеет инструмента для rule introspection
- Rule engine используется только внутри dashboard и health — невидим как самостоятельный механизм

**Impact:** rule engine — это hidden feature. Пользователь не знает что rules существуют, AI не может их использовать целенаправленно.

### Target Users

| Персона | Боль |
|---------|------|
| AI-агент (MCP) | Не может узнать "какие правила применяются к этому артефакту?" |
| Разработчик (CLI) | Должен читать config.yaml чтобы понять активные rules |

### Differentiators

- Использует **существующий** rule engine из `core/fpf/ext/rules.rs` — 0 нового кода в core
- Surface-only PRD: только CLI commands + MCP tool definitions
- Закрывает **deferred задачу из Sprint 12** (RFC-001 Phase 3)

---

## Success Criteria

| ID | Criterion | Metric | Target |
|----|-----------|--------|--------|
| SC-1 | CLI показывает активные rules | `forgeplan fpf rules` output | Lists all rules with name, condition, action |
| SC-2 | CLI проверяет артефакт против rules | `forgeplan fpf check PRD-039` | Shows triggered/passed rules |
| SC-3 | MCP tools доступны | tools registered | `forgeplan_fpf_rules`, `forgeplan_fpf_check` in MCP registry |

---

## Product Scope

### MVP (In-Scope)

- `forgeplan fpf rules` — list active rules from FpfConfig
- `forgeplan fpf rules --json` — machine-readable output
- `forgeplan fpf check <id>` — run rule engine against artifact, show triggered rules
- `forgeplan fpf check <id> --json` — JSON output
- `forgeplan_fpf_rules` MCP tool
- `forgeplan_fpf_check` MCP tool

### Out of Scope

- Создание новых rules через CLI (только через config.yaml)
- Rule editor / TUI
- Rule import/export / sharing
- Rule versioning
- Rule conflict resolution UI

### Growth Vision

- `forgeplan fpf rule add/remove/edit` для interactive management
- Rule presets (security, performance, methodology)
- Rule effectiveness analytics

---

## User Journeys

### Journey 1: AI-агент проверяет артефакт через MCP

| Шаг | Действие | Ответ |
|-----|----------|-------|
| 1 | `forgeplan_fpf_rules()` | JSON: список rules с name, condition, action |
| 2 | `forgeplan_fpf_check(id: "PRD-039")` | JSON: triggered + passed rules + score |
| 3 | Агент видит что сработало → решает action | — |

### Journey 2: Разработчик дебажит rule engine

| Шаг | Действие | Ответ |
|-----|----------|-------|
| 1 | `forgeplan fpf rules` | Table: 5 rules с описанием и source (default/config) |
| 2 | Видит wrong параметры в одном rule | — |
| 3 | Правит config.yaml, снова `forgeplan fpf rules` | Видит обновлённые параметры |
| 4 | `forgeplan fpf check PRD-018` | Видит что rule больше не срабатывает |

---

## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | CLI | Must | [User] can list all active FPF rules with `forgeplan fpf rules`, seeing name, condition, action, source (default/config) | Journey 2 |
| FR-002 | CLI | Must | [User] can check a specific artifact against rules with `forgeplan fpf check <id>`, seeing triggered and passed rules | Journey 2 |
| FR-003 | MCP | Must | [AI agent] can call `forgeplan_fpf_rules` MCP tool to get JSON list of active rules | Journey 1 |
| FR-004 | MCP | Must | [AI agent] can call `forgeplan_fpf_check` MCP tool to evaluate artifact and get structured response | Journey 1 |

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric |
|----|----------|-------------|--------|
| NFR-001 | Performance | Rule check shall complete | < 50ms per artifact |
| NFR-002 | Compatibility | No changes to existing rule engine API | 0 breaking changes |
| NFR-003 | Coverage | New CLI/MCP code shall have unit tests | 100% pub fn covered |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation |
|----|------|-------------|--------|------------|
| R-1 | MCP tool naming conflicts | Low | Low | Prefix `forgeplan_fpf_*` уже established |
| R-2 | JSON output format diverges от CLI | Med | Low | Shared types между CLI и MCP |

---

## Affected Files

- `crates/forgeplan-cli/src/commands/fpf.rs` — extend с rules + check subcommands
- `crates/forgeplan-cli/src/main.rs` — register subcommands
- `crates/forgeplan-mcp/src/server.rs` — add 2 MCP tools (`forgeplan_fpf_rules`, `forgeplan_fpf_check`)
- `crates/forgeplan-mcp/src/types.rs` — request/response types

**Note on rules.rs:** API уже public — все необходимые функции доступны:
- `pub fn run_rules(rules, data) -> Option<SuggestedAction>` (rules.rs:330)
- `pub fn check_basic(rule, data) -> bool` (rules.rs:246)
- `pub fn check_enriched(rule, data) -> bool` (rules.rs:291)
- `pub fn default_rules() -> Vec<Rule>` (rules.rs:371)
- `pub struct Rule, EnrichedData, ArtifactData` (rules.rs:20, 228)

CLI/MCP просто читают `FpfConfig.rules` и вызывают `run_rules()`. **0 LOC в core**.

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-003 | parent epic | draft |
| RFC-001 | parent (FPF Engine) | active |
| EVID-057 | source evidence (Sprint 12 rule engine) | active |
| sources/RuVector | inspiration (filter expressions) | external |


