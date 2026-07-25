---
depth: standard
id: SPEC-006
kind: spec
last_modified_at: 2026-07-25T08:31:35.279200+00:00
last_modified_by: claude-code/2.1.219
links:
- target: PRD-081
  relation: based_on
status: draft
title: Минимальный вертикальный срез space-mesh (Фаза 1)
---

## Summary

Контракт минимального вертикального среза space-mesh (Фаза 1): формат события
`SpaceEvent`, формат журнала (append-only NDJSON), точка эмита в `LanceStore`,
и один MCP-тул `space_subscribe` с durable-курсором. Срез доказывает ценность
без демона и без брокера: один space `gertsai-platform`, два реальных проекта,
один тип события `artifact.activated`, ~60-строчный SSE-дашборд.

Топология (композиция SpaceJournal + ForgeMesh, read-only хаб, мутации в
свежеспавненный per-project процесс) уже выбрана в ADR-018 — здесь она не
пересматривается, здесь фиксируются подписи, пути и инварианты, достаточные
для реализации без повторного чтения handoff'а.

---

## Scope

**Входит в срез** (handoff §10):

| Ось | Значение среза |
|---|---|
| Spaces | ровно один: `gertsai-platform` |
| Проекты | `~/Work/GertsAi/shared` (A) и `~/Work/GertsHub` (B) |
| Типы событий | ровно один: `artifact.activated` |
| Точка эмита | ровно одна: `LanceStore::update_artifact` |
| MCP-тулы | ровно один: `space_subscribe` |
| UI | ~60 строк Node + одна HTML-страница на SSE |
| Discovery | хардкод пути / `walkdir`-скан |

**НЕ входит** (явный список из §10, каждый пункт — отдельная работа Фазы 1
или Фазы 2): хаб-демон `forgeplan serve`; fan-out по space (`space_query`,
`space_list_projects`); кросс-проектные claims; cross-project semantic search
и резидентный BGE-M3; хуки `space_on`; второй тип события (`artifact.created`,
`artifact.superseded`, `claim.acquired`, …); эмит из `create_artifact`,
`update_body`, `ClaimStore::{claim,release}`; Windows-паритет; `registry.json`;
retention/компакция сегментов; HMAC-привязка project→space.

---

## Data Models

### `SpaceEvent` (крейт `forgeplan-mesh`, wire-формат = одна NDJSON-строка)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceEvent {
    pub v: u32,                          // версия схемы события; в Фазе 1 всегда 1
    pub ts: String,                      // RFC3339 UTC с миллисекундами (chrono)
    pub seq: String,                     // ULID (26 символов, Crockford base32)
    pub space_id: String,                // из mesh.space_id, non-empty
    pub project_id: String,              // см. «Validation Rules», строка project_id
    pub artifact_id: String,             // canonical display id, напр. "PRD-081"
    pub artifact_type: String,           // "PRD" | "RFC" | "ADR" | "SPEC" | ...
    pub kind: String,                    // event kind: в Фазе 1 только "artifact.activated"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_status: Option<String>,     // статус ДО мутации; None если не удалось прочитать
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_status: Option<String>,       // статус ПОСЛЕ мутации; в Фазе 1 всегда Some("active")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r_eff: Option<f64>,              // кэш-колонка r_eff_score на момент эмита
    pub md_path: String,                 // путь файла артефакта ОТНОСИТЕЛЬНО корня проекта
    pub abs_path: String,                // канонический абсолютный путь КОРНЯ проекта
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,        // владелец claim'а / --agent, если известен
    pub actor: String,                   // "cli" | "mcp" | "watch" | "unknown"
}
```

Обязательные (всегда сериализуются): `v`, `ts`, `seq`, `space_id`, `project_id`,
`artifact_id`, `artifact_type`, `kind`, `md_path`, `abs_path`, `actor`.
Опциональные (пропускаются при `None`): `from_status`, `to_status`, `r_eff`,
`agent_id`.

Разделение `md_path` / `abs_path` не избыточно: `abs_path` — корень проекта
(идентифицирует проект и позволяет `LanceStore::open` со стороны хаба), `md_path` —
путь внутри проекта. Потребитель склеивает их сам; относительный `md_path`
остаётся валидным, если проект переехал.

`seq` — ULID, а **не** порядковый номер: он служит для дедупликации и грубой
сортировки по времени, но **не** является позицией в журнале. Позиция —
только `Cursor` (байтовое смещение), см. ниже.

`kind` — иерархическое имя `<домен>.<глагол>`, зарезервированное пространство:
`artifact.*`, `claim.*`, `space.*`. В Фазе 1 эмитится ровно `artifact.activated`.

### `MeshConfig` (крейт `forgeplan-core`, `config/types.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MeshConfig {
    /// Членство в space. None → проект standalone, эмит выключен целиком.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_id: Option<String>,
    /// Kill-switch без удаления space_id. Отсутствие поля = true.
    #[serde(default = "default_mesh_enabled")]
    pub enabled: bool,
}
```

Подключается как новое top-level опциональное поле в `Config`, рядом с
`playbook` / `phase`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub mesh: Option<MeshConfig>,
```

