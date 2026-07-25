---
depth: standard
id: ADR-018
kind: adr
last_modified_at: 2026-07-25T08:29:15.879027+00:00
last_modified_by: claude-code/2.1.219
links:
- target: PRD-081
  relation: based_on
- target: ADR-003
  relation: based_on
status: draft
title: 'Топология space-mesh: композиция SpaceJournal + ForgeMesh'
---

## Context and Problem Statement

На машине живёт 21 проект с `.forgeplan/` под `~/Work`, часть из них — микросервисы
одного продукта (§2 хендофф-документа `~/Work/forgeplan-space-mesh-handoff.md`).
Сегодня forgeplan 0.32.1 → 0.33.0 закрывает каждый проект внутри своего cwd: `serve`
привязан к cwd без аргумента пути, у команд нет глобального `-C/--workspace`, и как
следствие `mcp__forgeplan__*` любого агента **заперт в своём проекте** — кросс-проектная
работа возможна только через Bash с физическим `cd` (§3).

Провал делится на два разных по цене слоя (§5):

- **Адресация/видимость.** «Прочитать артефакт соседа» и «спросить соседа `list --json`»
  работают уже сегодня через `cd` + `Read` — это не фича, это отсутствие эргономики.
  Отсутствуют: реестр проектов, группировка по смыслу (а не по каталогам), realtime-вид.
- **Реактивность.** «Проект B реагирует на изменение в проекте A» не существует вообще:
  нет событий, подписок, хуков, кросс-проектных claims. Это net-new (§3, §5).

Ресёрч пяти движков (BMAD-METHOD, GitHub Spec Kit, OpenSpec + prior-art survey по
tmux/Watchman/NATS/D-Bus/Nx/Turborepo/LSP) дал общий вывод: **ни один
spec-driven-инструмент экосистемы не имеет кросс-проектного слоя или событий** (§7) —
прецедента нет, стоимость надо обосновывать самим.

Жёсткое ограничение сверху — ADR-003: markdown-файлы остаются source of truth, LanceDB —
derived-индекс, пересобираемый `scan-import`. Что бы ни добавлял space-mesh, оно обязано
остаться **derived и пересобираемым**; появление второго authoritative не-markdown стора
сломало бы центральный инвариант продукта. Вопрос ADR: **какая топология даёт адресацию
и события, не ломая markdown-first и не вводя SPOF.**

## Decision Drivers

- **Markdown-first обязан выжить (ADR-003).** Любой новый стор — derived, с явной
  процедурой пересборки (аналог `scan-import`). Валидация извне: BMAD на ~48k★ работает
  zero-runtime поверх markdown (§7.1) — демон/шина допустимы только как ускоритель.
- **Zero-SPOF предпочтителен.** Топология, где смерть одного процесса останавливает работу
  во всех 21 проектах, неприемлема для single-owner-машины.
- **Dogfood: CLI никогда не блокируется на mesh.** `forgeplan activate` должен отрабатывать
  мгновенно даже если журнал недоступен, залочен или `chmod 000`. Это отсекает любой
  дизайн с синхронным ожиданием общего ресурса (§8, H3 major 1).
- **Реальность одной macOS-машины.** `XDG_RUNTIME_DIR` эмпирически **пуст** → «primary
  socket path» демонных вариантов мёртв; `sun_path` = 104 B; `PIPE_BUF` = **512 B** против
  4096 на Linux → атомарность голого `O_APPEND` не покрывает типичную строку события (§2, §8 H1/H3).
- **LanceDB — single-writer.** Любая топология, где демон и N коротких процессов пишут в
  один индекс, воспроизводит конфликт писателей (§8, major у H1 и H4).
- **Red-line «мутации только через legal write-path».** Прямой `Edit` артефактов запрещён
  правилами проекта; значит все мутации уже проходят через узкое горло — и это же горло
  надо переиспользовать для эмита, а не просить LLM «не забыть эмитнуть» (§7.2).
- **Обратимость.** Решение должно откатываться удалением крейта и одного поля конфига.

## Considered Options

Четыре гипотезы, оценённые состязательно (§8):

