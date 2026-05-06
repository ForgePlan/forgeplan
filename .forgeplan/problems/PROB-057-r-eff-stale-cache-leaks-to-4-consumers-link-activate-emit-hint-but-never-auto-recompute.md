---
depth: standard
id: PROB-057
kind: problem
status: active
title: R_eff stale cache leaks to 4 consumers — link/activate emit hint but never auto-recompute
---

# PROB-057: R_eff stale cache leak — link/activate hints don't enforce recompute

## Signal

В session 2026-05-06 при подготовке PROB-053 PR был обнаружен расхождение между **stored** и **live** R_eff:

| Артефакт | `forgeplan get` (stored) | `forgeplan score` (live) |
|---|---:|---:|
| PRD-074 (linked to EVID-104) | **0.00** | 1.00 |
| RFC-008 (based_on PRD-074) | 0.00 | 0.90 |
| PROB-053 (linked to EVID-104) | 0.00 | 1.00 |

Despite EVID-104 being linked + `active` + содержащим валидные structured fields (verdict, CL3, evidence_type), cached `r_eff_score` в LanceDB остался `0.00` пока не запустили `forgeplan score <ID>` руками.

**Корень**: `forgeplan link` (`crates/forgeplan-cli/src/commands/link.rs:33-39`) и `forgeplan activate` (`crates/forgeplan-cli/src/commands/activate.rs:93-97`) только **emit hint** `Hint::info("verify R_eff").with_action("forgeplan score <ID>")` — никогда не вызывают `r_eff_recursive` + `update_r_eff_score` напрямую. Контракт держится **только если consumer (LLM/operator) выполнит hint**. Hint можно пропустить (timeout, переключение задач, agent automation skipping rules) — тогда cached value остаётся stale **до бесконечности**.

## Constraints

- **MUST NOT** ломать существующий `forgeplan score` / `score-all` workflow — explicit recompute остаётся основным entry point для batch reconciliation.
- **MUST NOT** привести к performance regression > 100ms на typical `link` / `activate` invocation (current baseline ~30-50ms).
- **MUST** работать identically в LanceDB (production) и InMemoryStore (tests) drivers.
- **MUST** preserve ADR-003 invariant — markdown source of truth, LanceDB derived index.
- **MUST NOT** требовать schema migration без explicit backwards-compat path (workspaces до v0.30 продолжают читаться).

## Optimization Targets

- **Eliminate stale state в 4 downstream consumers** (UI, search filter, F-G-R quality, LLM ADI context) — все читают `r_eff_score` без проверки актуальности.
- **Reduce operator cognitive load** — пользователь не должен помнить про `forgeplan score` после каждого `link/activate`.
- **Detect drift automatically** — current state даёт zero сигналов когда cache stale.

## Observation Indicators (Anti-Goodhart)

- Total `update_r_eff_score` invocations per session — НЕ оптимизировать вниз (могут отражать healthy churn evidence).
- Average `r_eff_recursive` execution time — мониторить, но не сделать единственным KPI (рекомпьютс на dense graph будут естественно медленнее).
- Number of artifacts с `r_eff_score == 0.0` — не оптимизировать к нулю (legitimate cases: новый artifact без evidence).

## Acceptance Criteria

После закрытия PROB-057 все следующие checks должны проходить **без ручного `forgeplan score`**:

- [ ] **AC-1**: После `forgeplan link <SRC> <EVID> informs` cached `r_eff_score` у `<SRC>` отражает recomputed value (не остаётся 0.00).
- [ ] **AC-2**: После `forgeplan unlink <SRC> <EVID> informs` cached `r_eff_score` у `<SRC>` пересчитан.
- [ ] **AC-3**: После `forgeplan activate <ID>` cached `r_eff_score` у `<ID>` соответствует live recursive computation.
- [ ] **AC-4**: `forgeplan list --has-evidence` (если такая опция exists) возвращает корректный список после link без manual score.
- [ ] **AC-5**: F-G-R quality grade в `forgeplan score` output не регрессирует на artifacts которые previously имели grade A через ручной score flow.
- [ ] **AC-6**: LLM ADI prompt context (`crates/forgeplan-core/src/llm/reason.rs:218`) включает свежий R_eff в `**R_eff score**` line.
- [ ] **AC-7**: Performance regression tests — `link` и `activate` operations не превышают 200ms на test workspace с >100 artifacts.
- [ ] **AC-8**: Tests cover both LanceStore и InMemoryStore drivers — same behavior.

