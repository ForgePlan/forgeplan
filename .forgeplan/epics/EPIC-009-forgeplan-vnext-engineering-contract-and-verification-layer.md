---
depth: standard
id: EPIC-009
kind: epic
last_modified_at: 2026-08-01T18:53:48.273990+00:00
last_modified_by: claude-code/2.1.220
status: draft
title: 'ForgePlan vNext: engineering contract and verification layer'
---

# ForgePlan vNext: engineering contract and verification layer

Якорь программы vNext в графе. Материал программы лежит в
`docs/vnext/engineering-contract-layer/` — этот артефакт **ссылается** на него, а не
копирует.

## Problem

Сегодня `R_eff` считается по прозе, которую агент написал сам про себя. Измерено:
148 EvidencePack в `.forgeplan/evidence/`, из них **0** содержат `base_sha` или
`result_sha`. В `crates/forgeplan-core/src/scoring/` нет ни одной ссылки на
git-провенанс. То есть «доказательство» — это утверждение исполнителя о самом себе,
и текущая архитектура структурно не может отличить выполненную работу от рассказа
о выполненной работе.

Внесённый пакет предлагает закрыть этот разрыв, переопределив ForgePlan как
«repository-native engineering contract and verification layer»: Protocol v1,
WorkContract, ExecutionReceipt, EvidenceBundle, VerificationVerdict, AuthorityPolicy,
адаптеры хостов и оркестраторов, Marketplace v2, Web v2 и опциональный сервер —
16 задач FPV-00…FPV-15 в трёх репозиториях.

## Goals

- Проверять фактическую git-дельту вместо доверия к заявлению исполнителя.
- Сохранить правило слабейшего звена (`R_eff = min`, никогда не среднее).
- Довести коллизии программы с активными решениями до явно зафиксированного
  состояния, а не до состояния «обсудили в чате».

## Non-Goals

- Превращение Core в task tracker, scheduler, agent runtime или worktree manager.
- Создание 16 PRD авансом под ещё не принятую программу.
- Публикация копии Protocol v1 до посадки FPV-03/04/05.
- Индексация нешипнутых возможностей в `docs/README.md` — это реклама планируемого
  как готового.

## Target Users

Сопровождающий ForgePlan (принимает решение по программе) и агенты-исполнители,
которые пойдут по FPV-задачам после снятия блокеров.

## Success Criteria

- [ ] Ни одна FPV-задача не создаётся на GitHub, пока не закрыт PROB-082.
- [ ] Судьба каждого активного ADR, задетого границей, объявлена явно:
      unaffected / amended / superseded-by-new-ADR.
- [ ] Определено место хранения WorkContract / ExecutionReceipt / EvidenceBundle /
      VerificationVerdict относительно ADR-003 (markdown — source of truth).
- [ ] Слайс git-delta provenance gate (GitHub #360) отгружен против **текущей**
      границы, без зависимости от Protocol v1, адаптеров и сервера.

## Phases

Фазы взяты из `docs/vnext/engineering-contract-layer/issues/manifest.json` —
единственного источника порядка. Ни одна не начата.

| Фаза | Состав | Состояние |
|---|---|---|
| 0 — Alignment | FPV-01 (продуктовая граница) | **BLOCKED** — PROB-082 |
| 1 — Protocol + Docs | FPV-02, FPV-10 | BLOCKED — зависит от фазы 0 |
| 2 — Core foundations | FPV-03, FPV-04, FPV-05, FPV-06, FPV-07, FPV-08 | BLOCKED — FPV-03/04/05 дополнительно ждут решения по persistence (PROB-082 B3) |
| 3 — Conformance + Extensions | FPV-09, FPV-11 | BLOCKED |
| 4 — Host adapters | FPV-12 | BLOCKED — требует разрешения ADR-001 |
| 5 — Orchestrators + Web | FPV-13, FPV-14 | BLOCKED |
| 6 — Optional server | FPV-15 | Рекомендован к отклонению — дублирует ADR-018 |

Расхождение источников порядка: `EXECUTION-ORDER.md` объявляет FPV-07 параллельным
FPV-03, а `manifest.json` — зависимым от него. Требуется один источник.

## Artifacts

| Артефакт | Роль | Статус |
|---|---|---|
| PROB-082 | реестр коллизий с активными ADR — гейт FPV-01 | draft |
| PROB-083 | дефекты субстрата графа, обнаруженные по ходу | draft |
| EVID-149 | запись аудита из 13 агентов | draft |

Дочерние PRD/RFC/ADR **намеренно не созданы**: программа не принята, создавать
16 артефактов авансом — прямой Non-Goal.

## Status

**Программа НЕ принята.** Адверсариальный аудит из 13 агентов (10 ревьюеров +
план размещения + инвентарь потерь + синтез + критик) вернул `NEEDS_REWORK` по
всем девяти областям: 130 находок, из них 27 BLOCKER и 66 MAJOR.

Материал импортирован для ревью и сохранён. Исполнение заблокировано.

## Related

- Материал: `docs/vnext/engineering-contract-layer/`
- Провенанс импорта: `docs/vnext/engineering-contract-layer/_import/README.md`
- Отрендеренные тела 16 задач: `docs/vnext/engineering-contract-layer/issues/`