1. **H3 SpaceJournal** — shared append-only NDJSON-журнал на space. Балл **7/10**, survives, effort M.
2. **H4 ForgeMesh** — meta-MCP-агрегатор `forgeplan-mcp-hub`. Балл **6.5/10**, survives, effort M.
3. **H1 forgeplan-hub** — долгоживущий демон `forgeplan serve`. Балл **6.5/10**, survives, но как ускоритель.
4. **H2 SpaceBus** — встроенный брокер (iggy / embedded-NATS) поверх outbox. Балл **5/10**, **не выживает**.

## Decision Outcome

**Выбрана КОМПОЗИЦИЯ H3 + H4** — один продукт в двух слоях и двух фазах (§9):

- **SpaceJournal (H3) = durable transport + emit.** Эмит на единственном choke-point:
  `forgeplan_core::db::store::LanceStore::{create_artifact, update_artifact, update_body}`
  + `ClaimStore::{claim, release}`. Эти пять точек покрывают **CLI и MCP одновременно**;
  обёртка вокруг MCP-dispatch пропустила бы CLI-мутации (§7.4). Эмит стоит последним шагом,
  уже после персиста frontmatter и reindex, и обёрнут в **fire-and-swallow** — событие
  описывает состоявшуюся запись в markdown, а не заменяет её (§10, шаг 3).
- **ForgeMesh (H4) = addressing/read plane.** Meta-MCP читает реестр, открывает любой
  проект по abs_path и проксирует тулы с `target={project|space}`.
- **JOIN и DISCOVERY у обеих гипотез идентичны** (§9): декларативный `mesh.space_id` в
  committed `.forgeplan/config.yaml` (`Config` уже помечен `#[serde(default)]`, legacy без
  блока парсится как standalone) + cascade `explicit > registry.json > walkdir`. **НЕ mDNS**
  — single-machine, лишняя attack surface (§7.5, decision 1). Гипотезы расходятся только
  по 2 осям из 4, по 2 совпадают дословно → комбинируются без склейки.

**Критический инвариант композиции:**

> Хаб открывает чужие `LanceStore` **строго READ-ONLY** (агрегированный вид / search /
> синтез нотификаций), а **КАЖДУЮ мутацию роутит в свежеспавненный короткоживущий
> per-project процесс** по `target.project`. Подписка хаба (`space_subscribe` с
> since-cursor) реализуется через **tail журнала SpaceJournal**, НЕ через polling-diff.

Обоснование выбора композиции — три аргумента (§9):

1. **Каждая половина лечит major другой.** Главный major H4 — эфемерность событий
   (нет durable / backfill / replay) — закрывается журналом H3. Главный major H3 —
   отсутствие плоскости адресации («журнал есть, а зайти в проект нечем») — закрывается
   meta-MCP H4. Инвариант выше снимает разом ещё три: LanceDB single-writer, противоречие
   «один хаб на машину vs stdio спавнится клиентом», и сохранение red-line
   «мутации только через legal write-path».
2. **Markdown-first строго цел.** И журнал, и хаб — derived: журнал пересобирается
   `space replay --from-scan` ровно как LanceDB пересобирается `scan-import`, хаб не хранит
   ничего. **Единственный не-markdown артефакт членства — декларативный `space_id` в
   committed config**, то есть текст под git-review.
3. **Dogfood защищён by construction на обоих слоях.** Журнал пишет сама мутация, а не
   демон — нет процесса, чья смерть остановит запись. Хаб мёртв → откат на обычный
   per-project MCP, ноль деградации в исходном сценарии.

**Фазирование** (§13):

- **F1 (v1):** журнал + `mesh.space_id` + `space_subscribe` + read-only ForgeMesh.
  Минимальный вертикальный срез (§10): 1 space, 2 реальных проекта, 1 тип события
  `artifact.activated`, 1 MCP-тул `space_subscribe`, ~60-строчный dashboard-tail на SSE.
  Дашборд сознательно читает NDJSON напрямую, не через MCP — доказательство, что журнал
  остаётся обычным файлом.
- **F1.5 (опционально):** warm-демон с резидентным BGE-M3 — **только** если измерения
  докажут боль cold-start.
- **F2:** hooks (`space_on`), кросс-проектные claims, agent-activity-map, продуктовый мозг
  (impact radius / contradictions / context pack), фронт.

**Отвергнуто:** H1 как основа (только Фаза 1.5/2-ускоритель), H2 целиком до перехода
в multi-machine.

## Consequences

### Positive Consequences

