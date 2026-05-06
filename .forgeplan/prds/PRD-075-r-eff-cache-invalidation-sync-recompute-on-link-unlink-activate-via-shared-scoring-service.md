---
depth: standard
id: PRD-075
kind: prd
links:
- target: PROB-057
  relation: refines
status: active
title: R_eff cache invalidation — sync recompute on link/unlink/activate via shared scoring service
---

# PRD-075: R_eff cache invalidation — sync recompute on link/unlink/activate

## Problem

`forgeplan link` (`crates/forgeplan-cli/src/commands/link.rs:33-39`) и `forgeplan activate` (`crates/forgeplan-cli/src/commands/activate.rs:93-97`) после mutation эмитят hint `forgeplan score <ID>` для пересчёта R_eff, но **не вызывают** `r_eff_recursive` + `update_r_eff_score` сами. Cached значение `r_eff_score` в LanceDB остаётся stale до тех пор, пока operator/LLM не выполнит hint вручную.

Прямая утечка stale state в **4 разных downstream consumer'а**:

| Surface | File:line | Impact when stale |
|---|---|---|
| `forgeplan get` UI | `crates/forgeplan-cli/src/commands/get.rs:80` | User видит R_eff=0.00 несмотря на linked evidence — confusion |
| Search filter `--has-evidence` | `crates/forgeplan-core/src/search/filter.rs:93-94` | False negative — `r.r_eff_score > 0.0` пропускает свежие linked artifacts |
| F-G-R quality computation | `crates/forgeplan-core/src/scoring/fgr.rs:150` | `score = r_eff_score * w.reff` → grade Formality/Granularity/Reliability **wrong** |
| LLM ADI context | `crates/forgeplan-core/src/llm/reason.rs:218` | Reasoning prompt получает stale R_eff → влияние на hypothesis ranking |

**Reproducer (session 2026-05-06)**: после `forgeplan link PRD-074 EVID-104 informs` значение `forgeplan get PRD-074 → R_eff: 0.00`, тогда как `forgeplan score PRD-074 → R_eff: 1.00 Adequate`. PROB-057 фиксирует наблюдение, эта PRD scopes решение.

**Impact**: trust calculus всего workspace silently работает на stale данных в любой момент после link/activate без явного score. CLAUDE.md red line #7 («R_eff must be > 0 для активации») формально соблюдён через `score-all` runs, но между ними есть стабильное окно неконсистентности.

## Goals

1. **Eliminate stale cache window** (FR-001, FR-002, FR-003): после успешного `link/unlink/activate` cached `r_eff_score` source-артефакта **синхронно** пересчитывается + persist'ится. Operator больше никогда не видит stale value в `forgeplan get` после mutation.
2. **Shared scoring service** (FR-004): экстракт `r_eff_recursive` + `update_r_eff_score` invocation в `forgeplan_core::scoring::sync_score_target` helper. Эта функция — единственная точка входа для "recompute + persist" pattern. CLI `score` / `score-all` / mutators все вызывают её.
3. **Bounded performance** (FR-005): single-target recompute outside parents (cap depth = 1 для default mutator path). `score-all` остаётся для full-tree reconciliation.
4. **Driver parity** (FR-006): identical behavior в `LanceStore` и `InMemoryStore` (test-helpers feature). Tests cover both.
5. **Backwards compatible** (FR-007): no schema change. Existing `forgeplan score` / `score-all` workflow unchanged. Workspaces ниже v0.30 продолжают читаться без миграции.
6. **Regression coverage** (FR-008): test matrix покрывает link/unlink/activate paths + downstream consumer (search filter) для предотвращения reintroduction.
7. **Hint Protocol consistency** (FR-009, references PRD-071): hints после mutation отражают новое состояние (recompute уже сделан) — не предлагают "forgeplan score" если уже done.

## Non-Goals

- **Auto-recompute parents up the chain** (full graph propagation): out of scope. Если `EVID-104` linked к `PRD-074` и `PRD-074 ← RFC-008 (based_on)`, `link` пересчитает только `PRD-074`. RFC-008 пересчитается на следующем link/activate ИЛИ через `forgeplan score-all`. Глубокая каскадная инвалидация — отдельный follow-up если измерения покажут problem.
- **Live computation в `get`** (Option B из PROB-057): отвергнуто ADI — лечит только UI, не лечит search/F-G-R/LLM.
- **Dirty flag schema bump** (Option C): отвергнуто — over-engineered для текущего scope. Возвращаемся если sync recompute упрётся в performance.
- **UX-индикатор stale** (Option D): отвергнуто — не лечит non-UI consumers.
- **Concurrency / lock-free recompute**: out of scope. Single-process sync OK; multi-agent dispatch (PRD-057) использует `forgeplan_dispatch_claim` для serialization mutations.
- **MCP tool semantic changes**: MCP `link/activate` уже наследуют CLI behavior через shared core; их parity — implementation detail, не отдельный goal.

## Functional Requirements

