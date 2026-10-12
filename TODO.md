# TODO — Forgeplan

> **История релизов**: `CHANGELOG.md` — единственный источник правды по тому, что
> уже вышло. Этот файл больше не дублирует релизный лог (дублировал до
> 2026-09-08 и не обновлялся два релиза подряд, пока CHANGELOG.md уходил вперёд).
> **Roadmap**: [`docs/ROADMAP.md`](docs/ROADMAP.md) — gap-анализ по категориям, обновлён 2026-09-08.
> **Текущая версия**: не верь заголовку этого файла — смотри
> `Cargo.toml [workspace.package].version` + `git tag --sort=-v:refname | head`.
> На 2026-09-08 это v0.37.0.

Этот файл — список приоритетов: что делать дальше, а не что уже сделано.
Раньше он был реверс-хронологическим логом релизов, который обновлялся только
в момент релиза — и в последний раз обновился на v0.34.0, пока репозиторий
ушёл на три релиза вперёд (v0.35/v0.36/v0.37). Теперь это отдельный список,
живущий своей жизнью между релизами.

---

## Health signals (снято `forgeplan health` на 2026-09-08, бинарь v0.37.0)

Это самые дешёвые следующие шаги — дешевле любого пункта из backlog ниже,
потому что инструмент уже нашёл проблему и назвал команду для починки.

- [ ] **At-risk (R_eff ниже 0.3)**: `PRD-005 "Depth-Aware Validation"` (R_eff=0.00), `RFC-004 "Files-First Architecture"` (R_eff=0.10). Разобраться, почему — `forgeplan score PRD-005` / `forgeplan score RFC-004`.
- [ ] **Сирота без связей**: `PROB-090` — `forgeplan link PROB-090 based_on <other>`.
- [ ] **Рассогласование фазы** (10 артефактов, включая `PRD-082`, `PRD-083`, `PROB-083`) — `forgeplan phase <id>` для каждого.
- [ ] **EvidencePack без структурных полей** — `EVID-033`, `EVID-034`, `EVID-035` оценены в CL0 (0.1) вместо реального вердикта. Это тот же класс дефекта, что был описан в апреле 2026 под другими ID (EVID-015/025/026/027, backlog "F6") — тогда почтили точечно, класс не закрыли. Стоит не просто дописать три файла, а решить, нужен ли `validate`-гейт, который ловит EvidencePack без `## Structured Fields` до того, как он попадёт в граф.

---

## Backlog по приоритету

### P1 — дёшево, высокий эффект (Distribution)

Из `docs/ROADMAP.md`: этот раздел не двигался пять месяцев, при этом самый
дорогой пункт из апрельского списка (публичный сайт) уже закрыт. Стоит
перевернуть порядок.

- [ ] `cargo publish` — метаданные в Cargo.toml (homepage/keywords/categories) на месте, публикации на crates.io нет (проверено: `GET crates.io/api/v1/crates/forgeplan` → 404). 🔴 `cargo publish` — RED LINE в safety hook, делать вручную и осознанно.
- [ ] Docker-образ — `Dockerfile` в репозитории отсутствует.
- [ ] Опубликовать переиспользуемый GitHub Action (`forgeplan/action@v1`) для внешних репозиториев — сейчас `validate --ci` / `health --ci --fail-on --strict` работают только как внутренние workflow этого репо (`ci.yml`, `forgeplan-health.yml`).

### P2 — CLI UX polish (старые пункты NOTE-029/030, подтверждены всё ещё открытыми)

Проверено по `Commands` enum в `crates/forgeplan-cli/src/main.rs` — этих
команд по-прежнему нет:

- [ ] `forgeplan doctor` — общая диагностика workspace. **Не путать** с существующим `forgeplan plugins doctor` — тот проверяет только plugin detection/hints, не workspace целиком.
- [ ] `forgeplan links <id>` — визуализация связей одного артефакта.
- [ ] `forgeplan diff <id1> <id2>` — сравнение артефактов.
- [ ] Уточнить, соответствует ли существующий `forgeplan watch` пункту «watch v2 hot-reload» из старого backlog, или это разные вещи.

### P2 — Performance (из ROADMAP, категория 3)

