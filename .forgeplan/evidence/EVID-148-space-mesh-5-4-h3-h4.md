---
depth: standard
id: EVID-148
kind: evidence
last_modified_at: 2026-07-25T08:35:53.649889+00:00
last_modified_by: claude-code/2.1.219
links:
- target: PRD-081
  relation: informs
- target: ADR-018
  relation: informs
status: draft
title: 'Ресёрч space-mesh: 5 досье движков + 4 гипотезы, композиция H3+H4 выбрана'
---

Ресёрч кросс-проектного слоя ForgePlan («space-mesh»): изучены 5 движков
spec-driven / agile-AI инструментов и prior-art по транспортам событий, после чего
состязательно оценены 4 архитектурные гипотезы топологии. Результат — выбрана
композиция **SpaceJournal (H3, append-only NDJSON журнал = durable transport) +
ForgeMesh (H4, meta-MCP aggregator = addressing/read plane)** с критическим
инвариантом «хаб читает чужие `LanceStore` строго READ-ONLY, каждую мутацию роутит
в свежеспавненный per-project процесс».

Полный контекст, из которого сделан этот дистиллят:
`/Users/explosovebit/Work/forgeplan-space-mesh-handoff.md` (512 строк, RU).

## Structured Fields

verdict: supports
congruence_level: 2
evidence_type: audit

**Почему CL2, а не CL3.** Ресёрч выполнен в другом workspace —
`~/Work/GertsAi/shared`, сессия `6eae00df`, 28–29 мая 2026, динамический workflow
на 20 агентов — а применяется к кодовой базе ForgePlan. Домен тот же (артефактный
слой forgeplan, его же `LanceStore` / `Config` / MCP-dispatch, та же машина и те же
21 проект с `.forgeplan/`), но контекст исполнения другой: ни один вывод не был
получен внутри репозитория ForgePlan и не проверялся его тестами. Это ровно
определение CL2 — same domain, different context.

## Что исследовано

**5 досье по движкам** (§7 handoff) — как у каждого устроены engine, state,
coordination, events, multi-project:

1. **BMAD-METHOD** (`bmad-code-org/BMAD-METHOD`, ~48k★, V6 «native Skills») —
   §7.1. Нет runtime/демона: «движок» = host-LLM, читающий markdown; код только в
   build-time CLI (`npx bmad-method install` выходит после установки). State —
   YAML frontmatter + `config.yaml` per-module + `module-help.csv` с
   `preceded-by/followed-by` как декларативный sequencer. Координация
   document-mediated, конфликты разруливаются **прозой**
   (`preventing-agent-conflicts.md`), без claim/lease/mutex. Событий нет вообще.
2. **GitHub Spec Kit** (`github/spec-kit`, CLI `specify` v0.0.93, ~43k★,
   Python/Typer) — §7.2. Scaffolder/engine split, `--json` как детерминированный
   контракт вывода, discovery cascade «explicit override > git > filesystem scan».
3. **OpenSpec** (`Fission-AI/OpenSpec`, npm `@fission-ai/openspec`, TS/Node,
   ~51k★, MIT) — §7.3. Registration-based discovery
   (`openspec`-эквивалент `workspace link <path>` → machine-local registry в
   `$XDG_DATA_HOME`), JOIN через малый committed config (`workspace.yaml` с
   `links:`), delta-folder (ADDED/MODIFIED/REMOVED) как git-diffable форма
   cross-project SHAPE, «directory/file location = state» как дешёвый источник
   событий, mesh строго opt-in и view-only.
4. **ForgePlan-internals** (§7.4) — точечная разведка по собственному коду: где
   физически находится choke-point мутаций (`forgeplan_core::db::store::LanceStore::{create_artifact,
   update_artifact, update_body}` + `ClaimStore::{claim,release}`), куда встаёт JOIN
   (`mesh: Option<MeshConfig>` в `Config`, `config/types.rs`; `Config` уже помечен
   `#[serde(default)]`, self-registration — в `workspace/init.rs`), где живёт
   addressing (`forgeplan-mcp/src/server.rs`, JSON-RPC dispatch), какие deps уже
   есть (`notify` / `notify-debouncer-mini`, `walkdir`, `fs2`) и какой примитив
   координации уже готов к расширению (`forgeplan claim --ttl`: atomic
   temp+rename, hardened id/agent validation, низкоуровневый `fs2`-lock
   `.forgeplan/.lock` в `workspace/lock.rs` с bounded wait, backoff, symlink guards
   и OS-release-on-crash).
