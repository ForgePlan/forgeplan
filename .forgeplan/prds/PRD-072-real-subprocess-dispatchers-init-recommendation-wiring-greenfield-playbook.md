---
depth: standard
id: PRD-072
kind: prd
last_modified_at: 2026-04-28T12:17:44.746840+00:00
last_modified_by: claude-code/2.1.121
links:
- target: PRD-065
  relation: refines
- target: PRD-067
  relation: refines
- target: EPIC-007
  relation: refines
- target: ADR-009
  relation: based_on
status: draft
title: Real subprocess dispatchers + init recommendation wiring + greenfield playbook
---

---
created: 2026-04-28
depth: deep
id: PRD-072
kind: prd
title: Real subprocess dispatchers + init recommendation wiring + greenfield playbook
status: draft
---

# PRD-072: Real subprocess dispatchers + init recommendation wiring + greenfield playbook (Phase 6)

## Problem

Phase 5 (PRD-065/066/067, merged 2026-04-28) shipped Playbook runtime, Ingest engine и Plugin detection как **engine layer без user-facing activation**:

- `forgeplan playbook run <name> --yes` использует `MockDispatcher::AlwaysOk` — реального subprocess invocation нет. Юзер запускает playbook, получает journal entries, но плагины (c4-architecture, autoresearch, ddd-domain-expert) не вызываются. AC-3/AC-4 PRD-065 mock-only — реализация FR-1..FR-5 нужна.
- `forgeplan init` не подключён к recommendation engine (`build_recommendations`, `detect_signals`, `format_recommendations`) — юзер на пустом репо или legacy code не получает hint «попробуй greenfield-kickoff playbook». PRD-067 AC-3/4/5 — explicit deferred в EVID-089. Реализация FR-6 закрывает этот gap.
- В `marketplace/playbooks/` только 1 canonical playbook (`brownfield-code.yaml`). Greenfield use case — самый частый для onboarding — не покрыт. FR-7 добавляет canonical greenfield-kickoff.

Результат: **Phase 5 = красиво документированный движок без user value**. Adversarial /audit Round 1+2 это подтвердили. Phase 6 переводит engine из mock-only в real production execution.

## Goals

1. **Real subprocess dispatchers**: 5 production implementations `Dispatcher` trait для Plugin / Agent / Skill / Command / ForgeplanCore — invoke реальные subprocess через Task tool API или shell-aware wrapper. AC-3/4 PRD-065 переходит из mock в real. См. FR-1, FR-2, FR-3, FR-4, FR-5.
2. **Init recommendation hints**: `forgeplan init` собирает project signals, ищет installed plugins, выдаёт top-3 applicable playbooks через PRD-071 hint contract. Закрывает PRD-067 AC-3/4/5. См. FR-6.
3. **Greenfield playbook**: canonical `marketplace/playbooks/greenfield-kickoff.yaml` (5-7 шагов: capture vision → ADR-001 stack decision → EPIC-001 + 3 PRD stubs → docs scaffold → skills install). Все шаги через `ForgeplanCore` delegations — не зависит от внешних плагинов. См. FR-7.

## Non-Goals

- NOT новые типы делегаций (5 variants per SPEC-003 — closed by design)
- NOT subprocess sandboxing на уровне ОС (security model = `--yes` gate + Command opt-in уже есть в FR-10)
- NOT autoresearch / ddd / sparc canonical playbooks — отдельные follow-up PRDs (по одному за upstream fixture для CL3)
- NOT MCP/CLI duplication refactor (отдельный sprint, Arch-H1 из EVID-089)
- NOT criterion benchmarks (отдельный perf sprint)

## Target Users

