---
depth: tactical
id: EVID-139
kind: evidence
links:
- target: PRD-078
  relation: informs
status: draft
title: PRD-078 adversarial audit — 2-agent (security + architecture) verdict CONCERNS/fix-first
---

# EVID-139: PRD-078 adversarial audit — security + architecture

## Summary

2-agent adversarial audit собранного PRD-078 (10 commits на свежем origin/dev, branch feat/prd-078-integration). Security-expert + architect-reviewer, оба нашли ≥3 substantive issues. Code-reviewer agent оборвался mid-run (вывод усечён) — его dimensions покрыты architect'ом (mass-migration completeness, trace plumbing, test gaps). Оба независимо выдали verdict **CONCERNS / FIX-FIRST** — не BLOCKER (нет design defect, ADR-003 invariant сохранён), но активировать нельзя до закрытия HIGH findings.

## Structured Fields

verdict: weakens
congruence_level: 3
evidence_type: audit

## Verdict rationale

Audit weakens the "PRD-078 ready to activate" claim: 1 real concurrency bug (F-1) + 1 scope gap undermining end-to-end story (read tools) + several hardening/gate items. CL3 — direct audit of the exact code under review (same context). evidence_type=audit.

## Findings (severity ↓, deduped across both agents)

### HIGH-1 — Lock-key vs store-cache-key canonicalization mismatch (CWE-667/362, STRIDE-Tampering)
- **Where**: `server.rs:512` (cache key = `canonicalize(workspace_dir)`) vs lock sites (`acquire_workspace_lock(&ws)` with raw non-canonicalized path) + `lock.rs:78-79` + `init.rs:224-235` (`find_workspace` never canonicalizes).
- **What**: Two spellings of one logical workspace (`/x/repo` vs `/x/repo/` trailing slash, symlink alias, `/x/./repo`) → SAME cached `Arc<LanceStore>` (canonical key) but DIFFERENT `.lock` files (path-literal). Two writers each "acquire" the lock (different files → flock не сериализует) → mutate через один store handle → lost mutual exclusion → PROB-067 race reopens for ALL mutating handlers **except** `forgeplan_new` (которое использует отдельный id_alloc git-common-dir lock).
- **Fix**: canonicalize once in `resolve_workspace`, use single canonical path для cache key И lock. + regression test (`/x/repo` vs `/x/repo/` → same lock_path).

### HIGH-2 — Read tools NOT migrated → split-brain read/write model (architectural)
- **Where**: 21 read handlers (`forgeplan_get:3110`, `forgeplan_list:2281`, `forgeplan_score`, `forgeplan_validate`, ...) still call `require_workspace()` (frozen startup path), 0 accept `workspace` param.
- **What**: PRD-078 §Out-of-Scope ДЕКЛАРИРУЕТ это (reads deferred) — НЕ silent drift. НО следствие: в production-сценарии subagent пишет PRD в worktree через `forgeplan_new(workspace=…)`, читает обратно через `forgeplan_get` (no param) → запрос идёт в MAIN repo store → "not found" → **тот же Guardian loop, что PROB-072 хотел убить, смещён с write на read**. PRD §User-Journeys Journey-1 step-2 предполагает `forgeplan_get с workspace` — но `GetParams` НЕ имеет поля workspace. Journey-1 НЕ deliverable с текущим scope.
- **Decision needed (USER)**: (a) расширить scope — добавить workspace param read-tools тоже, ИЛИ (b) принять limitation, amend Journey-1, follow-up issue.

