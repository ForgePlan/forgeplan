---
depth: standard
id: PROB-083
kind: problem
last_modified_at: 2026-08-01T19:23:15.905865+00:00
last_modified_by: claude-code/2.1.220
links:
- target: EPIC-009
  relation: informs
status: draft
title: 'Artifact substrate: identity triple lost on write, doubled frontmatter produced by current new'
---

## Filed upstream

| Issue | Covers |
|---|---|
| [#418](https://github.com/ForgePlan/forgeplan/issues/418) | Problem 1 + Problem 2 — identity triple written to the non-canonical block, destroyed by `update`; doubled frontmatter |
| [#419](https://github.com/ForgePlan/forgeplan/issues/419) | `update --title` leaves the stale file behind, creating a duplicate-id collision (found while correcting this artifact's own title) |
| [#420](https://github.com/ForgePlan/forgeplan/issues/420) | Problem 0 — docs prescribe `scan-import` where `reindex` is the correct command |

Not filed: Problem 4 is a duplicate of PROB-047 (active, three of five mitigations
unimplemented). Problem 5 is by-design detection scope; only its misleading `limitations`
list is arguably a defect, severity LOW.

---


# Artifact substrate: identity triple lost on write, doubled frontmatter produced by current `new`

Дефекты **субстрата**. К vNext отношения не имеют.

> **Исправление 2026-08-01.** Первая редакция этого артефакта утверждала, что ADR-016/ADR-017
> не резолвятся «даже после реиндекса». Это **неверно**: реиндекс не запускался. Была
> запущена команда `forgeplan scan-import`, которая перестраивает индекс не полностью.
> Правильная команда — `forgeplan reindex` («Rebuild LanceDB index from .md files»).
> После неё ADR-016, ADR-017 и EVID-143 резолвятся штатно. Ошибка оператора, не дефект.
> Оставлено ниже как Problem 0, потому что породивший её документационный дефект — реален.

## Problem 0 — документация называет неверную команду восстановления индекса

`CLAUDE.md` в четырёх местах предписывает `forgeplan scan-import` как способ пересобрать
LanceDB:

```
:31   fallback: `forgeplan scan-import` пересоберёт LanceDB из markdown.
:545  Last-resort fallback: `forgeplan scan-import` rebuilds LanceDB from markdown
:666  lance/  ← gitignored (derived index — forgeplan scan-import)
:671  Fresh clone: git clone → forgeplan init -y → forgeplan scan-import → forgeplan list
```

Фактически это делает `forgeplan reindex`. `scan-import` — команда **обнаружения и импорта**
артефактов из произвольных markdown-файлов; она не синхронизирует уже существующие.

Проверка:

```
forgeplan get ADR-017            -> Error: Artifact 'ADR-017' not found
forgeplan scan-import            -> 9 imported, 0 failed
forgeplan get ADR-017            -> Error: Artifact 'ADR-017' not found   (не помогло)
forgeplan reindex                -> 51 synced, 360 unchanged, 5 errors
forgeplan get ADR-017            -> ADR-017 — claude-code LLM provider…   (резолвится)
```

Прямое следствие: оператор, следующий CLAUDE.md, получает молчаливо неполный индекс и
делает по нему ложные выводы. Ровно это и произошло при подготовке этого артефакта.

## Problem 1 — identity-триплет ADR-012 не доживает до диска

**Это главный дефект.** `forgeplan new` рапортует полный триплет, но на диск он попадает
не туда, а `forgeplan update` его уничтожает.

Изолированный репродьюсер (чистый workspace, вне репозитория):

```
forgeplan init -y
forgeplan new prd "Slug persistence probe"
```

Файл содержит **два** блока frontmatter:

```yaml
---                                    # блок 1 — канонический, его читает парсер
depth: standard
id: PRD-001
kind: prd
status: draft
title: Slug persistence probe
---

---                                    # блок 2 — из шаблона, парсеру невидим
assigned_number: 1
depth: tactical / standard / deep / critical
predicted_number: 1
slug: prd-slug-persistence-probe
status: Draft
...
---
```

`slug`, `predicted_number` и `assigned_number` лежат **только во втором блоке**.

Дальше:

```
forgeplan update PRD-001 --body @body.md
grep -E '^(slug|predicted_number|assigned_number):' <file>   ->  пусто
```

`update` заменяет тело, второй блок уходит вместе с ним, и триплет **исчезает
безвозвратно**.

Наблюдаемое следствие по репозиторию: `grep -rl '^slug:' .forgeplan/` → **1 файл**, и это
`SPEC-005`, то есть сама спецификация контракта. Ни один рабочий артефакт слуга не несёт.
Четыре артефакта, созданных в этой сессии (EPIC-009, PROB-082, PROB-083, EVID-149), прошли
ровно этот путь: `new` вернул `slug` в JSON-ответе, файл его не сохранил.

ADR-012 (**active**) объявляет slug каноничной immutable-идентичностью. В write-path он
не реализован.

## Problem 2 — сдвоенный frontmatter порождается ТЕКУЩИМ кодом, а не наследием

66 файлов в `.forgeplan/` несут два блока; у **37** статусы в них противоречат друг другу
(например `PRD-008`: блок 1 `status: deprecated`, блок 2 `status: Draft`).

Изначально это выглядело как остаток миграции. Репродьюсер выше доказывает обратное:
**свежий `forgeplan new` в чистом workspace производит два блока немедленно.** Причина —
шаблоны `templates/*/_TEMPLATE.md` сами начинаются с `---`, рендер их не срезает, а
проекция дописывает свой frontmatter сверху.

## Problem 3 — коллизия id внутри evidence

Два файла объявляли `id: EVID-143`. Второй озаглавлен «id-collision detector … scan-import
flags two .md with one id» — живой экземпляр дефекта, который сам документирует.
После `forgeplan reindex` EVID-143 резолвится, но два файла остаются.

## Problem 4 — scan-import затягивает шаблоны и примеры репозитория

`forgeplan scan-import` создаёт артефакты из `templates/` и
`docs/brownfield-extraction-package/examples/` — с телом `{{term_name}}` и
`author: scan-import`. Повторные запуски множат их (DM-001, DM-002, DM-003…), потому что
шаблоны несут id-плейсхолдеры вида `DM-{{auto}}`, не проходящие `is_safe_artifact_id`, и
управление уходит в цикл выдачи следующего свободного номера.

**Это дубль PROB-047** (active), чей раздел Impact уже содержит: «Idempotency violation:
повторный run множит duplicates, противоречит ADR-003». Отдельного артефакта не требуется —
но три из пяти mitigations PROB-047 не реализованы.

## Problem 5 — forgeplan_contradictions не показывает явные рёбра contradicts

Три ребра `contradicts` записаны в frontmatter PROB-082; `forgeplan_contradictions`
возвращает `[]`. Проверено: инструмент **по замыслу** детектирует другие категории —
это не дефект детекции. Дефект в том, что поле `limitations` перечисляет только отложенные
до LLM категории и **не сообщает**, что явные рёбра `contradicts` вне его области. Читатель
делает вывод, что покрытие есть.

Severity низкая, но следствие практическое: рекомендация «зафиксируй коллизию ребром
`contradicts`, чтобы она стала машинно-видимой» цели не достигает.

## Next

1. `new` обязан писать identity-триплет в **канонический** блок; `update` не должен его терять.
2. Рендер шаблона обязан срезать frontmatter шаблона — иначе два блока у каждого артефакта.
3. Исправить CLAUDE.md: команда восстановления индекса — `reindex`, не `scan-import`.
4. Разрешить коллизию EVID-143 и 37 конфликтующих статусов.
5. ADR-013 — привести к YAML-frontmatter (единственный по-настоящему невидимый артефакт).

Все мутации `.forgeplan/**` — только через MCP/CLI (RED LINE #11).