## Blast Radius

| Surface | File:line | Impact when stale |
|---|---|---|
| `forgeplan get` UI | `crates/forgeplan-cli/src/commands/get.rs:80` | User думает evidence не работает (этим session подтверждено) |
| Search filter `--has-evidence` | `crates/forgeplan-core/src/search/filter.rs:93-94` | False negative — свежие linked artifacts не попадают в `r.r_eff_score > 0.0` query |
| F-G-R quality computation | `crates/forgeplan-core/src/scoring/fgr.rs:150` | `score = r_eff_score * w.reff` → grade Formality/Granularity/Reliability **wrong** |
| LLM ADI context | `crates/forgeplan-core/src/llm/reason.rs:218` | Reasoning prompt получает stale R_eff → influence на hypothesis ranking |

Дополнительные потенциальные impacted areas (TBD при ADI):
- `forgeplan health` derived status — может ошибочно показывать UNDERFRAMED вместо EVIDENCED
- `forgeplan blocked` decision gate — если читает stored vs live
- MCP tools — `forgeplan_get`, `forgeplan_search`, `forgeplan_score` parity

## Reversibility

**High** — fix локализован в pull-vs-push логике scoring; rollback через `git revert <commit>` без schema migration. Existing `forgeplan score` / `score-all` flow остаётся load-bearing fallback в любой опции.

## Options Considered (для PRD ADI)

### Option A — Auto-recompute on link/activate

`link/unlink/activate/deprecate/supersede` после persist mutation вызывают `r_eff_recursive` + `update_r_eff_score` для source artifact + всех parent artifacts up the chain.

**Pros**: cache always fresh; никаких hints не теряется; existing semantics preserved.
**Cons**: performance hit на dense graph (recursive walk через все parents); coupling между mutator и scoring.

### Option B — Live computation в `get`

`forgeplan get` всегда вызывает `r_eff_recursive` вместо чтения cached field. Stored `r_eff_score` deprecated to "best-effort hint".

**Pros**: UI всегда показывает свежий value.
**Cons**: `get` становится медленнее (recursive walk per call); НЕ лечит search/F-G-R/LLM consumers (они тоже должны переключиться на live).

### Option C — Dirty flag + lazy recompute

Schema bump: добавить `r_eff_dirty: bool` в Record. Mutators (`link/unlink/activate/deprecate/supersede` + evidence body update + valid_until expiry) ставят `dirty=true`. Readers (`get`, search filter, F-G-R, LLM context) видят `dirty` → recompute on-demand + clear flag.

**Pros**: best of both — cheap mutations + always-fresh reads; explicit invariant в schema.
**Cons**: schema migration; touch ~6 read sites; нужен durability story (что если crash между set-dirty и persist value).

### Option D — Status quo + UX-индикатор (status quo +)

Добавить `r_eff_updated_at: Option<DateTime>` field; `get` показывает warning `R_eff: 0.00 (stale, run forgeplan score)` если cached older than threshold (e.g. 1h since last `link/activate`).

**Pros**: минимальный scope; четкий signal к operator/LLM.
**Cons**: НЕ лечит search filter, F-G-R, LLM (они silently возвращают stale data); только UI fix.

## Related Artifacts

| Artifact | Relation |
|---|---|
| PRD-073 | informs (ADR-003 file-first invariant — solution не должно нарушать markdown=truth) |
| EPIC-003 | parent (multi-agent dispatch infrastructure — touched через scoring consumers) |
| PRD-071 | informs (Hint Protocol — текущее решение через hints не работает в practice) |
| ADR-003 | informs (file-first storage — recompute path должен respect projection helpers) |





