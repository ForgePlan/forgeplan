---
depth: standard
id: PROB-083
kind: problem
last_modified_at: 2026-08-04T13:41:50.634425+00:00
last_modified_by: claude-code/2.1.220
links:
- target: EPIC-009
  relation: informs
status: active
title: 'Artifact substrate: identity triple lost on write, doubled frontmatter produced by current new'
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

## Filed upstream

| Issue | Covers |
|---|---|
| [#418](https://github.com/ForgePlan/forgeplan/issues/418) | Problem 1 + Problem 2 — identity triple written to the non-canonical block, destroyed by `update`; doubled frontmatter |
| [#419](https://github.com/ForgePlan/forgeplan/issues/419) | `update --title` leaves the stale file behind, creating a duplicate-id collision (found while correcting this artifact's own title) |
| [#420](https://github.com/ForgePlan/forgeplan/issues/420) | Problem 0 — docs prescribe `scan-import` where `reindex` is the correct command |

Not filed: Problem 4 is a duplicate of PROB-047 (active, three of five mitigations
unimplemented). Problem 5 is by-design detection scope; only its misleading `limitations`
list is arguably a defect, severity LOW.



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

---                                    # блок 2 — из шаблона, парсеру невидим
assigned_number: 1
depth: tactical / standard / deep / critical
predicted_number: 1
slug: prd-slug-persistence-probe
status: Draft
...
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



---

## Verified 2026-08-04 — workflow re-investigation (4 investigators + skeptics)

Каждый пункт root-caused против кода, классификация проверена скептиком, спорное — эмпирикой на живой базе. Итог: **PROB-083 по существу верна**, но два места переоценены и одно фактически неверно. Ниже — исправления к собственным утверждениям выше.

### Итоговая классификация

| Пункт | Класс | Severity | Комментарий |
|---|---|---|---|
| Problem 0 (docs `scan-import`→`reindex`) | OPERATOR-ERROR + docs (#420) | — | верно, исправлено в `0487e12` |
| Problem 1+2 (#418 identity-триплет + сдвоенный frontmatter) | REAL-BUG | MAJOR | **фикс проверен работающим — см. ниже** |
| Problem 3 (EVID-143 / PRD-012 id-коллизии) | REAL-BUG (data + code) | MAJOR | код-гейт = GitHub #394 |
| Problem 5 (`forgeplan_contradictions` = `[]`) | **BY-DESIGN** | — | утверждение ниже **фактически неверно**, исправлено |
| ADR-013 без frontmatter | BY-DESIGN + data-hygiene | — | сканер корректно пропускает |

### Исправление к Problem 1+2 — фикс #418 ПРОВЕРЕН работающим

Утверждение выше («identity-триплет теряется») верно **для ветки без фикса**. На ветке `fix/frontmatter-identity-triple` (PR #424) проверено эмпирически на свежем workspace:

```
forgeplan new prd "..."          → get --json slug: prd-...   (резолвится)
forgeplan update --body @prose   → get --json slug: prd-...   (пережил update)
forgeplan reindex                → get --json slug: prd-...   (пережил перебор индекса из markdown)
```

То есть slug **резолвится и переживает reindex** — идентичность на write-path закрыта фиксом #424. Ранняя гипотеза «фикс неполный, slug остаётся в нечитаемом блоке 2» **опровергнута**: DB получает slug на create, и reindex его сохраняет. Двухблочность у новых артефактов остаётся, но она **инертна** (блок 1 авторитетен и корректен).

### Исправление к Problem 5 — фактическая ошибка

Утверждение выше «зафиксируй коллизию ребром `contradicts`, чтобы она стала машинно-видимой — цели не достигает» **неверно**. Рёбра `contradicts` **машинно-видимы** через `forgeplan graph` (рендерит `PROB-082 -->|contradicts| ADR-001/009/011`, structural-relation). `forgeplan_contradictions` — это НЕ листер типизированных рёбер, а brownfield-эвристика дедупа гипотез (Epic #287, `brownfield.rs`), records-only. Оператор спросил не тот инструмент. Правильная формулировка: коллизия видна через `forgeplan graph`; пустой `contradictions` — by-design.

### Исправление к Next — «37 конфликтующих статусов» переоценено

Пункт «Разрешить … 37 конфликтующих статусов» переоценивает status-дрейф как отдельную задачу. 37 конфликтов **инертны**: `parse_frontmatter` читает блок 1, `forgeplan get PRD-008` → `deprecated` (верно). Это косметический дрейф, исчезающий как side-effect перерендера legacy-файлов, а не самостоятельная работа.

### Остаётся реальным — Problem 3 (id-коллизии), план data-hygiene

- **EVID-143** — два разных содержательных файла (профиль PROB-073 + детектор коллизий PRD-008), независимо сминтивших один номер на параллельных ветках (это GitHub #394, не #419-rename). План: оставить ранний как EVID-143, поздний перезавести под следующий свободный EVID через MCP.
- **PRD-012** — пустая заглушка (`PRD-012-project-onboarding.md`, draft) + реальный active (`...-init-scan.md`). План: deprecate заглушку через CLI.
- **Код-гейт**: `reindex` не зовёт `find_duplicate_ids` — коллизия схлопывается молча (last-writer-wins). Это GitHub #394; предложение — вызвать детектор в `reindex` и падать громко, печатая оба пути.

Data-hygiene и код-гейт вынесены в отдельный проход (терминальные lifecycle-переходы + код на ветке от dev), не выполнены в этой сессии.


---

## Update 2026-08-06 — Problem 3 закрыт (детектор + data-hygiene)

Оба хвоста Problem 3 из раздела «Остаётся реальным» выполнены (ранее числились «вынесены в отдельный проход, не выполнены в этой сессии»):

- **Код-гейт (#394)** — `reindex` теперь собирает карту `id → файлы` во время обхода и **громко** докладывает каждую коллизию (оба пути ws-relative + счётчик `id-collisions` в summary + warning-hint), оставаясь **non-fatal** (exit 0 — чтобы `git clone && reindex` на уже-грязном репо не ломался; молчание было багом, а не завершение). Реальный E2E `cli_reindex_id_collision.rs` (позитив + негативный контроль) + догфуд на живой базе (поймал ровно 2 коллизии: PRD-012 и EVID-143). PR **#429**. `fmt 0, clippy 0`.
- **Data-hygiene** — PRD-012 заглушка удалена (оставлен active `init-scan`, refines EPIC-001); EVID-143 разведён по git-первенству: ранний профиль PROB-073 (00:42) сохраняет номер, поздний детектор (01:08) перезаведён через `forgeplan new` как **EVID-149** (содержимое сохранено, git видит 83% rename → blame цел). Поправлена одна устаревшая ссылка `EVID-143-collision` в `docs/v0.33-handoff.md`. Проверено свежим ребилдом индекса (`lance/` в сторону → `reindex`): **`0 id-collisions`**, каждый id резолвится в один артефакт. PR **#430**.

Классификация Problem 3: REAL-BUG (data + code) → **устранён**. Оба PR — draft, ждут ревью/мержа владельцем (RED LINE #2). Остаётся только чужой хвост, не Problem 3: 5 pre-existing health-debt ошибок реиндекса (ADR-013 без frontmatter + 4 строки на отсутствующие файлы) — отдельный трек v0.33-plan.