`Config` целиком уже размечен `#[serde(default)]` по полям, поэтому legacy-конфиги
без блока `mesh:` парсятся как standalone без миграции. Это MUST-регрессия
(см. критерий приёмки E).

Committed-вид в `.forgeplan/config.yaml`:

```yaml
mesh:
  space_id: gertsai-platform
  enabled: true
```

Членство — единственный не-markdown артефакт mesh'а, который трекается git'ом;
всё остальное (журнал, зеркало, индексы) — derived и gitignored.

### `Cursor` (крейт `forgeplan-mesh`, wire-формат в MCP)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Cursor {
    pub segment_file: String, // имя сегмента, напр. "2026-07-25.ndjson"
    pub byte_offset: u64,     // смещение ПЕРВОГО БАЙТА СЛЕДУЮЩЕЙ недоставленной строки
}
```

Семантика: курсор указывает **после** последней доставленной строки. `byte_offset`
всегда стоит на границе строки (сразу за `\n`); значения, не совпадающие с границей,
подписчик получает только в результате повреждения и обязан обработать их как
`truncated` (см. контракт `space_subscribe`).

---

## Storage Contract

### Пути

| Роль | Путь | git |
|---|---|---|
| Первичный журнал проекта | `<project>/.forgeplan/events/<YYYY-MM-DD>.ndjson` | gitignored |
| Зеркало space (то, что читают подписчики) | `<space_dir>/<project_id>/<YYYY-MM-DD>.ndjson` | вне репо |
| Lock записи | `<space_dir>/.events.lock` | вне репо |

`<space_dir>` = `$XDG_DATA_HOME/forgeplan/spaces/<space_id>/`, если `XDG_DATA_HOME`
задан и непуст, иначе `~/.local/share/forgeplan/spaces/<space_id>/`.
Сознательно **не** `~/Library/Application Support` — один путь на macOS и Linux.
`XDG_RUNTIME_DIR` не используется вообще: на целевой машине он эмпирически пуст
(handoff §2), из-за чего в Фазе 1 нет сокетов и нет «primary socket path».

`.forgeplan/events/` — sibling к `lance/`, `claims/`, `state/`, `.lock`.
В `.gitignore` добавляется строка `.forgeplan/events/` рядом с существующими
`.forgeplan/lance/` и `.forgeplan/session.yaml`.

### Формат

- Один JSON-объект = одна строка, **всегда** завершённая `\n`.
- Кодировка UTF-8, без BOM, без pretty-print, без внутренних переводов строки
  (сериализация `serde_json::to_string`, не `to_string_pretty`).
- Сегментация — посуточная по UTC-дате в момент записи. Ротация без внешнего
  триггера: имя сегмента вычисляется из `ts` каждого события.
- Файлы только дописываются. Никаких перезаписей, `truncate`, `rename` поверх
  существующего сегмента в Фазе 1.

### Порядок записи одного события

Обе записи — под **одним** захватом `fs2`-lock'а на `<space_dir>/.events.lock`:

1. append в первичный журнал проекта (`.forgeplan/events/<date>.ndjson`);
2. append в зеркало space (`<space_dir>/<project_id>/<date>.ndjson`).

Первичный журнал идёт первым потому, что именно он — источник для будущего
`space replay --from-scan`: если зеркало не записалось, строка не потеряна и
восстанавливается пересборкой. Обратный порядок терял бы данные.

Журнал — **derived**, а не source of truth: markdown-фронтматтер артефакта
остаётся единственной истиной (ADR-003), журнал пересобираем.

---

## Emit Contract

### Точка эмита (ровно одна в Фазе 1)

`forgeplan_core::db::store::LanceStore::update_artifact`
(`crates/forgeplan-core/src/db/store.rs:1029`) — **последним шагом**, после
`builder.execute().await?` и перед `Ok(())`.

Обоснование выбора именно этого метода: через него проходят обе легальные ветки
мутации статуса — `projection::update_artifact_status` (`projection/mod.rs:913`,
делает `sync_before_mutation` → `update_artifact` → `render_after_mutation`) и
ctx-вариант (`projection/mod.rs:1536`), а также driver-обёртка
`driver/lance.rs:56`. Обёртка вокруг MCP-dispatch покрыла бы MCP и пропустила CLI;
`LanceStore` покрывает оба одним хуком.

**Известное следствие порядка**, которое обязан учитывать потребитель:
`render_after_mutation` выполняется **после** `update_artifact`, поэтому в момент
доставки события файл по `md_path` может быть ещё не перерисован. Поэтому
подписчик обязан полагаться на поля события (`to_status`, `r_eff`,
`artifact_type`), а не перечитывать markdown ради статуса. Событие
самодостаточно by design.

### Условие срабатывания

Эмит происходит **только** при `status == Some("active")`. Это даёт ровно
`artifact.activated`. Переход `stale → active` (`renew`) в Фазе 1 тоже эмитит
`artifact.activated`; различает их поле `from_status`. Это принято сознательно.

### Резолв mesh-контекста

`LanceStore` не хранит путь workspace и не читает конфиг. Поэтому:

- в `LanceStore` добавляется поле `mesh: Option<MeshSink>`;
- оно заполняется **однократно** в `LanceStore::open(workspace_path)`
  (`store.rs:440`; `workspace_path` — это каталог `.forgeplan`): читается
  `<workspace_path>/config.yaml`, берётся `mesh.space_id` + `mesh.enabled`;
- `space_id == None` или `enabled == false` или ошибка чтения конфига →
  `mesh = None` + одна `tracing::warn!` при ошибке. `open` не падает никогда;
- `MeshSink` держит: `space_id`, `project_id`, канонический `abs_path` корня
  проекта (= канонический родитель `workspace_path`), путь `<space_dir>`.

Для 19 из 21 проекта на машине (handoff §2) `mesh` будет `None`, и весь эмит
вырождается в один `if let Some(...)` — нулевая стоимость для standalone.

### Снимок «до» (нужен для `from_status` / `r_eff` / `artifact_type`)

Когда `mesh.is_some()`, в начале `update_artifact` выполняется **один** запрос:

```rust
pub(crate) async fn mesh_snapshot(&self, id: &str)
    -> Option<(String /*kind*/, String /*status*/, Option<f64> /*r_eff_score*/)>