5. **prior-art survey** (§7.5) — tmux, Watchman, NATS, D-Bus, Bazel, Turborepo,
   Nx, LSP, Tilt, Mercurial: откуда брать transport, event model, hooks,
   addressing, JOIN, discovery, coordination и «see all + walk into» UX.

**4 архитектурные гипотезы** (§8), оценённые состязательно, с полными
pros/cons/verdict/effort/best_use_case по каждой.

## Ключевой вывод

**Ни у одного из изученных инструментов нет кросс-проектного слоя или событий.**
BMAD — строго single-repo, событий нет ни в каком виде (§7.1). Spec Kit — тоже
per-repo, «события» существуют только как `--json` вывод команды. OpenSpec ближе
всех — у него есть machine-local registry и links, но он даёт только discovery,
и явно НЕ даёт transport, addressing-by-event и coordination; более того, у самого
OpenSpec cross-runtime path-translation (XDG / Windows / WSL) остался нерешённым
(§7.3, негативный урок).

Практическое следствие для ForgePlan: space-mesh — **net-new для всей экосистемы
spec-driven инструментов**, а не догоняющая фича. Значит его нельзя обосновать
аналогией «у конкурента это есть» — обосновывать надо стоимостью: что именно
покупаем и за сколько. Handoff §5 показывает, что «читать соседа» и «спросить
соседа» уже работают за $0 (`Read <path>/.forgeplan/adrs/ADR-*.md`,
`cd B && forgeplan list --json`), а net-new и дорогое — ровно две вещи: реестр /
realtime / группировка (ergonomics) и события (reactivity).

## Уроки, определившие решение

- **BMAD → markdown-first с zero-runtime валидирован на 48k★.** Инструмент без
  единой строки runtime-кода держит производственный workflow. Отсюда прямой
  вывод для нас: журнал и хаб обязаны быть **derived**, а демон — только
  опциональный ускоритель, никогда не source of truth. Это же убило H1 как основу
  (§7.1 → §9).
- **Spec Kit → события как byproduct детерминированной мутации.** У Spec Kit
  `--json` — не «попросили модель напечатать JSON», а гарантированный выход
  детерминированного кода. Перенос: red-line ForgePlan уже запрещает прямой Edit
  артефактов, все мутации идут через CLI/MCP → 3 метода `LanceStore` +
  `ClaimStore::{claim,release}` образуют **естественный choke-point**, покрывающий
  CLI и MCP одновременно. Обёртка вокруг MCP-dispatch пропустила бы CLI-мутации
  (§7.2, §7.4). Формулировка «НЕ просить LLM не забыть эмитнуть» — дословно из
  этого досье.
- **OpenSpec → registration-based discovery копируем дословно**, плюс его
  негативный урок: сам OpenSpec не решил cross-runtime path-translation, поэтому
  реестр берём, а вопрос путей/курсоров на Windows решаем отдельно (или явно
  выносим за v1 — открытый вопрос §12.4).
- **prior-art → Watchman `subscribe(scope, filter, since=offset)`** — реплей
  после курсора, затем live; плюс **per-subscriber курсоры персистятся**. Именно
  это чинит дыру D-Bus «события теряются при дисконнекте» (§7.5). Отсюда
  `space_subscribe` с since-cursor и критерий успеха среза «закрыл дашборд,
  активировал в B, открыл с `last_offset` → видишь пропущенное» (§10).
- **prior-art → анти-рекомендации**, которые прямо отсекли варианты: не делать
  демон authoritative; не встраивать брокер (взять только семантику);
  не mDNS (single-machine, лишняя attack surface); не D-Bus fire-and-forget без
  журнала.
