# Issue tracker — ForgePlan

Этот контракт читают `task-cycle`, `task-runner` и координатор до начала работы.
Он задаёт трекер и границы записи; порядок этапов остаётся в
[Unified Workflow](../methodology/UNIFIED-WORKFLOW.ru.md), а правила программы
Constellation — в [AGENTS.md](../../AGENTS.md#working-mode-smith-first-then-role-matched-subagents).
Для этой программы утверждённый corrective plan и AGENTS.md имеют приоритет
над устаревшим советом продолжать source-задачи с отложенной синхронизацией в TODO.md.

| Параметр | Значение |
|---|---|
| Tracker | Orchestra |
| Workspace | ForgePlan — `kgYnhWodYw7kNbJMAK5q` |
| Project | Development — `ZB8WYUpwNskvAZdRK8Jh` |
| ROOT программы | `fdXN7kQgRQDqUnJRBRYu` |
| Task endpoint | `http://127.0.0.1:28174/mcp`, agent, bearer |
| Principal | Forge Agent — `7eea6f7ad3fc4b3196385e9d3e3cb856`, bot |
| Имя сервера OMP | `orchestra-beta-forge-agent` |
| Pin file | [orchestra.json](orchestra.json), schema v2, роль `servers.default` |
| Token | Только имя переменной `ORCHESTRA_FORGE_AGENT_TOKEN`; значение в репозиторий не попадает |
| Единственный tracker-writer | `task-runner` |

## Endpoint, identity и допуск к записи

Workspace и principal закреплены намеренно. Имя runtime-сервера берётся из
`names` pin file, а не угадывается по похожему префиксу. В этом контракте настроен
только task endpoint. Admin endpoint `orch` на `http://localhost:28173/mcp`
указан справочно: записи через него в этой программе **не разрешены**. Отказ task
endpoint не разрешает переключение на admin, другую identity или другой workspace.

Перед первой записью `task-runner` читает `skill://task-cycle`,
`skill://orchestra-mcp` и соответствующий Orchestra guide. Каталог установленного
`orchestra-mcp` разрешается в текущем runtime, без зависимости от другого checkout.
Координатор запускает из интеграционной рабочей копии:

```sh
"<resolved-skill-dir>/scripts/orch-verify.sh" default --runtime omp --json
```

При проверке ещё не интегрированного config передать `--config` с абсолютным путём
к проверяемому `orchestra.json`. Exit 0 / `MATCH` допускает следующий шаг;
любой другой exit блокирует записи. Успешный shell-verifier не заменяет проверку
доступа к ROOT через mounted task endpoint.

Через тот же mounted endpoint перечитать `get_current_context`, `list_workspaces`
и прямой `get_entity` ROOT. Подтвердить workspace, principal и доступность ROOT из
таблицы выше, затем прочитать его описание, поля, доступные сообщения с пагинацией
и чек-листы; то же сделать для затронутых задач. Перед действием от имени бота
прочитать его deployed instructions через `get_agent_prompt`.

Если токен доступен только внутри mounted MCP, его нельзя извлекать для shell.
Допустим тот же handshake через `get_current_context` + `list_workspaces` + прямой
`get_entity` ROOT; в отчёте указать подтверждённые workspace/principal и отдельно
`shell-verifier NOT_RUN`. Наличие слушающего порта не доказывает доступ к доске.

Несовпадение identity/workspace, недоступный трекер или `Entity {uid} is not
accessible in this session` блокируют новые source-задачи и tracker-записи.
При access error назвать UID и не искать обход через поиск, другой endpoint или
добавление участников. Разрешены чтение и диагностика без source dispatch и без
заявления о выполненной синхронизации. TODO.md не является запасным трекером.

Pin file содержит только публичные координаты endpoint, identity и имя tokenEnv.
Значение токена и полный env не читать, не выводить и не сохранять. Не копировать
`.mcp.json`, credentials, auth stores или чужой `.serena/`. Новый checkout использует
собственный tracked контракт, а не файл из соседней рабочей копии.

## Field roles → поля доски

Каждый запуск `task-runner` разрешает поля через `list_fields` для фактического
контекста и типа task. Построить карты field name → UID и option name → option UID
для каждого поля. Имена ниже — роли, не UID; field/option IDs не закреплять
в коде, инструкциях или pin file. Workspace/principal pins выше — другая сущность.

| Роль | Имя поля / ожидаемый смысл |
|---|---|
| status | `Status` — Backlog / To Do / Doing / Review / Done / Blocked |
| stage marker | `Phase` — Shape / Validate / Code / Evidence / Done |
| runtime | `Executor` — фактически выполняющий runtime |
| model | `Model` — фактически подтверждённая модель |
| branch | `Branch` — ветка рабочего среза |
| checks | `Rust checks` — для Rust-среза NOT_RUN до подтверждения build-owner |
| blocked-by | `Blocked by` — зависимости допуска |
| depth | `Depth` — Tactical / Standard / Deep / Critical |
| artifact | `Artifact` — ID связанного артефакта, не его body |
| type | `Type` — вид задачи |
| sprint / tags | `Sprint` / `Tags` |

## Операции и честное состояние

Имена tools ниже даны без runtime-префикса. Все записи выполняет только
`task-runner`; координатор и остальные роли передают ему наблюдения и результаты.

| Операция | Вызовы и условия |
|---|---|
| Найти рабочий срез | `query_entities` / `search_entities`; сначала проверить существующие имя и parent, не дублировать ROOT или S1–S4 |
| Прочитать задачу | `get_entity`, описание через `read_document`, `read_messages` с пагинацией, `get_checklists` |
| Создать недостающий срез | `create_entity` под существующей задачей C/R/V/G; поля и проверяемый checklist сразу |
| Начать | Одна `update_entity`: Status=Doing, Phase=Code, Executor=OMP, фактическая Model, Branch |
| Отметить доказанный критерий | `manage_checklist_item` сразу после получения доказательства; дополнение без удаления и снятия чужих галочек |
| Передать авторский patch | Status=Review, Phase=Evidence; для Rust-среза Rust checks остаётся NOT_RUN |
| Подтвердить Rust checks | PASSED только по отчёту единственного build-owner на текущем source/test/harness fingerprint |
| Заблокировать | Status=Blocked, Phase не менять; в описании конкретные `BLOCKED` и `TRIGGER` |
| Закрыть | Status=Done и Phase=Done только после PASS соответствующего guardian gate и доказательства всех acceptance lines |
| Отчитаться | `send_message` только по ограниченному разрешению ниже |

После каждого изменения читать `failed`, `failedFields`, `warning`; успешный
вызов с отклонённым полем не считать выполненной записью. Значимые поля и галочки
перечитать. Прежний прогон и отчёт автора не превращают NOT_RUN в PASSED.
Не менять assignee, owner, due date и участников; не удалять задачи, историю,
зависимости или существующие чек-листы.

Для чисто governance/docs или read-only среза Rust checks может быть
NOT_APPLICABLE с конкретной причиной, но это не Rust PASS. Неизвестную Model
не угадывать; отсутствие подтверждения указать явно.

## Сообщения и доказательства

В `orchestra.json` явно включено `chatWriting=true`. Это **не** право рассылки.
Единственный писатель `task-runner` публикует короткие task-local отчёты при
передаче и после независимой проверки только в чатах соответствующих задач
дерева ROOT `fdXN7kQgRQDqUnJRBRYu`. Никаких каналов, групп, личных сообщений,
упоминаний людей или дублирования одного отчёта по всем родителям.

Форма отчёта: изменено / проверено / не выполнено / evidence ID /
source+binaries SHA-256 / открытые замечания. Невыполненное помечать NOT_RUN;
отсутствующее доказательство не заменять уверенным текстом.

Orchestra хранит ход работы и ссылки, не body артефактов, R_eff или вторую копию
валидации. SPEC/RFC/ADR и EVID меняют назначенные Profile A/B только через штатный
Forgeplan CLI/MCP в интеграционной копии с явным workspace. Кодеры не пишут
`.forgeplan/`, не записывают EVIDENCE и не активируют артефакты.
