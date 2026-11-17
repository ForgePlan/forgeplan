---
title: forgeplan fgr
description: "Показывает Formality, Granularity и Reliability независимо от жизненного цикла. Каждая ось — от 0 до 1. Общая оценка — геометрическое среднее (F × G × R)^(1/3), а не арифметическое среднее или шкала 0–3."
---

Показывает Formality, Granularity и Reliability независимо от жизненного цикла. Каждая ось — от 0 до 1. Общая оценка — геометрическое среднее `(F × G × R)^(1/3)`, а не арифметическое среднее или шкала 0–3.

## Использование

```text
forgeplan fgr [OPTIONS] [ID]
```

## Аргументы

```text
  [ID]  Artifact ID (scores all if omitted)
```

## Опции

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Как читать оценки

Formality отражает соблюдение правил валидации, Granularity — полноту содержимого. Reliability объединяет кешированный R_eff, связи и актуальность с настроенными весами. Пороги строгие: A > 0.8, B > 0.6, C > 0.4, D > 0.2, иначе F. Без ID оцениваются все артефакты. Команда не заменяет валидацию.

## Примеры

```bash
forgeplan fgr PRD-001
forgeplan fgr --json | jq '.results | sort_by(.overall) | reverse | .[0:10]'
forgeplan fgr --json | jq '.results[] | select(.formality > 0.8 and .reliability < 0.4)'
```

## JSON и рабочий процесс

JSON содержит `results`, `_next_action`, `hints`. Поля результата: `id`, `title`, `formality`, `granularity`, `reliability`, `overall`, `grade`. Улучшайте структуру через `validate`, подробность — при проработке, надёжность — доказательствами и `score`. Для правил FPF используйте `fpf check`; подкоманды `fpf trust` нет.

## См. также

- [`forgeplan validate`](/ru/docs/cli/validate/) - детализация по правилам для оси F
- [`forgeplan score`](/ru/docs/cli/score/) - R_eff, который питает ось R
- [`forgeplan health`](/ru/docs/cli/health/) - агрегированные F-G-R по всему проекту
- [Quality Gates](/ru/docs/methodology/evidence/) - FGR и Trust Calculus
- [Обзор CLI](/ru/docs/cli/)