```

Одна выборка по `id` колонок `kind`, `status`, `r_eff_score` (колонка
`r_eff_score` Float64 уже есть в схеме, `db/schema.rs:37`). Существующий
`get_artifact` не подходит — `ArtifactSummary` не несёт `r_eff_score`.
Отдельного запроса под `r_eff` быть не должно: это удвоило бы стоимость мутации.
Ошибка снапшота → `None` по всем трём полям, эмит продолжается с
`from_status: None`, `r_eff: None`.

### Fire-and-swallow (жёсткое правило)

Эмит **никогда** не влияет на исход мутации. Весь блок эмита обёрнут так, что
любая из ошибок — `EACCES` / `ENOSPC` / отсутствие каталога / таймаут lock'а /
ошибка сериализации / слишком длинная строка — приводит к
`tracing::warn!` и продолжению. Мутация возвращает `Ok(())` в 100 % случаев,
где она бы вернула `Ok(())` без mesh.

`?` внутри блока эмита запрещён. Эмит не возвращает ошибку наружу и не меняет
сигнатуру `update_artifact`.

### Lock и атомарность

- `fs2::FileExt::lock_exclusive` на `<space_dir>/.events.lock`, синхронный вызов
  — в `tokio::task::spawn_blocking`, по образцу `workspace/lock.rs:116`
  (`acquire_workspace_lock_with_timeout`, backoff, symlink guards).
- Таймаут — **200 мс** (в допустимом коридоре 100–250 мс). Существующий
  `DEFAULT_LOCK_TIMEOUT` из `workspace/lock.rs` (секунды) здесь НЕ использовать:
  handoff §8 фиксирует «30s wait недопустим для интерактивного CLI».
- По таймауту эмит становится **no-op** + один `warn!`. Потеря события под
  contention допустима: `space replay --from-scan` дособерёт.
- `lock+append` — норма. Голый `O_APPEND` без lock'а — **opt-in** fast-path,
  разрешён только для строк ≤ `PIPE_BUF`. На darwin `PIPE_BUF` = 512 байт
  (против 4096 на Linux), а типичная строка `SpaceEvent` с `abs_path` + `md_path`
  превышает 512 байт → в Фазе 1 fast-path фактически выключен. Реализовать его
  можно, включать по умолчанию — нельзя.
- Строка длиннее **64 KiB** не пишется: `warn!` + swallow (защита от
  патологического `title` / пути).
- Per-line checksum для детекта усечённой при крэше строки в Фазе 1 не делается;
  вместо этого подписчик не продвигает курсор через строку без завершающего `\n`
  (см. ниже).

---

## API Contracts

### MCP tool: `space_subscribe`

Регистрируется в JSON-RPC dispatch `crates/forgeplan-mcp/src/server.rs`,
DTO — в `crates/forgeplan-mcp/src/types.rs`.

**Request**:

```json
{
  "space_id": "gertsai-platform",
  "kinds": ["artifact.activated"],
  "since": { "segment_file": "2026-07-25.ndjson", "byte_offset": 4096 }
}
```

| Поле | Тип | Обяз. | Дефолт |
|---|---|---|---|
| `space_id` | string | нет | `mesh.space_id` из конфига текущего workspace; если и его нет — ошибка `MESH_NOT_JOINED` |
| `kinds` | string[] | нет | `null` = все типы |
| `since` | `Cursor` \| `"beginning"` \| `"now"` | нет | `"now"` |

**Response** (immediate, синхронный результат вызова):

```json
{
  "space_id": "gertsai-platform",
  "subscription_id": "01J8Z9Q2W7K3M5N7P9R1S3T5V7",
  "replayed": [ { "v": 1, "ts": "...", "seq": "01J8...", "kind": "artifact.activated", "...": "..." } ],
  "cursor": { "segment_file": "2026-07-25.ndjson", "byte_offset": 8192 },
  "truncated": false
}
```

`replayed` — все строки после `since`, прошедшие фильтр `kinds`, в порядке
файла. `cursor` — позиция после последней доставленной строки.
`subscription_id` — ULID, используется для корреляции последующих нотификаций.

**Streaming**: после ответа сервер тейлит зеркало space через `notify`
(зависимость `notify` 7 / `notify-debouncer-mini` 0.5 уже в workspace) и шлёт
каждую новую подходящую строку как MCP server-notification:

```json
{
  "method": "notifications/forgeplan/space_event",
  "params": {
    "subscription_id": "01J8Z9Q2W7K3M5N7P9R1S3T5V7",
    "event": { "...SpaceEvent..." },
    "cursor": { "segment_file": "2026-07-25.ndjson", "byte_offset": 8704 }
  }
}
```

**Семантика курсора**:

1. `"beginning"` → самый ранний доступный сегмент, offset 0.
2. `"now"` → текущий (последний по имени) сегмент, offset = его длина.
3. Явный `Cursor` → seek в `segment_file` на `byte_offset`.
4. Переход через границу сегмента: когда `byte_offset == len(segment_file)` и
   существует лексикографически больший сегмент — курсор становится
   `{next_segment, 0}` и чтение продолжается. Имена сегментов `YYYY-MM-DD.ndjson`
   лексикографически = хронологически.
5. Незавершённая строка (прочитан хвост без `\n`) — **не** доставляется и
   **не** двигает курсор. Ожидание `\n`. Это же покрывает торн-райт при крэше.
6. `segment_file` из `since` не существует (сегмент удалён/скомпакчен) →
   ответ с `truncated: true` и продолжение с самого раннего доступного сегмента.
   Молчаливый пропуск запрещён — подписчик должен узнать о дыре.
7. Курсор персистит **клиент** (дашборд хранит `last_offset`). Сервер не хранит
   состояние подписчиков в Фазе 1.

**Errors**:

| Code | Условие |
|---|---|
| `MESH_NOT_JOINED` | `space_id` не передан и в конфиге нет `mesh.space_id` |
| `SPACE_NOT_FOUND` | каталог `<space_dir>` не существует |
| `INVALID_CURSOR` | `byte_offset` > длины сегмента, либо `segment_file` не матчит `^\d{4}-\d{2}-\d{2}\.ndjson$` |

### Dashboard SSE (не через MCP — сознательно)

Node-сервис в `dev/space-mesh-dashboard/` (вне `crates/`, не шипится в релиз):
`chokidar` на `<space_dir>/*/*.ndjson` → одна HTML-страница, `GET /events`
(`text/event-stream`), каждая NDJSON-строка → SSE-фрейм `data: <исходная строка>`.
Клиент рендерит карточку `project_id · artifact_id · kind · R_eff · ts`.

Дашборд ходит **мимо MCP и мимо forgeplan-бинаря** намеренно: это и есть
доказательство того, что журнал — обычный файл, а не привилегированный канал.

`GET /events?since=<segment>:<offset>` реализует ту же семантику курсора,
что и пункты 1–6 выше, — этим проверяется критерий B.

---

## Validation Rules

| Поле / инвариант | Правило | Поведение при нарушении |
|---|---|---|
| `space_id` | non-empty, `^[a-z0-9][a-z0-9-]{0,63}$` | эмит no-op + `warn!` |
| `project_id` | В Фазе 1 = `config.project_name`. Схема с хешем `abs_path` (открытый вопрос §12.8) отложена: на срезе два проекта, коллизий нет, а `abs_path` в каждом событии — фактический дизамбигуатор на стороне потребителя | — |
| `artifact_id` | проходит существующий `validate_artifact_id` | эмит no-op |
| строка события | ≤ 64 KiB, ровно один `\n` в конце, нет `\n` внутри | не пишется + `warn!` |
| lock | таймаут 200 мс | эмит no-op + `warn!`, мутация успешна |
| legacy `config.yaml` без `mesh:` | парсится, `mesh == None`, эмит выключен | тест-регрессия, MUST |
| `LanceStore::open` при битом `config.yaml` | `mesh = None`, open успешен | `warn!` |
| каталог `events/` недоступен (`chmod 000`) | эмит no-op | мутация успешна, CLI не блокируется |

**Новые зависимости**: `ulid = "1"` (для `seq` и `subscription_id`) — единственная
новая внешняя крейт-зависимость среза; `chrono`, `fs2`, `notify`, `walkdir`,
`serde_json` уже в `[workspace.dependencies]`. Добавление `ulid` проходит
supply-chain review (PROB-070).

**Направление зависимостей**: `forgeplan-core` → `forgeplan-mesh`, строго
односторонне. `forgeplan-mesh` **не** зависит от `forgeplan-core`: иначе цикл,
потому что `MeshConfig` живёт в `core::config::types`. Следствие: `emit()`
принимает уже разрезолвленные примитивы, а не `Config`:

```rust
pub fn emit(sink: &MeshSinkPaths, event: &SpaceEvent);  // никогда не возвращает Err
```

---

## Acceptance Criteria

Все критерии проверяются на реальных `~/Work/GertsAi/shared` (A) и
`~/Work/GertsHub` (B) с `mesh.space_id: gertsai-platform` в обоих
`config.yaml`, установленным `forgeplan`-бинарём (не `cargo run`) — dogfood
через ту же поверхность, которой пользуется человек.

- [ ] **A. Realtime.** Дашборд открыт. `forgeplan activate <id>` в проекте A →
      карточка появляется в дашборде **≤ 2 с**, без перезапуска дашборда и без
      перезагрузки страницы. Воспроизведено 3 раза из 3, в том числе для
      проекта B (обе стороны space пишут в один поток).
- [ ] **B. Durable catch-up.** Дашборд закрыт (процесс убит). В B выполняется
      активация. Дашборд запущен с сохранённым `last_offset` → показывает ровно
      пропущенное событие: без потерь, без дубликатов, курсор после реплея равен
      длине сегмента.
- [ ] **C. Dogfood — журнал удалён.** `rm -rf ~/.local/share/forgeplan/spaces/gertsai-platform`
      и `rm -rf <A>/.forgeplan/events`. Активация в A и в B завершается кодом
      выхода `0`, wall-clock не превышает baseline (тот же сценарий с
      `mesh.enabled: false`) более чем на **250 мс**. `space replay --from-scan`
      пересобирает журнал из markdown-фронтматтера, и после этого критерий B
      воспроизводится заново.
- [ ] **D. Graceful degradation — доступ запрещён.** `chmod 000` на
      `<A>/.forgeplan/events` и на `<space_dir>`. `forgeplan activate` в A:
      exit code `0`, артефакт реально активирован (`forgeplan get` показывает
      `status: active`), CLI не зависает (те же ≤ baseline + 250 мс), в stderr
      ровно одна строка WARN про эмит.
- [ ] **E. Legacy standalone цел.** Unit-тест: `config.yaml` без блока `mesh:`
      парсится, `Config.mesh == None`; интеграционный тест: активация в таком
      workspace не создаёт `.forgeplan/events/` и не обращается к `<space_dir>`.
- [ ] **F. Курсорные крайние случаи покрыты тестами.** Незавершённая строка не
      доставляется и не двигает курсор; переход через границу суточного сегмента
      продолжает поток; отсутствующий `segment_file` в `since` даёт
      `truncated: true`, а не тихий пропуск.
- [ ] **G. Пайплайн.** `cargo fmt --check` — 0 diff; `cargo check` — 0 warnings;
      `cargo clippy --workspace --all-targets -- -D warnings` — 0 warnings;
      `cargo test` — 0 failures на обеих feature-конфигурациях.

---

## Implementation Steps

Шаги идут строго по порядку: 1 без 0 не проверяем, 3 без 2 не компилируется.

### Шаг 0 — зафиксировать платформенные константы (~30 мин)

Никакого кода. Записать фактические значения на целевой машине в
EvidencePack: `getconf PIPE_BUF /` (ожидается 512 на darwin),
`echo "[$XDG_RUNTIME_DIR]"` (ожидается пусто), `echo "[$XDG_DATA_HOME]"`,
наличие валидного `.forgeplan/` и версия `forgeplan --version` в обоих проектах
(на машине стоит brew-бинарь; в handoff зафиксирован 0.32.1, в репозитории
уже 0.33.0 — расхождение версии бинаря и рабочего дерева проверить до кода,
иначе тестируется не то, что собрано).

### Шаг 1 — крейт `forgeplan-mesh`

Новые файлы:
- `crates/forgeplan-mesh/Cargo.toml` (deps: `serde`, `serde_json`, `chrono`,
  `fs2`, `ulid`, `tracing`, `thiserror`)
- `crates/forgeplan-mesh/src/lib.rs`
- `crates/forgeplan-mesh/src/event.rs` — `SpaceEvent`
- `crates/forgeplan-mesh/src/cursor.rs` — `Cursor`, парсинг/сравнение сегментов
- `crates/forgeplan-mesh/src/paths.rs` — `MeshSinkPaths`, резолв `<space_dir>`
  (XDG-каскад)
- `crates/forgeplan-mesh/src/emit.rs` — `emit()`, lock 200 мс, двойной append,
  fire-and-swallow

Изменяемые:
- `Cargo.toml` (корень) — `members += "crates/forgeplan-mesh"`,
  `[workspace.dependencies] ulid = "1"`

Тест на каждую `pub fn` сразу (правило проекта), включая: сериализация без
переводов строки внутри, отказ при строке > 64 KiB, no-op при недоступном
каталоге, no-op при таймауте lock'а.

### Шаг 2 — `MeshConfig` в `Config`

- `crates/forgeplan-core/src/config/types.rs` — `MeshConfig` + поле `mesh`
- `crates/forgeplan-core/Cargo.toml` — зависимость на `forgeplan-mesh`
- `.gitignore` — строка `.forgeplan/events/`
- тест: legacy-конфиг без `mesh:` парсится (критерий E)

### Шаг 3 — точка эмита

- `crates/forgeplan-core/src/db/store.rs`:
  - поле `mesh: Option<MeshSink>` в `struct LanceStore` (:399)
  - резолв в `LanceStore::open` (:440)
  - `pub(crate) async fn mesh_snapshot(&self, id)` рядом с `get_artifact` (:962)
  - вызов эмита последним шагом `update_artifact` (:1029)
- проверить, что путь `driver/in_memory.rs` эмит **не** делает (тестовый драйвер
  остаётся чистым), а `driver/lance.rs:56` эмитит транзитивно, без своего кода

### Шаг 4 — MCP-тул `space_subscribe`

- `crates/forgeplan-mcp/src/types.rs` — request/response DTO, `Cursor`
- `crates/forgeplan-mcp/src/server.rs` — регистрация тула в dispatch, реплей,
  tail через `notify`, отправка server-notification
- drift-детектор количества MCP-тулов (`scripts/check-mcp-tool-count.sh`)
  требует синхронного обновления счётчика в документации: 73 → 74

### Шаг 5 — dashboard tail

- `dev/space-mesh-dashboard/server.js` (~60 строк: `chokidar` + SSE +
  `?since=`), `dev/space-mesh-dashboard/index.html`, `README.md` с командой
  запуска. Не входит в cargo-workspace и не попадает в релизный бинарь.

### Шаг 6 — ручное доказательство инвариантов

Прогнать критерии A–D руками на A и B, зафиксировать вывод (timestamps, exit
codes, замеры wall-clock, содержимое stderr) в EvidencePack со
структурированными полями `verdict` / `congruence_level` / `evidence_type`.
Без этого R_eff = 0 и артефакт не активируется.

---

## Open Questions Pinned

Открытые вопросы handoff §12 не решаются в срезе; ниже — что именно принято
как временное допущение **только на Фазу 1**, чтобы срез не блокировался.

| § | Вопрос | Решение среза |
|---|---|---|
| 12.1 | Retention журнала | Не делается. Суточная сегментация есть, компакции нет. `truncated: true` в `space_subscribe` — заранее заложенный интерфейс под будущую компакцию |
| 12.2 | `space_id` как граница доверия | Honor-system + git-review. Оба проекта среза — одного владельца. HMAC не делается |
| 12.3 | RCE через `space_on{run}` | Вопрос неприменим: хуков в срезе нет |
| 12.4 | Windows-паритет | Нет. Срез — darwin. Курсор `{segment_file, byte_offset}` намеренно **без** inode/file-id, чтобы не тащить `#[cfg(windows)]`-ветку раньше времени |
| 12.5 | Team-shared `journal_root` (Dropbox/NFS) | Нет. Явное ограничение: только локальная FS |
| 12.6 | Cross-project semantic search | Нет → резидентный BGE-M3 (~150 МБ) не нужен, Фаза 1.5 остаётся опциональной и включается только по измеренной боли cold-start |
| 12.7 | Кто исполняет реакцию агента | В срезе реакции нет, есть только наблюдатель (дашборд) |
| 12.8 | Схема `project_id` | `config.project_name`; дизамбигуация — через `abs_path` в событии |

---

## Related

- **PRD-081 «ForgePlan Space-Mesh: кросс-проектная видимость и события»** (parent) —
  scope = capability-меню handoff §11, требования §4. Срез реализует FR-001, FR-003,
  FR-010, FR-011, FR-013 и закрывает открытые вопросы OQ-7 и OQ-8.
- **ADR-018 «Топология space-mesh: композиция SpaceJournal + ForgeMesh»** — композиция
  SpaceJournal (H3, 7/10) + ForgeMesh (H4, 6.5/10); отвергнуты H1 forgeplan-hub (6.5,
  только Фаза 2) и H2 SpaceBus (5, до перехода в multi-machine). Этот SPEC — контрактная
  детализация фазы F1·срез из ADR-018.
- **ADR-003** — markdown source of truth; журнал и зеркало space остаются derived.
- **Источник:** `~/Work/forgeplan-space-mesh-handoff.md` — §10 (срез + 6 шагов),
  §7.4 (точные швы в коде), §2 (платформенные константы), §12 (открытые вопросы).

