---
title: forgeplan anomalies
description: "Issue #289 - обнаружение аномалий конвейера (застрявшие черновики, оборванные связи, опечатки в based_on и т. д.). Каждая запись содержит severity + tier + suggested_resolution, чтобы оркестраторы могли направить исправления"
---

Обнаруживает аномалии конвейера во всём рабочем пространстве: застрявшие черновики, оборванные связи,
опечатки в отношениях `based_on`, отсутствующие обязательные секции, просроченные доказательства и
так далее. Каждая запись содержит **severity** и **resolution tier**, а также
`suggested_resolution`, чтобы оркестрирующий агент мог направить исправление без
повторной классификации.

`anomalies` - это CLI-зеркало MCP-инструмента
[`forgeplan_anomalies`](/ru/docs/mcp/forgeplan_anomalies/): те же
входные данные, та же семантика и тот же JSON-пейлоад при `--json`.

## Использование

```text
forgeplan anomalies [OPTIONS]
```

## Опции

```text
      --kind <KIND>          Фильтр по типу аномалии (snake_case: stuck_draft, orphan_link, mistyped_based_on, missing_must_section, expired_evidence, weakest_link_unresolvable, phase_mismatch, circular_dependency, duplicate_artifact)
      --severity <SEVERITY>  Фильтр по серьёзности (low, medium, high)
      --since <SINCE>        Возвращать только аномалии, обнаруженные в указанную или после указанной RFC 3339 метки времени
      --json                 Вывод в формате JSON вместо человекочитаемой таблицы
  -h, --help                 Вывести справку
  -V, --version              Вывести версию
```

## Что делает

1. Открывает хранилище рабочего пространства и запускает канонический детектор из
   `forgeplan_core::anomalies`.
2. Сравнивает обнаруженный набор с `.forgeplan/anomalies-journal.jsonl`,
   журналом append-only, содержащим каждый id аномалии и метку времени первого
   обнаружения.
3. Применяет фильтры `--kind`, `--severity` и `--since`.
4. Сохраняет обновлённый журнал (атомарный tempfile + rename), чтобы следующий запуск
   мог сообщить о разнице, а не о полном снимке.
5. Отображает человекочитаемую таблицу или структурированный пейлоад при `--json`.

## Каталог аномалий

Каталог определён в `crates/forgeplan-core/src/anomalies.rs`. Первые девять типов - это каталог конвейера v1 из issue #289; следующие пять - расширение brownfield из Epic #287; `unbacked_displacement` - защита от score-laundering из ADR-020.

| Тип | Что означает |
|---|---|
| `stuck_draft` | Доказательства полны и связаны, но артефакт всё ещё в статусе `draft` спустя 24 часа |
| `orphan_link` | Отношение указывает на id артефакта, которого нет в рабочем пространстве |
| `mistyped_based_on` | Доказательство связано с PRD/RFC/ADR через `based_on` вместо `informs` |
| `missing_must_section` | Артефакт со `status=active` всё ещё содержит маркеры-заглушки (`TODO`, `TBD`, `placeholder`, `XXX`) |
| `expired_evidence` | `valid_until` в прошлом, но доказательство всё ещё связано с активным артефактом |
| `weakest_link_unresolvable` | R_eff упал до 0 из-за жёсткого штрафа CL выше по цепочке |
| `phase_mismatch` | `status=active`, но фаза жизненного цикла всё ещё Shape или Validate |
| `circular_dependency` | Два артефакта связаны друг с другом через `based_on` |
| `duplicate_artifact` | Два артефакта с эквивалентными заголовками и пересекающимися телами |
| `hypothesis_duplicate` | Две гипотезы с Jaccard-индексом токенов заголовка 0.6 или выше |
| `uncovered_use_case` | У use case нет связанного сценария |
| `unverified_invariant` | У инварианта нет связанного сценария |
| `orphan_glossary_term` | Термин глоссария никогда не упоминается в теле use case или инварианта |
| `untriangulated_hypothesis` | Гипотеза ссылается менее чем на два различных EvidencePack |
| `unbacked_displacement` | Враждебное доказательство было прекращено без преемника `supersedes`, пока его цель всё ещё активна |

