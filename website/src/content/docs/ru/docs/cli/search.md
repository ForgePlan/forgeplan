---
title: forgeplan search
description: "Умный поиск - BM25 + семантический + расширение графа с русской морфологией"
---

Поиск по артефактам сочетает keyword-ранжирование и, при наличии условий, семантические сигналы и расширение графа. Для семантики нужны совместимая сборка, доступная локальная модель и подходящие опубликованные векторы. Чтение не скачивает модель и не создаёт отсутствующие эмбеддинги автоматически.

В локальной сборке `0.39.0+constellation.s1.1` мигрированные workspace
используют Constellation. Обычный `search` по умолчанию возвращает до
**20** результатов; отдельная команда `constellation search` - до **10**.
Параметры и формат ответа одного интерфейса нельзя переносить на другой.

Принудительный `--semantic` на мигрированном workspace не гарантирует
keyword fallback. Проверяйте доступность и coverage, а не считайте пустой
ответ полноценным результатом semantic-поиска. См.
[руководство Constellation](/ru/docs/guides/constellation/) и
[справочник](/ru/docs/reference/constellation/).

Проверяйте фильтры в выбранном режиме: принудительные keyword/semantic пути не обязаны применять все фильтры умного поиска одинаково. Наличие флага в справке не доказывает одинаковую семантику каждого пути.

## Когда использовать

- Вы помните концепцию, но не ID
- Онбординг - "что у нас есть по аутентификации?"
- Создание черновика нового артефакта - поиск связанных предыдущих решений для связывания
- AI-агенты, выполняющие рассуждения с дополненным поиском

## Когда не использовать

- Вы знаете точный ID → используйте [`forgeplan get`](/ru/docs/cli/get/)
- Нужен структурный вид → [`forgeplan tree`](/ru/docs/cli/tree/) или [`forgeplan graph`](/ru/docs/cli/graph/)
- Нужны личные записи памяти, а не артефакты → [`forgeplan recall`](/ru/docs/cli/recall/)

## Использование

```text
forgeplan search [OPTIONS] <QUERY>
```

## Аргументы

```text
  <QUERY>  Поисковый запрос
```

## Опции

```text
  -t, --type <TYPE>      Фильтр по типу (prd, rfc, adr, note, ...)
  -s, --status <STATUS>  Фильтр по статусу (draft, active, superseded, deprecated, stale)
      --depth <DEPTH>    Фильтр по глубине (tactical, standard, deep, critical)
      --with-evidence    Только артефакты со связанными доказательствами (R_eff > 0)
      --no-evidence      Только артефакты БЕЗ доказательств (слепые пятна)
      --since <SINCE>    Только артефакты, созданные после этой даты (YYYY-MM-DD)
      --no-expand        Отключить расширение графа (соседи на 1 переход в результатах)
      --keyword          Принудительный поиск только по ключевым словам (grep по подстроке)
      --semantic         Принудительный поиск только по семантике (векторное сходство)
  -n, --limit <LIMIT>    Максимальное количество результатов (по умолчанию: 20) [по умолчанию: 20]
      --json             Вывод в формате JSON для машинной обработки
  -h, --help             Вывести справку
  -V, --version          Вывести версию
```

## Примеры

Обычный поиск; использование семантики зависит от доступности модели и векторов:

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

Иллюстративный вывод, не записанный runtime transcript:

```
PRD-001  [active ]  score=8.42  Auth system (BM25 + semantic)
RFC-002  [active ]  score=6.71  Token refresh flow      (graph-expanded from PRD-001)
ADR-004  [active ]  score=5.30  JWT vs session cookies
```

Столбцы: `ID`, `STATUS`, `SCORE`, `TITLE`, плюс маркер `(graph-expanded)`, когда совпадение было получено как сосед на 1 переход, а не как прямое совпадение термина. Оценки не сопоставимы между запросами - они ранжируются только в рамках одного вызова.

Русская нормализация в соответствующем BM25-пути использует Snowball stemmer. Это не гарантия совпадения всех однокоренных слов, в частности «авторизация» и «авторизованный», и не описание принудительного поиска подстроки.

## Как это вписывается

`search` - это основная поверхность для обнаружения:

```
search → get → link (during artifact authoring)
search → get → reason (during AI planning)
```

Подробности конвейера см. в [руководстве Search v2](/ru/docs/guides/search-v2/).

## См. также

- [Руководство Search v2](/ru/docs/guides/search-v2/) - архитектура поиска
- [`forgeplan get`](/ru/docs/cli/get/) - прочитать результат полностью
- [`forgeplan recall`](/ru/docs/cli/recall/) - поиск записей памяти
- [`forgeplan reindex`](/ru/docs/cli/reindex/) - перестройка индекса

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan search --help`.

```text
Search artifacts (smart by default: keyword + semantic + boosters)

Usage: forgeplan search [OPTIONS] <QUERY>

Arguments:
  <QUERY>  Search query

Options:
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
<!-- forgeplan-runtime:end -->
