---
depth: standard
id: RFC-010
kind: rfc
links:
- target: PRD-078
  relation: based_on
- target: PROB-072
  relation: informs
- target: PROB-067
  relation: informs
- target: PROB-073
  relation: informs
- target: ADR-003
  relation: informs
status: draft
title: Implementation phases for MCP worktree-aware projection routing
---

# RFC-010: Implementation phases for MCP worktree-aware projection routing

## Progress

```
Phase 1  ░░░░░░░░░░░░░░░░░░░░░░░░  0/4  (  0%)  Resolution chain + params
Phase 2  ░░░░░░░░░░░░░░░░░░░░░░░░  0/3  (  0%)  Detection + error response (Option E)
Phase 3  ░░░░░░░░░░░░░░░░░░░░░░░░  0/3  (  0%)  Per-workspace lock refactor + e2e tests
─────────────────────────────────────────────────
TOTAL                               0/10 (  0%)
```

---

## Summary

Трёхфазная реализация PRD-078: (1) добавление optional `workspace` параметра и resolution chain на mutating MCP tools; (2) multi-worktree detection + error response при missing workspace в multi-worktree env; (3) per-workspace lock refactor + e2e regression tests. Каждая фаза имеет файлы-владельцев, изолированные test gates и условия завершения для следующей фазы.

## Motivation

Cross-ref PRD-078 для полного problem statement и design justification. Этот RFC отвечает только на вопрос «как именно реализуем» — порядок изменений, разбиение на phases, ownership grid для возможной параллельной работы команды агентов.

Краткое напоминание: текущий MCP server фиксирует CWD на старте (`main.rs:11`), что ломает multi-worktree pipelines (PROB-072 signal). Solution — H1 (per-call workspace param) + H2 (FORGEPLAN_WORKSPACE env var) с FPF Option E (error при detect + missing workspace).

## Goals

- Phased rollout с тестируемым state после каждой фазы (можно прервать или паузить между phases без полу-сломанного состояния)
- File ownership grid для возможной параллельной работы 2-3 workers (Pattern A team lead orchestration)
- Test plan покрывает все AC-1..AC-5 из PRD-078
- 0 регрессий existing single-worktree workflows на каждой phase boundary
- Backward compat сохраняется на каждом merge в feature branch

## Non-Goals