- **ForgePlan-internals → две платформенные константы, которые переформулировали
  дизайн.** На darwin `XDG_RUNTIME_DIR` **пуст** — это убивает «primary socket
  path» демонных вариантов (главный подтверждённый major H1). `PIPE_BUF` на macOS
  = **512 B** против 4096 на Linux — поэтому `lock+append` становится нормой, а
  O_APPEND-fast-path — опцией только для строк ≤PIPE_BUF (§2, §8-H3).

## Оценка гипотез

| # | Гипотеза | Топология | Балл | Выживает | Effort |
|---|---|---|---|---|---|
| H3 | **SpaceJournal** — shared append-only NDJSON journal | shared-append-journal | **7** | ✅ | M |
| H4 | **ForgeMesh** — meta-MCP aggregator (`forgeplan-spaced`) | mcp-aggregator | 6.5 | ✅ | M |
| H1 | **forgeplan-hub** (`forgeplan serve`) — демон | centralized-hub-daemon | 6.5 | ✅ как Фаза 2 | L |
| H2 | **SpaceBus** — embedded broker over NDJSON outbox | message-broker | 5 | ❌ | L |

- **H3 (7) выживает** — победитель по чистоте: нулевой ops-footprint (нет демона,
  брокера, SPOF), graceful degradation by construction (журнал недоступен → emit
  no-op, проект само-управляется), журнал derived и пересобираем
  (`space replay --from-scan` по образцу `scan-import` для LanceDB). Страх
  «LanceDB single-writer» здесь оказался ложной тревогой: каждый проект пишет в
  свой LanceDB, общий ресурс — только append-журнал. Оба его major локальны и
  дёшевы: lock-timeout **100–250 мс → on-timeout emit no-op** вместо
  недопустимых для интерактивного CLI «30s wait», и cross-platform (PIPE_BUF 512,
  курсор `{segment, byte_offset, file_id}` под `#[cfg(windows)]`, per-line
  checksum против усечённой строки при крэше).
- **H4 (6.5) выживает** как read-плоскость: цель №1 «видеть всё и зайти в любой»
  достигается уже на v1 через `space_list` + per-project fan-out, без нового
  транспорта; топология самая обратимая (удалил крейт + поле конфига). Её три
  major — самопротиворечие «один хаб на машину» vs «stdio спавнится клиентом»,
  LanceDB single-writer и эфемерность событий (polling-diff, нет
  durable/backfill/replay) — снимаются read-only-инвариантом и durable-журналом,
  то есть **мостом к H3**.
- **H1 (6.5) выживает, но не как основа.** Единственный её козырь —
  *технический*, а не продуктовый: амортизация cold-start резидентного BGE-M3
  (~150 МБ на каждый короткий процесс). Против неё: второй durable
  не-markdown стор; LanceDB single-writer при «демон + N CLI + N MCP»;
  cross-platform **подтверждён нерабочим** (пустой `XDG_RUNTIME_DIR` на darwin,
  `sun_path` 104 B, Windows без UDS); version skew — brew-бинарь
  (`/opt/homebrew/bin`, 0.32.1 на момент ресёрча, 0.33.0 сейчас)
  auto-upgrade'ится, а старый `serve` остаётся со старой схемой. Итог: строго
  опциональный ускоритель поверх журнала = Фаза 1.5/2.
- **H2 (5) не выживает.** Правильная семантика (subject-naming, offset-replay,
  durable fan-out) в неправильной оболочке: брокер — самый тяжёлый транспорт,
  второй непрозрачный не-git-friendly durable-стор, двойная запись
  (outbox + стрим), supervision дочернего процесса. Её собственный
  `cheapest_fix` — «выкинуть брокер, оставить семантику над NDJSON» — буквально
  превращает H2 в H3. Отложена до перехода в multi-machine (сеть, контейнеры,
  remote-агенты).

## Ограничения доказательства

Честно о том, чем это НЕ является:

- Это **ресёрч и состязательная оценка, а не измерения на работающем коде**.
  Тип доказательства — audit, не measurement / benchmark.
