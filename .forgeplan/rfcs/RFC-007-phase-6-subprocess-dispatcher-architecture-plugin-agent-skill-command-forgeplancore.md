---
depth: standard
id: RFC-007
kind: rfc
last_modified_at: 2026-04-28T12:25:51.422867+00:00
last_modified_by: claude-code/2.1.121
links:
- target: PRD-072
  relation: based_on
status: draft
title: Phase 6 — Subprocess dispatcher architecture (Plugin/Agent/Skill/Command/ForgeplanCore)
---

---
created: 2026-04-28
depth: deep
id: RFC-007
kind: rfc
title: Phase 6 — Subprocess dispatcher architecture (Plugin/Agent/Skill/Command/ForgeplanCore)
status: draft
---

# RFC-007: Phase 6 Subprocess dispatcher architecture

## Summary

Архитектура для перехода Phase 5 dispatchers из mock в production. 5 типов делегаций (Plugin/Agent/Skill/Command/ForgeplanCore) реализуются через единый async subprocess pattern с явными контрактами для timeout, kill-on-drop, output capture и journal integration. Init recommendation flow добавляется как pure-data step в `commands::init::run`. Greenfield playbook собирается из `ForgeplanCore` steps без внешних зависимостей.

## Motivation

PRD-072 определил scope. RFC детализирует **как именно** реализовать FR-1..FR-10 чтобы:
- Не сломать Phase 5 surface (CLI/MCP/journal contracts стабильны)
- Subprocess lifecycle был testable (mockable trait сохраняется для unit tests)
- Security boundary держалась под нагрузкой (Command-only-with-yes, no shell expansion)
- Resumability работала после kill -9 (per-step flush уже в Phase 5)

## Options Considered

### Option A — Async tokio::process per dispatch (CHOSEN)

`Dispatcher::dispatch(&self, &Step)` остаётся async. Каждая impl создаёт `tokio::process::Command`, настраивает `Stdio::piped()` для stdout/stderr, `Stdio::null()` для stdin, `kill_on_drop(true)`. Timeout через `tokio::time::timeout(duration, child.wait())`. Output captured via async streaming.

**Pros**: единая модель для всех 4 subprocess-based dispatchers. tokio integration естественна (executor уже async). Cancel-safe через `kill_on_drop`. Streaming output не блокирует executor thread.

**Cons**: Добавляет `tokio::process` to forgeplan-core deps (уже есть через tokio full features). Все 4 impls делят helper `run_subprocess(cmd, timeout) -> SubprocessOutcome` чтобы избежать копипаст.

### Option B — sync std::process в spawn_blocking

Использовать `tokio::task::spawn_blocking` для wrap sync `std::process::Command`.

**Pros**: проще mental model для разработчиков непривычных к tokio::process.
**Cons**: thread blocking (worker thread sits idle while subprocess waits) — для playbook с 7 steps × 5 min = пол часа thread занят. Не нужно — Option A async path даёт лучшую concurrency без drawbacks.

**Rejected**.

### Option C — Single Dispatcher impl с runtime dispatch on Delegation variant

One concrete `ProductionDispatcher` struct, match'ит Delegation внутри dispatch.

**Pros**: меньше boilerplate, один state shared между всеми типами.
**Cons**: God-struct anti-pattern. Nullifies trait-based mockability. Тесты должны mock'ать ENTIRE dispatcher — теряется Phase 5 RecordingDispatcher pattern. **Rejected** в пользу 5 separate impls с shared `subprocess_helper`.

## Proposed Direction

**Option A**, organised как:

```
crates/forgeplan-core/src/playbook/
├── dispatch/
│   ├── mod.rs                  ← trait Dispatcher (existing) + facade
│   ├── helpers.rs              ← shared run_subprocess + SubprocessOutcome (NEW)
│   ├── plugin_dispatcher.rs    ← FR-1 (NEW)
│   ├── agent_dispatcher.rs     ← FR-2 (NEW)
│   ├── skill_dispatcher.rs     ← FR-3 (NEW)
│   ├── command_dispatcher.rs   ← FR-4 (NEW)
│   ├── forgeplan_core_dispatcher.rs  ← FR-5 (NEW)
│   ├── mock.rs                 ← MockDispatcher (existing — kept for tests)
│   └── recording.rs            ← RecordingDispatcher (existing — kept)
```

**Existing `dispatch.rs` splits into directory** — clean separation, each impl ~150 LOC.

### `subprocess_helpers::run_subprocess`

```rust
pub struct SubprocessSpec<'a> {
    pub program: &'a str,
    pub args: &'a [String],
    pub env: &'a HashMap<String, String>,
    pub cwd: Option<&'a Path>,
    pub timeout: Duration,
    pub stdin_data: Option<&'a [u8]>,
}

pub struct SubprocessOutcome {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub timed_out: bool,
    pub duration: Duration,
}

pub async fn run_subprocess(spec: SubprocessSpec<'_>) -> Result<SubprocessOutcome, DispatchError>;
```

Все 4 subprocess-based dispatchers строят `SubprocessSpec` исходя из `Step.delegate_to` и вызывают `run_subprocess`. ForgeplanCoreDispatcher (FR-5) НЕ использует subprocess — direct internal call.

### Init flow extension (FR-6)

`commands::init::run` после стандартной workspace creation:

