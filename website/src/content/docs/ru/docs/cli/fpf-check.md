---
title: forgeplan fpf check
description: "Проверка, каким правилам FPF соответствует артефакт и какое действие они рекомендуют"
---

`forgeplan fpf check <ID>` проверяет активные правила **First Principles Framework** для одного артефакта. Сначала выводится победившее правило, затем остальные совпадения по возрастанию приоритета. Группировки по действию и загрузки цитируемых разделов базы знаний нет.

Это проекция на уровне артефакта для [`fpf rules`](/ru/docs/cli/fpf-rules/) и [`fpf dashboard`](/ru/docs/cli/fpf-dashboard/).

## Когда использовать

- **Перед активацией PRD/RFC/ADR** — изучите рекомендацию по рассуждению вместе с отдельным lifecycle-review.
- **Когда структурной валидации недостаточно** — изучите условия настроенных правил и результаты совпадений.
- **При отладке рекомендации** — найдите первое совпавшее правило, которое побеждает.
- **Во время состязательной ревью** - используйте вывод правил в качестве структурированного контрольного списка для критики.

## Когда НЕ использовать

- Для синтаксических / структурных проверок - используйте [`forgeplan validate`](/ru/docs/cli/validate/).
- Для обзора в масштабе проекта - используйте [`fpf dashboard`](/ru/docs/cli/fpf-dashboard/).
- Для разрешения активации: рекомендации не являются lifecycle-гейтами ни на одной глубине.

## Использование

```text
forgeplan fpf check [OPTIONS] <ID>
```

## Аргументы

```text
  <ID>  Artifact ID (e.g. PRD-041)
```

## Опции

```text
      --verbose  Show unmatched rule names too
      --json     Output full RuleCheckResult as JSON
  -h, --help     Print help
  -V, --version  Print version
```

## Примеры

```bash
# Default: show matched rules grouped by action
forgeplan fpf check PRD-019

# Show unmatched rules too (useful while debugging why a rule didn't fire)
forgeplan fpf check PRD-019 --verbose

# Machine-readable output for MCP clients and audit scripts
forgeplan fpf check ADR-005 --json
```

## Правила, вывод и ограничения

Непустой `fpf.rules` в `.forgeplan/config.yaml` заменяет встроенный набор.
Правила сортируются по возрастанию числового приоритета; первое совпадение
побеждает. Условия используют сохранённые данные артефакта и производные
сигналы trust/связей/F-G-R. `explore_reff`, `investigate_reff` и `exploit_reff`
— пороги конфигурации, не три отдельные оценки контекста.
Универсальной таблицы гейтов «глубина × действие» команда не обеспечивает.

`--json` содержит `artifact_id`, `kind`, `status`, `matched`, `unmatched`,
`winning`, `summary` и `_next_action`. Совпавшее/победившее правило содержит
`name`, `priority`, `action` и `message`; без совпадений `winning` равен null.
`unmatched` — массив имён правил, присутствующий в JSON независимо от `--verbose`.

Рекомендуется `activate` для EXPLOIT, `reason` для EXPLORE или INVESTIGATE
и `get`, если совпадений нет. Это совет, не выполнение.
Успешная проверка завершается с кодом `0` независимо от действия;
отсутствующий артефакт или сбой проверки — с кодом `1`. Требуется рабочее
пространство, но не загруженная база знаний FPF. Для замечаний по активации
запускайте `review` отдельно; `activate` применяет собственные проверки.


## См. также

- [`forgeplan fpf`](/ru/docs/cli/fpf/) - родительская команда
- [`forgeplan fpf rules`](/ru/docs/cli/fpf-rules/) - полный набор правил
- [`forgeplan fpf dashboard`](/ru/docs/cli/fpf-dashboard/) - обзор в масштабе проекта
- [`forgeplan validate`](/ru/docs/cli/validate/) - структурная валидация
- [`forgeplan reason`](/ru/docs/cli/reason/) - генерировать больше гипотез, когда срабатывают правила EXPLORE
- [`forgeplan score`](/ru/docs/cli/score/) - входные данные R_eff, лежащие в основе порогов
