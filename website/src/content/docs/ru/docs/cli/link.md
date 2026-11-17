---
title: forgeplan link
description: "Связывает два артефакта типизированной связью"
---

Создает типизированную, направленную связь от одного артефакта к другому. Связи - это то, как Forgeplan строит граф решений; они обеспечивают работу проверок состояния, подсчета R_eff, графа зависимостей Mermaid, топологической сортировки для планирования спринтов и обнаружения слепых пятен.

Каждый значимый артефакт должен иметь хотя бы одну входящую или исходящую связь. Сироты (артефакты без связей) - это главный сигнал, выявляемый [`forgeplan health`](/ru/docs/cli/health/).

## Forgeplan 0.38.0

`--replace` меняет тип связи на той же паре концов, не направление и не статус артефакта. `--auto-activate-complete` проверяет у source kind=evidence, status=draft и явно заданные допустимые verdict (supports/weakens/refutes) и congruence_level (0–3); evidence_type не входит в этот предикат. Ошибка автоактивации даёт предупреждение после записи связи, не откатывает её. Имя relation само по себе не задаёт verdict доказательства. Source должен существовать; отсутствующий target допускается как предупреждаемая будущая ссылка, но не может предоставить оценку существующего решения.

## Использование

```text
forgeplan link [OPTIONS] <SOURCE> <TARGET>
```

## Аргументы

```text
  <SOURCE>  Source artifact ID
  <TARGET>  Target artifact ID
```

## Опции

```text
      --relation <RELATION>     Relationship type: informs, based_on, supersedes, contradicts, refines [default: informs]
      --replace                 Issue #286: if an edge already exists between source and target with a DIFFERENT relation, replace it (idempotent upsert). Without this flag, an existing edge with a different relation surfaces as an error and the call fails — matching the pre-issue-#286 behaviour. Used to fix mis-typed `based_on` / `informs` choices without destroying the artifact via `forgeplan delete`
      --auto-activate-complete  Issue #288 Part A: when the link source is a complete evidence artifact (kind=evidence + status=draft + body has verdict + congruence_level), silently activate it draft → active in the same call. Mirrors `auto_activate_source_if_complete` on the MCP `forgeplan_link` tool. No-op when the source doesn't meet the criteria
  -h, --help                    Print help
  -V, --version                 Print version
```

## Допустимые связи (v0.18)

Forgeplan использует закрытый словарь. Принимаются только эти пять типов связей - любое другое значение будет отклонено валидатором.

| Связь         | Направление читается как...                          | Когда использовать                                                                 |
|---------------|------------------------------------------------|-----------------------------------------------------------------------------|
| `informs`     | *источник* информирует *цель*                      | Доказательство, подтверждающее PRD; Note, используемый в ADR. Наиболее распространенный.         |
| `based_on`    | *источник* основан на *цели*                  | RFC, построенный на основе более раннего PRD; PRD, полученный из ProblemCard.         |
| `supersedes` | старый ID → ID замены | Направление, которое записывает релизная команда `forgeplan supersede`; не новый → старый. |
| `contradicts` | *источник* противоречит *цели*                  | Доказательство, опровергающее решение; RFC, отвергающий более ранний подход.    |
| `refines`     | *источник* уточняет *цель*                      | Spec, который уточняет PRD; ADR, который сужает пространство проектирования RFC.          |

## Примеры

Канонический поток доказательств - свяжите новый EvidencePack с PRD, который он поддерживает:

```bash
forgeplan new evidence "Benchmark: BM25 vs TF-IDF on Russian corpus"
# ... fill in verdict, congruence_level, evidence_type ...
forgeplan link EVID-001 PRD-039 --relation informs
forgeplan score PRD-039   # Inspect the actual minimum; support is not guaranteed.
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

Старые документы и результаты LLM иногда предлагают типы связей, которые **не входят** в словарь v0.18. Вот таблица соответствий:

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

Связи направленные. `forgeplan link PRD-001 EVID-001 --relation informs` означает *PRD-001 информирует EVID-001*, что почти никогда не то, что нужно. Стандартный шаблон всегда **доказательство → решение**:

```bash
forgeplan link EVID-001 PRD-001 --relation informs   # correct
```

Если вы установили связь в неправильном направлении, исправьте это с помощью [`forgeplan unlink`](/ru/docs/cli/unlink/), а затем создайте новую `link`.

## Самостоятельные ссылки заблокированы

Начиная с PROB-019, `forgeplan link PRD-001 PRD-001` отклоняется. Это предотвращает опечатки, которые в противном случае создали бы самореферентные узлы в графе и запутали бы топологическую сортировку.

## Что происходит после связывания

- Связь сохраняется в хранилище связей LanceDB.
- CLI запрашивает пересчёт кешированной оценки источника, не цели. После изменения доказательств явно выполните `forgeplan score TARGET`: новый пакет может снизить минимум.
- [`forgeplan graph`](/ru/docs/cli/graph/) включает ребро в вывод Mermaid.
- [`forgeplan blocked`](/ru/docs/cli/blocked/) и [`forgeplan order`](/ru/docs/cli/order/) повторно запускают топологическую сортировку по рёбрам `based_on` / `refines`.

## См. также

- [Обзор CLI](/ru/docs/cli/)
- [`forgeplan unlink`](/ru/docs/cli/unlink/) - удалить связь
- [`forgeplan score`](/ru/docs/cli/score/) - оценить R_eff после связывания доказательств
- [`forgeplan graph`](/ru/docs/cli/graph/) - визуализировать граф связей
- [`forgeplan health`](/ru/docs/cli/health/) - найти сирот (артефакты без связей) и недостающие доказательства
- [Руководство по методологии](/ru/docs/methodology/overview/)
