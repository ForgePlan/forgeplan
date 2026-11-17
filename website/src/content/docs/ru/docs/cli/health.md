---
title: forgeplan health
description: "Проверяет состояние пространства в начале сессии, перед PR или после массовых изменений. Показывает решения без поддержки, артефакты без связей, просроченные доказательства, риски, активные шаблоны, признаки дубликатов и рекомендательные замечания об этапах или gitignore."
---

Проверяет состояние пространства в начале сессии, перед PR или после массовых изменений. Показывает решения без поддержки, артефакты без связей, просроченные доказательства, риски, активные шаблоны, признаки дубликатов и рекомендательные замечания об этапах или gitignore.

## Использование

```text
forgeplan health [OPTIONS]
```

## Опции

```text
      --compact            Compact one-line output for hooks/scripts
      --json               Output as JSON for machine consumption
      --ci                 CI mode: exit code 1 if issues found (for pipeline gates)
      --fail-on <FAIL_ON>  Fail thresholds for --ci (e.g., "orphans=5,blind_spots=3,stale=2")
      --strict             Strict mode: exit 1 if verdict is NeedsAttention/Unhealthy or any of {orphans, blind_spots, active_stubs, at_risk} > 0. Designed for CI gates that want a single boolean signal. Empty workspaces and advisory-only signals (e.g. phase mismatches) keep exit 0
  -h, --help               Print help
  -V, --version            Print version
```

## Примеры

```bash
forgeplan health
forgeplan health --compact
forgeplan health --json
forgeplan health --ci --fail-on "orphans=0,blind_spots=0,stale=5"
forgeplan health --strict --json
```

<a id="ci-mode---ci"></a>

## Пороги CI

В обычном текстовом режиме `--ci` возвращает 1 при превышении порога. По умолчанию допускается ноль orphans и blind_spots; stale и at_risk не ограничены, пока не заданы явно. `--fail-on` принимает `orphans=N`, `blind_spots=N`, `stale=N`, `at_risk=N` через запятую. После клонирования перед проверкой выполните `reindex`: `scan-import` не обновляет отслеживаемые артефакты.

## Строгий режим и оговорка JSON

`--strict` возвращает 1 при критических сигналах: orphans, blind_spots, active_stubs, at_risk либо verdict needs_attention/unhealthy. Дубликаты и просрочка влияют через пороги verdict, а не как безусловная отдельная ошибка. Одни рекомендательные расхождения этапов не проваливают проверку. В 0.38.0 ветка JSON возвращается до проверки порогов `--ci`: не используйте `--json --ci` как барьер. `--strict --json` применяет строгую проверку и добавляет `exit_code`. Компактный режим также возвращается до этих проверок.

## Разбор находок

При нулевом R_eff изучите `score` и структурированные поля; ноль не доказывает отсутствие связи с доказательством. У orphans нет входящих и исходящих связей. Перед `renew` или `reopen` пересмотрите просроченную работу. Для отдельного артефакта используйте `context` или `validate`. Отчёт не исправляет, не активирует и не проверяет код автоматически.

## Машинный вывод

JSON использует общую схему health-report: `total`, `verdict`, `blind_spots`, `orphans`, `stale_count`, `at_risk` и дополнительные диагностические поля. Добавляются проект и `_next_action`. Иллюстративная текстовая панель не является стабильной схемой.

## См. также

- [`forgeplan status`](/ru/docs/cli/status/) - необработанные счётчики без анализа состояния
- [`forgeplan blindspots`](/ru/docs/cli/blindspots/) - только вид слепых пятен
- [`forgeplan gaps`](/ru/docs/cli/gaps/) - обнаружение отсутствующих артефактов
- [`forgeplan stale`](/ru/docs/cli/stale/) - массовый обзор просроченных артефактов
- [`forgeplan context`](/ru/docs/cli/context/) - глубокий анализ каждого артефакта
- [Методология: гейты качества](/ru/docs/methodology/overview/)
