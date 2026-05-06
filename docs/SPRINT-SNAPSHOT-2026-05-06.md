# Sprint Snapshot — feat/prob-053-allow-shell-gate (2026-05-06)

> Pre-push audit fix point. Branch `feat/prob-053-allow-shell-gate` carries
> 3 commits ahead of `dev`. Bundles PROB-053 + PROB-057/PRD-075 + PROB-058
> partial closure. Working tree clean, all gates green, NOT pushed.
>
> Companion to: [`ROADMAP-2026-05-06.md`](ROADMAP-2026-05-06.md).

---

## 1. Что закрыто в этом branch

### Commit `4784f5a` — PROB-053 / PRD-074 / RFC-008 (security gate)

| Item | Status | Notes |
|---|---|---|
| PROB-053 `Delegation::Command` CWE-78 surface | ✅ active | Default-deny shell exec |
| PRD-074 shell-execution gate scope | ✅ active, R_eff=1.00 | FR-001..007 all checked |
| RFC-008 implementation phases | ✅ active, R_eff=0.90 | Invariants + Rollback Plan added |
| EVID-104 Round 7 audit closure | ✅ active | 1985 tests, 3 audit agents |

Roadmap alignment: **was Wave 1 #1 в Tier 2 (v0.30.0)**, scheduled `2026-05-06, 4d`. Closed in 1 day.

### Commit `e6f3352` — PROB-057 / PRD-075 / EVID-105 (cache self-healing)

| Item | Status | Notes |
|---|---|---|
| PROB-057 R_eff stale cache leak | ✅ active, R_eff=1.00 | Discovered этим session |
| PRD-075 sync_score_target design | ✅ active, R_eff=1.00 | FR-001..009 |
| EVID-105 Round 8 audit closure | ✅ active | 1993 tests, 2 audit agents |
| `sync_score_target` helper в `forgeplan_core::scoring` | ✅ shipped | Single canonical recompute+persist entry point |
| `sync_score_target_or_warn` CLI wrapper | ✅ shipped | DRY + Fix: marker |
| `reconcile_parents_hint()` constant | ✅ shipped | FR-009 single-source string |

Roadmap alignment: **NOT в roadmap** (discovered post-roadmap date). PROB-057 + PRD-075 — emergent finding из PROB-053 PR review session. Закрытие предотвращает silent stale state в 4 downstream consumers (UI, search filter, F-G-R, LLM ADI context).

### Commit `db0f80a` — PROB-058 partial + Round 9 closure

| Item | Status | Notes |
|---|---|---|
| PROB-058 R_eff helper deferrals | ✅ active, R_eff=1.00 | 4 of 6 ACs closed in this commit |
| MCP `forgeplan_link` parity | ✅ shipped | Lock + sync_score_target |
| MCP `forgeplan_activate` parity | ✅ shipped | Lock + sync (ordered before render) |
| MCP `forgeplan_score` persist | ✅ shipped | **Latent bug fixed** — pre-Round-9 это never persisted |
| `forgeplan score` workspace lock | ✅ shipped | Single + batch via `open_store_locked` |
| Concurrent-writer regression test | ✅ shipped | Real fs2 lock test, 2 processes |
| FR-009 negative coverage (link/unlink/activate) | ✅ shipped | Line-shape match, 3 tests |
| PRD-075 §Threat Model (side-channel) | ✅ shipped | Mitigation posture + trigger conditions |
| `sync_score_target` docstring rewrite | ✅ shipped | 3-concern distinction (Round 9 HIGH-3) |

Roadmap alignment: **NOT в roadmap** (Round 8/9 audit follow-ups). PROB-058 fills the architectural gap that Round 8 + Round 9 audits surfaced.

---

## 2. Что найдено и требует внимания (открытые items)

### 2.1 В этом sprint — known trade-offs (документированы)