Используйте `--kind <snake_case>` для сужения до одного типа. Это фильтр с одним значением - один тип на вызов, точное совпадение. Неизвестное значение отклоняется, а не возвращает пустой результат:

```bash
forgeplan anomalies --kind bogus
```

Типичный вывод:

```text
Error: Unknown anomaly kind: bogus
Valid kinds (pipeline): stuck_draft, orphan_link, mistyped_based_on, missing_must_section, expired_evidence, weakest_link_unresolvable, phase_mismatch, circular_dependency, duplicate_artifact
Valid kinds (brownfield, Epic #287): hypothesis_duplicate, uncovered_use_case, unverified_invariant, orphan_glossary_term, untriangulated_hypothesis
```

## Severity и tier

`--severity` принимает `low`, `medium` или `high` и требует точного совпадения. Любое другое значение приводит к быстрому отказу:

```bash
forgeplan anomalies --severity critical
```

Типичный вывод:

```text
Error: Unknown severity: critical
Valid severities: low, medium, high
```

`--tier` **не существует**. Tier - это поле каждой записи, а не фильтр, и оно предназначено для чтения оркестратором, который потребляет отчёт:

| Tier | Значение | Кто действует |
|---|---|---|
| `auto` | Низкая серьёзность, однозначно | Оркестратор исправляет молча |
| `adi` | Средняя неоднозначность | Цикл ADI в FPF в оркестраторе или агенте |
| `user` | Высокая серьёзность, необратимо или неоднозначно | Эскалация с вопросом |

Передача `--tier` - жёсткая ошибка:

```text
$ forgeplan anomalies --tier auto
error: unexpected argument '--tier' found
```

## Фильтр diff `--since`

`--since` принимает метку времени RFC 3339 и сравнивает её с записью журнала для каждой аномалии, а не с текущим временем сканирования. Аномалия, впервые обнаруженная вчера, сохраняет вчерашний `observed_at`, поэтому цикл опроса с `--since <last-poll>` видит только действительно новое. Любая нотация часового пояса работает (`Z`, `+00:00`, `+05:00`), потому что сравнение идёт по распарсенным меткам времени. Неправильное значение отклоняется с примером:

```text
$ forgeplan anomalies --since yesterday
Error: --since is not a valid RFC 3339 timestamp (yesterday): premature end of input
Example: 2026-05-19T00:00:00Z
```

Если журнал отсутствует или нечитаем, каждая аномалия считается свежей для этого сканирования - фильтр деградирует до семантики полного снимка вместо отказа.

## Структура записи

| Поле | Тип | Описание |
|---|---|---|
| `id` | string | Детерминированный id, выведенный из типа и затронутых артефактов, стабилен между запусками |
| `kind` | string | Канонический snake_case тип из каталога |
| `severity` | string | `low`, `medium` или `high` |
| `tier` | string | `auto`, `adi` или `user` |
| `affected` | array of string | Id артефактов, первичный артефакт первым |
| `observed_at` | string | Метка времени RFC 3339, сохраняется между сканированиями через журнал |
| `description` | string | Человекочитаемое резюме, не более 200 символов |
| `evidence` | object | Поддерживающие поля для конкретного типа (например, `age_hours`, `jaccard_score`) |
| `suggested_resolution` | object or null | `tier`, `action`, `target` и однострочный `rationale` |

В текстовом режиме ненулевое разрешение отображается как отступная строка `-> {action} {target}` под аномалией; когда `action` отсутствует, рендерер выводит `(manual)`.

## Примеры

### Пример 1: здоровое рабочее пространство

```bash
forgeplan anomalies
```

Типичный вывод:

```text
No pipeline anomalies detected. ✓
Done.
```

### Пример 2: активный артефакт, полный шаблонных маркеров

```bash
forgeplan anomalies
```

Типичный вывод:

