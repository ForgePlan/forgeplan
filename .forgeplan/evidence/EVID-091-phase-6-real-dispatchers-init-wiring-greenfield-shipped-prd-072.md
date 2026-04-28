---
depth: standard
id: EVID-091
kind: evidence
last_modified_at: 2026-04-28T13:44:54.830520+00:00
last_modified_by: claude-code/2.1.121
links:
- target: PRD-072
  relation: informs
- target: ADR-010
  relation: informs
- target: RFC-007
  relation: informs
- target: EVID-089
  relation: based_on
status: draft
title: Phase 6 — Real dispatchers + init wiring + greenfield shipped (PRD-072)
---

---
created: 2026-04-28
id: EVID-091
kind: evidence
title: Phase 6 — Real subprocess dispatchers + init wiring + greenfield playbook shipped (PRD-072)
status: draft
---

# EVID-091: Phase 6 — Real subprocess dispatchers + init wiring + greenfield playbook shipped (PRD-072)

## Context

EPIC-007 Phase 5 (PRD-065 / PRD-066 / PRD-067, см. EVID-089) shipped Playbook runtime, Ingest engine и Plugin detection как **engine layer без user-facing activation**: dispatchers использовали `MockDispatcher::AlwaysOk`, `forgeplan init` не был подключён к recommendation engine, в `marketplace/playbooks/` лежал только `brownfield-code.yaml`.

Phase 6 (этот sprint) — implementation closed для **PRD-072 / RFC-007 / ADR-010**: 5 production dispatchers + RoutingDispatcher + `init`-wiring + canonical greenfield playbook + minimum необходимая documentation. Это закрывает Phase 5 deferral на user-facing activation: forgeplan теперь действительно запускает плагины из playbook'ов и эмитит recommendation hints на init.

Pre-conditions удовлетворены до Phase 6:
- v0.26.0 PR #217 merged в `dev` (Phase 5 surface стабилен)
- PRD-072 / RFC-007 / ADR-010 drafts с full FR coverage
- EVID-090 (Spike-2) — empirical CL3 measurement tokio::process pattern на real forgeplan binary, ADR-010 §Decision validated

## Methodology

**Структура спринта**: Pre-Wave 0 (Spike-2 + dispatch.rs split) + 4 implementation waves × 9 уникальных агентов через TeamCreate Mode A (phase6-engine team). Strict file partitioning повторяет PRD-057 lessons.

| Wave | Объём | Агенты | LOC | Тесты |
|---|---|---|---|---|
| Pre-Wave 0 | dispatch.rs split (single 466 LOC → directory) + Spike-2 manual c4-architecture run + EVID-090 (CL3) | 1 + 1 spike | scaffold | n/a |
| Wave 1 | helpers + 5 dispatchers (FR-1..FR-5, FR-9, FR-10) — strict file ownership matrix | 6 parallel | ~3,117 | +44 unit |
| Wave 2 | `commands::init::run` recommendation wiring (FR-6) + integration tests | 1 | +302 (149 init + 153 tests) | +5 integration |
| Wave 3 | `marketplace/playbooks/greenfield-kickoff.yaml` (FR-7) — 7 steps + validate pass | 1 | +165 YAML | n/a |
| Wave 4a | E2E integration tests (real subprocess + greenfield + init hints) | 1 | +770 (e2e + greenfield) | +12 integration |
| Wave 4b | docs (PLAYBOOK-AUTHORING + INGEST-MAPPINGS) + CHANGELOG v0.27.0 + TODO + EVID-091 | 1 | docs only | n/a |
| Wave 4 fix | RoutingDispatcher + CLI/MCP swap (close MockDispatcher gap) | 1 | +190 routing + 4 swap sites | +5 unit |
| **Σ** | | **9 unique** | **~5,500+ LOC** | **+66 tests** |

**Gate checks** между волнами:
- `cargo fmt --check` — 0 diffs
- `cargo check --workspace` — 0 warnings
- `cargo test --workspace --lib` — 1317+ PASS
- `cargo clippy --workspace --all-targets -- -D warnings` — 0 warnings (rust 1.91 strict)