- Цель №1 («видеть всё и зайти в любой проект своего space») достигается уже на v1 через
  `space_list_projects` + per-project fan-out, **без нового транспорта** (§8, H4).
- **Durable catch-up:** подписчик, отключившийся на сутки, догоняет пропущенное по
  since-cursor. Это чинит известную дыру D-Bus «события теряются при дисконнекте» (§7.5).
- **Zero ops-footprint:** нет демона, нет брокера, нет SPOF; `LanceDB single-writer`
  оказывается ложной тревогой — каждый проект пишет в СВОЙ индекс, общий ресурс только
  append-журнал (§8, H3).
- **Graceful degradation by construction:** журнал недоступен → `emit` no-op, проект
  продолжает жить самостоятельно.
- **Обратимость:** откат = удалить крейт `forgeplan-mesh` и поле `mesh` из `Config`;
  артефакты и markdown не затронуты.
- Существующий `forgeplan claim --ttl` расширяется до space-scope вместо изобретения
  lock-сервера (§7.2, §7.5).

### Negative Consequences

- **Второй derived-стор требует политики retention.** Журнал растёт монотонно; сегментация
  решает размер файла, но не отвечает, кто компактит и удаляет старые сегменты (§12, вопрос 1).
- **`space_id` становится границей доверия.** Любой репозиторий, объявивший `space_id=X` в
  committed config, читает события всех членов X и пишет в общий журнал. Смешивать репо
  разного уровня доверия в одном space нельзя — и на v1 это ограничение держится дисциплиной,
  а не механикой (§12, вопрос 2).
- **Windows-паритет не решён.** Курсор `{file, offset, inode}` ломается на NTFS (нет
  стабильного inode, нужен `GetFileInformationByHandle`); Unix-сокет дашборда → named pipe (§12, вопрос 4).
- **Эмит best-effort.** При lock-таймауте (100–250 мс) событие теряется; точность
  восстанавливается только `space replay --from-scan`. То есть журнал даёт «мгновенно —
  почти всегда, точно — после replay», и это надо честно писать в документации.
- **Мутация через хаб дороже прямой.** Каждая write-операция = спавн отдельного процесса
  (плата за инвариант read-only); для батч-сценариев это заметно.
- **Две поверхности вместо одной.** Крейт `forgeplan-mesh` и space-тулы MCP развиваются
  синхронно с 73 существующими тулами — растёт цена изменения контракта.

## Pros and Cons of the Options

### H3 SpaceJournal — 7/10, победитель по чистоте

Каждый space = папка append-only NDJSON-событий на машине (`.forgeplan/events/*.ndjson`,
gitignored, sibling к `lance/`, `claims/`, `state/`, `.lock`; зеркало в
`~/.local/share/forgeplan/spaces/<space_id>/`). Проекты дописывают строку при каждой
мутации, подписчики делают tail с курсором. Демонов и брокеров нет; `notify` /
`notify-debouncer-mini` уже в зависимостях (§7.4).

**Плюсы.** Максимально markdown-first: журнал derived, `space replay --from-scan`
пересобирает его так же, как `scan-import` пересобирает LanceDB. Нулевой ops-footprint,
graceful degradation by construction, `LanceDB single-writer` не применим.

**Major 1 — lock-timeout vs dogfood.** Исходные «30 s wait» недопустимы для интерактивного
CLI. **Дешёвый фикс:** таймаут **100–250 мс**, по истечении — **emit no-op + log**.
Артефакт активируется мгновенно всегда; потеря события под contention приемлема, replay дособерёт.

**Major 2 — cross-platform.** macOS `PIPE_BUF` = 512 B, то есть типичная строка `SpaceEvent`
не влезает в атомарный write. **Дешёвый фикс:** `lock + append` — **норма**, а
`O_APPEND`-fast-path без блокировки — опция только для строк ≤ `PIPE_BUF`. Курсор с самого
начала имеет форму `{segment, byte_offset, file_id}`, где `file_id` под `#[cfg(windows)]`
берётся из `GetFileInformationByHandle`; per-line checksum детектит усечённую строку после крэша.

**best_use_case:** один владелец, одна локальная macOS/Linux-машина, ~10–20 проектов, нужен
live-дашборд и кросс-проектный реактивный агент. **НЕ для:** Windows-first, team-shared
журнал на Dropbox/NFS, multi-tenant машина.

