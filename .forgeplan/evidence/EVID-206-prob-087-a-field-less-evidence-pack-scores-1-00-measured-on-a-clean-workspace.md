---
depth: tactical
id: EVID-206
kind: evidence
links:
- target: PROB-087
  relation: informs
status: active
title: 'PROB-087: a field-less evidence pack scores 1.00 — measured on a clean workspace'
---

---
assigned_number: 206
predicted_number: 206
slug: evid-prob-087-a-field-less-evidence-pack-scores-1-00-measured-on-a-clean
---

## Summary

Направление отказа для структурных полей эвиденции измерено, а не выведено из чтения кода. Пакет
без единого поля `verdict` / `congruence_level` / `evidence_type` приносит артефакту **R_eff = 1.00**,
а не 0.1, как утверждали CLAUDE.md и около двадцати страниц документации.

(На ветке-источнике этот пак имел номер EVID-158; в readiness номер 158 уже занят
другим документом — `five-target-dist-build`, поэтому пак перенесён под EVID-206.)

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

CL3: измерение выполнено тем же бинарём и на той же версии парсера, о которых говорит PROB-087.

Git-provenance намеренно отсутствует: пакет не утверждает, что код изменился. Он фиксирует
поведение **существующего** кода, поэтому по PRD-082 корректно классифицируется как `NotClaimed`.

## Measurement

Чистый временный workspace, реальный CLI, без фикстур:

```
forgeplan init -y
forgeplan new prd "Target"
forgeplan new evidence "Bare"
# из тела пакета удалены все три строки structured fields
#   (проверено: grep -c даёт 0)
forgeplan reindex
forgeplan link EVID-001 PRD-001 --relation informs
forgeplan activate EVID-001
forgeplan score PRD-001

→   EVID-001 [Supports] CL3 = 1.0
→   R_eff:        1.00 -- Adequate
```

Ни предупреждения, ни пометки, ни строки в `factors`. Пустой пакет не отличим в выводе от
честно заполненного.

## Analysis

Механизм подтверждён чтением `crates/forgeplan-core/src/scoring/evidence.rs`:

| Поле | Отсутствует | Присутствует, но не парсится |
|---|---|---|
| `congruence_level` | CL3 | **CL0, штраф 0.9 — fail-closed**, печатает warning |
| `verdict` | `supports` (1.0) | `supports` (1.0) — fail-open, молча |
| `evidence_type` | `measurement` (0.0) | `measurement` (0.0) — fail-open, молча |

Комментарий у ветки `congruence_level` прямо называет цель — «to prevent silent trust inflation».
Для двух других полей того же блока эта цель не достигнута.

## Почему это важнее, чем выглядит

Ошибка в документации была направленной. Читателю обещали, что забытые поля дадут **заметный ноль**
— то есть провал, который видно. На деле он получает **молчаливую единицу**: артефакт выглядит
полностью подтверждённым, хотя не измерено ничего.

Шаблон при этом не страхует, хотя это и предполагали: он предзаполнен значениями
`measurement` / `supports` / `3`, то есть **ровно дефолтами парсера**. Пользователь, оставивший
шаблон нетронутым, получает тот же результат, что и пользователь, удаливший блок целиком.

## Scope

Продуктовый код этой правкой не затронут — изменение политики отказа breaking и требует отдельного
замера по живым графам (здесь на дефолтах держится один пакет, `EVID-126` с
`evidence_type: code_review`). Исправлена документация: CLAUDE.md RED LINE #7, секция EvidencePack,
и все найденные страницы в `docs/` и на сайте.
