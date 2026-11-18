---
title: forgeplan health
description: "Панель мониторинга состояния проекта - слепые пятна, сироты, просроченные и рискованные артефакты"
---

`forgeplan health` показывает задолженность workspace: **слепые пятна**, **сироты**, просроченные даты, **рискованные** доказательства, активные заглушки, возможные дубликаты и рекомендательные сигналы фаз. Для migrated используется канонический снимок, для legacy - старый индекс. Эти сигналы не заменяют структурную валидацию или приёмку реализации.

**Единый протокол рабочего процесса** делает `forgeplan health` обязательной командой в начале каждой сессии - прежде чем вы напишете код, выполните роутинг задачи или откроете новый PR. Если проверка состояния показывает наличие задолженности, вы устраняете её, прежде чем приступать к новой работе. Не накапливайте задолженность.

## Когда использовать

- **Начало сессии** - всегда, без исключений. Часть стандартного протокола.
- Перед созданием PR - убедитесь, что ваша ветка не внесла новую задолженность.
- В CI/CD - используйте режим `--ci`, чтобы блокировать конвейеры при регрессиях.
- После `forgeplan scan-import` или любого массового импорта.
- Еженедельный обзор спринта - проверяйте общие тенденции состояния проекта.
- Перед релизом - `--ci --fail-on "orphans=0,blind_spots=0"` в качестве гейта релиза.

## Когда НЕ использовать

- В плотном цикле редактирования-компиляции - слишком много шума для запусков при каждом сохранении.
- Для одного артефакта - используйте `forgeplan context <ID>` или `forgeplan validate <ID>`.
- В качестве замены `forgeplan status` - `health` отмечает проблемы; `status` даёт необработанные счётчики.

## Использование

```text
forgeplan health [OPTIONS]
```

## Опции

```text
      --compact            Сокращённый однострочный вывод для хуков/скриптов
      --json               Вывод в формате JSON для машинной обработки
      --ci                 Текстовый гейт: ошибка при превышении настроенных порогов
      --fail-on <FAIL_ON>  Пороги сбоя для --ci (например, "orphans=5,blind_spots=3,stale=2")
      --strict             Ошибка при критических сигналах; действует и с --json
  -h, --help               Вывести справку
  -V, --version            Вывести версию
```

## Примеры

### Пример 1: Проверка в начале сессии

```bash
forgeplan health
```

Иллюстрация здоровой сводки, не записанный transcript и не полная схема вывода:

```
Project Health
==============
  Artifacts:    147 (PRD: 42, RFC: 18, ADR: 12, Evidence: 56, ...)
  Active:       98
  Blind spots:  0
  Orphans:      0
  Stale:        0
  At risk:      0

Project looks healthy!
```

### Пример 2: Состояние с задолженностью - устраните её перед новой работой

```bash
forgeplan health
```

Иллюстрация сводки задолженности, не записанный runtime transcript:

```
Project Health
==============
  Blind spots:  2
    - PRD-019 (active, 0 evidence, R_eff=0.00)
    - RFC-007 (active, 0 evidence, R_eff=0.00)
  Orphans:      1
    - NOTE-033 (no parent, no children, no links)
  Stale:        3
    - ADR-004 (valid_until 2026-03-01, expired 41 days ago)
  At risk:      0

Next actions:
  1. Create EvidencePack for PRD-019 and RFC-007
  2. Link NOTE-033 to its parent or deprecate
  3. Review and renew/supersede ADR-004
```

Устраните каждую проблему, прежде чем приступать к новой работе.

### Пример 3: Гейт конвейера CI

```bash
forgeplan health --ci --fail-on "orphans=0,blind_spots=0,stale=5"
```

Код 1 означает наличие сирот, слепых пятен или больше пяти просроченных записей. Запускайте после предусмотренного импорта или обслуживания канонического индекса. `scan-import` импортирует документы, не восстанавливает Constellation штатным образом.

### Пример 4: Машиночитаемый вывод

```bash
forgeplan health --json
```

JSON содержит `total`, `verdict`, `by_kind`, `by_status`, `blind_spots`, `orphans`, `stale_count`, `at_risk`, `active_stubs`, возможные дубликаты и рекомендательные поля. Просроченность представлена счётчиком `stale_count`, не массивом `stale`. `--strict --json` также добавляет `exit_code`.

### Пример 5: Компактный режим для командных строк

```bash
forgeplan health --compact
# Вывод включает проект, число артефактов, слепые пятна, просроченность и at-risk.
```

Используйте в командной строке zsh/bash или в строке состояния tmux, чтобы состояние было всегда видно.