### H4 ForgeMesh (meta-MCP aggregator) — 6.5/10, лучший first-step для read-плоскости

Один meta-MCP `forgeplan-mcp-hub` (stdio) находит все проекты и переэкспонирует 73 тула с
`target={project|space}`, демультиплексируя и фанауча вызовы. На v1 без отдельного
транспорта событий — синтез через polling-diff `state.yaml` и эфемерные MCP-нотификации.

**Плюсы.** Самый обратимый вариант (удалил крейт и поле — всё как было) и единственный,
который закрывает цель №1 без нового транспорта.

**Major 1 — «один хаб на машину» vs stdio.** stdio-сервер спавнится клиентом и живёт с ним →
получается N хабов, что обнуляет warm-index payoff и воссоздаёт конфликт писателей.
**Major 2 — LanceDB single-writer.** **Major 3 — эфемерность событий:** нет durable-хранения,
backfill и replay, то есть требование «надёжно реагировать» не выполняется.

**Cheapest fix — и он же мост к H3:** (1) хаб открывает чужие `LanceStore` READ-ONLY, каждую
мутацию роутит в свежеспавненный per-project процесс; (2) `target` опционален с default = cwd;
(3) durable NDJSON-журнал + since-cursor вместо эфемерных нотификаций. Пункт (3) — буквально H3.
Именно это совпадение и делает композицию, а не выбор «или-или», правильным ответом.

### H1 forgeplan-hub (`forgeplan serve`) — 6.5/10, Фаза 2, не основа

Один долгоживущий демон на машину держит warm BGE-M3 и пул `LanceStore`; CLI/MCP — тонкие
клиенты с откатом в файловый режим.

**Уникальный технический козырь:** единственная топология с *техническим*, а не продуктовым
аргументом — амортизация cold-start резидентного **BGE-M3 (~150 MB)**, который иначе грузится
в каждый короткоживущий процесс (§7.4, §8).

**Почему всё-таки не основа.** Второй durable **не-markdown** стор: журнал с offset'ами — это
история переходов, не пересобираемая из frontmatter, то есть прямое нарушение духа ADR-003.
LanceDB single-writer (демон + N CLI + N MCP). Cross-platform подтверждён нерабочим: на этой
машине `XDG_RUNTIME_DIR` пуст → primary socket path мёртв, `sun_path` = 104 B, на Windows нет
UDS (§2). Плюс daemon lifecycle и version skew: brew-бинарь auto-upgrade'ится, а старый `serve`
остаётся со старой схемой. **Cheapest fix — расщепить на две фазы**: демон строго опциональный
ускоритель поверх журнала, с жёсткими инвариантами (демон НИКОГДА не пишет и не реиндексит
чужие LanceDB; emit fire-and-swallow; version-handshake CLI↔демон). Ровно это и записано как F1.5/F2.

### H2 SpaceBus (embedded broker) — 5/10, не выживает

Встроенный брокер (iggy / embedded-NATS) внутри `forgeplan hub`; проекты публикуют в outbox,
брокер раздаёт по subject `space.<id>.<project>.<event>`.

**Правильная семантика в неправильной оболочке.** Subject-naming, offset-replay и durable
fan-out — действительно то, что нужно, и они целиком переносимы. Но брокер как транспорт —
самый тяжёлый вариант: второй непрозрачный, не git-friendly durable-стор, двойная запись
(outbox + стрим) и supervision дочернего процесса — для single-machine MIT CLI это несоразмерно.
Показательно, что сам ресёрч по prior-art пришёл к выводу **«бери семантику брокера поверх
NDJSON, а не шипи брокер»** (§7.5, анти-рекомендации) — а гипотеза H2 этот собственный вывод
проигнорировала. **Cheapest fix** — «SpaceBus Lite»: выкинуть брокер, оставить семантику над
NDJSON, — превращает H2 в H3. **best_use_case:** когда single-machine перестанет быть
single-machine (сеть, контейнеры, remote-агенты, «ForgePlan Teams/Cloud»); до тех пор отложено.

## Invariants

Нарушение любого из четырёх аннулирует решение — их обязаны проверять код-ревью и тесты,
а не дисциплина автора.