| Issue | Location | Mitigation | Tracker |
|---|---|---|---|
| `score --all` holds workspace lock for entire batch | `score.rs::run_all` | Documented в PROB-058 AC-3 + CHANGELOG. Operators schedule batch outside mutator windows. | PROB-058 AC-3 |
| MCP transport timing side-channel | `forgeplan_link` mutation latency | PRD-075 §Threat Model — accepted residual risk under current single-operator threat model + trigger-to-revisit conditions | PROB-058 AC-5 (closed as documented) |
| Branch bundles 3 PROBs | `feat/prob-053-allow-shell-gate` | User-requested cohesive sprint; Round 9 review/security оба советовали SPLIT для review burden | per-user decision |

### 2.2 PROB-058 deferred ACs (separate sprint)

| AC | Status | Why deferred | Effort estimate |
|---|---|---|---|
| **AC-1** `sync_score_target` driver-trait parity | ⏳ Deferred | Требует rework `r_eff_recursive` signature (entire scoring pipeline `&LanceStore`-bound); RFC pending | M (3-5d) |
| **AC-3** `r_eff_local` perf-bound variant | ⏳ Deferred | Нужен benchmark scaffold; current FR-005 100ms budget держится в практике (workspace ≤300 artifacts) | S-M (2-3d) |

### 2.3 Audit findings deferred (Round 7/8/9, but not blocking)

Все blocking HIGH findings закрыты. Документировано в EVID-104 (Round 7) + EVID-105 (Round 8 + Round 9). Non-blocking deferrals:

- **Round 8 review MED-4** — tests bypass projection helper. Kept as fast-path; full E2E covered by `cli_reff_cache_invalidation.rs`.
- **Round 9 sec MED-1** — `score --all` lock window — see 2.1.
- **Round 9 sec MED-2** — N+1 evidence corpus reload в `score --all` — tracker via PROB-058 AC-3.
- **Round 9 review LOW-2** — `_lock` binding fragility — would need `#[deny(let_underscore_drop)]` lint addition.

### 2.4 Health workspace orphans (carry-over, не from sprint)

`forgeplan health` reports 2 orphans (PRD-001, SPEC-001). These are **stale LanceDB rows** from removed scan-import smoke-test artifacts. The .md files were cleaned up в этом sprint (LOW-3 closure), но LanceDB index не purges automatically — это **PROB-028** "phantom rows after .md delete", уже tracked в roadmap Tier 2 Wave 3.

**Verdict**: not a sprint regression, pre-existing issue.

---

## 3. Roadmap alignment — идём ли мы по плану?

### Tier 2 v0.30.0 — Defensive sprint (target ~2 weeks)

| Roadmap item | Effort | Status now |
|---|---|---|
| **Wave 1.1**: PROB-053 `Delegation::Command` gate | M-L (3-5d) | ✅ Closed (1 day) |
| **Wave 1.2**: PROB-052 TOCTOU + symlink-follow | M (2-3d) | ⏳ Open — next sprint |
| **Wave 1.3**: PROB-051 Wave-1 Round 5 architectural | L (5-7d) | ⏳ Open — next sprint |
| **Wave 2.1**: PROB-054 `produces_at` validator | XS (0.5d) | ⏳ Open |
| **Wave 2.2**: PROB-056 leaky verdict abstraction | S (1d) | ⏳ Open |
| **Wave 2.3**: PROB-049 follow-up retry-loop consumer | M (2-3d) | ⏳ Open |
| **Wave 3**: paper cuts batch (028/027/030/032/033/041/038) | ~5d | ⏳ Open |
| **NEW (not roadmapped)**: PROB-057 + PRD-075 R_eff cache | unbudgeted | ✅ Closed |
| **NEW (not roadmapped)**: PROB-058 partial (4 of 6 ACs) | unbudgeted | ✅ Closed |

**Status**: ahead of plan на Wave 1.1 (1 day vs 3-5 day estimate). Added 2 emergent PROBs that не были в roadmap. Wave 1.2 (PROB-052) + Wave 1.3 (PROB-051) остаются для следующего sprint.

### Что меняется в roadmap после этого PR

После merge в `dev`:
- PROB-053 закрыт → удалить из CRITICAL section
- PROB-057 + PRD-075 + PROB-058 partial → добавить в "Что закрыто в v0.30.0" section
- PROB-058 AC-1 + AC-3 → добавить в Tier 2 Wave 2 как новые items
- Roadmap snapshot file → можно создать `ROADMAP-2026-05-XX.md` после merge с обновлёнными счётчиками

