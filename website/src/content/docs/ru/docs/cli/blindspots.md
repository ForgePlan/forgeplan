---
title: forgeplan blindspots
description: "Решения без связанных доказательств и изолированные артефакты — диагностика состояния"
---

Показывает **слепые пятна** — решения подходящих типов без связанных доказательств — и **сироты**, у которых нет ни входящих, ни исходящих связей. Отсутствие доказательств и `R_eff = 0` — разные проверки: пакет может быть связан, но не поддерживать решение. Решения в `draft`, `deprecated`, `superseded` исключены из поиска слепых пятен.

## Когда использовать

-   **Начало сессии** - запустите эту команду сразу после `forgeplan health` и исправьте
    проблемы, прежде чем начинать новую работу (не накапливайте долги).
- **Отчёт для CI** — изучайте находки; для отдельного гейта с кодом выхода используйте `health --strict`.
-   **Перед релизом** - устраните все слепые пятна перед созданием тега.
-   **Ретроспектива** - подсчитайте, сколько слепых пятен накопилось за спринт.

## Когда не использовать

- Зависимости → [`forgeplan blocked`](/ru/docs/cli/blocked/)
- Общая диагностика → [`forgeplan health`](/ru/docs/cli/health/)
- Временная шкала → [`forgeplan journal`](/ru/docs/cli/journal/) с `--risk`

## Использование

```text
forgeplan blindspots
```

## Опции

```text
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

Вывести все слепые пятна:

```bash
forgeplan blindspots
```

Для гейта с кодом выхода используйте `health --strict`. Сам `blindspots` не завершается ошибкой только из-за наличия находок:

```bash
forgeplan health --strict
```

В сочетании с [`health --ci`](/ru/docs/cli/health/), у которого собственные правила отказа:

```bash
forgeplan health --ci && forgeplan blindspots
```

## Интерпретация вывода

Иллюстрация находок, не дословный CLI-транскрипт и не измерение вашего workspace:

```
BLIND DECISIONS (no linked evidence)
  PRD-046  Docs v0.18.0 catch-up    → create EvidencePack + link
  ADR-008  Cloudflare Pages choice  → create EvidencePack + link

ORPHANS (no incoming or outgoing links)
  PRD-044  Unused PRD               → link to Epic or deprecate
  NOTE-031 One-off observation      → link to artifact or ignore

Summary: 2 blind decisions, 2 orphans
```

| Секция                      | Что это значит                                        |
|-----------------------------|-------------------------------------------------------|
| Слепые решения | Решения подходящих типов без связанных доказательств |
| Сироты | Артефакты без входящих и исходящих связей |

`blindspots` возвращает успех после вывода находок. У `health --ci` и `health --strict` собственные правила отказа; не выводите их из поведения этой команды.

## Как это работает

`blindspots` выделяет два класса из [`health`](/ru/docs/cli/health/) для разбора.
Для CI выбирайте команду с явными правилами отказа.
Рекомендуемый рабочий процесс:

```
начало сессии → health → blindspots → (исправление) → новая работа
```

Если `blindspots` не пуст, _исправьте это в первую очередь_. Никогда не позволяйте активным решениям
дрейфовать без измерений - именно это модель R_eff (слабое звено)
призвана предотвращать.

## См. также

- [`forgeplan health`](/ru/docs/cli/health/) - общая диагностика с режимами гейтинга
- [`forgeplan journal --risk`](/ru/docs/cli/journal/) - решения во времени
- [`forgeplan score`](/ru/docs/cli/score/) - R_eff одного артефакта
- [`forgeplan new evidence`](/ru/docs/cli/new/) - создание EvidencePack
- [`forgeplan link`](/ru/docs/cli/link/) - связь с доказательством через `informs`

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan blindspots --help`.

```text
Show blind spots — decisions without evidence, orphan artifacts

Usage: forgeplan blindspots

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