- [ ] Background embedding при первом запуске — по коду не найдено (`grep background.*embed` в `forgeplan-core/src` — 0 совпадений).
- [ ] Новый пункт (не было в апреле): холодный старт `search --semantic` вырос до ~2.0–2.7s (было ~1.5s), до 8.3s при вытесненных из кеша весах — плата за переход на `tract` (v0.35.0). Оценить, стоит ли кэшировать граф модели между запусками.
- [ ] `forgeplan reindex` (не `embed`) — не проверялось, инкрементальный ли он сейчас; `embed` уже пропускает актуальные записи по content-hash (v0.36.0).

### P3 — Architecture (низкий приоритет, без изменений с апреля)

- [ ] Pluggable storage drivers (RFC-003 Phase 3-4: SQLite driver + config-driven selection) — модуля `StorageDriver` в `crates/forgeplan-core/src/db/` не найдено, по-прежнему не вживлено.
- [ ] DSL-скриптинг для кастомных правил (NOTE-039 — Lua/Rhai).
- [ ] Delta-specs (OpenSpec pattern, из PRD-015) — по-прежнему отложено.

### P3 — Ecosystem (без изменений с апреля)

- [ ] VS Code / JetBrains расширение.
- [ ] GitHub Issues bridge — артефакт ↔ issue автосинхронизация (сейчас `gh issue list` используется вручную как трекер, автосинка нет).
- [ ] Linear/Jira export adapter.
- [ ] Slack/Teams-уведомления.

### P3 — Desktop (без изменений с апреля, 5 месяцев без движения)

- [ ] EPIC-004: Tauri 2.0 + React UI. Приоритет ниже Distribution по эффекту/трудозатратам — если решение остаётся в силе, стоит переспросить, актуален ли ещё спрос на десктоп-клиент.

### Очень старые пункты — актуальность не подтверждена, требуют ревизии перед взятием в работу

- [ ] NOTE-027 Ruflo/Gastown Integration — пункт из апреля 2026, не проверялся, может быть неактуален.
- [ ] PROB-022 Brownfield onboarding improvements — вероятно частично закрыт Epic #287 (brownfield surface, v0.32.0), детально не сверялось.
- [ ] PRD-025 Nx Monorepo Migration — не проверялось.

---

## GitHub Issues (живой backlog, не дублируется здесь построчно)

На 2026-09-08 открыто **29** issue (`gh issue list --state open`). Часть —
свежие находки после релиза v0.37.0 (#488, #485, #484, #483, #482, #479,
#475, #474), часть — накопленный technical debt с мая-июня (#454, #411,
#397, #375, #374, #364, #363, #362, #353, #335, #334, #332, #329, #328,
#318, #307, #304, #301, #298, #297, #296, #287). Список меняется каждый
день — смотри `gh issue list --state open` напрямую, а не копию здесь.

---

## Known Issues

- [ ] **1 STUB artifact** — `forgeplan health` показывал 2 STUB на момент этого измерения (было "1 STUB, unidentified" в старой версии файла); не идентифицирован, низкий приоритет.
- [x] ~~**changelog commit_hash**: LanceDB schema migration для старых workspace~~ — похоже устарело: миграция v2→v3 (`crates/forgeplan-core/src/db/migrate.rs`) уже добавляет колонку `commit_hash`. Не переоткрывать без нового репродьюсера на свежем клоне.
- [ ] **`--semantic` feature flag**: `search --semantic` требует сборки с фичей `semantic-search` — не проверялось, актуально ли это ограничение после перехода на `tract` (v0.35.0, который встроен во все 5 релизных бинарей по умолчанию).

---

## Как проверить актуальность этого файла в следующий раз

Не верь ни одному числу здесь без проверки. Способ измерения:

```bash
# версия
grep -A2 '^\[workspace.package\]' Cargo.toml
git tag --sort=-v:refname | head -5

# артефакты + health signals
<published-binary> health

# тесты/команды/MCP-tools — счётчик на главной странице README.md
grep -A5 'tracked artifacts' README.md

# живой backlog
gh issue list --state open --limit 100
```

Использовать **опубликованный** бинарь нужной версии, не тот, что на PATH —
он может отставать (см. `feedback_verify_version_state_not_todo` в памяти
агента: TODO.md исторически лагает релизам, доверять нужно
Cargo.toml + git tags, не старым записям в этом файле).
