---
depth: tactical
id: PROB-087
kind: problem
links:
- target: PROB-086
  relation: based_on
status: active
title: 'Evidence structured fields fail OPEN: a field-less pack scores 1.00, and the docs said the opposite'
---

---
assigned_number: 87
predicted_number: 87
slug: prob-evidence-structured-fields-fail-open-a-field-less-pack-scores-1-00-and-the
---

## Problem

Три поля структурированной эвиденции разбираются с **противоположными** политиками отказа, и та,
что применяется к двум из трёх, завышает доверие.

| Поле | Отсутствует | Присутствует, но не парсится |
|---|---|---|
| `congruence_level` | CL3 (по умолчанию) | **CL0, штраф 0.9 — fail-closed**, с предупреждением |
| `verdict` | `supports` (1.0) | `supports` (1.0) — **fail-open, молча** |
| `evidence_type` | `measurement` (вычет 0.0) | `measurement` (вычет 0.0) — **fail-open, молча** |

Пак вообще без секции `## Structured Fields` получает `supports` / CL3 / `measurement` и приносит
артефакту **R_eff = 1.00**. Пустое доказательство выглядит не сломанным, а идеальным.

## Reproducer

Проверено на чистом workspace, а не выведено из чтения кода:

```
forgeplan init -y
forgeplan new prd "Target"
forgeplan new evidence "Bare"
# удалить из тела пака все три строки verdict/congruence_level/evidence_type
forgeplan reindex
forgeplan link EVID-001 PRD-001 --relation informs
forgeplan activate EVID-001
forgeplan score PRD-001
→ EVID-001 [Supports] CL3 = 1.0
→ R_eff: 1.00 -- Adequate
```

## Root cause

`crates/forgeplan-core/src/scoring/evidence.rs`, `parse_evidence_from_record`:

- `verdict`: `_ => Verdict::Supports` в match плюс `.unwrap_or(Verdict::Supports)` — и мусор, и
  отсутствие дают самый щедрый вердикт.
- `evidence_type`: `_ => EvidenceType::Measurement` плюс `.unwrap_or(EvidenceType::Measurement)` —
  и мусор, и отсутствие дают нулевой вычет, наравне с `test`.
- `congruence_level`: единственное поле с осознанной политикой — при неразбираемом значении
  печатается предупреждение и ставится CL0 «to prevent silent trust inflation».

Комментарий у CL прямо называет цель — не допустить тихого завышения доверия. Ровно эта цель не
достигнута для двух других полей того же блока.

## Живой экземпляр

`EVID-126` несёт `evidence_type: code_review` — значения нет в перечислении из четырёх вариантов.
Оно молча становится `measurement` с вычетом 0.0, то есть попадает в самый доверительный тариф.
Никакого предупреждения не печатается, и никакой детектор это не ловит.

## Документация утверждала обратное

Опаснее самого дефекта то, что все три места описывали провал в **обратную** сторону:

- `CLAUDE.md` RED LINE #7: «без этих structured fields parser тихо ставит CL0 (silent failure →
  R_eff = 0.1)» — исправлено в рамках работы по PROB-086.
- `CLAUDE.md` §EvidencePack: «Without these fields the R_eff parser sets CL0 (penalty 0.9) and
  score = 0» — исправлено там же.
- `docs/methodology/FORGEPLAN-GUIDE.{md,ru.md}` и `docs/architecture/c4-context.md` — то же
  утверждение, правится в том же PR.

Читатель, поверивший этим строкам, ожидал, что забытые поля дадут заметный ноль. На деле он
получает молчаливую единицу — и никакого сигнала, что мерить забыли.

## Почему это тот же класс, что PROB-034

PROB-034 был про тихую подстановку CL3 из-за плейсхолдера в HTML-комментарии шаблона. Тогда
починили парсер комментариев и добавили fail-closed для неразбираемого CL. Но политику довели до
конца только для одного поля из трёх; два остальных сохранили исходное поведение «молча взять
самое хорошее».

## Предлагаемое направление (не решено)

Симметрия важнее конкретного выбора: все три поля должны отказывать одинаково.

1. **Мусор → fail-closed + предупреждение** для `verdict` и `evidence_type`, как уже сделано для
   `congruence_level`. Нераспознанный `evidence_type` разумнее трактовать как самый дорогой
   (`audit`, вычет 0.2), а не самый дешёвый.
2. **Полное отсутствие блока** — отдельный вопрос: молчаливая 1.00 хуже, чем явный отказ
   активировать пак без структурных полей. `is_evidence_complete` уже существует и используется
   MCP-гейтом; логично задействовать её и в `activate`.
3. Оба варианта — breaking для существующих графов, поэтому нужен замер: сколько живых паков
   держатся на дефолтах. Здесь такой один (`EVID-126`).

## Related

Обнаружено при работе над PROB-086 / ADR-021. Тот же класс тихого завышения доверия, что PROB-034.




