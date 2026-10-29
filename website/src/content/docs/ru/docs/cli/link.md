---
title: forgeplan link
description: "Связывает два артефакта типизированной связью"
---

Создает типизированную, направленную связь от одного артефакта к другому. Связи - это то, как Forgeplan строит граф решений; они обеспечивают работу проверок состояния, подсчета R_eff, графа зависимостей Mermaid, топологической сортировки для планирования спринтов и обнаружения слепых пятен.

У значимого артефакта должна быть осмысленная связь. [`forgeplan health`](/ru/docs/cli/health/) показывает сирот; не добавляйте бессодержательные связи только ради счётчика.

## Использование

```text
forgeplan link [OPTIONS] <SOURCE> <TARGET>
```

## Аргументы

```text
  <SOURCE>  ID исходного артефакта
  <TARGET>  ID целевого артефакта
```

## Опции

```text
      --relation <RELATION>  Тип связи: informs, based_on, supersedes, contradicts, refines [по умолчанию: informs]
  -h, --help                 Вывести справку
  -V, --version              Вывести версию
```

## Допустимые связи (v0.18)

Forgeplan использует закрытый словарь. Принимаются только эти пять типов связей - любое другое значение будет отклонено валидатором.

| Связь         | Направление читается как...                          | Когда использовать                                                                 |
|---------------|------------------------------------------------|-----------------------------------------------------------------------------|
| `informs`     | *источник* информирует *цель*                      | Доказательство, подтверждающее PRD; Note, используемый в ADR. Наиболее распространенный.         |
| `based_on`    | *источник* основан на *цели*                  | RFC, построенный на основе более раннего PRD; PRD, полученный из ProblemCard.         |
| `supersedes`  | *источник* замещает *цель*                   | Новый ADR заменяет старый. Устанавливается автоматически командой `forgeplan supersede`.    |
| `contradicts` | *источник* противоречит *цели*                  | Доказательство, опровергающее решение; RFC, отвергающий более ранний подход.    |
| `refines`     | *источник* уточняет *цель*                      | Spec, который уточняет PRD; ADR, который сужает пространство проектирования RFC.          |

## Примеры

Канонический поток доказательств - свяжите новый EvidencePack с PRD, который он поддерживает:

```bash
forgeplan new evidence "Benchmark: BM25 vs TF-IDF on Russian corpus"
# ... заполните verdict, congruence_level, evidence_type ...
forgeplan link EVID-001 PRD-039 --relation informs
forgeplan score PRD-039   # изучите вклад; одна связь не доказывает положительную оценку
```

Выведите RFC из PRD:

```bash
forgeplan link RFC-006 PRD-025 --relation based_on
```

Зафиксируйте противоречие (Доказательство, которое опровергает):

```bash
forgeplan link EVID-017 ADR-004 --relation contradicts
```

Уточните PRD с помощью Spec:

```bash
forgeplan link SPEC-003 PRD-018 --relation refines
```

## Распространенные ошибки: несуществующие связи

Старые документы и результаты LLM иногда предлагают типы связей, которые **не** являются допустимыми в v0.18. Вот таблица соответствий:

| Вы могли бы ввести... | Используйте вместо                          | Почему                                                       |
|-------------------|---------------------------------------|-----------------------------------------------------------|
| `solves`          | `based_on` (PRD → ProblemCard)       | Решение *основано на* проблеме, которую оно решает         |
| `extends`         | `refines` или `based_on`              | `refines` для сужения области; `based_on` для наследования |
| `blocks`          | `based_on` для заблокированного артефакта   | Зависимости выражаются тем, на чем построен дочерний элемент  |
| `depends_on`      | `based_on`                            | Та же семантика                                            |
| `implements`      | `refines`                             | Spec/RFC уточняет намерение PRD                      |
| `references`      | `informs`                             | Мягкая связь по умолчанию                                  |

Если сомневаетесь, выберите `informs` - это нейтральная связь «этот артефакт имеет отношение к тому».

## Направление имеет значение

Связи направлены: `forgeplan link PRD-001 EVID-001 --relation informs` означает *PRD-001 информирует EVID-001*. Канонический поток поддержки: **доказательство → решение**. Скорер находит доказательства в обе стороны, но смысл связей от этого не становится взаимозаменяемым.

```bash
forgeplan link EVID-001 PRD-001 --relation informs   # правильно
```

При неверном направлении удалите конкретную связь через [`forgeplan unlink`](/ru/docs/cli/unlink/) и создайте правильную.

## Самостоятельные ссылки заблокированы

Начиная с PROB-019, `forgeplan link PRD-001 PRD-001` отклоняется. Это предотвращает опечатки, которые в противном случае создали бы самореферентные узлы в графе и запутали бы топологическую сортировку.

## Что происходит после связывания

- Канонический Markdown хранит связь; legacy-таблица `relations` производная. На migrated-workspace изменения идут file-first с reconcile Constellation, без воссоздания старых таблиц.
- Вызовите [`forgeplan score`](/ru/docs/cli/score/) для решения. Новый supports-пакет не обязательно увеличивает минимум: более слабый допустимый пакет может его уменьшить.
- [`forgeplan graph`](/ru/docs/cli/graph/) показывает ребро.
- [`forgeplan blocked`](/ru/docs/cli/blocked/) и [`forgeplan order`](/ru/docs/cli/order/) используют структурные зависимости `based_on` / `refines` / `supersedes`, а не блокирующую связь `informs`.

## См. также

- [Обзор CLI](/ru/docs/cli/)
- [`forgeplan unlink`](/ru/docs/cli/unlink/) - удалить связь
- [`forgeplan score`](/ru/docs/cli/score/) - оценка R_eff
- [`forgeplan graph`](/ru/docs/cli/graph/) - граф
- [`forgeplan health`](/ru/docs/cli/health/) - задолженность
- [Методология](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan link --help`.

```text
Link two artifacts with a typed relationship

Usage: forgeplan link [OPTIONS] <SOURCE> <TARGET>

Arguments:
  <SOURCE>  Source artifact ID
  <TARGET>  Target artifact ID

Options:
      --relation <RELATION>     Relationship type: informs, based_on, supersedes, contradicts, refines [default: informs]
      --replace                 Issue #286: if an edge already exists between source and target with a DIFFERENT relation, replace it (idempotent upsert). Without this flag, an existing edge with a different relation surfaces as an error and the call fails — matching the pre-issue-#286 behaviour. Used to fix mis-typed `based_on` / `informs` choices without destroying the artifact via `forgeplan delete`
      --auto-activate-complete  Issue #288 Part A: when the link source is a complete evidence artifact (kind=evidence + status=draft + body has verdict + congruence_level), silently activate it draft → active in the same call. Mirrors `auto_activate_source_if_complete` on the MCP `forgeplan_link` tool. No-op when the source doesn't meet the criteria
  -h, --help                    Print help
  -V, --version                 Print version
```
<!-- forgeplan-runtime:end -->