### MED-3 — NFR-001 latency bench deferred but declared pre-activate gate
- **Where**: PRD-078 SC-2 ("До PRD activate"), NFR-001 (<5ms p95), ADR-015 Evidence-Requirements + Post-conditions; bench file `crates/forgeplan-core/benches/workspace_detection.rs` listed в Affected-Files но НЕ существует в diff.
- **What**: design добавляет 2× `git rev-parse` subprocess на cold-start path каждого mutating call. Docstring говорит "~microseconds" — но process fork+exec обычно 1-10ms. PRD's own SC-2 требует измерить ДО activate. Shipping gate как "deferred to PROB-073" противоречит acceptance contract артефакта.
- **Fix**: либо run bench (PROB-073 / task #5), либо amend PRD/ADR — move bench из pre-activate в post-activate follow-up явно.

### MED-4 — Error paths bypass $HOME sanitizer (CWE-209, info disclosure)
- **Where**: `server.rs:322-497` — все error в `resolve_workspace` через `McpError::invalid_params(format!(...))` с raw `.display()`, минуя `safe_mcp_error`/`sanitize_error_chain` (SEC-H3 hardening). -32602 error (`:460-473`) embeds full canonical `suggested_workspace` (включая username).
- **Fix**: route через `safe_mcp_error`, маскировать $HOME → `<HOME>/...`, сохранив actionable suggested-path hint.

### MED-5 — Unbounded store cache; cap advisory-only (CWE-770)
- **Where**: `server.rs:532-541` — `WORKSPACE_STORE_CACHE_CAP=32` проверяется но `insert` выполняется unconditionally (warn-don't-evict).
- **What**: long-running server + many worktrees → один LanceStore handle на каждый canonical path FOREVER (fd + memory exhaustion). При stated scale (≤19 worktrees) не проблема, но latent incident в daemon.
- **Fix**: LRU eviction (lru уже в deps) или hard cap, или document+meter ceiling.

### MED-6 — detect_multi_worktree edge cases untested (ADR-015 Weakest-Link bar unmet)
- **Where**: ADR-015 §Weakest-Link называет submodules/nested-worktrees/symlinked-.git/bare-repos как gate, поднимающий trust 0.6→0.85; тесты (`init.rs:340-416`, `worktree_error_e2e.rs`) покрывают 0 из 4.
- **What**: `.parent()` assumption (`init.rs:277`) — fragile seam: для submodule/bare-repo может дать false-positive (error на legit single-worktree → NFR-002 regression) или false-negative (PROB-072 reopens).
- **Fix**: добавить ≥3 edge-case detection scenarios per ADR's own bar.

### LOW-7 — FORGEPLAN_DISABLE_WORKTREE_DETECT rollback hatch specified but NOT implemented
- **Where**: ADR-015 §Rollback step-1 называет env var как Phase-2 deliverable; architect не нашёл implementation в server.rs → paper control.
- **Fix**: implement OR strike из ADR (documented first-line rollback currently unavailable).

### LOW-8 — detect failure swallowed to false, no observability
- **Where**: `init.rs:266-296` (все None/Err → false), `git.rs` swallows non-zero exit.
- **What**: когда detection=false из-за git failure (vs genuinely-single) — нет log line. Thematic inconsistency с ADR thesis "silent fallback = root evil".
- **Fix**: `tracing::warn!` distinguishing assumed-single from confirmed-single.

### LOW-9 — symlink policy asymmetry (detection canonicalizes/follows vs lock refuses)
- **Where**: `init.rs:285-295` vs `lock.rs:86-101`. Folds into HIGH-1 fix (canonicalize once uniformly).

## Confirmed sound (with evidence)

- **ADR-003 file-first invariant: PRESERVED by construction** — every mutating handler resolves (workspace_dir, store) together via `resolve_workspace`, routes через `projection::MutationContext`. RED-LINE #8 clean: 0 direct `LanceStore::*_artifact` calls in server.rs/workspace. `tests/adr_003_invariant.rs` guard in force.
- **ADR-015 re-frame matches code: YES** — detection gate sits on cold-start branch only (after Step-3 early return for init'd servers). Production server (init'd, cwd=main) structurally bypasses gate → H1-param is the only correct router. Known Limitation table true given code. Honest artifact.
- **Backward compat (AC-2/NFR-002): preserved by construction** — Step-3 reuses exact legacy workspace_path+store for init'd servers, byte-identical to v0.32.x.
- **git command injection: CLEAN** — fixed-arg `Command::args`, no shell, cwd via `-C` separate argv, cwd always absolute (not caller-controlled).
- **Path traversal via workspace param: CLEAN** — ~expansion + is_relative() rejection + canonicalize; no privilege boundary to escape (local agent threat model).
- **Per-workspace lock (FR-006/PROB-067)**: closes for different-workspace case (proven by lock isolation test); HIGH-1 is the same-workspace-different-spelling gap.
- **Trace observability (FR-007/NFR-004)**: resolved_workspace + resolved_via in mutating responses, verified 5 tools.

## Activation recommendation

**FIX-FIRST** (both agents concur). Blockers before activate:
1. HIGH-1 (lock canonicalization) — real concurrency bug, ~10 line fix
2. HIGH-2 (read-tool scope) — USER DECISION required (expand scope vs accept+document)
3. MED-3 (bench gate) — run OR amend artifact

MED-4/5/6 + LOW-7/8/9 — fold into fix round or explicit follow-up issue.

## Provenance

- Security audit: agents-pro:security-expert, ~370K tokens, 77 tool calls, manual source audit (no scanners run — Rust workspace, disk-tight)
- Architecture review: agents-pro:architect-reviewer, ~257K tokens, 174 tool calls, static analysis + live reproducer for ADR-015 claim + both e2e suites pass + forgeplan-core compiles clean
- Code review: agents-core:code-reviewer — INCOMPLETE (output truncated mid-investigation); dimensions covered transitively by architect
- Pre-audit pipeline: fmt ✅ clippy ✅ test ✅ (~3100 tests, 0 failed)


