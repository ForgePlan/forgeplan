---
title: forgeplan fpf check
description: "Проверка, каким правилам FPF соответствует данный артефакт и какое действие они рекомендуют"
---

`forgeplan fpf check <ID>` проверяет активные правила для одного артефакта. Выводит победившее правило с наивысшим приоритетом, остальные совпадения по приоритету и, по запросу, имена несовпавших правил. Действия: EXPLORE / INVESTIGATE / EXPLOIT. Отчёт не загружает цитату раздела FPF для каждого правила.

Это представление по одному артефакту для [`fpf rules`](/ru/docs/cli/fpf-rules/) и [`fpf dashboard`](/ru/docs/cli/fpf-dashboard/).

## Когда использовать

- **Перед активацией PRD/RFC/ADR** - убедитесь, что правила INVESTIGATE больше не срабатывают.
- **После структурной валидации** - изучить сигналы метаданных и качества доказательств; встроенные правила не считают гипотезы ADI.
- **При разборе застрявшего артефакта** - изучить рекомендации, не считая каждое совпадение блокирующим гейтом.
- **Во время состязательной ревью** - используйте вывод правил в качестве структурированного контрольного списка для критики.

## Когда НЕ использовать

- Для структурных проверок используйте [`forgeplan validate`](/ru/docs/cli/validate/).
- Для обзора проекта используйте [`fpf dashboard`](/ru/docs/cli/fpf-dashboard/).
- Для элементов глубины Tactical, где обоснование FPF намеренно обходится.

## Использование

```text
forgeplan fpf check [OPTIONS] <ID>
```

## Аргументы

```text
  <ID>   ID артефакта (например, PRD-041)
```

## Опции

```text
      --verbose  Показывать также названия несопоставленных правил
      --json     Вывести полный RuleCheckResult в формате JSON
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

```bash
# По умолчанию: победившее правило и остальные совпадения по приоритету
forgeplan fpf check PRD-019

# Показывать также несопоставленные правила (полезно при отладке, почему правило не сработало)
forgeplan fpf check PRD-019 --verbose

# Машиночитаемый вывод для клиентов MCP и аудиторских скриптов
forgeplan fpf check ADR-005 --json
```

## Как работают пороги explore / investigate / exploit

`explore_reff`, `investigate_reff` и `exploit_reff` - пороги конфигурации, а не три производные оценки надёжности. Встроенные правила проверяют:

| Правило | Условие | Действие |
|---|---|---|
| `blind-spot` | draft и R_eff < 0.01 | EXPLORE |
| `weak-evidence` | active/stale и R_eff < 0.5 | INVESTIGATE |
| `orphan-active` | active и нет связей | EXPLORE |
| `medium-quality` | 0.5 ≤ R_eff < 0.7 | INVESTIGATE |
| `ready-to-build` | R_eff ≥ 0.7 и общая оценка ≥ 0.6 | EXPLOIT |

Непустой `fpf.rules` заменяет встроенные правила. Сама команда не активирует артефакт и не меняет гейты жизненного цикла. Отдельно соблюдайте применимые требования валидации, ADI, ревью и доказательств.

## Как это работает

Выбранная CLI-реализация читает записи артефактов и связи, выбирает активные правила и проверяет обогащённые метаданные. Она не загружает тела разделов KB для объяснения совпадений. Не воссоздавайте старые таблицы ради этой команды. `--json` возвращает matched/unmatched/winning, сводку и подсказку следующего действия.

## См. также

- [`forgeplan fpf`](/ru/docs/cli/fpf/) - родительская команда
- [`forgeplan fpf rules`](/ru/docs/cli/fpf-rules/) - активные правила
- [`forgeplan fpf dashboard`](/ru/docs/cli/fpf-dashboard/) - обзор проекта
- [`forgeplan validate`](/ru/docs/cli/validate/) - структурная валидация
- [`forgeplan reason`](/ru/docs/cli/reason/) - ADI
- [`forgeplan score`](/ru/docs/cli/score/) - оценка R_eff

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan fpf check --help`.

```text
Check which FPF rules match a given artifact

Usage: forgeplan fpf check [OPTIONS] <ID>

Arguments:
  <ID>  Artifact ID (e.g. PRD-041)

Options:
      --verbose  Show unmatched rule names too
      --json     Output full RuleCheckResult as JSON
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
