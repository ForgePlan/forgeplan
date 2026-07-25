---
depth: standard
id: PRD-081
kind: prd
last_modified_at: 2026-07-25T08:24:44.078413+00:00
last_modified_by: claude-code/2.1.219
links:
- target: PRD-078
  relation: based_on
status: draft
title: 'ForgePlan Space-Mesh: кросс-проектная видимость и события'
---

## Problem

На одной машине под `~/Work` лежит **21 проект с `.forgeplan/`** (§2), и часть из них —
микросервисы одного продукта: `GertsAi/{shared,gerts-hub}`, `GertsHub`, группа
`ExtraBoost*`, группа `ForgePlan*`. Артефакты продукта физически разорваны по 21
изолированному хранилищу: контрактный ADR живёт в `shared`, а RFC, который его
реализует, — в сервисе, и связи между ними не существует как данных.

Технически это следствие того, что forgeplan 0.32.1–0.33.0 **cwd-bound целиком** (§3):
`forgeplan serve` привязан к текущему каталогу, глобального `-C/--workspace` нет ни у
одной из ~76 CLI-команд. Значит `mcp__forgeplan__*` любого агента заперт в его проекте:
кросс-проектное действие возможно только через Bash с физическим `cd`. Нет реестра
проектов, нет группировки, нет кросс-проектных событий, подписок, поиска, графа и
claims.

**Что здесь честно уже работает за $0** (§5) — и на что этот PRD не претендует:
чтение соседа (`Read <path>/.forgeplan/adrs/ADR-*.md`) и запросы в соседа
(`cd B && forgeplan list --json`) работают сегодня; поиск проектов затыкается
`find ~/Work -name .forgeplan`. «Видеть и ходить» в режиме чтения — это `cd` + `Read`,
а не фича.

Net-new и дорогое ровно две вещи:
1. **Ergonomics** — реестр, группировка по `space_id` (по смыслу продукта, не по
   каталогам), realtime-видимость, typed-адресация вместо `cd`-шелла.
2. **Reactivity** — событий нет вообще (§5, строка «B реагирует на A» = ❌). Сегодня
   ни один проект и ни один агент не может узнать, что в соседнем проекте
   активировался артефакт, кроме как опросив его вручную.

Второе — то, ради чего строится транспорт: без durable-событий сценарии вида
«контрактный ADR в `shared` активировался → зависимые сервисы получили NOTE» или
«R_eff сервиса упал ниже порога → пинг» невозможны в принципе.

## Goals

Формулировки проверяемые; критерии успеха G1–G3 дословно взяты из §10.

- **G1 — durable catch-up.** Дашборд закрыт, в проекте B активирован артефакт, дашборд
  открыт заново с сохранённым `last_offset` → пропущенное событие видно. Потерь
  событий при штатной работе (без contention) — ноль.
- **G2 — graceful degradation by construction.** `chmod 000` на `.forgeplan/events/` →
  `forgeplan activate` проходит успешно и CLI **не виснет**; каталог `events/` удалён →
  все проекты продолжают работать standalone, `space replay --from-scan` восстанавливает
  журнал из markdown.
- **G3 — realtime без рестарта.** Активация артефакта в проекте A даёт карточку в
  дашборде живьём, без перезапуска дашборда и без polling-diff по `state/*.yaml`.