## Режим CI (`--ci`)

Добавленный в Спринте 11.3 (PRD-034, гейты целостности методологии), `forgeplan health --ci` завершается с ненулевым кодом, когда рабочее пространство превышает настроенные пороги задолженности. Это превращает панель мониторинга состояния в **гейт PR** или **блокировщик конвейера**, а не просто в рекомендательную проверку в начале сессии.

### Поведение

- Обычный `health --ci` отказывает при любых `orphans` или `blind_spots`. Неуказанные пороги `stale` и `at_risk` не ограничены.
- `--fail-on` принимает `метрика=N` через запятую для `orphans`, `blind_spots`, `stale` и `at_risk`; ошибка означает превышение порога.
- В выбранной реализации `--json` и `--compact` завершают обработку до ветки `--ci`. Не полагайтесь на `--ci` в этих режимах. Для JSON есть отдельный `--strict`; это другая политика гейта.

### Фрагмент GitHub Actions

Добавьте это в `.github/workflows/ci.yml` после вашей задачи `cargo test`:

```yaml
- name: Forgeplan health gate
  run: |
    # После обслуживания владеющего индекса для нужных источников.
    forgeplan health --ci --fail-on "blind_spots=0,orphans=0"
```

Эти пороги отвергают учтённые слепые пятна и сирот; stale и at-risk не ограничены без явного порога. Это не самая строгая возможная политика. Выбирайте нужный гейт осознанно; не ослабляйте уже установленный гейт только ради зелёного CI.

Совместите это с [`forgeplan validate --ci`](/ru/docs/cli/validate/), чтобы отдельно проверять методологическую задолженность и структурную полноту.

## Интерпретация вывода

- **Слепые пятна** - нечерновые и нетерминальные решения без связанных доказательств. Нулевая оценка при связанном refutes-доказательстве - другое условие. Добавляйте реальные EvidencePack с `verdict`, `congruence_level` и `evidence_type`.
- **Сироты** - нет входящих или исходящих связей. Либо незавершённые, либо забытые. Исправление: свяжите с родительским/дочерним артефактом или отмените.
- **Просроченные** - дата действия прошла; обнаружение само не меняет сохранённый статус. Проверьте статус и применимый переход `renew --reason --until` или `reopen --reason`, который создаёт новый draft и помечает исходный deprecated.
- **Рискованные** - живой артефакт со связанными доказательствами и плоским R_eff ниже 0.3. Структурная валидация и калибровка глубины проверяются отдельно.

Пороги для беспокойства:

- Любые слепые пятна = немедленное исправление (вы заявляете об активных решениях без доказательств).
- Сироты > 3 = утечка в рабочем процессе, найдите, где артефакты остаются без внимания.
- Просроченные > 5 - изучите записи через `forgeplan stale`; команды `forgeplan refresh` нет.

## Как это вписывается в рабочий процесс

```
[health] → route → Shape → Validate → Reason → Code → Evidence → Activate → [health]
   ^                                                                           ^
начало сессии                                                            конец спринта
```

- **Начало сессии**: всегда запускайте первым. Устраните задолженность перед новой работой.
- **Перед PR**: запустите снова, чтобы убедиться, что ваша ветка чиста.
- **Гейт релиза**: режим CI со строгими порогами `--fail-on`.

## См. также

- [`forgeplan status`](/ru/docs/cli/status/) - счётчики
- [`forgeplan blindspots`](/ru/docs/cli/blindspots/) - слепые пятна
- [`forgeplan gaps`](/ru/docs/cli/gaps/) - этапы конвейера
- [`forgeplan stale`](/ru/docs/cli/stale/) - просроченные даты
- [`forgeplan context`](/ru/docs/cli/context/) - контекст артефакта
- [Гейты качества](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan health --help`.

```text
Show project health dashboard — gaps, risks, blind spots, next actions

Usage: forgeplan health [OPTIONS]

Options:
      --compact            Compact one-line output for hooks/scripts
      --json               Output as JSON for machine consumption
      --ci                 CI mode: exit code 1 if issues found (for pipeline gates)
      --fail-on <FAIL_ON>  Fail thresholds for --ci (e.g., "orphans=5,blind_spots=3,stale=2")
      --strict             Strict mode: exit 1 if verdict is NeedsAttention/Unhealthy or any of {orphans, blind_spots, active_stubs, at_risk} > 0. Designed for CI gates that want a single boolean signal. Empty workspaces and advisory-only signals (e.g. phase mismatches) keep exit 0
  -h, --help               Print help
  -V, --version            Print version
```
<!-- forgeplan-runtime:end -->
