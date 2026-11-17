---
title: forgeplan search
description: "Умный поиск - BM25 + семантический + расширение графа с русской морфологией"
---

Умный поиск по артефактам. Начиная с версии v0.18.0, Forgeplan поставляется с **производственным BM25** (через крейт `bm25` v2.3.2) с русским Snowball stemmer, который убирает шум шаблонов и расширяет граф на 1 переход. Семантические векторы усиливают ранжирование, когда включен флаг функции `semantic-search`.

## Когда использовать

- Вы помните концепцию, но не ID
- Онбординг - "что у нас есть по аутентификации?"
- Создание черновика нового артефакта - поиск связанных предыдущих решений для связывания
- AI-агенты, выполняющие рассуждения с дополненным поиском

## Когда не использовать

- Вы знаете точный ID → используйте [`forgeplan get`](/ru/docs/cli/get/)
- Вы хотите структурное представление → используйте [`forgeplan tree`](/ru/docs/cli/tree/) или [`forgeplan graph`](/ru/docs/cli/graph/)
- Вы хотите искать личные воспоминания, а не артефакты → используйте [`forgeplan recall`](/ru/docs/cli/recall/)

## Использование

```text
forgeplan search [OPTIONS] <QUERY>
```

## Аргументы

```text
  <QUERY>  Search query
```

## Опции

```text
  -t, --type <TYPE>      Filter by kind (prd, rfc, adr, note, ...)
  -s, --status <STATUS>  Filter by status (draft, active, superseded, deprecated, stale)
      --depth <DEPTH>    Filter by depth (tactical, standard, deep, critical)
      --with-evidence    Only artifacts with evidence linked (R_eff > 0)
      --no-evidence      Only artifacts WITHOUT evidence (blind spots)
      --since <SINCE>    Only artifacts created after this date (YYYY-MM-DD)
      --no-expand        Disable graph expansion (1-hop neighbors in results)
      --keyword          Force keyword-only search (substring grep)
      --semantic         Force semantic-only search (vector similarity)
  -n, --limit <LIMIT>    Max results to return (default: 20) [default: 20]
      --json             Output as JSON for machine consumption
  -h, --help             Print help
  -V, --version          Print version
```

## Примеры

По умолчанию — каналы BM25/подстрок, необязательные семантические оценки и расширение графа:

```bash
forgeplan search "authentication flow"
```

Сузить до активных PRD, топ-5 результатов:

```bash
forgeplan search "auth" --type prd --status active --limit 5
```

Найти решения, не имеющие подтверждающих доказательств (вариант слепого пятна):

```bash
forgeplan search "rate limit" --no-evidence
```

## Интерпретация вывода

Текст smart-режима начинается с `Found N result(s) … (smart search)`.
Результат показывает общую оценку, `ID [kind|status] "title"`, затем
сигналы `kw`, `sem`, `r`, `g`. Для соседа добавляется
`via <id> (expanded neighbor)`. Оценки ранжируют один запрос, не сравниваются
между запросами. Стемминг объединяет формы слов, не произвольные синонимы
и не все производные.

`--json` возвращает `results` и `_next_action`. Smart-результат содержит
`id`, `kind`, `status`, `title`, `score`, `keyword_score`, `semantic_score`,
`r_eff`, `graph_centrality`, `expanded_from` (может быть null),
`mode: "smart"` и `semantic_available`. Текстовый `kw` — максимум BM25
и подстрочного канала; JSON `keyword_score` — только подстрочная оценка.

### Ограничения режимов в 0.38.0

- Smart по умолчанию применяет составные фильтры и `--limit`.
  Evidence-фильтры используют кеш `r_eff_score > 0` / `== 0`, не новое
  сканирование связей. `--since` — строго после полуночи UTC указанной даты.
- `--keyword` ищет подстроку и применяет только фильтр типа. Фильтры
  статуса/глубины/доказательств/даты и `--limit` игнорируются (#544),
  хотя неверный `--since` отклоняется до выбора режима. Поля JSON:
  `id`, `kind`, `status`, `title`, `mode: "keyword"`, без оценок.
- `--semantic` требует поддержки semantic-search и эмбеддингов. Извлекает
  до 50 векторных кандидатов, фильтрует тип и возвращает максимум 10.
  Составные фильтры и `--limit` тоже игнорируются. JSON добавляет
  `similarity`, `distance`, `mode: "semantic"`; конверт —
  `unindexed_artifacts`. Артефакты без векторов не могут попасть в результат.
- `--no-expand` отключает и соседей, и усиление centrality в smart.
  Если инициализация семантики или извлечение не удались, smart использует
  keyword/граф. Первое обращение может загрузить модель эмбеддингов.

Пустой результат успешен: `results: []`. Пустой запрос и неверная дата
`--since` — ошибки. MCP-поиск — отдельная поверхность: его фильтры и поля
оценок не обязательно совпадают с режимами CLI.

## Как это вписывается

`search` - это основная поверхность для обнаружения:

```
search → get → link (during artifact authoring)
search → get → reason (during AI planning)
```

Полную информацию о конвейере (индексирование, стемминг, параметры BM25, расширение графа) см. в [руководстве по Search v2](/ru/docs/guides/search-v2/).

## См. также

- [Руководство по Search v2](/ru/docs/guides/search-v2/) - архитектура и настройка
- [`forgeplan get`](/ru/docs/cli/get/) - прочитать совпадение полностью
- [`forgeplan recall`](/ru/docs/cli/recall/) - поиск воспоминаний (не артефактов)
- [`forgeplan reindex`](/ru/docs/cli/reindex/) - перестроить индекс BM25
