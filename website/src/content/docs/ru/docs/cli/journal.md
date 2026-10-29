---
title: forgeplan journal
description: "Артефакты решений по дате создания с текущими оценками и признаками риска."
---

Показывает артефакты решений по убыванию даты создания с текущими оценками и
признаками риска. Это не журнал lifecycle-событий ACTIVATE/SUPERSEDE/RENEW.
Для истории изменений используйте [`log`](/ru/docs/cli/log/).

## Когда использовать

- Онбординг - "как мы сюда пришли?"
- Ретроспективы - какие решения хорошо себя показали, а какие были замещены
- Режим `--risk`: поиск активных решений без доказательств (слепых пятен) на
  временной оси
- Обзор недавних решений; для выпуска по Git refs используйте `release-notes`

## Когда не использовать

- Вам нужны низкоуровневые CRUD-события → используйте [`forgeplan log`](/docs/cli/log/)
- Вам нужен текущий снимок → используйте [`forgeplan health`](/docs/cli/health/)

## Использование

```text
forgeplan journal [OPTIONS]
```

## Опции

```text
  -t, --type <TYPE>  Фильтровать по типу (adr, note, problem, solution)
      --risk         Показывать только рискованные решения (без доказательств, просроченные, с низким R_eff)
  -h, --help         Вывести справку
  -V, --version      Вывести версию
```

## Примеры

Полная временная шкала решений:

```bash
forgeplan journal
```

Только ADR - архитектурная запись, меньший набор:

```bash
forgeplan journal --type adr
```

Режим риска - активные решения без доказательств или с просроченными подтверждениями:

```bash
forgeplan journal --risk
```

## Интерпретация вывода

Одна запись на выбранный артефакт решения, сначала новые `created_at`.
Иллюстрация, не записанный вывод:

```text
  2026-04-11  ADR-007 [adr] "Storage engine"
         R_eff: 0.90 | 1 evidence
```

| Столбец   | Значение                                                   |
|-----------|------------------------------------------------------------|
| Дата | Дата создания артефакта |
| Артефакт | ID и тип решения |
| R_eff | Текущий плоский расчёт связанных доказательств, не рекурсивный score report |
| Заголовок | Краткий заголовок                                          |

Нулевой R_eff требует ревью: это может быть отсутствие поддержки или опровергающее доказательство, а не обязательно отсутствие связей. `--risk` включает отсутствие доказательств, просрочку и низкую оценку.

## Как это работает

`journal` находится между `log` (сырые изменения) и `health` (текущий свод):

```
log         → journal    → health
всё           решения      текущее состояние
```

Используйте его, чтобы рассказать историю решений; используйте `health`, чтобы увидеть, где вы находитесь сейчас.

## Смотрите также

- [`forgeplan log`](/docs/cli/log/) - полный след изменений
- [`forgeplan health`](/docs/cli/health/) - панель текущего состояния
- [`forgeplan score`](/docs/cli/score/) - R_eff для одного артефакта
- [`forgeplan blindspots`](/docs/cli/blindspots/) - все рискованные решения сразу

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan journal --help`.

```text
Show decision journal — chronological timeline with R_eff scores

Usage: forgeplan journal [OPTIONS]

Options:
  -t, --type <TYPE>  Filter by kind (adr, note, problem, solution)
      --risk         Show only at-risk decisions (no evidence, stale, low R_eff)
  -h, --help         Print help
  -V, --version      Print version
```
<!-- forgeplan-runtime:end -->