- **INV-001 — read-only хаб.** ForgeMesh открывает чужие `LanceStore` строго на чтение.
  Каждая мутация роутится в свежеспавненный короткоживущий per-project процесс по
  `target.project`. Ни один долгоживущий процесс не пишет в чужой индекс никогда.
- **INV-002 — fire-and-swallow эмит.** `EACCES`, `ENOSPC`, отсутствующий каталог, таймаут
  lock, ошибка сериализации → `warn!` и продолжение. Мутация артефакта возвращает `Ok`
  во всех случаях, где вернула бы его без mesh. `?` внутри блока эмита запрещён.
- **INV-003 — журнал derived.** Журнал и зеркало space пересобираемы из markdown
  (`space replay --from-scan`), как LanceDB пересобирается `scan-import`. Единственный
  не-markdown артефакт членства — `mesh.space_id` в committed `config.yaml`. ADR-003 цел.
- **INV-004 — graceful degradation.** Хаб мёртв → откат на обычный per-project MCP.
  Журнал недоступен → эмит no-op, проект работает standalone. Ни одна из 76 CLI-команд
  не приобретает зависимости от наличия space.

## Rollback Plan

Решение спроектировано обратимым в один шаг, и это было явным decision driver.

1. **Полный откат кода:** удалить крейт `crates/forgeplan-mesh`, снять его из
   `members` корневого `Cargo.toml`, удалить поле `mesh` из `Config`
   (`config/types.rs`) и вызов эмита из `LanceStore`. Артефакты, markdown и LanceDB
   не затронуты — эмит по построению ничего не пишет в них.
2. **Откат данных:** `rm -rf .forgeplan/events` в проектах и
   `rm -rf ~/.local/share/forgeplan/spaces/<space_id>` на машине. Журнал derived,
   потеря невосстановимых данных исключена.
3. **Частичный откат без изменения кода (kill-switch):** `mesh.enabled: false` в
   `.forgeplan/config.yaml` — эмит выключается, проект возвращается к standalone-поведению
   без пересборки бинаря. Это же — способ снять baseline для замера накладных расходов.
4. **Триггеры отката.** (а) замеренный оверхед мутации выходит за коридор
   «baseline + 250 мс»; (б) обнаружена запись в чужой `LanceStore` из долгоживущего
   процесса (нарушение INV-001); (в) активация артефакта хоть раз заблокировалась из-за
   недоступного журнала (нарушение INV-002/INV-004).

## Affected Files

Точки, которые решение затрагивает (адреса из §7.4 handoff, подтверждены чтением кода
при написании SPEC):

- **Новое:** `crates/forgeplan-mesh/` — `SpaceEvent`, `Cursor`, `MeshSinkPaths`, `emit()`.
- `Cargo.toml` (корень) — `members`, `[workspace.dependencies]`.
- `crates/forgeplan-core/src/config/types.rs` — `MeshConfig` + top-level `mesh: Option<MeshConfig>`.
- `crates/forgeplan-core/src/db/store.rs` — поле `mesh` в `struct LanceStore`, резолв в
  `LanceStore::open`, снимок «до», вызов эмита в `update_artifact` (в Фазе 1 — одна точка;
  в F1 полностью — плюс `create_artifact` и `update_body`).
- `crates/forgeplan-core/src/claim/` — `ClaimStore::{claim, release}` (F1, вторая половина
  choke-point).
- `crates/forgeplan-core/src/workspace/init.rs` — self-registration в `registry.json` (F1).
- `crates/forgeplan-mcp/src/server.rs` + `types.rs` — регистрация space-тулов в JSON-RPC
  dispatch. Счётчик MCP-тулов в документации обязан двигаться синхронно: drift-детектор
  `scripts/check-mcp-tool-count.sh` заблокирует PR иначе.
- `.gitignore` — строка `.forgeplan/events/`.
- `dev/space-mesh-dashboard/` — Node + SSE, вне cargo-workspace, в релизный бинарь не входит.

## Open Questions

Шесть вопросов из §12, каждый — с решением, а не с формулировкой.

**1. Retention журнала.** Опции: (a) без ограничений; (b) дневная сегментация + удаление
сегментов старше N дней; (c) компакция `replay --from-scan` поверх существующих файлов.
**Рекомендация: (b)** — дневные сегменты `events/YYYY-MM-DD.ndjson`, hard-retention 30 дней,
а `space replay --from-scan` пишет **новый** snapshot-сегмент и никогда не усекает старые.
*Почему:* журнал derived, терять хвост истории безопасно; но усечение внутри уже существующего
сегмента ломает `byte_offset` у всех живых подписчиков.

