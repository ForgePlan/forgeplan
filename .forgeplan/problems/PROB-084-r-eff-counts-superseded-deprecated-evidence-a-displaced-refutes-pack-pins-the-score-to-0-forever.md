---
depth: tactical
id: PROB-084
kind: problem
links:
- target: ADR-020
  relation: informs
status: draft
title: R_eff counts superseded/deprecated evidence — a displaced refutes pack pins the score to 0 forever
---

## Problem

`R_eff = min(evidence_scores)` включает в min() эвиденцию с терминальным статусом (`superseded`/`deprecated`). Следствие: артефакт, у которого когда-либо была refutes-эвиденция (score 0.0), **никогда не восстанавливает R_eff** — даже после честного исправления дефекта, подтверждённого новыми независимыми прогонами, и явного вытеснения старого пакета через `supersede --by`. Score перестаёт значить «текущая надёжность» и становится «худшее, что когда-либо было известно».

## Reproduction

Внешний репорт (upstream issue [#436](https://github.com/ForgePlan/forgeplan/issues/436), PROB-102 на стороне оркестрации, extraboost #373 / FOR-98):

```
PRD-177:
  EVID-249 [Refutes]  CL3 = 0.0   ← status: superseded, вытеснена EVID-250 --supersedes--> EVID-249
  EVID-250 [Supports] CL3 = 1.0   ← ре-верификация на 72c796f
  EVID-251 [Supports] CL3 = 1.0   ← ре-верификация на aa42283
forgeplan score PRD-177 → R_eff: 0.00 (движок v0.33.0)
```

Живой случай в этом репо: PROB-078 (refuted, deprecated, единственная эвиденция — [Refutes]-пак) — health показывал «At Risk, R_eff=0.00» с next-action `forgeplan score`, который не мог сработать никогда (закрыто для health-витрины в PR #431; движок не трогали).

## Root cause

Асимметрия двух путей сбора в `r_eff_recursive`:

- **Dependency-путь** (`scoring/reff.rs:313`) пропускает draft/deprecated/superseded с комментарием «should not drag down R_eff» — решение ADR-002 (active), коммит `a76c105a` (PROB-013).
- **Evidence-путь** (движок `reff.rs` ~254, CLI `score.rs:154`, MCP `server.rs:3062`) собирает с `ArtifactFilter { status: None }` — без фильтра, без комментария, без теста, без записи в доках. Непокрытый случай, не решение.

`EvidenceItem` не несёт `status`, поэтому фильтр внутри `r_eff()` структурно невозможен без изменения типа (пре-анализ в #436).

## Impact

- Гейт активации (`R_eff > 0`) становится навсегда недостижимым для затронутых артефактов — что на практике **давит на подделку истории**: upstream зафиксировал 3 подряд инцидента фальсификации verdict у EVID-249 (агент «чинил» запись о прошлом вместо кода).
- Поведение противоречит опубликованному контракту: сайт (cli/score.md: «strengthen that evidence, refute it, **or replace it**»; blog averages-lie: worked example с исключением устаревшего источника и восстановлением R_eff), METHODOLOGY-COURSE Ch8 («Deprecated/Superseded — skipped»), и первоисточникам формулы — quint-code (`decision.go:818`: `Verdict != "superseded"` отфильтровывается из WLNK min(), «FPF F.10:6.1 — superseded within same Window»), FPF (Window discipline; Deprecate → «claim support is reduced or removed», Refresh — путь восстановления).

## Fix direction

ADR-020: терминальная эвиденция исключается из min() (draft остаётся в счёте), фильтр в одной точке через `EvidenceItem.status`, пропуски логируются в factors, all-terminal → «no active evidence». Активный refutes продолжает обнулять — это намеренная семантика и она не трогается.


