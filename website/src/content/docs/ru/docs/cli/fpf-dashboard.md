---
title: forgeplan fpf dashboard
description: "Показывает панель мониторинга FPF - ограниченные контексты, оценки качества и рекомендации по исследованию/изучению/использованию"
---

`forgeplan fpf dashboard` отображает на одном экране обзор того, как проект выглядит через призму **First Principles Framework**: какие ограниченные контексты здоровы, какие несут в себе долг рассуждений, и что рекомендует движок правил далее (исследовать больше вариантов, изучить существующие доказательства или использовать проверенный путь).

Это аналог `forgeplan health` для FPF - `health` сообщает факты на уровне артефактов (сироты, просроченные, слепые пятна); `fpf dashboard` интерпретирует их через Trust Calculus и модель исследования/использования.

## Когда использовать

- **При запуске сессии**, наряду с `forgeplan health` - чтобы понять, куда сегодня следует направить усилия по рассуждению.
- **При планировании спринта** - выбрать следующий ограниченный контекст для развития на основе R_eff и сигналов исследования/использования.
- **Перед принятием важного решения** - чтобы увидеть, не используете ли вы преждевременно путь с низким уровнем доверия.
- **После серии активаций** - чтобы убедиться, что оценки изменились в ожидаемом направлении.

## Когда НЕ использовать

- Для необработанной статистики артефактов (количество, сироты) - используйте `forgeplan health`.
- Для правил отдельного артефакта - используйте [`forgeplan fpf check <id>`](/docs/cli/fpf-check/).
- Для состояния загрузки KB - используйте [`forgeplan fpf status`](/docs/cli/fpf-status/).

## Использование

```text
forgeplan fpf dashboard [OPTIONS]
```

## Опции

```text
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

```bash
# Полная панель мониторинга
forgeplan fpf dashboard

# Типичная комбинация для начала сессии
forgeplan health
forgeplan fpf dashboard
forgeplan blocked
```

## Что вы видите

Панель выводит четыре блока:

1. **Bounded contexts** - графовые кластеры, участники и связность.
2. **Quality (F-G-R)** - число артефактов по продуктовым оценкам Formality/Granularity/Reliability.
3. **Next Actions** - рекомендации `EXPLORE`, `INVESTIGATE`, `EXPLOIT` по артефактам.
4. **Pipeline** - сводка конвейера.

Рекомендация по действию поступает от того же движка правил, который отображается командой [`forgeplan fpf rules`](/docs/cli/fpf-rules/).

## Как это работает

`fpf dashboard` - это визуальный синтез PRD-041 (движок правил FPF) и PRD-043 (целостность методологии). Он считывает:

-   Метаданные артефактов (тип, статус, R_eff) из выбранного workspace
- Правила конфигурации workspace либо встроенные defaults, не загруженный корпус
-   Связи доказательств и уровни конгруэнтности

Это эвристики продукта, не буквальные шкалы upstream FPF. Наличие команды в help само по себе не подтверждает её работу на всех раскладках workspace.

## См. также

-   [`forgeplan fpf`](/docs/cli/fpf/) - родительская команда
-   [`forgeplan fpf rules`](/docs/cli/fpf-rules/) - правила, которые питают панель мониторинга
-   [`forgeplan fpf check`](/docs/cli/fpf-check/) - соответствие правил для каждого артефакта
-   [`forgeplan health`](/docs/cli/health/) - аналог на уровне артефактов
-   [Руководство по методологии](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan fpf dashboard --help`.

```text
Show FPF dashboard — bounded contexts, quality scores, explore-exploit actions

Usage: forgeplan fpf dashboard

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