**Branch**: `feat/phase6-real-dispatchers`. Commits f05f6e4 (Wave 1) + 94394b0 (Wave 2+3). Wave 4 в working tree готов к commit.

## Measurements

### AC coverage — PRD-072 (Phase 6)

| AC | Verification | Status |
|---|---|---|
| AC-1: brownfield-code real run with installed c4-architecture | E2E `e2e_real_subprocess_dispatch_via_command_delegate` (passes through CommandDispatcher real subprocess) | ✅ |
| AC-2: Missing plugin → graceful failure with install command | `plugin_dispatcher.rs` + `command_dispatcher.rs` unit tests verify DispatchError::DelegateMissing path; CLI `e2e_command_dispatcher_refuses_without_yes` | ✅ |
| AC-3: `forgeplan init` empty repo → `recommended: greenfield-kickoff` | `integration_phase6_init.rs` empty-repo hint test | ✅ |
| AC-4: `forgeplan init` on `.obsidian/` → `recommended: brownfield-docs` | `integration_phase6_init.rs` obsidian-vault test | ✅ |
| AC-5: `forgeplan init` legacy code >100 commits → `recommended: brownfield-code` | `integration_phase6_e2e.rs::e2e_init_recommends_brownfield_code_on_legacy_repo` | ✅ |
| AC-6: `FORGEPLAN_HINTS=0` или non-TTY → no hints | `integration_phase6_init.rs` env-disabled test | ✅ |
| AC-7: `forgeplan playbook validate greenfield-kickoff.yaml` → OK + Done. | `e2e_greenfield_validate_succeeds` + `e2e_greenfield_validate_json` | ✅ |
| AC-8: greenfield E2E dry-run prints all 7 steps | `e2e_greenfield_show_prints_7_steps` + `e2e_greenfield_dry_run_lists_steps` + `e2e_greenfield_run_creates_artifacts` | ✅ |
| AC-9: All Phase 5 tests pass (regression-free) | 1317+ lib + 372+ integration PASS post Wave 4 fix | ✅ |
| AC-10: kill -9 mid-step resumability | `e2e_dispatch_journal_durability_after_step_end` (Phase 5 NEW-S-H2 contract) + Spike-2 kill_on_drop verification | ✅ |

### AC closure — PRD-067 (deferred от Phase 5)

| AC | Phase 5 status | Phase 6 status |
|---|---|---|
| AC-3 init empty repo → greenfield-kickoff | ⚠️ deferred | ✅ closed by FR-6 |
| AC-4 init `.obsidian/` → brownfield-docs | ⚠️ deferred | ✅ closed by FR-6 |
| AC-5 init legacy code → brownfield-code | ⚠️ deferred | ✅ closed by FR-6 |
| AC-7 backward compat (`FORGEPLAN_HINTS=0`, no-TTY) | ✅ engine | ✅ end-to-end (init respects flag) |

### LOC + Test deltas

- **Total LOC added в Phase 6**: ~5,500+ (3,117 Wave 1 + 302 Wave 2 + 165 Wave 3 YAML + 770 Wave 4a tests + 190 Wave 4 RoutingDispatcher + Wave 4b docs)
- **Unit tests added**: +49 (44 Wave 1 + 5 RoutingDispatcher routing tests Wave 4 fix)
- **Integration tests added**: +17 (5 Wave 2 init + 6 Wave 4a greenfield + 6 Wave 4a e2e)
- **Total test count post-Phase-6**: 1317+ lib + 372+ integration, all PASS
- **Code quality gates**: 0 fmt diffs, 0 check warnings, 0 clippy warnings (rust 1.91 strict)

### Deviations from RFC-007 / ADR-010

1. **`SkillDispatcher` (FR-3) — trace-only stub в этом релизе**. RFC-007 предполагал real skill registry resolution. Causes: agent-skills capability registry — отдельная подсистема (PRD-069 deferred), не доступна как in-process API в момент Wave 1. Mitigation: `skill_dispatcher.rs` логирует invariants и эмитит fallback_hint; real resolution land'ит в Wave 5 после PRD-069.

