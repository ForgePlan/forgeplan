---
depth: standard
id: PROB-082
kind: problem
last_modified_at: 2026-08-01T18:51:32.120732+00:00
last_modified_by: claude-code/2.1.220
links:
- target: ADR-001
  relation: contradicts
- target: ADR-009
  relation: contradicts
- target: ADR-011
  relation: contradicts
- target: ADR-003
  relation: based_on
- target: EPIC-009
  relation: informs
status: draft
title: vNext boundary contradicts four active ADRs with no supersession path
---

# vNext boundary contradicts four active ADRs with no supersession path

Реестр коллизий. Гейт: FPV-01 не стартует, пока каждая строка не получит вердикт
`unaffected` / `amended` / `superseded-by-new-ADR`.

## Problem

Пакет `docs/vnext/engineering-contract-layer/` переопределяет продуктовую границу
ForgePlan и при этом **ни разу не ссылается ни на один существующий ADR**.

Репродьюсер:

```
grep -rnoE 'ADR-[0-9]{3}' docs/vnext/ | wc -l   ->  0
grep -rn '\.forgeplan' docs/vnext/ | wc -l      ->  1   (и тот read-only)
```

Правило проекта — «supersede, do not delete». Здесь нет ни supersede, ни упоминания.
Если FPV-01 стартует как есть, первым действием программы станет молчаливое
переопределение четырёх активных решений.

## Collision register

| # | Коллизия | Вердикт | Требуемое действие |
|---|---|---|---|
| B1 | `ADR-001` (**active**) «Отвергаем adapter traits. Forgeplan НЕ интегрируется напрямую с внешними системами» + явно отвергнутые `TaskTracker (Orchestra, Linear)` и `Plugin system` ↔ `09-ADAPTER-ARCHITECTURE.md` (ports), `11-ORCHESTRATOR-INTEGRATIONS.md` (Kandev/Vibe/Conductor/Paperclip), Extensions | UNDECLARED_SUPERSESSION | Либо supersede ADR-001 с разбором, какое из его пяти обоснований перестало держаться, либо сузить ports так, чтобы они доказуемо не были external-system traits. Отмечу: плагины **уже шипнуты** (`forgeplan_plugins_list/info/doctor`), то есть реальность обогнала ADR-001 независимо от vNext |
| B2 | `ADR-001` («AI agent is the orchestrator, not Forgeplan») ↔ `ADR-009` («Forgeplan-core становится оркестратором»). Оба `status: active`, ни один не объявляет supersession | PRE-EXISTING CONTRADICTION | **Существует независимо от vNext и переживёт его отмену.** Делает AC задачи FPV-01 «One canonical ADR is active» недостижимым, пока человек не выберет сторону |
| B3 | `ADR-003` (**active**) «Markdown файлы = единственный source of truth» ↔ пакет нигде не говорит, где живут WorkContract / ExecutionReceipt / EvidenceBundle / VerificationVerdict + append-only authority log | NEEDS_HUMAN_DECISION | Блокирует FPV-03/04/05. Машинно-пишущиеся объекты (receipts, verdicts) в git-tracked дерево артефактов не кладутся без поправки к ADR-003, иначе RED LINE #11 становится неисполнимым. `ADR-018` уже отказал «второму authoritative не-markdown стору» — переиспользовать это рассуждение, не переигрывать |
| B4 | `ADR-011` (**active**) «Plugin/Agent dispatchers invoke `claude --print` directly» + шипнутый `playbook/dispatch/agent_dispatcher.rs` ↔ `01-PRODUCT-BOUNDARY.md` «не является coding agent / general-purpose workflow engine» | CONTRADICTION (шипнутый код против объявленной границы) | FPV-01 обязан назвать судьбу playbook-runtime: KEEP / MOVE-TO-EXTENSION / DEPRECATE |

## Промах в адресации

FPV-01 предлагает «решить судьбу `forgeplan_dispatch`». Проверено: `dispatch.rs`
объявлен pure-read и не мутирует; `CLAUDE.md` прямо пишет «планер… **НЕ спавнер**».
То есть **помечена безобидная поверхность**. Настоящий шедулер — `playbook::dispatch`
с `Delegation::Command`, `budget_usd` и `timeout_seconds` — в пакете не упоминается
ни разу.

Аудит также насчитал ещё около пятнадцати шипнутых поверхностей, попадающих под
собственный not-list пакета (`claim`/`release`/`claims` — это assignee-лизы;
`phase`/`phase_advance` — колонка канбана; `remember`/`recall` при явном запрете
«memory платформы общего назначения»). Требуется таблица диспозиций **по каждой**,
иначе граница ложна в день ноль по пятнадцати позициям, а не по одной.

## Impact

Блокирует: FPV-01 (гейт), далее FPV-02/03 (через B3), FPV-11/12/13 (через B1).

## Evidence

Аудит из 13 агентов, 130 находок (27 BLOCKER, 66 MAJOR), `NEEDS_REWORK` по всем
девяти областям. Все несущие числа перепроверены независимо.