```rust
if !std::env::var("FORGEPLAN_HINTS").is_ok_and(|v| v == "0") && stderr_is_tty() {
    let signals = detect_signals(&workspace_root)?;
    let installed = detect_plugins(&extended_registry());
    let known_playbooks = discover_known_playbooks();  // workspace + plugin caches
    let recs = build_recommendations(&signals, &installed, &known_playbooks);
    let formatted = format_recommendations(&recs);
    eprintln!("{}", formatted);
}
```

Backward compat: zero-byte output если no recommendations OR env disabled.

### Greenfield playbook (FR-7)

```yaml
schema_version: "1.1"  # FR-8 — adds optional timeout_seconds
name: greenfield-kickoff
title: Greenfield project bootstrap
description: |
  Capture vision, decide stack (ADR-001), scaffold initial Epic + 3 PRDs,
  initialize docs/ structure. All steps ForgeplanCore — no external plugins.
triggered_by:
  empty_repo: true
  has_git: true        # implies fresh git init done
steps:
  - id: capture-vision
    delegate_to: { type: forgeplan_core, target: capture }
    input: { kind: note, prompt: "What is this project?" }
  - id: stack-decision
    delegate_to: { type: forgeplan_core, target: new }
    input: { kind: adr, title: "Initial stack decision" }
    requires: [capture-vision]
  - id: kickoff-epic
    delegate_to: { type: forgeplan_core, target: new }
    input: { kind: epic, title: "Project initial scope" }
    requires: [stack-decision]
  - id: prd-stub-1..3   # 3 parallel PRD stubs
    ...
  - id: docs-scaffold
    delegate_to: { type: skill, name: forge-scaffolder }
    fallback_hint: "claude skill install ForgePlan/forge-scaffolder"
```

Final step optional: skill `forge-scaffolder` (PRD-069 deferred). Если skill не install — fallback_hint выводится, playbook не падает.

## Implementation Phases

### Phase 1: Spike-2 (1-2 days)
Manually invoke c4-architecture от Claude Code subprocess. Capture exit code, stdout, stderr. Verify Task tool API surface. EVID-090 CL3.

### Phase 2: Wave 1 — 5 dispatchers (3-5 days, 5 parallel agents)
- a1: PluginDispatcher + helpers extraction
- a2: AgentDispatcher (consumes helpers)
- a3: SkillDispatcher
- a4: CommandDispatcher (security hardening)
- a5: ForgeplanCoreDispatcher
Strict file ownership — каждый owns свой dispatcher file + helpers.rs (lock-step write через one agent если конфликт).

### Phase 3: Wave 2 — init wiring (1-2 days, 1 agent)
Single agent edits `commands::init::run` + adds tests. Low blast radius.

### Phase 4: Wave 3 — greenfield playbook (1 day)
Author + validate + E2E test. Fixture-based.

### Phase 5: Wave 4 — integration + docs + EVID-090
- E2E: brownfield-code real run (с mocked Task tool в test)
- E2E: greenfield-kickoff actual run
- E2E: init hint emission
- Docs: PLAYBOOK-AUTHORING.ru.md update (mention timeout_seconds)
- EVID-090

### Phase 6: /audit ×2 + activate + PR

## Invariants

- **NEVER**: subprocess inherits FORGEPLAN_* env (env_clear() + explicit allow-list)
- **NEVER**: stdin passthrough (Stdio::null() — no path to interactive injection)
- **NEVER**: shell expansion (Command делает direct exec, не sh -c)
- **NEVER**: Command delegate без `--yes` flag (validate_command_delegate_security)
- **ALWAYS**: kill_on_drop(true) — guaranteed cleanup на cancel/panic
- **ALWAYS**: Stdio::piped() для stdout/stderr — иначе deadlock на large output
- **ALWAYS**: timeout enforced — default 300s, configurable per Step.timeout_seconds (FR-8)
- **ALWAYS**: journal flushed после каждого StepEnd (Phase 5 contract holds)
- **ALWAYS**: forgeplan-core::playbook::dispatch trait stays mockable (MockDispatcher остаётся для tests)

## Rollback Plan

**Triggers**:
- >3 production reports zombie processes blocking CI после Phase 6 ship
- Unexpected env leakage detected (security audit finding)
- Subprocess invocation breaks playbook composition (regression)

**Steps**:
1. Release v0.x.1 с env flag `FORGEPLAN_DISPATCHER=mock` — defaults dispatchers обратно в MockDispatcher::AlwaysOk
2. Investigate via journal entries + tracing logs
3. Hotfix как Wave 5 sub-sprint — fix root cause, re-enable real dispatchers feature flag

**Blast Radius**: medium. Playbook execution становится no-op до restore (degraded UX), но Phase 5 surface не ломается — base workflow (`forgeplan new/validate/activate`) не затронут.

## Related Artifacts

| Artifact | Type | Relation |
|---|---|---|
| PRD-072 | PRD | based_on |
| ADR-010 | ADR | informs (subprocess lifecycle decision) |
| ADR-009 | ADR | informs (orchestrator pivot context) |
| EPIC-007 | EPIC | refines (Phase 6 of pack marketplace) |
| EVID-089 | Evidence | informs (Phase 5 deferrals to address) |
| EVID-090 | Evidence | drives (Spike-2 CL3 measurement) |