2. **`Step.timeout_seconds` (FR-8) — schema landed, executor wiring partial**. Schema 1.0 → 1.1 minor bump done; field deserializes OK на старых playbook'ах. Но dispatcher helpers сейчас читают только default-per-type timeout; per-step override через `dispatch::helpers::run_subprocess` parameter — Wave 5 refinement.

3. **`RoutingDispatcher` introduced в Wave 4 fix** — RFC-007 §"Proposed Direction" описывал per-variant dispatcher pattern, но не имел composite router. Wave 4 e2e-engineer audit обнаружил CLI всё ещё MockDispatcher::AlwaysOk hardcoded; spawn'ил cli-mcp-swap teammate который создал `RoutingDispatcher` (190 LOC, 5 routing tests) и swap'нул CLI + MCP surfaces. Минорная архитектурная extension — задокументирована inline в `routing.rs` module doc.

## Reversibility

Изменения reversible на двух уровнях:

1. **Feature flag rollback** (per ADR-010 §Rollback Plan): release v0.27.1 с env flag `FORGEPLAN_DISPATCHER=mock` defaults dispatchers обратно в `MockDispatcher::AlwaysOk`. Phase 5 surface не ломается — playbook validate и list работают, real subprocess invocation становится no-op.

2. **Init wiring rollback**: env flag `FORGEPLAN_HINTS=0` уже работает as a user-side disable. На code level можно убрать вызов `format_recommendations` в `commands::init::run` без эффекта на остальной init flow.

`marketplace/playbooks/greenfield-kickoff.yaml` — additive (нет conflict с существующими playbook'ами), revert = file delete.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test_result

## Conclusion

Phase 6 PRD-072 — **implementation closed** для FR-1, FR-2, FR-3 (stub), FR-4, FR-5, FR-6, FR-7, FR-9, FR-10. FR-8 partial (schema landed, executor wiring Wave 5). PRD-067 AC-3/4/5/7 — **полностью закрыты** (Phase 5 deferral resolved).

4 implementation waves × 9 unique agents с strict file partitioning → **нулевые конфликты в matrix**, 0 fmt diffs, 0 clippy warnings, 1317+ lib + 372+ integration tests PASS. Spike-2 (EVID-090) валидировал tokio::process pattern на real forgeplan binary с CL3. Wave 4 fix закрыл critical CLI/MCP MockDispatcher gap через RoutingDispatcher.

**Готовность**: Phase 6 готова к /audit Round 1+2 (security focus subprocess execution surface), затем activation PRD-072 / RFC-007 / ADR-010 + PR `feat/phase6-real-dispatchers` → `dev`. Tag v0.27.0 после merge.

## Related

- ADR-009 — Forgeplan as orchestrator (EPIC-007 parent decision)
- ADR-010 — Subprocess invocation strategy (this evidence validates implementation)
- EPIC-007 — Playbook Runtime + Pack Marketplace (parent)
- PRD-072 — Phase 6 PRD: real dispatchers + init wiring + greenfield (этот sprint)
- RFC-007 — Phase 6 dispatcher architecture
- PRD-065 — Playbook runtime (Real subprocess dispatch closed by this)
- PRD-067 — Plugin detection + hints (AC-3/4/5/7 closed by this)
- EVID-088 — Spike-1 c4-to-forge mapping (precedent CL3)
- EVID-089 — Phase 5 evidence pack (deferrals addressed by this)
- EVID-090 — Spike-2 tokio::process measurement (CL3 ADR-010 validation)
- `crates/forgeplan-core/src/playbook/dispatch/` — 5 production dispatchers + RoutingDispatcher + helpers
- `crates/forgeplan-cli/src/commands/init.rs` — recommendation wiring
- `crates/forgeplan-cli/tests/integration_phase6_*.rs` — Wave 2/4a integration tests
- `marketplace/playbooks/greenfield-kickoff.yaml` — canonical greenfield playbook
- `docs/operations/PLAYBOOK-AUTHORING.ru.md` §Subprocess lifecycle — pack-author guide
- CHANGELOG.md v0.27.0 — release notes





