---
depth: standard
id: PROB-079
kind: problem
last_modified_at: 2026-07-24T22:33:24.522827+00:00
last_modified_by: claude-code/2.1.219
status: draft
title: graph --brownfield-only fails to parse hypothesis frontmatter, silently falls back to default lifecycle state
---

# PROB-079: `graph --brownfield-only` не парсит frontmatter гипотез и молча подставляет дефолтное состояние

## Problem

`forgeplan graph --brownfield-only` печатает предупреждение о том, что у артефакта-гипотезы
нет YAML frontmatter, хотя на диске frontmatter присутствует и корректен. Команда не падает —
она подставляет **дефолтное lifecycle-состояние** и продолжает. В результате граф показывает
состояние гипотез, не соответствующее действительности, и об этом сообщается только строкой
`warning:` в stderr.

Это тихий отказ того же класса, что PROB-035 и PROB-039: команда завершается успешно,
вывод выглядит правдоподобно, расхождение видно лишь при внимательном чтении предупреждений.

## Reproduction

Наблюдалось 2026-07-24 на `forgeplan 0.33.0` (Homebrew), ветка `docs/forgefarm-kb`.

```
$ forgeplan graph --brownfield-only
warning: graph: failed to parse hypothesis frontmatter for HYP-001: No YAML frontmatter found (missing opening ---) (using default state)
warning: graph: failed to parse hypothesis frontmatter for HYP-002: No YAML frontmatter found (missing opening ---) (using default state)
warning: graph: failed to parse hypothesis frontmatter for HYP-003: No YAML frontmatter found (missing opening ---) (using default state)
graph LR
    subgraph DM-001["DM-001"]
    end
    ...
```

При этом файл на диске frontmatter **содержит**:

```
$ head -8 .forgeplan/hypotheses/HYP-001-hypothesis-template.md
---
author: scan-import
depth: standard
id: HYP-001
kind: hypothesis
status: draft
title: hypothesis.template
---
```

Другие команды тот же артефакт читают без ошибок:

- `forgeplan get HYP-001` — возвращает kind, status, author, R_eff, тело.
- `forgeplan validate HYP-001` — выдаёт содержательный вердикт (FAIL: 23 плейсхолдера,
  отсутствуют обязательные секции). Значит и frontmatter, и тело доступны корректно.

Расходится именно путь `graph --brownfield-only`, а не хранилище.

## Hypothesis (root cause)

Ветка `--brownfield-only` берёт **тело** артефакта из LanceDB, где frontmatter уже отделён
при проекции, и повторно пытается распарсить его как frontmatter. Опорного `---` в теле нет,
парсер возвращает `No YAML frontmatter found`, и код уходит в fallback.

То есть frontmatter обрабатывается дважды: один раз при проекции, второй раз в графе — уже
над телом, из которого он вырезан. Гипотеза требует проверки по коду (`crates/forgeplan-core/src/graph/`),
но согласуется со всеми наблюдениями: файл на диске валиден, остальные команды работают,
ломается только та ветка, что читает тело из индекса.

## Impact

- Lifecycle-состояние гипотез в brownfield-графе **всегда дефолтное**, независимо от реального.
  Для `hypothesis` состояние — смысловой центр артефакта (proposed / confirmed / refuted),
  так что граф вводит в заблуждение ровно в том, ради чего его строят.
- Отказ тихий: exit code 0, граф отрисован, расхождение только в `warning:`.
- Затрагивает Epic #287 (brownfield extraction surface): `--brownfield-only` — его флаг,
  и `hypothesis` — один из шести введённых им kind'ов.
- Не проверяется тестами: ни один тест не покрывает `--brownfield-only` на артефакте-гипотезе,
  иначе баг был бы пойман.

## Evidence preservation

Гипотезы HYP-001..003, на которых баг воспроизводится, — мусор от непреднамеренного
`scan-import` (`author: scan-import`) и подлежат удалению при очистке рабочего пространства.
Эта запись создана **до** удаления, чтобы репродьюсер не исчез вместе с ними.

Полный вывод команды и содержимое frontmatter приведены выше дословно. Для повторного
воспроизведения после удаления достаточно создать любую гипотезу:
`forgeplan new hypothesis "test"` → `forgeplan graph --brownfield-only`.

## Fix direction

1. Найти в ветке `--brownfield-only` место, где парсится frontmatter, и проверить, читает ли
   оно тело из LanceDB или файл с диска.
2. Если тело — брать состояние из уже распарсенных полей проекции, не парсить повторно.
3. Регрессионный тест: `graph --brownfield-only` на гипотезе с непустым lifecycle-состоянием
   должен отдавать именно его, а не дефолт.
4. Отдельно: fallback должен быть громче. Молчаливая подстановка дефолта в граф, который
   читают как источник истины, — это тот же класс ошибки, что уже дважды закрывали
   (PROB-035, PROB-039).

## Related

- Epic #287 — brownfield extraction surface, вводит `--brownfield-only` и kind `hypothesis`.
- PROB-035, PROB-039 — прецеденты тихих отказов; тот же класс, та же цена.
- PROB-047 — непреднамеренный `scan-import`, породивший HYP-001..003.