```text
forgeplan anomalies — 1 detected
  by severity: 1 high, 0 medium, 0 low
  by tier:     0 auto, 0 adi, 1 user

  ✗  [user] missing_must_section — PRD-001: active artifact has stub markers (TODO/TBD/placeholder)

Next: forgeplan get <affected-id>  # review user-tier anomalies
```

Завершающая подсказка выбирается из счётчиков tier: сначала выигрывает `auto`, затем `user`, затем `adi`.

### Пример 3: тот же отчёт в JSON

```bash
forgeplan anomalies --json
```

```json
{
  "_next_action": "1 anomaly(ies) detected — 1 require user input. Run `forgeplan_get <id>` on affected artifacts to review.",
  "anomalies": [
    {
      "affected": ["PRD-001"],
      "description": "PRD-001: active artifact has stub markers (TODO/TBD/placeholder)",
      "evidence": {
        "markers_found": 6,
        "message": "Body appears to be unfilled template (6 markers found)",
        "title": "Tighten the widget pipeline"
      },
      "id": "anom-missing-must-PRD-001",
      "kind": "missing_must_section",
      "observed_at": "2026-09-16T13:50:13.040412+00:00",
      "severity": "high",
      "suggested_resolution": null,
      "tier": "user"
    }
  ],
  "by_severity": { "high": 1, "low": 0, "medium": 0 },
  "by_tier": { "adi": 0, "auto": 0, "user": 1 },
  "total": 1
}
```

### Пример 4: только срез с высокой серьёзностью

```bash
forgeplan anomalies --severity high
```

Типичный вывод:

```text
forgeplan anomalies — 1 detected
  by severity: 1 high, 0 medium, 0 low
  by tier:     0 auto, 0 adi, 1 user

  ✗  [user] missing_must_section — PRD-001: active artifact has stub markers (TODO/TBD/placeholder)

Next: forgeplan get <affected-id>  # review user-tier anomalies
```

### Пример 5: по одному типу за раз

```bash
forgeplan anomalies --kind stuck_draft
```

Типичный вывод:

```text
No pipeline anomalies detected. ✓
Done.
```

### Пример 6: опрос только того, что изменилось с последней проверки

```bash
forgeplan anomalies --since 2026-09-16T00:00:00Z
```

Типичный вывод:

```text
No pipeline anomalies detected. ✓
Done.
```

## Примечания

- **Код выхода всегда 0 при успешном сканировании**, независимо от того, были ли обнаружены аномалии. Это консультативная поверхность для оркестраторов, а не гейт; используйте [`forgeplan health --ci`](/ru/docs/cli/health/), когда вам нужен ненулевой выход для блокировки конвейера.
- Журнал находится в `.forgeplan/anomalies-journal.jsonl` - один JSON-объект на строку (`id`, `observed_at`). Это эвристическое хранилище, поэтому нераспарсиваемая строка пропускается, а не прерывает сканирование.
- Отчёт идёт в stdout, поэтому `--json` остаётся чистым для передачи в `jq`.
- Id аномалий детерминированы (`anom-missing-must-PRD-001`), что делает их безопасными для использования в качестве ключей дедупликации или ссылок на тикеты.

## Связанные команды

- [`forgeplan health`](/ru/docs/cli/health/) - панель запуска сессии, которая оборачивает те же обнаружения в терминах слепых пятен / сирот / просроченных.
- [`forgeplan blindspots`](/ru/docs/cli/blindspots/) - представление только слепых пятен.
- [`forgeplan gaps`](/ru/docs/cli/gaps/) - обнаружение отсутствующих артефактов.
- [`forgeplan stale`](/ru/docs/cli/stale/) - массовая проверка `valid_until`.

## См. также

- [Обзор CLI](/ru/docs/cli/)
- [`forgeplan_anomalies`](/ru/docs/mcp/forgeplan_anomalies/) - MCP-зеркало с идентичным выводом
- [`forgeplan health`](/ru/docs/cli/health/) - панель запуска сессии
- [`forgeplan score`](/ru/docs/cli/score/) - исследование сигналов R_eff за tier adi
- [Руководство по методологии](/ru/docs/methodology/overview/)