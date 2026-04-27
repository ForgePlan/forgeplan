---
title: forgeplan dispatch
description: "Compute a parallel-safe work plan for N sub-agents — buckets, serial queue, reasoning. Read-only; the entry point for multi-agent orchestration."
---

`forgeplan dispatch` планирует параллельную работу для нескольких агентов сразу. Вы говорите, сколько агентов есть; команда разбивает кандидатные артефакты на корзины (buckets) — по одной на агента — плюс последовательную очередь (serial queue) для всего, что нельзя пустить параллельно. Цель: каждую корзину можно вести одновременно, без того чтобы два агента трогали одни и те же файлы.

Как планировщик избегает конфликтов:

- **Активные клеймы пропускаются** — если другой агент уже зарезервировал артефакт через [`forgeplan claim`](/ru/docs/cli/claim/), он исключается из плана.
- **Проверяется пересечение файлов** — если два артефакта затрагивают одни и те же файлы, второй уходит в последовательную очередь. Планировщик использует **меру пересечения Жаккара** (Jaccard similarity) — это доля совпадающих путей: 0.3 значит «30% и более общих файлов».
- **Учитываются зависимости** — если артефакт A блокирует B по графу, B не попадёт в корзину, пока A не закрыт.
- **Опциональные фильтры** — `--epic` / `--kind` сужают кандидатов до одного Epic или одного типа артефактов.

Команда только читает: ничего не меняет в воркспейсе. После dispatch каждый агент должен вызвать [`forgeplan claim`](/ru/docs/cli/claim/) на свой артефакт, прежде чем трогать файлы. Аналог [`forgeplan_dispatch`](/ru/docs/mcp/forgeplan_dispatch/) на MCP-стороне — LLM-агенты обычно используют MCP-инструмент, shell-скрипты этот CLI.

## Когда использовать

- Старт multi-agent спринта на 2–5 саб-агентов, которые должны работать без столкновений.
- После [`forgeplan release`](/ru/docs/cli/release/) — освободился слот, перепланируйте, чтобы его заполнить.
- После [`forgeplan new`](/ru/docs/cli/new/) появились свежие черновики — перепланируйте, чтобы новые артефакты получили исполнителей.
- Зависший клейм истёк по TTL — перепланируйте, чтобы передать артефакт свежему агенту.

## Когда НЕ использовать

- Только один агент — планировать нечего, просто берите следующий артефакт.
- Ожидаете, что команда что-то заклеймит или изменит — она этого не делает. После dispatch каждый агент должен вручную вызвать [`forgeplan claim`](/ru/docs/cli/claim/).
- Без предварительной сверки с [`forgeplan claims`](/ru/docs/cli/claims/) — dispatch и так исключает заклейменные артефакты, но знание о работе в полёте поможет правильно выбрать число `--agents`.

## Использование

```text
forgeplan dispatch [OPTIONS] --agents <AGENTS>
```

## Опции

```text
  -n, --agents <AGENTS>
          Number of sub-agents the orchestrator can hand work to (>=1, max 64)
      --epic <EPIC>
          Optional filter: only artifacts with this parent Epic ID
  -t, --kind <KIND>
          Optional filter: only consider artifacts of this kind (prd/rfc/spec/...)
  -s, --status <STATUS>
          Status filter (default `draft`; pass `any` for all states) [default: draft]
      --overlap-threshold <OVERLAP_THRESHOLD>
          Jaccard threshold for file-overlap conflict detection (default 0.3) [default: 0.3]
      --json
          Output as JSON for machine consumption
  -h, --help
          Print help
  -V, --version
          Print version
```

## Примеры

### Пример 1: Планирование 3 агентов на черновые PRD

```bash
forgeplan dispatch --agents 3 --kind prd
```

Возвращает три корзины черновых PRD, которые можно вести параллельно без конфликтов по файлам. Оркестратор передаёт `buckets[0]` агенту 0, `buckets[1]` агенту 1 и так далее.

### Пример 2: Перепланирование всего Epic независимо от статуса

```bash
forgeplan dispatch --agents 4 --epic EPIC-005 --status any
```

Планирует все артефакты `EPIC-005`, включая активные и superseded (не только черновики). Полезно для ретроспектив или когда Epic смешивает черновые и активные артефакты, требующие внимания.

### Пример 3: Строже отлавливать конфликты

```bash
forgeplan dispatch --agents 2 --overlap-threshold 0.15 --json
```

Понижает порог пересечения файлов до 0.15 (15% общих путей вместо дефолтных 30%) — даже умеренные пересечения уйдут в последовательную очередь, а не пойдут параллельно. Используйте, когда агенты постоянно сталкиваются на общих файлах.

## Место в рабочем процессе

Multi-agent работа — это четырёхшаговый цикл: `dispatch` → `claim` → работа → `release` → снова `dispatch`. Оркестратор владеет `dispatch` (и `release --force` для подбирания за упавшими агентами); каждый саб-агент владеет `claim` и `release` для своего артефакта. Между диспатчами используйте [`forgeplan claims`](/ru/docs/cli/claims/), чтобы видеть, кто над чем работает.

## См. также

- [`forgeplan_dispatch`](/ru/docs/mcp/forgeplan_dispatch/) — MCP-эквивалент
- [`forgeplan claim`](/ru/docs/cli/claim/) — саб-агент берёт элемент bucket
- [`forgeplan release`](/ru/docs/cli/release/) — вернуть слот в пул
- [`forgeplan claims`](/ru/docs/cli/claims/) — мониторить in-flight работу
- [Обзор CLI](/ru/docs/cli/)