- **G4 — стоимость эмита ограничена сверху.** Худший случай задержки мутации = таймаут
  lock **100–250 мс**, после чего emit становится no-op (§8, major #1 H3); медианный
  оверхед — на уровне одного append. Интерактивный dogfood не деградирует.
- **G5 — typed cross-project addressing.** Агент из проекта A вызывает read-тул в
  проекте B своего space через `mcp__forgeplan__space_query`, без `cd` и без Bash.
  Сегодня это невозможно (§3).
- **G6 — нулевая регрессия для standalone.** Все 21 существующий проект без блока
  `mesh` в `.forgeplan/config.yaml` парсятся и работают ровно как сейчас (`Config`
  помечен `#[serde(default)]`, §7.4).
- **G7 — группировка по смыслу.** Проекты видны сгруппированными по `space_id` из
  committed-конфига, а не по расположению каталогов на диске (§4).

## Non-Goals

- **Демон-хаб (`forgeplan serve` как резидент, H1) в v1.** Отвергнут как основа
  (балл 6.5, §8): на darwin `XDG_RUNTIME_DIR` пуст → primary socket path мёртв;
  `sun_path` 104 B; brew-автоапгрейд бинаря даёт version skew между новым CLI и старым
  `serve`. Возвращается только как F1.5-ускоритель и только по измерению.
- **Встроенный брокер (SpaceBus / iggy / embedded-NATS, H2).** Балл 5, не выживает
  (§8): правильная семантика в неправильной оболочке — второй непрозрачный
  не-git-friendly durable-стор, двойная запись outbox+стрим, supervision дочернего
  процесса. Берём семантику (subject-naming, offset-replay, durable fan-out) поверх
  NDJSON, брокер не шипим.
- **mDNS-дискавери.** Явная анти-рекомендация §7.5: single-machine, лишняя attack
  surface. Только реестр + filesystem-cascade.
- **Windows-паритет в v1.** Курсор `{file, offset, inode}` на NTFS требует
  `GetFileInformationByHandle` за `#[cfg(windows)]`, unix-сокет дашборда → named pipe.
  Открытый вопрос OQ-4; на v1 целевая платформа — macOS/Linux.
- **Team-shared `journal_root` на Dropbox / iCloud / NFS.** O_APPEND-атомарность и
  inode-курсоры там не гарантированы; v1 фиксирует «только локальная FS».
- **Кросс-проектный semantic search в v1.** Он тянет резидентный BGE-M3 (~150 MB на
  каждый короткий процесс) и тем самым переносит warm-демон из F2 в F1.5 (§12, OQ-6).
  В v1 — вне scope.
- **Multi-tenant машина и репозитории разного уровня доверия в одном space.**
  `space_id` = граница доверия (OQ-2).
- **Замена Herdr.** Herdr — про терминальные сессии и PTY, другой слой; он не покажет
  артефакты соседнего проекта (§6). Заимствуем три идеи (socket, NDJSON, blocking
  `wait`), не интегрируемся с ним и не «коннектимся к herdr.dev».

## Target Users

- **Владелец локального парка проектов** (основной, он же единственный на v1):
  один человек, одна macOS-машина, 21 репозиторий с `.forgeplan/`, из которых
  несколько групп — микросервисы одного продукта. Ему нужны ergonomics (видеть
  продукт целиком) и reactivity (не пропустить изменение контракта в `shared`).
- **Агенты, работающие внутри одного space.** Сегодня агент видит только свой проект;
  ему нужны адресация (`space_query`), координация (кросс-проектные claims, чтобы два
  агента не взялись за один эпик в разных репозиториях) и подписка (реагировать, а не
  опрашивать).
- **Будущий ForgeFarm-оркестратор как потребитель.** Модель-агностик оркестратор над
  ForgePlan нуждается ровно в двух вещах, которых сейчас нет: единая лента событий по
  всем проектам и адресуемая read-плоскость. Space-mesh — их поставщик; ForgeFarm —
  первый внешний клиент журнала.

## Requirements

Разбиение по слоям §11. Маркеры фаз: **F1·срез** — минимальный вертикальный срез §10;
**F1** — остальная Фаза 1; **F1.5** — опциональный warm-демон; **F2** — Фаза 2+.

### Инварианты (действуют на все FR)

- **INV-001.** Хаб открывает чужие `LanceStore` **строго READ-ONLY**; **каждая** мутация
  роутится в свежеспавненный короткоживущий per-project процесс по `target.project`
  (§9, критический инвариант). Снимает LanceDB single-writer и сохраняет red-line
  «мутации только через legal write-path».
- **INV-002.** Emit — **fire-and-swallow**: EACCES, полный диск, таймаут lock → log и
  проглотить; мутация артефакта всегда успешна.
- **INV-003.** Журнал и хаб — **derived** (ADR-003 цел). Единственный не-markdown
  артефакт членства — declarative `space_id` в committed `config.yaml`.
  `.forgeplan/events/` — gitignored, sibling к `lance/`, `claims/`, `state/`, `.lock`.
- **INV-004.** Хаб мёртв → откат на per-project MCP; журнал недоступен → emit no-op,
  проект самоуправляется.

### 👁 Видеть — discovery и realtime

- **FR-001 [F1·срез].** Членство: новое top-level `mesh: Option<MeshConfig{space_id:
  Option<String>, enabled: bool}>` в `Config` (`crates/forgeplan-core/src/config/types.rs`),
  committed в `.forgeplan/config.yaml`. Тест: legacy-конфиг без `mesh` парсится через
  serde default → standalone цел.
- **FR-002 [F1].** Реестр `~/.config/forgeplan/registry.json` + self-registration в
  `init_workspace` (`crates/forgeplan-core/src/workspace/init.rs`). Discovery-cascade:
  explicit override > registry > `walkdir`-скан (dep уже есть). Обобщает нынешний
  `find_workspace` (cwd-walk-up) до `list_workspaces()`. На срезе §10 реестра нет —
  хардкод/walkdir.
- **FR-003 [F1·срез].** Dashboard-tail: ~60 строк Node (chokidar на `events/*.ndjson`) +
  одна HTML-страница на SSE; NDJSON-строка → карточка `project · artifact · activated ·
  R_eff · ts`. Сознательно **не через MCP** — доказываем, что журнал есть обычный файл.
- **FR-004 [F2].** Heartbeat «кто живой» + полноценный дашборд spaces × projects.

### 🚶 Ходить и читать — addressing

- **FR-005 [F1].** MCP-тул `space_list_projects` — список spaces и проектов из реестра.
- **FR-006 [F1].** MCP-тул `space_query{target: project|space_id, tool, args}` — вызов
  любого read-тула в любом проекте своего space; хаб делает `LanceStore::open(abs_path)`
  read-only (INV-001) и использует встроенный stale-handle auto-recovery
  (`with_retry_on_stale`, спроектирован под long-running процесс).
- **FR-007 [F1].** Write-path хаба: мутационные тулы принимают `target`, но исполняются
  спавном per-project процесса; `target` **опционален** — default = cwd, чтобы обычный
  однопроектный вызов не менялся.
- **FR-008 [F1].** Fan-out: `space_query(space, "health")` и аналоги — агрегированный
  ответ по всем членам space (on-demand fan-out дешевле live-дашборда, §7.2).
- **FR-009 [F1.5].** Кросс-проектный semantic search («где во всём продукте решали про
  rate-limiting?»). Требует резидентного BGE-M3 → включается вместе с warm-демоном.

### ⚡ Реагировать — события

- **FR-010 [F1·срез].** Новый тонкий crate `forgeplan-mesh`: тип `SpaceEvent {v, ts,
  seq(ULID), space_id, project_id, artifact_id, artifact_type, kind, from_status,
  to_status, r_eff, md_path, abs_path, agent_id, actor}` + `emit(workspace, event)`.
  emit резолвит `space_id` из `Config` (None → return, standalone); берёт `fs2`-lock на
  `<space>/.events.lock` с таймаутом **100–250 мс** (таймаут → no-op + log, не
  блокировать); аппендит одну serde_json-строку + `\n`. **`lock+append` — норма**;
  O_APPEND без lock — опциональный fast-path только для строк ≤ PIPE_BUF, а на darwin
  `PIPE_BUF = 512 B` (против 4096 на Linux), поэтому fast-path по умолчанию выключен.
- **FR-011 [F1·срез].** Emit choke-point: вызов `forgeplan_mesh::emit()` внутри
  `LanceStore::update_artifact` **сразу после** персиста frontmatter и reindex, последним
  шагом. Один тип события — `artifact.activated`.
- **FR-012 [F1].** Полный choke-point: `LanceStore::{create_artifact, update_artifact,
  update_body}` + `ClaimStore::{claim, release}` — пять точек, покрывающих **все**
  мутации артефактов для CLI и MCP сразу. Обёртка вокруг MCP-dispatch пропустила бы
  CLI-мутации, поэтому эмит живёт в store, а не в транспорте. События: created /
  activated / superseded / deprecated / stale + изменения score.
- **FR-013 [F1·срез].** MCP-тул `space_subscribe{space_id, kinds, since}` в dispatch
  `crates/forgeplan-mcp/src/server.rs`: открыть `events/*.ndjson`, seek к since-offset,
  отдать строки с `kind ∈ filter` (реплей), затем tail через `notify` и стримить новые
  как server-sent notifications. Курсор — `{segment_file, byte_offset}`, персистится
  **per-subscriber** (чинит D-Bus-дыру «события теряются при дисконнекте», §7.5).
- **FR-014 [F1].** `space replay --from-scan` — пересборка журнала из markdown по образцу
  `scan-import` для LanceDB. Доказательство, что журнал derived (INV-003).
- **FR-015 [F2].** `space_wait` — блокирующее ожидание события (Herdr-паттерн,
  correlate по JSON-RPC id).
- **FR-016 [F2].** `space_on{match, run}` — server-side триггеры watchman-стиля,
  переживающие рестарт. Только opt-in и только из committed review-only файла (OQ-3).

### 🤝 Координировать

- **FR-017 [F2].** Кросс-проектные claims: поднять существующий file-TTL claim до
  space-scope, lock в форме artifact-id `EPIC-12@spaceA`. Не строить lock-сервер —
  переиспользовать TTL + atomic temp+rename + hardened id/agent validation; низкоуровневый
  `fs2`-lock `workspace/lock.rs` (bounded wait, backoff, symlink guards) — шаблон.
- **FR-018 [F2].** Карта активности агентов: rollup blocked / working / done по всем
  проектам space (модель Herdr).
- **FR-019 [F2].** Кросс-проектный dispatch: эпик на 3 сервиса → 3 агента с
  conflict-free bucket'ами поверх space-claims.

### 🧠 Продуктовый мозг

- **FR-020 [F2].** **Space Context Pack** — при создании нового модуля автосбор общих
  ADR / контрактов / конвенций / тех-стека продукта, чтобы новый сервис стартовал
  консистентным by construction.
- **FR-021 [F2].** **Impact radius**: «deprecate ADR-005 в `shared` → что сломается и в
  каких сервисах?».
- **FR-022 [F2].** **`space_contradictions`**: сервис A решил X про auth, сервис B — не-X
  → флаг.
- **FR-023 [F2].** **Детект shared-kernel**: три сервиса независимо написали ADR про одну
  retry-policy → предложить поднять решение на space-уровень.
- **FR-024 [F2].** **Кросс-проектные связи**: `link RFC-в-A implements ADR-в-shared`
  (сегодня связи только внутри проекта) → граф зависимостей через весь продукт.
- **FR-025 [F2].** **Product-level health / timeline**: один R_eff, blindspots и лента
  решений на весь продукт; ответ на «какой сервис — слабое звено?».

## Phasing

**Шаг 0 (до кода, ~полчаса, §10).** Зафиксировать платформенные константы: `getconf
PIPE_BUF /` на darwin, пустоту `XDG_RUNTIME_DIR`, валидность `.forgeplan` у проекта B.
Плюс закрыть минимум OQ-4 (Windows), OQ-1 (retention), OQ-2 (trust boundary).

**Фаза 1 — минимальный вертикальный срез (§10).** 1 space `gertsai-platform`,
2 реальных проекта (`~/Work/GertsAi/shared` + `~/Work/GertsHub`), 1 тип события
`artifact.activated`, 1 MCP-тул `space_subscribe`, ~60-строчный dashboard-tail.
FR-001, FR-003, FR-010, FR-011, FR-013. Вне среза: хаб-демон, fan-out, кросс-проектные
claims, semantic search, hooks, второй тип события, Windows-паритет, реестр.
Закрытие среза = ручное доказательство G1 и G2.

**Фаза 1 — расширение.** FR-002 (реестр + self-registration), FR-005–FR-008
(read-плоскость ForgeMesh: `space_list_projects`, `space_query`, fan-out, write-routing
по INV-001), FR-012 (полный choke-point из пяти точек), FR-014 (`space replay
--from-scan`).

**Фаза 1.5 — опциональная.** Warm-демон с резидентным BGE-M3 и пулом warm
`LanceStore`-хэндлов (FR-009). Включается **только** если измерена боль cold-start или
если кросс-проектный semantic search признан обязательным (OQ-6). Демон — строго
опциональный ускоритель поверх журнала, никогда не source of truth; жёсткие инварианты:
демон никогда не пишет и не реиндексит чужие LanceDB, emit fire-and-swallow,
version-handshake CLI↔демон. Lifecycle `hub status|start|stop|logs|clean`, каждая
операция обязана работать при мёртвом хабе.

**Фаза 2+.** FR-004 (heartbeat + дашборд), FR-015–FR-016 (`space_wait`, hooks),
FR-017–FR-019 (координация), FR-020–FR-025 (продуктовый мозг), фронт.

**Отложено до multi-machine.** SpaceBus / встроенный брокер (H2). Возвращается, когда
single-machine перестанет быть single-machine: сеть, контейнеры, remote-агенты,
«ForgePlan Teams/Cloud».

## Open Questions

- **OQ-1 — retention журнала.** Дневная сегментация решает рост, но кто компактит и
  удаляет старые сегменты, по какой политике (N дней / M событий), и должен ли
  `space replay --from-scan` усекать журнал до текущего состояния — *решается в ADR*.
- **OQ-2 — `space_id` как граница доверия.** Любой проект, объявивший `space_id=X` в
  committed-конфиге, читает события всех членов X и пишет в общий журнал: нужна ли
  опциональная HMAC-привязка project→space из shared-секрета, или honor-system +
  git-review достаточно — *решается в ADR*.
- **OQ-3 — RCE-вектор `space_on{run: ...}`.** Хуки исполняют команды: достаточно ли
  opt-in `--allow-hooks` плюс требование, чтобы хуки жили только в committed review-only
  файле (никогда по сети), или хуки надо отложить за v1 — *решается в ADR*.
- **OQ-4 — Windows-паритет на v1.** Курсор `{file, offset, inode}` ломается на NTFS
  (нужен file-id через `GetFileInformationByHandle`), сокет дашборда → named pipe:
  фиксируем ли macOS/Linux-only или тянем Windows из коробки — *решается в ADR*.
- **OQ-5 — team-shared `journal_root` на Dropbox / iCloud / NFS.** Там не гарантированы
  ни O_APPEND-атомарность, ни inode-курсоры: фиксируем ли «только локальная FS» как
  явное ограничение v1 — *решается в ADR*.
- **OQ-6 — кросс-проектный semantic search на v1.** Если да, warm-демон становится
  Фазой 1.5, а не отложенной Фазой 2, потому что резидентный BGE-M3 (~150 MB) тянется
  раньше — *решается в ADR*.
- **OQ-7 — кто исполняет реакцию агента.** Ad-hoc процесс, тейлящий журнал, или
  опциональный foreground `forgeplan space watch`, и насколько «мгновенной» должна быть
  реакция — *решается в SPEC*.
- **OQ-8 — формат `project_id`.** Брать из `project_name` (риск коллизий между 21
  проектом) или стабильный slug `project_name + hash(abs_path)`, вычисляемый при join —
  *решается в SPEC*.

## Related

- **ADR «Топология space-mesh»** *(создаётся следом)* — фиксирует решение §9: композиция
  SpaceJournal (H3, 7/10, durable transport + emit) и ForgeMesh (H4, 6.5/10,
  addressing/read plane) как одного продукта в двух фазах, плюс критический инвариант
  read-only хаба (INV-001). Отклонённые альтернативы с баллами: H1 forgeplan-hub (6.5,
  Phase-2 ускоритель), H2 SpaceBus (5, не выживает).
- **SPEC «Минимальный срез space-mesh»** *(создаётся следом)* — контракт `SpaceEvent`,
  формат NDJSON-строки, семантика курсора `{segment_file, byte_offset}`, сигнатура
  `space_subscribe`, протокол emit (lock, таймаут, fire-and-swallow). Закрывает OQ-7 и
  OQ-8.
- **EVID «Ресёрч space-mesh»** *(создаётся следом)* — 5 досье по движкам
  (BMAD-METHOD ~48k★, GitHub Spec Kit ~43k★, OpenSpec ~51k★, forgeplan-internals,
  prior-art survey tmux/Watchman/NATS/D-Bus/Bazel/Turborepo/Nx/LSP/Tilt/Mercurial),
  4 состязательно оценённые гипотезы, grounded-пробинг машины (§2, §3). Ключевой вывод:
  **ни один из изученных движков не имеет кросс-проектного слоя или событий** — это
  net-new для экосистемы и требует обоснования стоимостью.
- **PRD-078 + ADR-015 (MCP worktree-aware routing)** — ближайший существующий шов и
  прямое prior art: они уже сделали MCP-тулы **workspace-адресуемыми** (опциональный
  параметр `workspace` со strict write-gate и soft read на store-resolution тулах,
  v0.33.0, закрыл PROB-072). Space-адресация (`target={project|space}`, FR-006/FR-007) —
  расширение того же семантического шва с одного worktree на N проектов, поэтому она
  должна переиспользовать эту цепочку резолва, а не строить вторую. Смежно: **ADR-016**
  (единая цепочка резолва через `WorkspaceResolver`, недопущение store split-brain).
- **ADR-003 (markdown — source of truth)** — рамка, внутри которой space-mesh обязан
  остаться: журнал и хаб derived и пересобираемы (INV-003, FR-014), единственный
  не-markdown артефакт членства — `mesh.space_id` в committed `config.yaml`.
- **Внешние референсы:** Herdr (`github.com/ogulcancelik/herdr`) — образец socket +
  NDJSON + blocking `wait` + discovery-cascade; OpenSpec (`Fission-AI/OpenSpec`) —
  registration-based discovery и JOIN-схема; GitHub Spec Kit — scaffolder/engine split и
  «события как byproduct детерминированной мутации».
- **Источник:** `~/Work/forgeplan-space-mesh-handoff.md` (512 строк) — полный контекст,
  условия, досье и вердикты; ссылки вида §N в теле PRD указывают на его разделы.




