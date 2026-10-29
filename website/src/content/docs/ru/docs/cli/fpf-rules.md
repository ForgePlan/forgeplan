---
title: forgeplan fpf rules
description: "Список активных правил FPF, сгруппированных по действиям: EXPLORE, INVESTIGATE, EXPLOIT"
---

`forgeplan fpf rules` выводит каждое активное правило в движке правил **First Principles Framework**, сгруппированное по рекомендуемой категории действий: **EXPLORE** (генерировать больше гипотез), **INVESTIGATE** (собирать больше доказательств для существующих гипотез) или **EXPLOIT** (придерживаться проверенного пути).

Это исполняемые правила методологии ForgePlan, не полная реализация upstream FPF. [`fpf check`](/ru/docs/cli/fpf-check/) проверяет их для артефакта, [`fpf dashboard`](/ru/docs/cli/fpf-dashboard/) показывает рекомендации по проекту. Непустой `fpf.rules` заменяет встроенный набор.

## Когда использовать

- **Перед спринтом** - просмотрите набор правил, чтобы понять, что ищет движок.
- **Когда правило срабатывает неожиданно** - прочитайте полный текст правила, чтобы понять причину срабатывания.
- **При написании документации по методологии** - ссылайтесь на правила по ID.
- **При отладке вывода `fpf check`** - см. полное определение правила, стоящее за совпадением.

## Когда НЕ использовать

- Для артефакта используйте [`fpf check <id>`](/ru/docs/cli/fpf-check/).
- Для проекта используйте [`fpf dashboard`](/ru/docs/cli/fpf-dashboard/).

## Использование

```text
forgeplan fpf rules [OPTIONS]
```

## Опции

```text
      --flat     Плоская таблица по приоритету вместо дерева, сгруппированного по действиям
      --json     Вывод полного дампа правил в формате JSON
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

Представление по умолчанию группирует правила по действиям (EXPLORE / INVESTIGATE / EXPLOIT). `--flat` переключает на единую таблицу, упорядоченную по приоритету, если вы хотите просмотреть правила с наивысшим приоритетом независимо от действия. `--json` выводит полный дамп правил для клиентов MCP и скриптов аудита.

## Примеры

```bash
# По умолчанию: дерево, сгруппированное по действиям
forgeplan fpf rules

# Плоская таблица, упорядоченная по приоритету
forgeplan fpf rules --flat

# Машиночитаемый полный дамп
forgeplan fpf rules --json

# Типичный процесс ревью
forgeplan fpf rules
forgeplan fpf check PRD-019
forgeplan fpf dashboard
```

## Три категории действий

Правила соответствуют оси explore→investigate→exploit из части B FPF:

- **EXPLORE** - встроенные правила отмечают draft с R_eff < 0.01 или active без связей.
- **INVESTIGATE** - встроенные правила отмечают слабые доказательства или среднюю надёжность.
- **EXPLOIT** - встроенная рекомендация требует R_eff ≥ 0.7 и общей оценки ≥ 0.6, но сама не разрешает активацию или замещение.

Правило содержит имя, приоритет, условие, действие и необязательное сообщение. Встроенный набор не считает гипотезы и не загружает актуальные цитаты KB.

## Как это работает

`rules` читает конфигурацию workspace и перечисляет правила без оценки артефакта и загрузки тел разделов KB. Проверка правил и жизненного цикла - отдельные операции:

- `fpf check <id>` - совпадения для артефакта
- `fpf dashboard` - рекомендации по проекту
- `forgeplan validate` / `activate` - отдельные проверки валидации и жизненного цикла

## См. также

- [`forgeplan fpf`](/ru/docs/cli/fpf/) - родительская команда
- [`forgeplan fpf check`](/ru/docs/cli/fpf-check/) - проверка артефакта
- [`forgeplan fpf dashboard`](/ru/docs/cli/fpf-dashboard/) - рекомендации по проекту
- [`forgeplan validate`](/ru/docs/cli/validate/) - структурная валидация
- [Методология](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan fpf rules --help`.

```text
List active FPF rules grouped by action (EXPLORE/INVESTIGATE/EXPLOIT)

Usage: forgeplan fpf rules [OPTIONS]

Options:
      --flat     Flat priority-linear table instead of action-grouped tree
      --json     Output full rule dump as JSON
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