- Design decisions (E vs D' vs A) — зафиксированы в ADR-NNN (создаётся отдельно)
- Latency optimization (session cache, etc.) — обрабатывается через PROB-073 + R-1 evidence gap
- Config toggle `workspace_mismatch_policy` (D' fallback) — Growth Vision PRD-078, не MVP
- CLI flag `--workspace` для `forgeplan` binary — scope только MCP server
- Plugin-layer enforcement (forgeplan-marketplace updates) — отдельная работа после merge

## Options Considered

### Option A: Big-bang single-PR implementation

**Description**: Все изменения (params + detection + lock refactor + tests) в одном PR.

**Pros**: один review, atomic merge, нет промежуточных полу-states.

**Cons**: large diff (estimate ~600 LOC новых + ~200 LOC модификаций), сложно audit, blast radius огромный, rollback дорогой. Не подходит для critical-path code (MCP server dispatch).

### Option B: Three sequential PRs (one per phase)

**Description**: Phase 1 PR → merge → Phase 2 PR → merge → Phase 3 PR → merge. Каждый PR полностью функционален в одиночку.

**Pros**: маленькие review-able PRs, incremental verification, rollback isolated. Audit per phase. Совместимо с continuation prompt's "/sprint" 2-week scope.

**Cons**: 3 раза проходить полный CI cycle (~15 минут каждый), 3 sync points с dev, потенциальные merge conflicts между фазами.

### Option C: Single feature branch с phased commits + один PR

**Description**: Все 3 phases в одной feature branch как separate commits (или groups of commits), один PR на ревью с явными phase boundaries в commit messages.

**Pros**: один CI cycle, один merge sync. Reviewer видит phased progression. Атомарный merge но логически разбит.

**Cons**: review всё ещё большой (но organized). Phase 1 не "shipped" пока весь PR не merged — нельзя early-deploy intermediate state. Squash merge потеряет phase commits (по CLAUDE.md мы используем merge commits, не squash для feat/* — это спасает).

## Trade-off Analysis

| Критерий | Option A | Option B | Option C |
|----------|----------|----------|----------|
| Review complexity | 🔴 high | 🟢 low (per PR) | 🟡 medium (organized) |
| CI cycle cost | 🟢 1× | 🔴 3× | 🟢 1× |
| Rollback isolation | 🔴 hard | 🟢 per phase | 🟡 git revert merge |
| Blast radius per merge | 🔴 max | 🟢 isolated | 🟡 atomic |
| Early-deploy intermediate | ❌ no | ✅ yes (Phase 1 alone deployable) | ❌ no |
| Sprint scope fit (continuation prompt Day 1-3) | 🟢 single PR | 🔴 3 PRs require 3-5 days | 🟢 single PR |
| Audit thoroughness | 🔴 superficial risk | 🟢 per-phase | 🟡 phase-by-phase review possible |
| Methodology fit (RED LINE #5: PR after Code→Audit→Fix→Test→Fmt→Lint→Verify) | 🟢 one cycle | 🔴 cycle ×3 | 🟢 one cycle |

## Proposed Direction

**Option C** — single feature branch с phased commits + один PR.

Reasoning:
1. **Sprint scope fit** — Day 1-3 budget на PROB-072 в continuation prompt предполагает один merge, не три. Option B растягивает на 3-5 дней с тремя CI/audit cycles
2. **Atomic semantic** — три фазы это логически одна feature (worktree-aware routing). Разбивать на 3 deployable стадии искусственно
3. **Merge commit policy** (CLAUDE.md): для `feat/*` → `dev` используется merge commit, не squash. Это сохраняет phase commits в истории для audit trail
4. **Audit per phase возможен** даже в одном PR — `gh pr diff --files-only --include "phase-1-*"` для focused review

Reviewer-friendly commit message structure:
```
feat(mcp): PRD-078 Phase 1 — workspace param + resolution chain
feat(mcp): PRD-078 Phase 2 — multi-worktree detection + error response
feat(mcp): PRD-078 Phase 3 — per-workspace lock refactor + e2e tests
```

## Risks & Open Questions

- **R-1** (cross-ref PRD-078): latency cost git rev-parse — mitigation: bench в PROB-073 sprint, evidence linked back
- **Q-1**: Если Phase 2 audit находит blocker, можно ли merge только Phase 1? **A**: Да через `git revert` Phase 2 + 3 commits на feature branch, переделать. Phase 1 alone — backward-compat additive (новый optional param), не должен ломать
- **Q-2**: `workspaceFolders` из MCP initialize handshake — игнорим или используем как override? **A**: Игнорим (отвергнут ADI как H3 brittle). Если потом окажется reliable — добавить в Phase 4 как opt-in
- **R-2**: Test fixtures для multi-worktree env требуют setup `git worktree add /tmp/...` в integration tests — могут быть flaky на CI runners с ограниченным disk. Mitigation: cleanup в test teardown, `cargo test` serial mode для этих tests

## Implementation Phases

### Phase 1: Resolution chain + workspace param

- [ ] **1.1** Добавить `workspace: Option<String>` в `NewParams`, `LinkParams`, `UpdateParams` в `crates/forgeplan-mcp/src/convert.rs`
- [ ] **1.2** Реализовать `resolve_workspace(params_workspace: Option<&str>) -> PathBuf` в `crates/forgeplan-mcp/src/server.rs` с chain: param → `FORGEPLAN_WORKSPACE` env → `std::env::current_dir()`. Lazy env read (на каждый call, не cached at startup)
- [ ] **1.3** Заменить hardcoded `self.workspace_root` в `forgeplan_new` / `forgeplan_update` / `forgeplan_link` handlers на `resolve_workspace(params.workspace.as_deref())`
- [ ] **1.4** Unit tests: resolution chain priority (param > env > cwd), missing-everywhere fallback, invalid path handling. `cargo test --workspace` → 3084+N PASS

**Phase 1 done when**: existing single-worktree workflows unchanged (cargo test PASS), новый `workspace` param accepted и работает, новый env var honored. Backward compat verified через regression test.

### Phase 2: Multi-worktree detection + Option E error response

- [ ] **2.1** Реализовать `detect_multi_worktree(cwd: &Path) -> bool` в `crates/forgeplan-core/src/workspace/init.rs` через `git rev-parse --git-common-dir` ≠ `--show-toplevel`. Graceful fallback если `git` not in PATH (assume single-worktree, no error)
- [ ] **2.2** В `resolve_workspace` (Phase 1.2) добавить error-on-detect path: если params.workspace=None AND env=None AND `detect_multi_worktree()` → вернуть MCP error `-32602` с actionable message содержащим auto-detected suggested path (`format!("multi-worktree detected; pass workspace: {}", suggested)`)
- [ ] **2.3** Integration test: AC-3 (Multi-worktree без workspace → explicit error) + assert error message содержит actionable suggestion

**Phase 2 done when**: AC-3 PASS, AC-1/AC-2 still PASS, error messages parseable (NFR-003).

### Phase 3: Per-workspace lock + comprehensive e2e

- [ ] **3.1** Refactor `acquire_workspace_lock` в `crates/forgeplan-core/src/workspace/lock.rs` чтобы lock file path вычислялся от resolved workspace, не от static `workspace_root`. Backward compat: single-worktree env → тот же lock file path как раньше (no breaking change)
- [ ] **3.2** Добавить `resolved_workspace: String` и `resolved_via: "param" | "env" | "cwd"` поля в success response payload каждого tool (FR-007 + NFR-004)
- [ ] **3.3** E2E tests в `crates/forgeplan-mcp/tests/worktree_routing_e2e.rs`: AC-1 (subagent в worktree happy path), AC-2 (single-worktree backward compat), AC-4 (CI strict mode), AC-5 (workaround deprecation simulation). Использовать `tempfile` + `git worktree add` для setup

**Phase 3 done when**: все AC-1..AC-5 PASS, concurrent multi-worktree writes используют разные locks, PROB-067 race scenarios не воспроизводятся.

---

## File Ownership Grid (для team Pattern A)

Если работа делается через team lead pattern (3 параллельных workers в отдельных worktrees):

| Worker | OWNED FILES (write/edit) | FORBIDDEN FILES | Phase |
|--------|--------------------------|-----------------|-------|
| **W1** (params + chain) | `crates/forgeplan-mcp/src/convert.rs`, `crates/forgeplan-mcp/src/server.rs` (только `resolve_workspace` + handler updates) | `forgeplan-core/src/workspace/*`, `workspace_detection_bench.rs` | Phase 1 |
| **W2** (detection + error) | `crates/forgeplan-core/src/workspace/init.rs` (детект функция), `crates/forgeplan-mcp/src/server.rs` (только error-on-detect path в `resolve_workspace`) | `convert.rs`, `lock.rs` | Phase 2 |
| **W3** (lock refactor + e2e tests) | `crates/forgeplan-core/src/workspace/lock.rs`, `crates/forgeplan-mcp/tests/worktree_routing_e2e.rs` (new) | `server.rs` handlers, `init.rs` | Phase 3 |

**Конфликт точка**: `server.rs` — W1 и W2 оба touch это. Resolution: W1 пишет skeleton `resolve_workspace` first (Phase 1), W2 расширяет (Phase 2). W2 starts после W1 merge в integration branch.

**Worktree allocation** (per CLAUDE.md feedback-use-worktrees-per-parallel-worker):
- `git worktree add ../forgeplan-w1-prd078-phase1 feat/prd-078-phase-1-w1` (off `feat/prd-078-mcp-worktree-routing`)
- `git worktree add ../forgeplan-w2-prd078-phase2 feat/prd-078-phase-2-w2` (off feat/prd-078 после Phase 1 merge внутрь branch)
- `git worktree add ../forgeplan-w3-prd078-phase3 feat/prd-078-phase-3-w3`

**Disk check** (per feedback-disk-full-parallel-worktrees): `df -h` перед spawn; `cargo clean --manifest-path <worktree>/Cargo.toml` если >80% used.

---

## Test Plan

| AC | Test type | Location | Phase | Notes |
|----|-----------|----------|-------|-------|
| AC-1 | Integration e2e | `tests/worktree_routing_e2e.rs::test_subagent_in_worktree` | 3 | Setup: `tempdir + git init + git worktree add`. Assert: file in worktree, `resolved_via=param` |
| AC-2 | Regression | existing `cargo test --workspace` | 1, 2, 3 | 3084 → ≥3084 PASS — backward compat unchanged |
| AC-3 | Integration e2e | `tests/worktree_routing_e2e.rs::test_multi_worktree_missing_workspace_errors` | 2 | Assert: MCP -32602 error, message contains actionable suggestion |
| AC-4 | Integration e2e | `tests/worktree_routing_e2e.rs::test_ci_strict_mode_via_env_var` | 1 | Assert: env-based resolution works, `resolved_via=env` |
| AC-5 | Manual on user branch | TBD coordination после release | post-merge | Verify-projection.sh может быть упрощён |

---

## Affected Files

### Phase 1
- `crates/forgeplan-mcp/src/convert.rs` (~30 LOC: 3 params struct additions)
- `crates/forgeplan-mcp/src/server.rs` (~80 LOC: `resolve_workspace` + handler updates)

### Phase 2
- `crates/forgeplan-core/src/workspace/init.rs` (~50 LOC: `detect_multi_worktree`)
- `crates/forgeplan-mcp/src/server.rs` (~30 LOC: error-on-detect path)

### Phase 3
- `crates/forgeplan-core/src/workspace/lock.rs` (~40 LOC: per-resolved-path lock)
- `crates/forgeplan-mcp/src/server.rs` (~30 LOC: response payload additions)
- `crates/forgeplan-mcp/tests/worktree_routing_e2e.rs` (~250 LOC NEW: AC-1, AC-3, AC-4 e2e)

**Estimate total**: ~510 LOC изменений / additions. Realistic для one-PR review.

---

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| PRD-078 | PRD | based_on (this RFC implements PRD-078) |
| PROB-072 | Problem | informs (root signal) |
| PROB-067 | Problem | informs (lock race — Phase 3 fixes) |
| PROB-073 | Problem | informs (latency budget shared — R-1 closure cross-link) |
| ADR-NNN | ADR | decided_by (E over D' over A — будет создан отдельно) |
| ADR-003 | ADR | preserves (file-first invariant — Phase 1 не нарушает) |

---

> **Next step**: создать ADR `forgeplan new adr "MCP workspace resolution: error on multi-worktree detect"` с reasoning trace из PRD-078 Reasoning Trace section.







