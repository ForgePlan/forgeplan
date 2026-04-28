---
depth: standard
id: EVID-090
kind: evidence
last_modified_at: 2026-04-28T12:30:32.446385+00:00
last_modified_by: claude-code/2.1.121
links:
- target: ADR-010
  relation: informs
- target: RFC-007
  relation: informs
- target: PRD-072
  relation: informs
- target: EVID-088
  relation: based_on
status: draft
title: Spike-2 tokio::process subprocess invocation validated for Phase 6 dispatchers
---

---
created: 2026-04-28
id: EVID-090
kind: evidence
title: Spike-2 tokio::process subprocess invocation validated for Phase 6 dispatchers
status: draft
---

# EVID-090: Spike-2 — tokio::process subprocess invocation validated

## Context

ADR-010 Phase 6 требует **subprocess invocation strategy для production dispatchers** (Plugin/Agent/Skill/Command per FR-1..FR-4). До этого spike — research-level evidence (CL2). Без empirical measurement R_eff capped, ADR-010 нельзя activate honestly.

Этот spike измеряет **tokio::process::Command + kill_on_drop + Stdio::piped pattern** на real subprocess invocation в forgeplan workspace (same context = CL3).

## Methodology

1. **Standalone Rust crate** `.local/spike-2/` (gitignored) — single `tokio = "1"` dep, no workspace pollution.
2. **Two scenarios**:
   - **A**: invoke prebuilt `target/release/forgeplan health` (deployed binary path — Phase 6 dispatcher target)
   - **B**: invoke `cargo run --bin forgeplan -- health` (anti-pattern — cold/warm cargo invocation)
3. **Configuration** mirrors ADR-010 §Decision: `Stdio::piped()` для stdout/stderr, `Stdio::null()` для stdin, `kill_on_drop(true)`, 30s timeout via `tokio::time::timeout`. Concurrent drain через `tokio::join!(read_to_end(stdout), read_to_end(stderr), child.wait())`.
4. **Lifecycle verification**: `pgrep -f forgeplan` после каждого run — детект zombie/orphan processes.

## Measurements

| Scenario | exit_code | stdout (B) | stderr (B) | duration (ms) | timed_out | zombies |
|---|---|---|---|---|---|---|
| **A** (prebuilt) | `Some(0)` | 883 | 0 | **1283** | false | 0 |
| **B** warm cache | `Some(0)` | 883 | 117 | **12726** | false | 0 |
| **B** cold cache | `None` | — | — | **30003** | **true** ⚠️ | 0 (killed clean) |

## Findings

### ✅ Verified
1. **`tokio::process::Command` works** для Phase 6 dispatcher pattern — exit code captured, stdout streamed, no deadlock на 883 B output.
2. **`kill_on_drop(true)` reaps subprocess tree** even на timeout path. After cold-cargo timeout (30s) `pgrep` показал 0 zombies (только unrelated `forgeplan serve` MCP daemons).
3. **Concurrent stream drain ordering** correct: `tokio::join!` reads stdout+stderr **before** `child.wait()`. Sequential read-then-wait would hang на stderr-heavy children — classic 64 KB pipe-buffer deadlock.
4. **`Child::wait()` ownership flow**: `wait()` consumes handle, move into `collect` future before `timeout` wraps — clean Rust ownership, нет double-mut errors.

### ⚠️ Constraints discovered
1. **Cold cargo run is NOT a viable dispatcher target** — first compile blew 30s budget. **ADR-010 ammendment**: dispatcher MUST shell out к prebuilt `target/release/forgeplan` (or `which forgeplan`), never к `cargo run`.
2. **`kill_on_drop` is async-only** — kill enqueued at drop, executed by tokio runtime. Документировать: do not drop dispatcher на stalled runtime (panics/poisoned executor).
3. **Default timeout per command type**:
   - Lightweight read (`health`, `list`): 5-10s
   - Scoring/Reason (LLM): 30-60s
   - Full plugin invocation (c4-architecture): 5-10 min
   Должно быть в `Step.timeout_seconds` (FR-8) configurable per step.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement

## Conclusion

ADR-010 §Decision **technically validated**: tokio::process pattern с kill_on_drop работает correctly для Phase 6 dispatchers. Three findings translate в ADR-010 amendments:

1. **Add invariant**: "dispatcher invokes prebuilt forgeplan binary, never `cargo run`"
2. **Add post-condition**: unit-test pattern (concurrent pipe drain + timeout + kill_on_drop verification) — canonical dispatcher harness
3. **Add NFR**: timeout defaults per delegate type (5s lightweight, 30s LLM, 300s plugin)

Spike-2 closes ADR-010 DoR Pre-condition #4. R_eff PRD-072/RFC-007/ADR-010 ready to ≥ 0.7 (A) с EVID-090 как CL3 measurement.

## Related Artifacts

- ADR-010 — Subprocess invocation strategy (this evidence validates §Decision)
- RFC-007 — Subprocess dispatcher architecture (helpers::run_subprocess pattern)
- PRD-072 — Phase 6 PRD (FR-1..FR-4 unblocked)
- EPIC-007 — Playbook Runtime + Pack Marketplace
- EVID-088 — Spike-1 c4-to-forge mapping (precedent для spike-driven CL3)
- Spike artifact: `.local/spike-2/src/main.rs` (~50 LOC, gitignored — preserved as fixture if needed)





