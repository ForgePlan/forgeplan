# forgeplan-map-pack — context & process (marketplace)

> **Что это.** Marketplace-пак (скиллы + агенты + инструкции + шаблоны), который в **любом проекте,
> где есть `forgeplan`**, по понятному оркестрованному процессу **изучает проект**, складирует
> **валидированные слоистые данные** (`.forgeplan/map/map.json`: layers · zones · nodes · mega-nodes ·
> edges), а затем **`forgeplan-web` рендерит** по ним интерактивную **онбординг-карту** проекта.
> Карта детерминирована: после валидации веб просто рисует — чисто, потому что все файлы проверены.
>
> Полная архитектура (схема данных, рендерер, MVP по дням, фазы, решения) — в **`MASTER-SPEC.md`**
> рядом с этим файлом. Этот README — про **процесс/скиллы/агентов** в marketplace.

---

## Откуда это и зачем

- **Вдохновение / референс-тула:** `dev/effective-html` (репо `plannotator/effective-html`) — скилл,
  который рисует self-contained HTML-диаграммы по доке. Мы делаем **своё, ForgePlan-native**: не
  «LLM рисует пиксели», а **агенты собирают валидированные данные → forgeplan-web рисует по правилам**.
  Локально: `/Users/explosovebit/Work/ForgePlan/dev/effective-html`. Корпус из 20 разных «видов»
  (`skills/html-diagram/references/html-effectiveness/`) — инспирация для библиотеки композиций.
- **Рабочий спайк (ground-truth UX + раскладка):** `/Users/explosovebit/Work/ForgePlan/dev/forge-understand/spike/index.html`.
- **Куда рендерит:** `forgeplan-web` (`github.com/ForgePlan/forgeplan-web`) — уже зрелый graph-фронтенд
  (7 видов + mosaic). Мы добавляем **8-й вид `composed-map`** + **отдельный онбординг-лэйаут** (см.
  MASTER-SPEC §3, §8, §17).

## Главный принцип (как и в BMAD/SPARC/smith)

**Оркестратор дирижирует, агенты работают каждый в своём контексте, guardian валидирует, и только
валидированные данные уходят в веб.** Один агент не делает всё «рандомно» — это **поэтапный
процесс с гейтами** (как у `agents-bmad`, `agents-sparc`, `smith`). После того как работа с файлами
закончена и всё провалидировано — **дальше работает сам web**, и там всё чётко, потому что слои /
ноды / мега-узлы / рёбра уже проверены.

## Объём: строим ПОЛНОЦЕННО (решение пользователя — «делаем сразу хорошо»)

Строим **полную целевую форму сразу** — весь ролевой веер (оркестратор + `code/forgeplan/docs-scanner`
+ `zone-extractor` + `edge-verifier` + `map-emitter` + детерминированный `map-guardian.mjs` +
advisory LLM-guardian поверх) + онбординг-лэйаут + append-loop — **а не тонкий MVP**.

**Но 5 вещей сохраняем ОБЯЗАТЕЛЬНО (это корректность, не урезание — иначе «полно» сломается):**
1. **Render-proof сначала:** рукописный `map.json` валидирует рендер в forgeplan-web **до агента** —
   изолирует баги рендера от багов извлечения.
2. **EMITTER-safe = 3 защиты:** denylist + PreToolUse `hooks/map-emitter-gate.sh` (write-path) +
   guardian single-write (denylist одного мало — `Write` мог бы попасть в `.forgeplan/prds/*.md`).
3. **Раздельный scratch на каждый сканер** (`.work/.scan.{code,fpl,docs}.json`), мерджит оркестратор —
   НИКОГДА общий `map.json` на запись из нескольких агентов (гонка PROB-060).
4. **Append = ЛОКАЛЬНЫЙ демон** `forgeplan map serve`, не веб-роут (веб не спавнит `claude` —
   `READ_ONLY_SUBCOMMANDS`); FIFO + localhost + explicit-click.
5. **Guardian-гейт = детерминированный скрипт**; LLM-judgment — advisory поверх, не сам гейт.

Полные брифы агентов, гейты G1-G4, дизайн guardian и онбординга — в `MASTER-SPEC.md` §23.

## Оркестрованный процесс (роли — каждая в своём контексте)

```
ORCHESTRATOR (map-orchestrator)  — дирижирует, сам данные не пишет; гейт между стадиями
   │
   ├─► SCAN (sharded, параллельно):  code-scanner · forgeplan-scanner · docs-scanner
   │        собирают сырые факты (модули, артефакты, граф, доки)
   ├─► TYPE:        project-typer        → тип проекта + confidence (pure scoring fn)
   ├─► SELECT:      composition-selector → выбирает шаблон-композицию из библиотеки
   ├─► EXTRACT:     zone-extractor       → zones · layers · nodes · mega-nodes (content-hash id)
   ├─► VERIFY EDGES: edge-verifier       → typed-link (из forgeplan_graph) vs code-dep (grep-gated)
   ├─► EMIT:        map-emitter          → собирает .forgeplan/map/map.json (status: proposed)
   └─► VALIDATE:    GUARDIAN (map-guardian) → schema + 3 инварианта + целостность зон/нод/рёбер
                                            PASS → status: confirmed; FAIL → назад на стадию
```

