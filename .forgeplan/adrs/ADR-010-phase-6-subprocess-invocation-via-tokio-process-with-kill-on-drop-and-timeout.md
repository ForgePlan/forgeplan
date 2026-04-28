---
depth: standard
id: ADR-010
kind: adr
last_modified_at: 2026-04-28T12:26:45.042185+00:00
last_modified_by: claude-code/2.1.121
links:
- target: PRD-072
  relation: based_on
- target: RFC-007
  relation: refines
status: draft
title: Phase 6 — Subprocess invocation via tokio::process with kill-on-drop and timeout
---

---
created: 2026-04-28
depth: deep
id: ADR-010
kind: adr
title: Phase 6 — Subprocess invocation via tokio::process with kill-on-drop and timeout
status: draft
---

# ADR-010: Subprocess invocation strategy для Phase 6 dispatchers

## Context

PRD-072 + RFC-007 требуют выбрать механизм subprocess invocation для Phase 6 production dispatchers (Plugin / Agent / Skill / Command). Решение влияет на:
- Concurrency: блокирует ли subprocess worker thread executor'а
- Cancel safety: что происходит если playbook прерывается mid-step
- Test mockability: можно ли держать MockDispatcher pattern из Phase 5
- Resource cleanup: zombie processes, file descriptor leaks

Phase 5 уже async (`Executor::run` — async fn) и зависит от tokio. Не start fresh — продолжение existing infrastructure.

## Decision

Использовать **`tokio::process::Command`** для всех 4 subprocess-based dispatchers с следующей конфигурацией:

```rust
let mut cmd = tokio::process::Command::new(&spec.program);
cmd.args(&spec.args)
   .env_clear()                             // no FORGEPLAN_* leak
   .envs(&spec.env)                         // explicit allow-list
   .stdin(Stdio::null())                    // no shell, no piped stdin
   .stdout(Stdio::piped())
   .stderr(Stdio::piped())
   .kill_on_drop(true);                     // cleanup on cancel/panic
if let Some(cwd) = spec.cwd { cmd.current_dir(cwd); }

let mut child = cmd.spawn().map_err(DispatchError::Spawn)?;
let result = tokio::time::timeout(spec.timeout, async {
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let (out, err, status) = tokio::join!(
        read_to_end(stdout),
        read_to_end(stderr),
        child.wait()
    );
    // ...
}).await;
match result {
    Ok(Ok(outcome)) => Ok(outcome),
    Ok(Err(e)) => Err(DispatchError::Io(e)),
    Err(_) => {
        let _ = child.kill().await;
        Err(DispatchError::Timeout(spec.timeout))
    }
}
```

## Alternatives Considered

| Option | Verdict | Why |
|---|---|---|
| **A. tokio::process с kill_on_drop** (CHOSEN) | Chosen | Native async, cancel-safe, clean integration с executor. tokio уже в deps. |
| B. std::process в spawn_blocking | Rejected | Worker-thread blocking — 7 steps × 5 min playbook = thread idle полчаса. Wastes tokio thread pool. |
| C. async-process crate | Rejected | Дополнительная dep, дублирует tokio::process функциональность. |
| D. Custom fork/exec wrapper | Rejected | Изобретение колеса. tokio::process тестирован миллионами downstream crates. |

## Consequences

### Positive
- **Concurrency**: subprocess wait не блокирует executor thread — другие async tasks (e.g., journal flush) продолжают работать
- **Cleanup**: `kill_on_drop(true)` гарантирует что zombie processes не остаются если executor cancel'ится (Ctrl+C) или panic'ит
- **Stream capture**: `Stdio::piped()` + tokio::join! читает stdout+stderr concurrently — нет deadlock на large outputs (классический pitfall sync subprocess)
- **Test mockable**: trait Dispatcher остаётся, MockDispatcher работает unchanged — production swap происходит на уровне `Executor::with_dispatcher(...)`
- **Security**: `env_clear()` + explicit allow-list — нет утечки `FORGEPLAN_*` env. `Stdio::null()` для stdin — нет path для interactive injection.

### Negative (trade-offs)
- **Cross-platform**: `kill_on_drop` на Unix отправляет SIGKILL после Drop. На Windows — TerminateProcess. Различается timing — документировать.
- **Output buffering**: weakness — если subprocess пишет 100 MiB в stdout, все буферизуется в memory. Mitigation: cap на read (10 MiB max — больше = error).
- **Tokio runtime requirement**: Dispatcher не используется вне tokio context. OK — Executor уже tokio.

### Risks
- **Process leak на panic before Drop runs** — крайне редко, mitigated through `kill_on_drop` + `tokio::select!` guards.
- **Timeout not respected if child ignores SIGKILL** — POSIX zombies in pathological cases. Out of scope — user can intervene with `pkill`.

## Invariants

- **NEVER**: `Stdio::inherit()` for stdin — закрывает path для interactive prompt injection
- **NEVER**: `sh -c` shell expansion — Command delegate uses `Vec<String>` typed args только
- **NEVER**: env passthrough by default — `env_clear()` + explicit allow-list
- **ALWAYS**: `kill_on_drop(true)` на spawned child
- **ALWAYS**: `Stdio::piped()` для stdout/stderr — иначе deadlock на large output
- **ALWAYS**: timeout enforced — default 300s configurable per `Step.timeout_seconds` (FR-8)

## Pre-conditions (DoR)