---

## 4. Audit + E2E inventory

### Adversarial audits run

| Round | Sprint | Agents | Findings | Closed in PR | Deferred |
|---|---|---|---:|---:|---:|
| **Round 7** | PROB-053 | 3 (architect + code-reviewer + security) | 9+ | All 9 HIGH/MED closed | F3/F4 + LOW cosmetic to PROBs |
| **Round 8** | PROB-057 | 2 (security + code-reviewer) | 18 (8 sec + 10 review) | 8 (HIGH-3 sec, MED-2/3/4 sec, MED-1/2 review, LOW-2/3 review) | 10 to PROB-058 |
| **Round 9** | PROB-058 | 2 (security + code-reviewer) | 17 (10 sec + 8 review, overlapping) | 8 (HIGH-1/2/3 sec, MED-3/4 sec, HIGH-1/2/3 review, MED-1/2/3 review) | 4 (MED-1/5 sec doc, LOW-1/2 review) |

**Total: 3 audit rounds, 7 agent invocations. 25+ findings closed, 14+ deferred-but-tracked.**

### E2E test inventory (real release binary)

| Coverage | Test type | Count | Result |
|---|---|---:|---|
| `forgeplan link` auto-recompute (CLI) | CLI integration | 1 | ✅ |
| `forgeplan unlink` auto-recompute (CLI) | CLI integration | 1 | ✅ |
| `forgeplan activate` auto-recompute (CLI) | CLI integration | 1 | ✅ |
| FR-009 hint contract negative (link / unlink / activate) | CLI integration | 3 | ✅ |
| Concurrent `score --all` × 2 processes via fs2 lock | CLI integration | 1 | ✅ |
| `sync_score_target` unit tests (no-evidence, stale-cache, unknown-id, malformed-id, cycle) | Unit | 5 | ✅ |
| **Total automated tests touched сегодня** | — | **11 new + integration suites** | ✅ |
| Manual real E2E на `target/release/forgeplan` | Release binary | 4 cells (link → get → unlink → activate → score-all) | ✅ |

### Quality gates (final state)

```
cargo fmt --check                                              clean
cargo clippy --workspace --all-targets --features test-helpers
  -- -D warnings                                               clean
cargo test --workspace --features test-helpers                 0 failures (38 suites)
cargo build --release                                          clean
forgeplan health                                               2 orphans (PROB-028 carry-over, not regression)
```

### Tests baseline

| Stage | Count | Delta |
|---|---:|---:|
| Pre-sprint (v0.29.0 baseline) | 1977 | — |
| After PROB-053 (EVID-104) | 1985 | +8 (Round 7 closures) |
| After PROB-057 (EVID-105) | 1993 | +8 (5 unit + 3 CLI integration) |
| After PROB-058 (this commit) | ~1996+ | +3 (1 concurrent-writer + 2 hint-negative) |

---

## 5. CLAUDE.md red lines compliance check

| # | Red line | Status |
|---|---|---|
| 1 | DO NOT `rm -rf .forgeplan` | ✅ Not invoked |
| 2 | DO NOT `git push` until user approves | ✅ **Holding — awaiting user approve** |
| 3 | DO NOT commit directly to main/dev | ✅ All work on feature branch |
| 4 | DO NOT push after PR merge (squash loss) | N/A (no merge yet) |
| 5 | DO NOT create PR before Code → Audit → Fix → Test → Fmt → Lint → Verify | ✅ Pipeline complete: 3 audit rounds + real E2E на release binary |
| 6 | DO NOT leave PRD stubs | ✅ PRD-074 + PRD-075 fully filled |
| 7 | DO NOT activate без code AND evidence | ✅ EVID-104 + EVID-105 backing R_eff=1.00 на active artifacts |
| 8 | DO NOT call `LanceStore::*` directly от commands/server.rs | ✅ All mutations через `projection::*` helpers; new `sync_score_target` calls `update_r_eff_score` only (allowed cache write, не structural) |
| 9 | DO NOT skip post-release sync | N/A (no release yet — this is feat branch) |
| 10 | DO NOT ignore Dependabot alerts at release | N/A (this is feat branch, not release) |