**Все агенты — профиль EMITTER:** разрешено `Read · Glob · Grep · Write` + read-only MCP
(`forgeplan_graph/list/get`). **Запрещено** `Edit` + все мутаторы (`forgeplan_new/update/link/...`).
Пишут **ровно в один файл** — `.forgeplan/map/map.json`. → **RED-LINE #11 нарушить структурно
невозможно**: агент физически не может рассинхронизировать LanceDB/markdown. Карта — derived
read-only view, не артефакт; ADR-003 не нарушается.

**Guardian-гейт** проверяет (как `adr_003_invariant.rs` для графа):
1. JSON соответствует `schemas/map.schema.json` (`forgeplan.map/v1`).
2. 3 инварианта: ячейки зон не перекрываются · каждый endpoint ребра ∈ nodes · каждый `node.zone ∈ zones`.
3. mega-nodes: каждый child ∈ nodes, нет циклов вложенности.
4. Все `relation` у `typed-link` ∈ 11 VALID_RELATIONS; `code-dep` имеет `verified_by`.
Только после PASS файл помечается `confirmed` и веб рендерит без «unverified»-ленты.

## mega-nodes (новое в схеме)

Узел, который **агрегирует под-узлы** (свёрнутый кластер / зона как один узел на верхней высоте C4).
`{ "id":"mn_core", "is_mega":true, "children":["n_routing","n_projection",...], "collapsed":true }`.
Нужны для **C4-rollup** (Context→Container→Component): на L0 видишь мега-узлы, кликаешь — раскрывается
в под-граф. Guardian проверяет целостность children.

## Скиллы (MVP: 3, остальное Phase 2)

- `zone-extractor` — dirs/modules/artifact-kinds → zones/layers/nodes/mega-nodes; id = `sha1(kind+":"+path_or_slug)[:12]`.
- `edge-verifier` — два namespace рёбер, grep-gating для code-dep, невалидные code-dep **дропаются**.
- `map-emitter` — сборка JSON + 3 инварианта + атомарная запись.
- *Phase 2:* `project-typer`, `composition-selector` (пока инлайн в оркестраторе), `map-differ`
  (инкрементальное дополнение), `discover-to-map` (мост к `forgeplan-brownfield-pack`).

## Агенты

- `map-orchestrator` (дирижёр, BMAD-style, данные не пишет).
- `code-scanner` / `forgeplan-scanner` / `docs-scanner` (EMITTER scan).
- `zone-extractor` / `edge-verifier` / `map-emitter` (EMITTER).
- `map-guardian` (read-only валидатор, выносит PASS/CONCERNS/BLOCKER).

## Раскладка плагина (зеркалит `forgeplan-brownfield-pack`)

```
plugins/forgeplan-map-pack/
├── .claude-plugin/plugin.json
├── agents/        map-orchestrator · *-scanner · zone-extractor · edge-verifier · map-emitter · map-guardian
├── skills/        zone-extractor · edge-verifier · map-emitter (+ Phase 2)
├── compositions/  rust-cli-mcp.yaml · web-fullstack.yaml · generic.yaml  (DATA, O/C-extensible)
├── schemas/       map.schema.json   ← канонический контракт (валидируют обе стороны)
├── playbooks/     map-build.yaml    ← оркестрованный процесс
└── mappings/      discover-to-map.yaml  (Phase 2, мост к brownfield)
```

## Онбординг-лэйаут в forgeplan-web (зачем процесс нужен)

В `forgeplan-web` — **отдельный layout онбординга** (не смешан с обычными артефакт-видами): он
**проводит по схеме/зонам в режиме навигации + анимации** и даёт **чат**, где агент рассказывает «что
это за проект и как им пользоваться» на базе собранных данных + артефактов forgeplan. Нужно
«провалиться в артефакты» — переходишь в **стандартный слой** (существующие 7 видов). Подробно —
MASTER-SPEC §8, §17. Данные для онбординга готовит **локальный headless-агент (Claude Code,
`claude -p` в том же каталоге)** — этот пак и есть тот процесс.

## Как пользоваться (в любом forgeplan-проекте)

1. Установить пак из marketplace (`/plugin install forgeplan-map-pack@ForgePlan`).
2. Запустить процесс: `map-orchestrator` (или тонкий CLI `forgeplan map build`, который шеллит
   `claude -p ... --allowedTools Read Glob Grep Write`).
3. Процесс изучает проект → пишет валидированный `.forgeplan/map/map.json`.
4. Открыть `forgeplan-web` → онбординг-лэйаут рендерит карту + чат-онбординг.
5. Дополнить позже: `map-orchestrator --refresh` → дописывает узлы → веб перестраивается органично.

## Примеры

- **Эталонный выход:** рукописный `map.json` для самого ForgePlan (re-key из IR спайка) — им
  валидируется весь путь рендера ещё до агента (MASTER-SPEC §12, Day 1-2 checkpoint).
- **Стили/композиции:** 20 видов в `dev/effective-html/.../html-effectiveness/`.
- **UX/раскладка/токены:** `dev/forge-understand/spike/index.html`.

---

_Полная архитектура, JSON-схема, план рендерера, MVP и решения — в `./MASTER-SPEC.md`._