- [x] Phase 5 merged в `dev` (PR #217) — Dispatcher trait, journal, executor — стабильны
- [x] PRD-072 + RFC-007 active drafts с full FR coverage
- [x] tokio в forgeplan-core workspace deps (existing — Phase 4)
- [ ] Spike-2 — manual c4-architecture invocation проходит на dev workstation, EVID-090 CL3 captured
- [ ] /audit Round 1 на PRD-072/RFC-007/ADR-010 shape — `forgeplan_score ≥ 0.50` minimum threshold

## Post-conditions (DoD)

- [ ] 5 dispatcher impls land — PluginDispatcher, AgentDispatcher, SkillDispatcher, CommandDispatcher, ForgeplanCoreDispatcher (FR-1..FR-5)
- [ ] All Phase 5 tests pass без regression (1337+ lib tests)
- [ ] Real subprocess E2E test — `e2e_real_brownfield_dispatch` runs c4-architecture (or fixture mock subprocess) и проверяет exit code, stdout capture, journal write
- [ ] No zombie processes after E2E run (verified via `ps` snapshot before/after)
- [ ] `kill_on_drop` test — spawn long-running subprocess, drop dispatcher, verify child killed within 5s
- [ ] Cross-platform CI green (Linux + macOS, Windows если matrix настроен)
- [ ] Doc updated: `docs/operations/PLAYBOOK-AUTHORING.ru.md` с разделом "Subprocess lifecycle" + `timeout_seconds` field

## Affected Files

| File | Change | Notes |
|---|---|---|
| `crates/forgeplan-core/src/playbook/dispatch.rs` | split → directory | Single file → `dispatch/{mod,helpers,plugin,agent,skill,command,forgeplan_core,mock,recording}.rs` |
| `crates/forgeplan-core/src/playbook/dispatch/helpers.rs` | NEW | `SubprocessSpec`, `SubprocessOutcome`, `run_subprocess` async helper |
| `crates/forgeplan-core/src/playbook/dispatch/plugin_dispatcher.rs` | NEW | FR-1 |
| `crates/forgeplan-core/src/playbook/dispatch/agent_dispatcher.rs` | NEW | FR-2 |
| `crates/forgeplan-core/src/playbook/dispatch/skill_dispatcher.rs` | NEW | FR-3 |
| `crates/forgeplan-core/src/playbook/dispatch/command_dispatcher.rs` | NEW | FR-4 |
| `crates/forgeplan-core/src/playbook/dispatch/forgeplan_core_dispatcher.rs` | NEW | FR-5 — direct internal call (no subprocess) |
| `crates/forgeplan-core/src/playbook/types.rs` | MODIFY | Add `Step.timeout_seconds: Option<u32>` field (FR-8, schema_version 1.1) |
| `crates/forgeplan-core/src/playbook/executor.rs` | MODIFY | Pass timeout/env to dispatcher; remove MockDispatcher::AlwaysOk default in production paths |
| `crates/forgeplan-core/src/playbook/loader.rs` | MODIFY | Accept schema_version 1.1 (^1.0 already covers minor bump) |
| `crates/forgeplan-cli/src/commands/init.rs` | MODIFY | FR-6 init recommendation wiring |
| `crates/forgeplan-cli/src/commands/playbook.rs` | MODIFY | Wire ProductionDispatcher (replace MockDispatcher::AlwaysOk in `run_execute`) |
| `crates/forgeplan-mcp/src/server.rs` | MODIFY | Same swap для `forgeplan_playbook_run` MCP tool |
| `marketplace/playbooks/greenfield-kickoff.yaml` | NEW | FR-7 canonical |
| `docs/operations/PLAYBOOK-AUTHORING.ru.md` | MODIFY | Add subprocess section + `timeout_seconds` |
| `crates/forgeplan-cli/tests/integration_phase6_*.rs` | NEW | E2E: real subprocess + greenfield kickoff + init hints |
| `.forgeplan/evidence/EVID-090-spike-2-tokio-process-c4-architecture.md` | NEW | Spike-2 CL3 measurement |

## Rollback Plan

**Trigger**: если real subprocess вызывает unexpected breakage в production (e.g., zombie processes blocking CI workflow).

**Steps**:
1. Release v0.x.1 с env flag `FORGEPLAN_DISPATCHER=mock` — defaults dispatchers обратно в MockDispatcher
2. Issue tracker для investigation
3. Hotfix в Wave 5

**Blast Radius**: medium — playbook execution становится no-op до restore. Recoverable.

## Evidence Requirements

- **E1**: Spike-2 — manual c4-architecture invocation через tokio::process на forgeplan repo. Capture stdout/exit/duration. CL3 measurement → EVID-090.
- **E2**: Unit tests — `run_subprocess` для simple program (echo, sleep, exit code, timeout) all pass.
- **E3**: Integration — actual playbook run brownfield-code на test repo. Measured: total time, journal correctness, no zombie processes (verify via `ps aux` after run).
- **E4**: Cross-platform — CI matrix (Ubuntu/macOS) для critical subprocess tests.

## Related Artifacts

| Artifact | Type | Relation |
|---|---|---|
| PRD-072 | PRD | based_on |
| RFC-007 | RFC | drives (architecture detail) |
| ADR-009 | ADR | informs (orchestrator pivot context) |
| ADR-002 | ADR | informs (existing tokio choice) |
| EPIC-007 | Epic | informs |

