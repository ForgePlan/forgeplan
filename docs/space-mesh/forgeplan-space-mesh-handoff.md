# ForgePlan Space-Mesh — полный контекст и хендофф

> **Что это:** самодостаточный документ с идеей, условиями, результатами ресёрча и
> архитектурными гипотезами для кросс-проектного слоя ForgePlan («space-mesh»).
> Собран из сессий в `~/Work/GertsAi/shared` (28–29 мая 2026). Предназначен для
> переноса в проект **`~/Work/ForgePlan`** и продолжения работы там (можно с нуля —
> здесь есть всё нужное).
>
> **Язык:** русский + английские технические термины/идентификаторы.
> **Статус:** product-discovery / pre-PRD. Решение «строить» принято; топология
> выбрана (рекомендация §8); не реализовано.

---

## Оглавление

0. [Происхождение и как читать](#0-происхождение)
1. [Идея и проблема](#1-идея)
2. [Условия и факты (grounded)](#2-условия)
3. [Что forgeplan умеет/не умеет сегодня](#3-сегодня)
4. [Требования пользователя (решение)](#4-требования)
5. [Что уже работает за $0](#5-сегодня-бесплатно)
6. [Herdr — что это, connect vs borrow](#6-herdr)
7. [Ресёрч движков — 5 досье](#7-досье)
8. [4 архитектурные гипотезы + вердикты](#8-гипотезы)
9. [Рекомендация: композиция SpaceJournal + ForgeMesh](#9-рекомендация)
10. [Минимальный срез + шаги прототипа](#10-срез)
11. [Capability-меню «что ещё может уметь»](#11-меню)
12. [Открытые вопросы](#12-вопросы)
13. [Следующие шаги в проекте ForgePlan](#13-дальше)
14. [Приложение: где лежат сырые данные](#14-приложение)

---

<a name="0-происхождение"></a>
## 0. Происхождение

- Ресёрч выполнен динамическим workflow (20 агентов) 28.05.2026 в сессии
  `6eae00df` (gertsai/shared). Сессия упала на API-ошибке про `thinking` blocks,
  но выходной артефакт workflow уцелел (246 KB).
- Дозбор (что forgeplan/Herdr умеют сегодня) + capability-брейншторм — 29.05.2026.
- Это **net-new разработка в самом forgeplan** (Rust: новый crate + эмит-хук в
  `LanceStore` + space-тулы в MCP + реестр + хаб). Не правки прикладных репо.

<a name="1-идея"></a>
## 1. Идея и проблема

На машине одновременно живёт много проектов, в каждом локальная папка `.forgeplan/`
с артефактами (PRD/RFC/ADR/SPEC/EVID/…). Проекты могут быть **микросервисами
одного продукта**. Хочется:

1. **Видеть все проекты** — сразу, в realtime, **сгруппированными по `space_id`**
   (не сканом каталогов).
2. **Любой агент может «зайти» в любой проект** своего space и получить данные /
   вызвать там тулы.
3. **События**: проекты эмитят изменения; можно подписаться, ждать, вешать хуки —
   чтобы один проект/агент реагировал на изменения в другом.
4. Вдохновение — **Herdr** (socket API: «вижу все сессии, захожу в любую, жду
   состояния»). Изучить применимость/исходник.

Главный вопрос изначально был «нужно ли это вообще» → **ответ: да, строим**, но
фазированно (см. §9, §13). Большая часть «просто читать соседей» работает уже
сегодня (§5) — net-new и дорогое только событийно-реактивная часть.

<a name="2-условия"></a>
## 2. Условия и факты (grounded на машине пользователя)

- **forgeplan 0.32.1**, установлен через brew (`/opt/homebrew/bin`).
- В каждом проекте MCP через **stdio**: `.mcp.json` → `{command: "forgeplan",
  args: ["serve"], transport: "stdio"}`.
- Платформа: **macOS (Darwin)**. Эмпирически: `XDG_RUNTIME_DIR` **пуст** (важно —
  ломает «primary socket path» демонных вариантов); `PIPE_BUF` на macOS = **512 B**
  (на Linux 4096) — влияет на атомарность append.
- **21 проект** с `.forgeplan/` под `~/Work` (есть микросервисные группы):
  `GertsAi/{shared,gerts-hub}`, `GertsHub`, несколько `ExtraBoost*`, несколько
  `ForgePlan*`, `pollmevals`, `TolikH`, `elirum.me`, `ai.elirum.me`,
  `TripSales`, `PetAdoptionApi`, `MoleculerPy`, и др.

<a name="3-сегодня"></a>
## 3. Что forgeplan 0.32.1 умеет / не умеет сегодня

**Умеет:**
- `serve` — MCP-сервер (stdio), **привязан к cwd** (без аргументов пути).
- `watch` — следит за `.forgeplan/` и синкает в LanceDB в реальном времени (но это
  per-project sync markdown→index, **не** кросс-проектные события).
- ~76 CLI-команд (list/get/search/graph/health/blindspots/blocked/stale/journal/
  claim/dispatch/activate/…) — все **single-project, cwd-bound**.
- `activity` — append-only JSONL лог всех MCP-вызовов в `.forgeplan/logs/` (есть
  готовый прецедент «журнала событий», но он per-project и про tool-calls).

**НЕ умеет (подтверждено пробингом):**
- Нет глобального `-C/--workspace/--cwd`. У команд (напр. `list`) нет флага пути.
- Значит **MCP каждого агента заперт в его проекте**: `mcp__forgeplan__*` нельзя
  направить в чужой проект. Кросс-проект — только через Bash CLI с физическим `cd`.
- Нет `space`-команд, нет реестра проектов, нет кросс-проектных событий/подписок,
  нет кросс-проектного поиска/графа/claims. Всё это — **net-new**.

<a name="4-требования"></a>
## 4. Требования пользователя (зафиксированное решение)

- **Discovery через реестр + сам/регистрацию**, не через скан диска (скан — только
  однократный бутстрап/fallback).
- **Группировка по `space_id`** (по смыслу/продукту, а не по папкам).
- **Realtime** — видеть проекты и их состояние вживую (реестр + heartbeat).
- **Любой агент → любой проект** своего space: «зайти» и получить данные/вызвать тул.
- **Socket-слой событий** в духе Herdr (подписка, wait, хуки).

<a name="5-сегодня-бесплатно"></a>
## 5. Что уже работает сегодня за $0 (capability ladder)

Три разных желания с очень разной ценой:

| Хочу | Сегодня | Как |
|---|---|---|
| **Читать** артефакты соседа | ✅ работает | `Read <path>/.forgeplan/adrs/ADR-*.md` |
| **Запросы** в соседа (list/health/graph/search) | ✅ работает | `cd B && forgeplan list --json` |
| **Мутации** в соседе | ⚠️ CLI да, MCP нет | typed `mcp__forgeplan__*` заперт в своём проекте |
| **Найти** все проекты | ✅ (но как затычка) | `find ~/Work -name .forgeplan` |
| **События** (B реагирует на A) | ❌ нет | net-new |

**Вывод:** «видеть и ходить» в режиме *чтения* — это `cd` + `Read`, не фича.
Net-new и дорогое — *реестр/realtime/группировка* (ergonomics) и *события*
(reactivity). Пользователь хочет именно их.

<a name="6-herdr"></a>
## 6. Herdr — что это, connect vs borrow

- **Что:** agent-aware терминальный мультиплексор (как tmux/Zellij, но «понимает»
  агентов: показывает blocked/working/done по сессиям, rollup). Self-hosted/локальный,
  **без облака**. Repo: `github.com/ogulcancelik/herdr`.
- **Архитектура:** session-server (фоновые PTY), **Socket API = newline-delimited
  JSON (NDJSON)**, CLI, локально/по SSH. Агент создаёт панели, гоняет команды,
  читает вывод, **`events.wait`** (ждёт смены состояния).
- **Слой другой, чем forgeplan:** Herdr про терминальные сессии, не про артефакты.
  Он НЕ покажет артефакты соседнего проекта.
- **Как использовать:**
  - **Референс** — образец, КАК сделать socket+NDJSON+`wait`+discovery-cascade чисто
    (для нашей событийной фазы).
  - **Ортогонально** — гонять/наблюдать наших агентов по 21 проекту в одном
    терминале (rollup blocked/working/done про агентскую работу).
  - **«Коннектиться к herdr.dev» смысла нет** — это локальный бинарь, не сервис.
    Либо запускаем локально, либо заимствуем 3 идеи (socket + NDJSON + wait).

<a name="7-досье"></a>
## 7. Ресёрч движков — 5 досье (ключевые уроки)

Изучены движки spec-driven/agile-AI инструментов: как у них устроены engine,
state, coordination, events, multi-project. Главный вывод: **никто из них не имеет
кросс-проектного слоя или событий** — это net-new для экосистемы ForgePlan, и его
надо обосновывать стоимостью.

### 7.1 BMAD-METHOD (`bmad-code-org/BMAD-METHOD`, ~48k★, V6 «native Skills»)
- **Движок:** НЕТ runtime/демона/сервера. «Движок» = host-LLM, читающий markdown.
  Код только в build-time CLI (`npx bmad-method install` — копирует skills, генерит
  IDE-файлы; выходит после установки).
- **State:** plain files; cross-turn state через **YAML frontmatter**; `config.yaml`
  per-module; `module-help.csv` с `preceded-by/followed-by` = декларативный sequencer.
- **Coordination:** document-mediated; **нет** scheduler/lock/bus. Multi-agent
  конфликты — **прозой** (`preventing-agent-conflicts.md`: git-ветки, disjoint files).
  Нет claim/lease/mutex.
- **Events:** чисто request/response. Нет событий/хуков/watchers/pubsub.
- **Multi-project:** по сути НЕТ — строго single-repo.
- **Уроки:** markdown-first с zero-runtime — проверенный паттерн на 48k★ (валидирует
  философию ForgePlan, демон/шина — только опциональный ускоритель, не source of
  truth). Self-contained handoff-артефакты бьют live-messaging (контекст «запекается»
  в story/spec). Декларативный sequencing в committed-файле дёшево заменяет event-движок.

### 7.2 GitHub Spec Kit (`github/spec-kit`, `specify` CLI v0.0.93, ~43k★, Python/Typer)
- **Уроки/applicable:**
  - **Scaffolder/engine split**: `forgeplan space join <id>` = one-shot scaffolder
    (пишет config + хуки), коллектор отдельный и опциональный — graceful degradation.
  - **События как byproduct детерминированной мутации** (как `--json` у Spec Kit).
    Мутации ForgePlan уже идут через CLI/MCP (red-line запрещает прямой Edit) → это
    **естественный choke-point** для append события. НЕ просить LLM «не забыть эмитнуть».
  - **Discovery cascade**: explicit override > git > filesystem scan.
  - **Markdown authoritative, коллектор — derived/rebuildable** (как scan-import для LanceDB).
  - **On-demand fan-out `space health/analyze`** дешевле live-дашборда — это MVP.
  - Существующий `forgeplan claim --ttl` = тот примитив координации, которого Spec Kit
    лишён → расширить до space-scope, не изобретать lock-сервер.

### 7.3 OpenSpec (`Fission-AI/OpenSpec`, npm `@fission-ai/openspec`, TS/Node, ~51k★, MIT)
- **Уроки/applicable:**
  - **Registration-based discovery** (копировать дословно): не авто-скан, а
    `forgeplan workspace link <path>` → machine-local registry
    (`$XDG_DATA_HOME/forgeplan/spaces/`), `doctor`, `list`. Это и есть хаб «видеть все
    и зайти в любой».
  - **JOIN-схема**: малый committed config (`workspace.yaml` с `links:`) → у ForgePlan
    `space:` блок в `.forgeplan/config.yaml` (declarative, committed).
  - **Delta-folder** для cross-project SHAPE — markdown-дельты (ADDED/MODIFIED/REMOVED),
    git-diffable, чтобы шина оставалась markdown-first.
  - **«directory/file location = state»** как дешёвый источник событий: state-файл
    flip draft→active = file write; тонкий emitter watch'ит их и синтезирует события.
  - Mesh — **opt-in и view-only** над authoritative per-repo state.
  - **Негатив:** OpenSpec НЕ даёт transport/addressing-by-event/coordination; даже его
    registry имеет нерешённый cross-runtime path-translation (XDG/Windows/WSL). → для
    этих под-решений опираться на Herdr + существующий claim.

### 7.4 ForgePlan-internals (что именно трогать в коде)
- **EMIT HOOK (несущий):** `emit(SpaceEvent{...})` внутри
  `forgeplan_core::db::store::LanceStore::{create_artifact, update_artifact,
  update_body}` (created/activated/superseded/deprecated/stale + score changes) +
  `ClaimStore::{claim,release}`. **Эти 3 метода — choke-point ВСЕХ мутаций артефактов
  для CLI И MCP сразу.** (Обёртка вокруг MCP-dispatch пропустила бы CLI-мутации.)
- **JOIN:** optional top-level `mesh: Option<MeshConfig{space_id, ...}>` в `Config`
  (`config/types.rs`). Config помечен `#[serde(default)]` → legacy без блока парсятся
  как standalone (подтверждено тестами). Self-registration в `init_workspace`
  (`workspace/init.rs`).
- **TRANSPORT default:** append-only **NDJSON** журнал в новом gitignored
  `.forgeplan/events/*.ndjson` (sibling к lance/claims/state/.lock), зеркалится в
  machine-level `~/.local/share/forgeplan/spaces/<space_id>/`. Append-only crash-safe,
  replayable, broker-free. `notify`/`notify-debouncer-mini` (уже deps) — для tail+fan-out.
- **DISCOVERY:** machine-global `~/.config/forgeplan/registry.json`, self-register в
  `init_workspace`; fallback — `walkdir`-скан (уже dep). Обобщает `find_workspace`
  (cwd-walk-up) через `list_workspaces()`.
- **ADDRESSING:** space-aware тулы в JSON-RPC dispatch `forgeplan-mcp/src/server.rs`:
  `space_list_projects`, `space_query{project|space_id, tool, args}`,
  `space_subscribe{space_id, kinds, since}`, `space_on{event, hook}`. Хаб открывает
  любой проект `LanceStore::open(abs_path)` и фанаутит. Blocking `events.wait`
  (Herdr-паттерн, correlate по JSON-RPC id) тейлит журнал. Использует встроенный
  stale-handle auto-recovery (`with_retry_on_stale`, спроектирован под long-running daemon).
- **GENERALIZE CLAIMS:** при наличии `space_id` claim/release доп-эмитят в шину →
  кросс-проектные lease поверх существующего TTL + atomic temp+rename + hardened
  id/agent validation. Низкоуровневый `fs2` `.forgeplan/.lock` (`workspace/lock.rs`:
  bounded wait, backoff, symlink guards, OS-release-on-crash) — шаблон для жёсткого lock.
- **WARM-INDEX payoff:** резидентный fastembed **BGE-M3 (~150 MB)** + warm
  `LanceStore`-хэндлы амортизируют cold-start cross-project search/embed. db/store.rs
  уже это поддерживает. LanceDB остаётся derived (scan-import пересобирает); писатели
  сериализуются через per-workspace `fs2` `.lock`; хаб мёртв → откат к per-process.

### 7.5 prior-art survey (tmux/Watchman/NATS/D-Bus/Bazel/Turborepo/Nx/LSP/Tilt/Mercurial)
- **TRANSPORT (decision 3):** primary = per-project append-only NDJSON журнал
  (monotonic offset = курсор); каждая мутация аппендит строку; tail работает standalone.
- **WARM DAEMON:** опциональный хаб (резидентный BGE-M3 + tail всех журналов на один
  сокет). Lifecycle `hub status|start|stop|logs|clean` (урок Turborepo); каждая
  операция работает hub-down (урок Nx). Хаб = кэш, никогда source-of-truth.
- **EVENT MODEL (push):** Watchman `subscribe(scope, filter, since=offset)` — реплей
  после курсора, затем live; Herdr blocking `events.wait`; **per-subscriber курсоры
  персистятся** (чинит D-Bus-дыру «события теряются при дисконнекте»).
- **HOOKS:** Watchman-style server-side triggers `{match, run}`, переживают рестарт.
- **ADDRESSING (decision 4):** иерархич. subject `space_id.project_id.artifact_type.event`
  + wildcard `spaceA.*.>` (NATS), маршрутизация LSP-by-URI (project_id = один root;
  space_id = fan-out).
- **JOIN (decision 2):** declarative + committed `space_id` в config.yaml (как
  Procfile/Tiltfile membership).
- **DISCOVERY (decision 1):** filesystem-based, **НЕ mDNS** (single-machine, лишняя
  attack surface). Реестр + cascade: `--hub-socket` flag > env > deterministic default
  по hash(space_id) (XDG).
- **COORDINATION:** поднять существующий file-TTL claim до space-scope, не строить
  lock-сервер. Кросс-проектный claim = artifact-id-shaped lock `EPIC-12@spaceA`.
- **«SEE ALL + WALK INTO» UX:** tmux/LSP/Tilt модель — `list` (как list-sessions/
  workspaceFolders) → per-project query (attach) → ОДИН агрегированный streaming
  endpoint (merged journal tail с since-cursor).
- **ANTI-РЕКОМЕНДАЦИИ:** НЕ делать демон authoritative; НЕ встраивать брокер
  (NATS/iggy — тяжело + opaque non-markdown store для single-machine MIT CLI, брать
  только семантику); НЕ mDNS; НЕ D-Bus fire-and-forget без журнала.

<a name="8-гипотезы"></a>
## 8. Четыре архитектурные гипотезы (состязательно оценены)

| # | Гипотеза | Топология | Балл | Survives | Effort |
|---|---|---|---|---|---|
| 3 | **SpaceJournal** — shared append-only NDJSON journal | shared-append-journal | **7** | ✅ | M |
| 4 | **ForgeMesh** — meta-MCP aggregator (`forgeplan-spaced`) | mcp-aggregator | 6.5 | ✅ | M |
| 1 | **forgeplan-hub** (`forgeplan serve`) — демон | centralized-hub-daemon | 6.5 | ✅ | L |
| 2 | **SpaceBus** — embedded broker over NDJSON outbox | message-broker | 5 | ❌ | L |

### 🏆 H3 SpaceJournal — 7/10 (победитель по чистоте)
- **Суть:** каждый space = папка append-only NDJSON событий на машине; проекты
  дописывают строку при каждой мутации; подписчики делают tail с курсором; **нет
  демонов и брокеров**.
- **Сильное:** максимально markdown-first (журнал derived, `space replay --from-scan`
  пересобирает как scan-import LanceDB); нулевой ops-footprint (нет демона/брокера/
  SPOF); **graceful degradation by construction** (журнал недоступен → emit no-op,
  проект само-управляется). **LanceDB single-writer = ложная тревога** (каждый проект
  пишет в СВОЙ LanceDB; общий ресурс — только append-журнал).
- **2 major (локальны, дёшево чинятся):**
  1. **lock-timeout vs dogfood:** «30s wait» недопустим для интерактивного CLI →
     заменить на **100–250 мс → on-timeout emit no-op** (артефакт всегда активируется
     мгновенно; потеря события под contention ок, replay дособерёт).
  2. **cross-platform:** macOS PIPE_BUF=512 → `lock+append` должен быть НОРМОЙ, а
     O_APPEND-fast-path — опцией только для строк ≤PIPE_BUF; Windows — нет stable
     inode → курсор `{segment, byte_offset, file_id}` через `GetFileInformationByHandle`
     за `#[cfg(windows)]`; per-line checksum для детекта усечённой строки при крэше.
- **best_use_case:** один владелец, одна локальная macOS/Linux машина, ~10–20
  проектов; нужен live-дашборд + cross-project reactive-агент; ценится отсутствие
  демона/SPOF и строгий dogfood. НЕ для: Windows-first, team-shared журнал на
  Dropbox/NFS, multi-tenant машины.

### H4 ForgeMesh MCP Aggregator — 6.5/10 (лучший first-step для read-плоскости)
- **Суть:** один meta-MCP `forgeplan-mcp-hub` (stdio) находит все проекты, переэкспонирует
  73 тула с обязательным `target={project|space}`, демультиплексит/фанаутит. На v1
  **без отдельного транспорта событий** (синтез через polling-diff state.yaml →
  эфемерные MCP-нотификации).
- **Сильное:** самый markdown-first и обратимый (удалил крейт + поле); цель №1
  («видеть всё и зайти в любой») достигается уже на v1 через `space_list` + per-project
  fan-out, без нового транспорта.
- **Major:** (1) самопротиворечие «один хаб на машину» vs «stdio спавнится клиентом»
  (stdio живёт с клиентом → N хабов, обнуляет warm-index payoff + воссоздаёт
  single-writer); (2) LanceDB single-writer; (3) события эфемерны (нет durable/
  backfill/replay) — не достигает «надёжно реагировать».
- **cheapest_fix:** (1) хаб открывает чужие LanceStore **READ-ONLY**, КАЖДУЮ мутацию
  роутит в свежеспавненный per-project процесс; (2) `target` опционален (default cwd);
  (3) durable NDJSON журнал + since-cursor для событий ← **это и есть мост к H3**.

### H1 forgeplan-hub (`forgeplan serve`) — 6.5/10 (Фаза 2, не основа)
- **Суть:** один долгоживущий демон на машину держит warm BGE-M3 + пул LanceStore,
  CLI/MCP — тонкие клиенты с откатом в файловый режим при падении.
- **Уникальный козырь:** единственная топология с *техническим* (не только
  продуктовым) аргументом — амортизация cold-start BGE-M3 (~150 MB на каждый короткий
  процесс).
- **Major:** второй durable не-markdown стор (журнал с offset'ами — история переходов,
  не пересобираемая из frontmatter); LanceDB single-writer (демон + N CLI + N MCP);
  **cross-platform подтверждён нерабочим** (macOS XDG_RUNTIME_DIR пуст → primary socket
  path мёртв; sun_path 104B; Windows без UDS); daemon lifecycle + version skew
  (brew-бинарь auto-upgrade → старый `serve` со старой схемой).
- **cheapest_fix:** расщепить на 2 фазы; демон строго опциональный ускоритель поверх
  журнала; жёсткие инварианты (демон НИКОГДА не пишет/реиндексит чужие LanceDB; emit
  fire-and-swallow; version-handshake CLI↔демон). → **= Фаза 2 поверх H3.**

### H2 SpaceBus (embedded broker) — 5/10 (не выживает)
- **Суть:** встроенный брокер (iggy/embedded-NATS) в `forgeplan hub`, проекты публикуют
  в outbox → брокер по subject `space.<id>.<project>.<event>`.
- **Почему не выживает:** правильная СЕМАНТИКА (subject-naming, offset-replay, durable
  fan-out) в неправильной ОБОЛОЧКЕ. Брокер — самый тяжёлый транспорт: второй
  непрозрачный не-git-friendly durable-стор + двойная запись (outbox+стрим) +
  supervision дочернего процесса. **Автор сам цитирует вывод «бери семантику брокера
  поверх NDJSON, а не шипи брокер» — и игнорирует.**
- **best_use_case:** когда single-machine перестанет быть single-machine (будущая
  «ForgePlan Teams/Cloud» фаза, сеть, контейнеры, remote-агенты).
- **cheapest_fix:** «SpaceBus Lite» = выкинуть брокер, оставить семантику над NDJSON →
  по сути превращается в H3.

<a name="9-рекомендация"></a>
## 9. Рекомендация: КОМПОЗИЦИЯ SpaceJournal + ForgeMesh

**Один продукт в двух слоях/фазах:**

- **SpaceJournal (H3) = durable transport + emit.** Эмит на единственном choke-point
  (3 метода `LanceStore` + `ClaimStore::{claim,release}` — покрывает CLI И MCP).
- **ForgeMesh (H4) = addressing/read plane.** Meta-MCP `forgeplan-mcp-hub` читает
  реестр, открывает любой проект по abs_path, проксирует 73 тула с `target={project|space}`.
- **JOIN (decision 2) и DISCOVERY (decision 1) у обеих ИДЕНТИЧНЫ** → декларативный
  `mesh.space_id` в committed `config.yaml` (`#[serde(default)]`) + cascade
  explicit > `registry.json` > walkdir. Поэтому они расходятся только по 2 осям из 4,
  по 2 совпадают дословно → идеально комбинируемы.

**Критический инвариант композиции** (снимает главный major обеих одним движением):
> Хаб открывает чужие `LanceStore` **строго READ-ONLY** (агрегированный вид/search/
> синтез нотификаций), а **КАЖДУЮ мутацию роутит в свежеспавненный короткоживущий
> per-project процесс** по `target.project`. Подписка хаба (`space_subscribe` с
> since-cursor) реализуется через **tail журнала SpaceJournal**, НЕ через polling-diff.

Это разом снимает: LanceDB single-writer, противоречие «один-на-машину vs stdio»,
эфемерность событий ForgeMesh, и сохраняет red-line «мутации только через legal
write-path».

- **forgeplan-hub (serve, H1)** — НЕ в v1; опциональная **Фаза 2** ускорителя
  (резидентный BGE-M3 для cross-project semantic search), включается ТОЛЬКО если
  измерения докажут боль cold-start.
- **SpaceBus (H2)** — отвергнут до перехода в multi-machine.

**Почему так (3 аргумента):** (1) каждая половина лечит major другой; (2) markdown-first
строго цел (журнал и хаб — derived, единственный не-markdown артефакт членства =
declarative `space_id` в committed config); (3) dogfood-инвариант защищён by
construction на обоих слоях (журнал пишет мутация, не демон; хаб мёртв → откат на
per-project MCP).

<a name="10-срез"></a>
## 10. Минимальный вертикальный срез + шаги прототипа

**Срез (доказать ценность без демона/брокера):** 1 space `gertsai-platform`, 2 реальных
проекта (`~/Work/GertsAi/shared` + второй локальный, напр. `~/Work/GertsHub`), 1 тип
события `artifact.activated`, 1 MCP-тул `space_subscribe`, ~60-строчный dashboard-tail
(SSE).

**Критерий успеха:** активируешь артефакт в A → карточка в дашборде в реальном времени
без перезапуска; закрыл дашборд, активировал в B, открыл с `last_offset` → видишь
пропущенное (**durable catch-up**); удалил `events/` → проекты живут, `space replay
--from-scan` восстанавливает (**dogfood**).

**НЕ входит:** хаб-демон, fan-out по space, кросс-проектные claims, semantic search,
hooks, второй тип события, Windows-паритет, реестр (на срезе — хардкод/walkdir).

**Шаги:**
0. (полчаса) Зафиксировать платформенные константы: `getconf PIPE_BUF /` на darwin,
   пустоту `XDG_RUNTIME_DIR`, валидный `.forgeplan` у проекта B.
1. Новый тонкий crate **`forgeplan-mesh`**: тип `SpaceEvent {v, ts, seq(ULID),
   space_id, project_id, artifact_id, artifact_type, kind, from_status, to_status,
   r_eff, md_path, abs_path, agent_id, actor}` + `emit(workspace, event)`. emit:
   резолвит `space_id` из Config (None → return, standalone); `fs2`-lock на
   `<space>/.events.lock` с таймаутом **100–250 мс** (timeout → no-op + log, НЕ
   блокировать); аппендит одну serde_json-строку + `\n`. `lock+append` как норма;
   O_APPEND без lock — опц. fast-path только ≤PIPE_BUF.
2. **Config:** под-структура `MeshConfig {space_id: Option<String>, enabled: bool}` как
   новое top-level Option `mesh` в `Config` (`config/types.rs`). Тест: legacy без
   `mesh` парсится (serde default) → standalone цел. gitignore `.forgeplan/events/`.
3. **Эмит choke-point:** вызвать `forgeplan_mesh::emit()` внутри
   `LanceStore::update_artifact` СРАЗУ ПОСЛЕ персиста frontmatter + reindex (последним
   шагом). Обёрнут в **fire-and-swallow** (EACCES/диск/lock-timeout → log + проглотить,
   мутация всегда успешна). На срезе хватит одной точки; create_artifact/update_body/
   ClaimStore — после.
4. **MCP read-by-space:** один тул `space_subscribe` в dispatch
   `forgeplan-mcp/src/server.rs`: открыть `events/*.ndjson`, seek к since-offset, отдать
   строки `kind∈filter` (реплей), затем tail через notify, стримить новые как MCP
   server-sent notifications. Курсор `{segment_file, byte_offset}`.
5. **Dashboard tail (~60 строк):** Node-сервис (chokidar на `events/*.ndjson`) + одна
   HTML-страница на SSE. Каждая NDJSON-строка → карточка `project · artifact ·
   activated · R_eff · ts`. Сознательно НЕ через MCP (доказываем: журнал = обычный файл).
6. **Доказать оба инварианта вручную:** (a) durable catch-up; (b) graceful degradation
   (`chmod 000` на `events/`, активировать, убедиться что прошло и CLI не завис; затем
   `space replay --from-scan` восстанавливает).

<a name="11-меню"></a>
## 11. Capability-меню «что ещё может уметь» (брейншторм для scope PRD)

**👁 Видеть (discovery/realtime):** список spaces и проектов; heartbeat «кто живой»;
live-дашборд spaces × projects.

**🚶 Ходить и читать (addressing — ядро «любой → любой»):** `space_query(target, tool,
args)` для любого read-тула в любом проекте; fan-out `space_query(space, "health")`;
**кросс-проектный semantic search** («где во всём продукте решали про rate-limiting?»).

**⚡ Реагировать (события — herdr-слой):** эмит на мутациях; `space_subscribe` (лента);
`space_wait` (блок до события); `space_on{match, run}` (хуки). Сценарии: контракт-ADR в
`shared` активировался → авто-NOTE в зависимые сервисы; R_eff сервиса < 0.5 → пинг;
stale где угодно → диспатч refresh-агента.

**🤝 Координировать (тоже herdr-like):** кросс-проектные claims (`EPIC-12@gertsai` виден
всем); **карта активности агентов** (кто где работает/заблокирован — rollup как у
Herdr); кросс-проектный dispatch (эпик на 3 сервиса → 3 агента без конфликтов).

**🧠 Продуктовый мозг (самое ценное «что ещё»):**
- **Space Context Pack** — при создании нового модуля авто-собрать общие ADR/контракты/
  конвенции/тех-стек продукта → новый сервис стартует консистентным by construction.
  (Прямой ответ на задачу «спроектировать новый модуль».)
- **Impact radius по продукту** — «deprecate ADR-005 в shared → что сломается в каких
  сервисах?»
- **Контроль консистентности / противоречий** — «сервис A решил X про auth, B — не-X →
  флаг» (`space_contradictions`).
- **Детект shared-kernel** — «3 сервиса независимо написали ADR про один retry-policy →
  поднять в space-level общее решение».
- **Кросс-проектные связи** — link RFC-в-A `implements` ADR-в-shared (сегодня связи
  только внутри проекта) → граф зависимостей через весь продукт.
- **Product-level health/timeline** — один R_eff/blindspots/лента решений на весь
  продукт; «какой сервис — слабое звено?»

**🖥 Фронт:** дашборд spaces×projects + live feed + agent-activity-map + продуктовый граф.

> **Архитектурный принцип (из BMAD):** не смешивать слой **адресации** (видеть/ходить/
> читать — нужно всем) со слоем **реакции** (события/хуки/координация — нужно агентам).
> Herdr силён в socket-слое реакции; ForgePlan — в слое артефактов; mesh = склейка.

<a name="12-вопросы"></a>
## 12. Открытые вопросы (решить в PRD/ADR)

1. **Retention журнала:** дневная сегментация решает рост, но кто компактит/удаляет
   старые сегменты? Политика (N дней / M событий)? Должен ли `space replay --from-scan`
   усекать журнал до текущего состояния?
2. **`space_id` = граница доверия:** любой проект, заявивший `space_id=X` в committed
   config, видит события всех членов X и пишет в общий журнал. Смешивать репо разного
   доверия — НЕЛЬЗЯ. Нужна ли опц. HMAC-привязка project→space из shared-секрета, или
   honor-system + git-review достаточно?
3. **RCE-вектор `space_on{run:...}`:** хуки исполняют команды. Достаточно ли opt-in
   `--allow-hooks` + hooks только в committed/review-only файле (не по сети)? Или хуки
   отложить за v1?
4. **Windows-паритет на v1 или нет?** Курсор `{file,offset,inode}` ломается на NTFS
   (нужен `GetFileInformationByHandle` file-id); Unix-сокет дашборда → named pipe. Это
   команда macOS/Linux (машина darwin) или нужен Windows из коробки?
5. **Team-shared `journal_root` (Dropbox/iCloud/NFS):** нужен ли? O_APPEND-атомарность и
   inode-курсоры там НЕ гарантированы. Если нет — зафиксировать «только локальная FS»
   как явное ограничение v1.
6. **Cross-project semantic search на v1?** Если да — warm-демон forgeplan-hub становится
   Фазой 1.5 (не отложенной Фазой 2), т.к. тащить резидентный BGE-M3 раньше.
7. **Кто исполняет реакцию агента:** ad-hoc процесс, тейлящий журнал, или опциональный
   foreground `forgeplan space watch`? Насколько «мгновенной» должна быть реакция?
8. **`project_id`:** из `project_name` (риск коллизий) или стабильный slug =
   `project_name + hash(abs_path)` при join?

<a name="13-дальше"></a>
## 13. Следующие шаги в проекте ForgePlan

1. **Оформить как forgeplan-артефакты** (через MCP/CLI, не прямой Edit):
   - **PRD** «ForgePlan Space-Mesh: cross-project visibility + events» (scope =
     capability-меню §11, требования §4).
   - **ADR** «Топология space-mesh» (решение = §9 композиция SpaceJournal + ForgeMesh,
     альтернативы = H1/H2 с баллами §8).
   - При желании — **EVID** на ресёрч (§7) и **SPEC** на минимальный срез (§10).
2. **Фазировать:**
   - **Фаза 1 (v1):** SpaceJournal-журнал + `mesh.space_id` + `space_subscribe` (poll/
     tail) + read-плоскость ForgeMesh (read-only хаб, мутации в per-project процесс).
     Минимальный срез §10 → расширение до fan-out/реестра/всех тулов.
   - **Фаза 1.5 (опц.):** warm-демон (BGE-M3) ТОЛЬКО если нужен cross-project semantic
     search / измерена боль cold-start.
   - **Фаза 2+:** hooks (`space_on`), кросс-проектные claims, agent-activity-map,
     продуктовый мозг (impact/contradictions/context-pack), фронт.
   - **Отложено до multi-machine:** SpaceBus/брокер.
3. **Прежде чем код:** Шаг 0 §10 (пин платформенных констант) + решить вопросы §12
   (минимум: Windows-паритет, retention, trust-boundary).

<a name="14-приложение"></a>
## 14. Приложение: где лежат сырые данные

- **Полный JSON-выхлоп ресёрча** (5 досье + 4 гипотезы с полными pros/cons/verdict +
  synth): `wemsii9v2.output` (246 KB).
  ⚠️ Лежал в `/private/tmp/claude-501/.../tasks/` — **macOS чистит `/tmp` при
  перезагрузке**. Если ещё доступен — скопировать рядом с этим файлом, иначе данные §7–§12
  здесь — дистиллят из него.
- **Сломавшаяся сессия с ресёрчем:** `~/.claude/projects/-Users-explosovebit-Work-
  GertsAi-shared/6eae00df-0d04-46fe-a365-ba75d45f0560.jsonl`.
- **Herdr:** `github.com/ogulcancelik/herdr`.
- **Изученные движки:** `bmad-code-org/BMAD-METHOD`, `github/spec-kit`,
  `Fission-AI/OpenSpec`.