- **Ни одна гипотеза не прототипирована.** Баллы 7 / 6.5 / 6.5 / 5 — экспертные
  оценки по фиксированному набору осей, а не результат A/B на реализациях.
  Минимальный вертикальный срез (§10: 1 space, 2 проекта, 1 тип события
  `artifact.activated`, 1 MCP-тул `space_subscribe`, ~60-строчный dashboard-tail)
  описан, но не выполнен.
- **Cold-start BGE-M3 назван как аргумент, но НЕ измерен.** Цифра ~150 МБ — это
  размер модели, а не замеренная задержка старта короткого процесса. Именно
  поэтому Фаза 1.5 (warm-демон) в рекомендации оставлена **условной**: включается
  только если измерения докажут боль (§9, §12.6).
- Платформенные константы (`PIPE_BUF` = 512 на darwin, пустой
  `XDG_RUNTIME_DIR`, 21 проект с `.forgeplan/` под `~/Work`) получены пробингом
  машины, но Шаг 0 §10 — формальный их пин перед кодом — тоже ещё не выполнен.
- **Всё, что в handoff §7–§12, — дистиллят, а не первоисточник.** Сам первоисточник
  уцелел: сырой JSON-выхлоп ресёрча (246 774 байта, `agentCount: 20`) сохранён в
  репозитории как `docs/space-mesh/forgeplan-space-mesh-research-raw.json` — то есть
  риск «данные в `/tmp`, macOS чистит его при перезагрузке» снят. Практическое
  следствие: полные pros/cons/verdict по каждой из 4 гипотез и полные 5 досье
  доступны для перепроверки, и ошибку дистилляции можно поймать сверкой с сырым
  файлом. Ограничение остаётся другое — дистиллят не проверялся построчно против
  первоисточника при написании этого EVID.
- 8 вопросов остаются открытыми и должны быть закрыты в PRD/ADR (§12): retention
  журнала, `space_id` как граница доверия (HMAC vs honor-system + git-review),
  RCE-вектор `space_on{run:...}`, Windows-паритет, team-shared `journal_root`
  на Dropbox/NFS, cross-project semantic search на v1, исполнитель реакции,
  форма `project_id`.

## Provenance

- **Сессия ресёрча:** `6eae00df` (`~/.claude/projects/-Users-explosovebit-Work-GertsAi-shared/6eae00df-0d04-46fe-a365-ba75d45f0560.jsonl`),
  workspace `~/Work/GertsAi/shared`, 28.05.2026 — динамический workflow на
  **20 агентов**. Сессия упала на API-ошибке про `thinking` blocks, но выходной
  артефакт workflow уцелел (246 КБ).
- **Сырой выхлоп (первоисточник):** `docs/space-mesh/forgeplan-space-mesh-research-raw.json`
  — 246 774 байта, поля `summary` / `agentCount: 20` / `logs` / досье / гипотезы.
  Лежит в репозитории, а не в `/tmp`, поэтому переживает перезагрузку машины.
- **Дозбор** (что forgeplan 0.32.1 и Herdr умеют сегодня) + capability-брейншторм
  — 29.05.2026, там же.
- **Дистилляция в handoff** — `/Users/explosovebit/Work/forgeplan-space-mesh-handoff.md`,
  512 строк (копия в репозитории: `docs/space-mesh/forgeplan-space-mesh-handoff.md`);
  §0 provenance, §7 досье, §8 гипотезы, §14 сырые данные.
- **Референс по socket-слою:** Herdr (`github.com/ogulcancelik/herdr`) —
  локальный бинарь, не сервис; заимствуются три идеи (socket + NDJSON +
  блокирующий `events.wait`), а не интеграция.
- **Изученные репозитории:** `bmad-code-org/BMAD-METHOD`, `github/spec-kit`,
  `Fission-AI/OpenSpec`.
- **Целевая кодовая база на момент применения:** ForgePlan, ветка
  `shape/space-mesh` от `origin/dev`, версия 0.33.0 (ресёрч делался на 0.32.1 —
  ни один из затронутых choke-point'ов между версиями не менялся, но это
  проверяется чтением кода перед реализацией, а не считается доказанным здесь).