- **Brownfield adopter**: после Phase 6 запуск `forgeplan init --scan` на legacy repo с git-историей даёт hint «recommended: brownfield-code playbook (requires c4-architecture)» (FR-6) + если плагин не установлен — exact install command. `forgeplan playbook run brownfield-code --yes` запускает реальный c4-architecture (FR-1), ингестит output в forge-граф через c4-to-forge mapping.
- **Greenfield kickstarter**: `forgeplan init` на empty repo → hint «recommended: greenfield-kickoff playbook» (FR-6). `forgeplan playbook run greenfield-kickoff --yes` собирает project vision (interactive — ForgeplanCore step, FR-5), создаёт ADR-001 (stack decision), EPIC-001 (overarching initiative), 3 PRD stubs, docs/ scaffold, skills install hint (всё через FR-7 playbook).
- **Pack author**: real dispatchers стабилизируют subprocess contract — pack authors могут полагаться что Plugin/Agent/Skill steps работают предсказуемо (FR-1, FR-2, FR-3) с известными timeout (FR-8) и lifecycle (FR-9) семантиками.

## Success Criteria / Acceptance

- **AC-1**: `forgeplan playbook run brownfield-code --yes` на repo с installed c4-architecture — реальный subprocess запускается (FR-1), output captured в `produces_at`, c4-to-forge mapping применяется, генерируются artifacts с `## Sources` блоком. Journal содержит RunStart + 5×StepStart/StepEnd + RunEnd + real exit codes (не AlwaysOk).
- **AC-2**: `forgeplan playbook run X --yes` на repo с **uninstalled** plugin — step fails gracefully с install command из `fallback_hint`, не crash. Journal записывает `Failed { exit_code, stderr_excerpt }`. Cross-references FR-1 + FR-9.
- **AC-3**: `forgeplan init` на empty git repo → stderr содержит `recommended: greenfield-kickoff playbook` (PRD-067 AC-3, реализация FR-6).
- **AC-4**: `forgeplan init` на repo с `.obsidian/` → hint `recommended: brownfield-docs playbook` (PRD-067 AC-4, FR-6).
- **AC-5**: `forgeplan init` на repo с >100 commits + no docs → hint `recommended: brownfield-code playbook` (PRD-067 AC-5, FR-6).
- **AC-6**: `FORGEPLAN_HINTS=0` или non-TTY stderr → no recommendation hints emitted (PRD-067 AC-7 backward compat, FR-6).
- **AC-7**: `forgeplan playbook validate marketplace/playbooks/greenfield-kickoff.yaml` → `OK: greenfield-kickoff (N steps)` + `Done.` hint (FR-7).
- **AC-8**: E2E test `e2e_greenfield_kickoff_writes_adr_and_epic` — на tempdir + dry-run prints все 5-7 steps; на actual run создаются ADR-001 + EPIC-001 + ≥3 PRD stubs (FR-5 + FR-7).
- **AC-9**: All existing tests pass (regression-free) — Phase 5 surface не меняется, только dispatcher backend (FR-1..FR-5 swap).
- **AC-10**: Per-step durability работает с real subprocess — kill -9 во время step → restart resumes от последнего persisted StepEnd (cross-references FR-9 + Phase 5 journal).

## Functional Requirements