**All red lines respected.**

---

## 6. PROB-058 AC tracking (final state)

| AC | Status | Detail |
|---|---|---|
| AC-1 driver-trait parity | ⏳ Deferred | Requires `r_eff_recursive` signature rework |
| AC-2 score lock policy | ✅ **Closed** | + concurrent-writer regression test |
| AC-3 `r_eff_local` perf bound | ⏳ Deferred | Needs benchmark scaffold |
| AC-4 negative hint contract | ✅ **Closed** | 3 tests, line-shape match |
| AC-5 side-channel doc | ✅ **Closed** | PRD-075 §Threat Model |
| AC-6 docstring scope | ✅ **Closed** | 3-concern distinction |

**4 of 6 ACs closed (66.7%). Remaining 2 require structural work outside this sprint scope.**

---

## 7. Decision points для пользователя

1. **Push-strategy**: single bundled PR vs split на 3 (Round 9 LOW-3 recommendation)
2. **PROB-052 next**: следующий sprint — TOCTOU `which_in_path` (Roadmap Wave 1.2)
3. **PROB-058 follow-up**: открыть отдельный sprint для AC-1 + AC-3 или подождать пока driver-trait RFC появится?
4. **Roadmap snapshot rewrite**: после merge — сгенерировать новый `ROADMAP-2026-05-XX.md` с обновлёнными счётчиками?
5. **Hindsight memory**: retain ключевые findings (sync_score_target architecture, MCP transport parity lesson, push-vs-pull cache invalidation trade-off)?

---

## 8. Files changed (full inventory)

### Modified

```
CHANGELOG.md                                          +163
crates/forgeplan-cli/src/commands/activate.rs         +21 -10
crates/forgeplan-cli/src/commands/common.rs           +15
crates/forgeplan-cli/src/commands/link.rs             +25 -23
crates/forgeplan-cli/src/commands/playbook.rs         +71 -1   (PROB-053)
crates/forgeplan-cli/src/commands/score.rs            +72 -25
crates/forgeplan-cli/src/main.rs                      +12 -1   (PROB-053)
crates/forgeplan-cli/tests/integration_phase6_e2e.rs  +14 -8   (PROB-053)
crates/forgeplan-core/src/config/types.rs             +53      (PROB-053)
crates/forgeplan-core/src/hints.rs                    +15
crates/forgeplan-core/src/playbook/dispatch/*         +149 +81 +38   (PROB-053)
crates/forgeplan-core/src/scoring/mod.rs              +214
crates/forgeplan-mcp/src/server.rs                    +156 -10
crates/forgeplan-mcp/src/types.rs                     +10 -1   (PROB-053)
marketplace/playbooks/release.yaml                    +9       (PROB-053)
```

### Created

```
.forgeplan/prds/PRD-074-shell-execution-gate-...md             (PROB-053)
.forgeplan/rfcs/RFC-008-shell-execution-gate-...md             (PROB-053)
.forgeplan/problems/PROB-053-...md                             (PROB-053)
.forgeplan/evidence/EVID-104-...md                             (PROB-053 closure)
.forgeplan/prds/PRD-075-r-eff-cache-invalidation...md          (PROB-057)
.forgeplan/problems/PROB-057-r-eff-stale-cache-leaks...md      (PROB-057)
.forgeplan/problems/PROB-058-r-eff-helper-deferrals...md       (PROB-058)
.forgeplan/evidence/EVID-105-prob-057-prd-075-closure...md     (PROB-057+58 closure)
crates/forgeplan-cli/tests/cli_reff_cache_invalidation.rs      (FR-008/AC-2/AC-4)
docs/ROADMAP-2026-05-06.md                                     (post-v0.29.0 snapshot)
docs/SPRINT-SNAPSHOT-2026-05-06.md                             (this file)
```

**Total**: 27 files changed, +2677 -115 (excluding this snapshot file).

---

*Snapshot generated 2026-05-06. Companion: ROADMAP-2026-05-06.md.*
