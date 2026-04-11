---
title: forgeplan_fpf_search
description: "Поиск по базе знаний FPF (First Principles Framework). По умолчанию используется поиск по ключевым словам. Передайте `semantic: true` для поиска по векторному сходству с использованием эмбеддингов BGE-M3 (требуется функция сборки `semantic-search`). Если `semantic: true` передано, но функция не скомпилирована, запрос корректно возвращается к поиску по ключевым словам, и ответ включает поле `warning`. Примечание: первое обращение с `semantic: true` может занять 10–30 секунд, если модель BGE-M3 необходимо загрузить (~150 МБ). Параметры: query (обязательный, 1..=8192 символов), limit (по умолчанию 5, макс. 50), semantic (по умолчанию false)."
---

Поиск по **базе знаний FPF (First Principles Framework)** — 204 структурированных раздела, охватывающих рассуждения, Trust Calculus (B.3), цикл ADI (B.5), ограниченные контексты и многое другое. По умолчанию используется поиск по ключевым словам BM25 с русской морфологией; передача `semantic: true` переключает на векторный поиск BGE-M3, если функция сборки `semantic-search` скомпилирована, в противном случае происходит корректный возврат к поиску по ключевым словам.

**Категория**: База знаний FPF

## Когда агент вызывает эту функцию

- **Поиск принципа** — "что FPF говорит о Trust Calculus?" → агенты часто обращаются к B.3.
- **Обоснование рассуждений ADI** — перед генерацией гипотез, получите соответствующий контекст FPF.
- **Поддержка принятия решений** — `fpf_search "explore exploit"` выводит правила баланса исследования/эксплуатации.
- **Онбординг** — помогите новому агенту быстро освоить терминологию FPF.

Конвейер идентичен обычному `forgeplan_search`: токенизатор BM25 с удалением шума шаблона + русский Snowball stemmer, опциональное векторное переранжирование с BGE-M3.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `query` | `string` | да | Поисковый запрос (1-8192 символов, обрезанный непустой). |
| `limit` | `integer` | нет (по умолчанию: `5`, макс.: `50`) | Максимальное количество возвращаемых результатов. |
| `semantic` | `bool` | нет (по умолчанию: `false`) | Использовать векторный поиск через BGE-M3. Возвращается к поиску по ключевым словам, если функция `semantic-search` не скомпилирована. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::FpfSearchParams`_

## Возвращает

```json
{
  "query": "trust calculus",
  "mode": "keyword",
  "results": [
    {
      "id": "B.3",
      "title": "Trust Calculus",
      "score": 9.41,
      "snippet": "Trust is not binary. It is a calculus over evidence, context, and recency…",
      "path": "B. Principles > B.3 Trust Calculus"
    }
  ],
  "total": 5,
  "warning": null
}
```

При возврате к поиску по ключевым словам:

```json
{
  "mode": "keyword",
  "warning": "semantic search requested but semantic-search feature not compiled — fell back to keyword"
}
```

## Пример вызова

```json
{ "query": "trust calculus", "limit": 5 }
```

Семантический вариант:

```json
{ "query": "how do agents handle uncertainty", "semantic": true }
```

## Типичная последовательность

1. `forgeplan_fpf_search` с нужным вам понятием.
2. `forgeplan_fpf_section` с верхним `id` — прочитать полный текст.
3. Передать раздел в `forgeplan_reason` для рассуждений ADI с обоснованием FPF.

## Эквивалент CLI

```bash
forgeplan fpf search "trust calculus"
forgeplan fpf search "uncertainty" --semantic
```

## См. также

- [`forgeplan_fpf_section`](/docs/mcp/forgeplan_fpf_section/) — получить раздел по ID.
- [`forgeplan_search`](/docs/mcp/forgeplan_search/) — поиск артефактов рабочего пространства (не базы знаний FPF).
- [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) — рассуждения ADI с опциональным обоснованием `--fpf`.
- [Руководство по методологии](/docs/methodology/overview/)