- [ ] **FR-001**: `forgeplan link <SRC> <TGT> <REL>` после успешного `add_link_with_projection` синхронно вызывает `sync_score_target(SRC)` который пересчитывает + persist'ит R_eff source-артефакта. Вышибает `forgeplan score <ID>` hint в случае success (hint остаётся для drill-down `score-all`).
- [ ] **FR-002**: `forgeplan unlink <SRC> <TGT> <REL>` после `delete_link_with_projection` синхронно вызывает `sync_score_target(SRC)`.
- [ ] **FR-003**: `forgeplan activate <ID>` после `lifecycle::activate` + render_projection синхронно вызывает `sync_score_target(ID)` для самого активируемого артефакта.
- [ ] **FR-004**: Новый helper `forgeplan_core::scoring::sync_score_target(store, id) -> Result<f64>` — единый entry point для "recompute current + persist". Вызывается из `link.rs`, `activate.rs`, `score.rs::run`, `score.rs::run_all`. Body инкапсулирует `r_eff_recursive` + `update_r_eff_score`.
- [ ] **FR-005**: `sync_score_target` соблюдает performance budget — average latency < 100ms на test workspace с 100+ artifacts. Регрессия теста требует обоснования и записи в `## Open Questions` следующего PRD.
- [ ] **FR-006**: Behavior identical в `LanceStore` (production). InMemoryStore parity formally **deferred to PROB-058** (Round 8 audit HIGH-1 — current `sync_score_target` signature is hardcoded к `&LanceStore`, not the `StorageDriver` trait, so InMemoryStore tests cannot exercise the helper end-to-end). PROB-057 closure covers LanceStore-only; cross-driver parity is structural follow-up tracked in PROB-058 AC-1.
- [ ] **FR-007**: Schema unchanged. `r_eff_score` field stored as before. No migration required. Workspaces opened from v0.29 продолжают работать identically.
- [ ] **FR-008**: Tests cover regression matrix:
  - link → get показывает correct R_eff (без manual `forgeplan score`)
  - unlink → get показывает recomputed R_eff
  - activate → get показывает correct R_eff
  - search filter `--has-evidence` (если есть CLI flag) — корректный список после link
- [ ] **FR-009**: Hint Protocol (PRD-071) compliance — после auto-recompute `link`/`unlink`/`activate` всё ещё эмитят `Next:` hint но другой (например, "verify chain → forgeplan score-all"); refuse path не меняется.

## Target Users

- **LLM-агенты** работающие с forgeplan через CLI/MCP — не должны помнить про `forgeplan score` после каждого link/activate. Auto-recompute убирает этот failure mode.
- **Operators (humans)** в interactive sessions — `forgeplan get` сразу показывает корректный R_eff после link, не нужен следующий step.
- **CI pipelines** запускающие `forgeplan health` после batch операций — F-G-R quality grade метрики не должны быть stale.
- **Future MCP consumers** через `forgeplan_link` / `forgeplan_activate` API — same auto-recompute behavior as CLI.

## Related Artifacts

| Artifact | Relation |
|---|---|
| PROB-057 | refines (this PRD scopes the closure of PROB-057 R_eff stale cache leak) |
| PRD-074 | informs (the session that exposed the bug — PRD-074 sprint observed stale cache на `forgeplan get`) |
| PRD-073 | informs (ADR-003 file-first invariant — `sync_score_target` использует только `update_r_eff_score`, не trogues markdown directly) |
| PRD-071 | informs (Hint Protocol — после fix hints тоже обновляются, не остаются "verify R_eff" если уже сделано) |
| ADR-003 | informs (file-first storage — projection helpers остаются единственным write path для structural changes) |
| EPIC-003 | parent (multi-agent dispatch infrastructure — touched через scoring consumers; sync recompute упрощает scoring contract для multi-agent workflows) |

## Out of Scope

- Каскадная invalidation parents up the chain (deferred — see Non-Goals)
- Live computation в `get` (Option B rejected by ADI)
- Schema-level dirty flag (Option C — over-engineered)
- **StorageDriver trait parity** для `sync_score_target` — deferred to PROB-058 AC-1 (требует rework `r_eff_recursive` signature; not in this sprint)
- **Workspace lock policy для `forgeplan score` / `score-all`** — closed in PROB-058 AC-2 (extended fix in same sprint)
- **Bounded recursive walk in mutator path** (DoS hardening на deep graphs) — deferred to PROB-058 AC-3 (требует separate `r_eff_local` variant + perf benchmark scaffold)
- **Side-channel mitigation** (mutation latency leaks graph topology) — accepted residual risk, see threat model below
- `r_eff_updated_at` timestamp field (Option D — partial fix только для UI)
- Multi-process / multi-agent concurrent recompute (uses existing claim/release contract)

## Threat Model — Mutation Latency Side-Channel (PROB-058 AC-5)

`sync_score_target` invokes `r_eff_recursive` synchronously inside the workspace lock window during `link` / `unlink` / `activate`. The recursive walk's runtime is roughly `O(D × E)` (depth × evidence-per-node). An adversary with `forgeplan link` permission can probe the graph by timing link operations to different targets и infer dense vs sparse evidence chains.

**Mitigation posture**: Forgeplan не targets the multi-tenant adversarial deployment где this attack matters. `link` permission is held by the workspace owner; PRD-057 multi-agent dispatch shares the owner's permission already. Constant-time mutation paths would require a major architectural refactor (background recompute queue, async cache invalidation) without proportional benefit at the current trust model. PROB-058 AC-3 (bounded recursive walk) when closed will reduce the variance window incidentally.

**Trigger to revisit** (escalates residual to actionable):

- `link` permission scope expands beyond workspace owner.
- Multi-tenant deployment (single Forgeplan instance shared by independent organizations).
- Public-facing forge service exposing `forgeplan_link` MCP tool to untrusted callers.

Until any trigger fires, this is documented и accepted.