- [ ] **FR-1**: Production `PluginDispatcher` impl — invokes Claude Code Task tool subprocess для plugin: variant. Captures stdout → `produces_at` path. Timeout default 5 min, configurable via `Step.timeout_seconds` (FR-8). Closes AC-1, AC-2.
- [ ] **FR-2**: Production `AgentDispatcher` — same pattern, для agent: variant. Closes AC-1.
- [ ] **FR-3**: Production `SkillDispatcher` — invokes loaded skill в current agent context (via slash-command emit или MCP tool call). Closes AC-1.
- [ ] **FR-4**: Production `CommandDispatcher` — std::process::Command wrapper с whitelist и `--yes` enforcement (already in `validate_command_delegate_security`). Stderr capture, timeout, exit-code propagation. Closes AC-1.
- [ ] **FR-5**: Production `ForgeplanCoreDispatcher` — direct internal call (no subprocess) для new/validate/activate/search/ingest. Used heavily в greenfield playbook (AC-8).
- [ ] **FR-6**: `commands::init::run` extension — после создания `.forgeplan/`, perform `detect_signals(workspace_root)` + `detect_plugins(extended_registry())` + `build_recommendations(...)` + emit `format_recommendations(...)` to stderr (respects `FORGEPLAN_HINTS=0` and TTY check). Closes AC-3, AC-4, AC-5, AC-6.
- [ ] **FR-7**: `marketplace/playbooks/greenfield-kickoff.yaml` — 5-7 шагов через `ForgeplanCore` + 1 optional Skill step (forge-scaffolder, fallback_hint provided). Closes AC-7, AC-8.
- [ ] **FR-8**: `Step` schema gets optional `timeout_seconds: Option<u32>` field (SPEC-003 minor bump 1.0 → 1.1, backward compat — old playbooks load OK без поля). Used by FR-1, FR-2, FR-3, FR-4.
- [ ] **FR-9**: Subprocess lifecycle: `tokio::process::Command` (async), `kill_on_drop(true)`, output captured via `Stdio::piped()`. On timeout — `child.kill().await`, journal `Failed { reason: timeout }`. Closes AC-2 + AC-10.
- [ ] **FR-10**: Security: `Command` delegate с command-array (already typed `Vec<String>`) — no shell expansion (no `Stdio::Inherit`, no `sh -c`); refuses при отсутствии `--yes` (already in dispatch.rs). Hardens FR-4.

## Non-Functional Requirements

- **Performance**: real subprocess invocation 100-500ms overhead per step (acceptable). Sequential — playbook с 7 steps = ~5-10 min real run на c4-architecture. Не оптимизируется в этом PRD.
- **Security**: subprocess inherits limited env (no FORGEPLAN_* leak), stdin closed (Stdio::null), no shell. Command delegate — `--yes` required + audit log via journal.
- **Backward compat**: SPEC-003 1.0 → 1.1 (additive). Старые playbooks (без `timeout_seconds`) load OK с default 300s. MockDispatcher остаётся для tests.
- **Observability**: каждый subprocess invocation эмитит `tracing::info!(target: "playbook.dispatch", ...)` events с step_id, delegate_type, exit_code, duration_ms.

## Implementation Plan (high-level — RFC-007 will detail)

### Phase 0: Shape
- [x] PRD-072 (this) — drafted
- [ ] RFC-007 — architecture: subprocess lifecycle, Task tool integration, init flow
- [ ] ADR-010 — decision на subprocess lifecycle (tokio::process с kill-on-drop)
- [ ] ADI reasoning на PRD-072 (3+ hypotheses)

### Phase 1: Spike
- [ ] Spike-2: prove Task tool subprocess invocation для one plugin (c4-architecture) end-to-end → CL3 evidence

### Phase 2: Implementation (waves)
- Wave 1: 5 dispatcher impls + lifecycle (timeouts, kill-on-drop, journal integration) — закрывает FR-1, FR-2, FR-3, FR-4, FR-5, FR-9, FR-10
- Wave 2: init recommendation wiring + tests — закрывает FR-6
- Wave 3: greenfield-kickoff.yaml + E2E — закрывает FR-7, FR-8
- Wave 4: integration tests + docs

### Phase 3: Validation
- [ ] /audit ×2 (security focus — subprocess execution surface)
- [ ] Activate PRD-072 + RFC-007 + ADR-010

## Related Artifacts

| Artifact | Type | Relation |
|---|---|---|
| PRD-065 | PRD | refines (closes AC-3/AC-4 from mock to real) |
| PRD-067 | PRD | refines (closes AC-3/4/5 init wiring) |
| EPIC-007 | Epic | refines (Phase 6 of overall pack marketplace) |
| ADR-009 | ADR | based_on (orchestrator pivot — this realizes the delegation) |
| RFC-007 | RFC | drives (architecture for subprocess dispatch) |
| ADR-010 | ADR | drives (subprocess lifecycle decision) |
| EVID-089 | Evidence | informs (deferrals от Phase 5 closed by this PRD) |
| PROB-046 | Problem | informs (output hint contract — dispatch failures emit hints) |