**2. `space_id` как граница доверия.** Опции: HMAC-привязка project→space из shared-секрета
против honor-system + git-review. **Рекомендация: honor-system + git-review на v1**, HMAC
отложить вместе с multi-machine; в docs — явное «не смешивать в одном space репозитории
разного уровня доверия». *Почему:* `mesh.space_id` — committed-поле, проходящее тот же ревью,
что и код, а раздача shared-секрета — задача ровно того multi-machine-мира, за границей которого
уже отложен H2.

**3. RCE-вектор `space_on{run: ...}`.** Опции: opt-in `--allow-hooks` + hooks только из
committed-файла; либо отложить хуки за v1. **Рекомендация: отложить целиком за v1** (F2), а при
внедрении — обязательно оба ограничения сразу: `--allow-hooks` по умолчанию выключен и `run:`
читается **только** из committed `.forgeplan/config.yaml`, никогда из журнала или из сети.
*Почему:* событие, пришедшее из чужого репозитория, — untrusted input; исполнять по нему команду
до того, как решён вопрос 2, значит превратить honor-system-границу в удалённое выполнение кода.

**4. Windows-паритет курсора.** Опции: паритет на v1 против «unix-only v1, форма курсора
зарезервирована». **Рекомендация: v1 = macOS/Linux**, но курсор с первого коммита имеет форму
`{segment, byte_offset, file_id}`, где `file_id` — `Option`, на Unix заполняемый из inode.
*Почему:* машина разработки — darwin (§2), но форма курсора входит в wire-контракт подписки, и
менять её после появления подписчиков дороже, чем зарезервировать поле сейчас.

**5. Team-shared `journal_root` на Dropbox / iCloud / NFS.** Опции: сделать `journal_root`
конфигурируемым против явного ограничения. **Рекомендация: не поддерживать на v1**, `journal_root`
не выносить в конфиг, записать «только локальная FS» отдельной строкой в ограничениях.
*Почему:* ни атомарность `O_APPEND`, ни стабильность inode-курсоров на синхронизируемых и сетевых
ФС не гарантированы (§8, «НЕ для» у H3), а тихая потеря строк убивает durable catch-up — то есть
ровно тот критерий успеха, ради которого строится срез (§10).

**6. Cross-project semantic search на v1.** Опции: включить (и тогда F1.5-демон становится
частью F1) против отложить. **Рекомендация: не включать в v1.** *Почему:* включение тянет
резидентный BGE-M3 (~150 MB) и форсирует warm-демон **до того**, как измерена боль cold-start —
а read-плоскость ForgeMesh и без эмбеддингов даёт кросс-проектные `list` / `get` / `health` /
`graph`, чего достаточно, чтобы боль стало чем измерять.

Не закрыто и осознанно оставлено на SPEC: форма `project_id` (`project_name` с риском коллизий
против стабильного slug `project_name + hash(abs_path)`, §12 вопрос 8) и исполнитель реакции
агента — ad-hoc процесс-тейлер против foreground `forgeplan space watch` (§12 вопрос 7).

## Related

- **PRD-081 «ForgePlan Space-Mesh: кросс-проектная видимость и события»** — родительский
  артефакт; scope = capability-меню §11, требования §4. Этот ADR фиксирует топологию для него.
- **SPEC-006 «Минимальный вертикальный срез space-mesh (Фаза 1)»** — контракт `SpaceEvent`,
  формат курсора, `MeshConfig`, поведение `emit` (§10).
- **ADR-003 «Markdown files as source of truth, LanceDB as index layer»** — инвариант,
  который данное решение защищает: и журнал, и хаб остаются derived и пересобираемыми.
- **PRD-078 + ADR-015/ADR-016 (worktree-aware MCP routing, WorkspaceResolver)** — ближайший
  прецедент: опциональный `workspace`-параметр на store-resolution тулах, strict write-gate /
  soft read. `target={project|space}` у ForgeMesh — то же разрешение адреса, расширенное за
  границу одного репозитория, и должно переиспользовать `WorkspaceResolver`, а не строить второй.



