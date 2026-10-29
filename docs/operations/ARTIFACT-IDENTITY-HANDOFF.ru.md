# ARTIFACT-IDENTITY-HANDOFF — надёжная идентичность документов ForgePlan

> Документ ведётся по мере исполнения. Последнее обновление: 2026-10-28.
> Канонические артефакты задачи — на английском; этот файл — рабочая передача
> состояния, русский.

## 1. Что делается

Завершается защита от коллизий документов между проектами, ветками, связанными
worktree и независимыми клонами. Рабочая модель — двухслойная, из ADR-012:

- **canonical_key** — постоянный ключ документа, носитель идентичности;
- **display_id** (`id`, `assigned_number`) — отображаемый номер, может совпадать
  у разных документов и в разных проектах.

Новый ключ: `<kind>-<title-part>-k<32 lowercase hex>`, 16 байт из
`OsRng.try_fill_bytes`. Ключ не зависит от времени, номера, PID, пути и хеша
содержимого.

## 2. Канонические артефакты задачи

| Артефакт | ID | Глубина | Статус |
|---|---|---|---|
| PRD «Collision-safe artifact identity across projects and Git workspaces» | PRD-090 | deep | draft |
| SPEC «Canonical artifact identity: key minting, resolution, storage and repair» | SPEC-009 | deep | draft |
| RFC «Implementation of canonical-key artifact identity» | RFC-019 | deep | draft |
| ADR «Canonical key as authoritative artifact identity, display id as alias» | ADR-030 | deep | draft |

Связи: PROB-067, SPEC-005, RFC-009, ADR-012, ADR-029, issue #552.

## 3. Подтверждённые измерения

### Дефект #552 (два worktree — один номер)

`artifact/id_alloc.rs:214-266` читает только локальное дерево. Замок общий
(Git common dir), учёт — нет. Два связанных worktree одного клона выдали
NOTE-002 дважды.

### Потеря идентичности в Constellation

`constellation/workspace_read.rs` сортирует по `canonical_key`, но хранит
`display_id`; `by_slug.insert` молча перезаписывает при коллизии; `get_record`
возвращает последнюю из равных записей. Канонический ключ в Constellation — это
frontmatter slug (`capture.rs:971-975`).

### Legacy-индекс фильтрует по `id`

14 мест в `db/store.rs` используют `id = '...'`; `merge_insert(&["id"])`.
Схема версии 4 (`db/migrate.rs:10-12`), поле `canonical_key` отсутствует.

### Перепись пилотных копий (2026-10-28)

| Проект | Документов | Групп коллизий |
|---|---|---|
| ForgePlanWeb | 205 | 5 (EVID-020, EVID-021, PRD-001, PRD-016, RFC-015) |
| ForgePlanMarketplace | 513 | 11 (EVID-119/133/141/143/146/150/153, NOTE-008, PRD-072, SPEC-001, slug `evid-adr-027-2` → EVID-267/EVID-268) |

Всего 16 групп — совпадает с `source-conflicts.json`. Ни в одной группе нет
общего slug, кроме `evid-adr-027-2`; остальные — коллизии display id при
отсутствующем slug.

Особенность разбора frontmatter: в 47 (Web) и 179 (Marketplace) документах
frontmatter состоит из **двух** блоков, и `slug` находится во втором. Скрипт,
читающий только первый блок, потеряет slug — это отдельное расхождение с
CI-парсером.

Оригиналы `/Users/explosovebit/Work/ForgePlanWeb` и `ForgePlanMarketplace` не
изменялись.

## 4. Состояние рабочей среды

| Объект | Путь / состояние |
|---|---|
| Исходное дерево | `/Users/explosovebit/Work/ForgePlan-Constellation-P0`, ветка `feat/constellation-p0-contract`, HEAD `c852f40c` |
| Рабочее дерево задачи | `/Users/explosovebit/Work/ForgePlan-Identity-Feature`, ветка `feat/artifact-identity` |
| Резервная копия состояния | `/Users/explosovebit/Work/ForgePlan-Identity-Backup/baseline-20261028` |
| Карта владения файлами | `/Users/explosovebit/Work/ForgePlan-Identity-Backup/ownership-map.md` |
| План | `local://artifact-identity-plan.md` |

Исходное дерево не изменено: 703 изменённых и 197 untracked — это накопленная
работа, не задача #552. Снимок проверен побайтово: 703/703 изменённых файлов и
192/192 untracked идентичны; расхождение — только производный индекс
`.forgeplan/tgrep/`, исключённый намеренно.

## 5. Модели исполнения

| Роль | Модель | Агент |
|---|---|---|
| Код (волны 1–2) | `minimax-code/MiniMax-M3.1-Flash-Preview` | `constellation-minimax-coder` |
| Ревью **и** проверка качества | `openai-codex/gpt-6-luna` | `constellation-luna-quality-tester` |
| Документация | `minimax-code/MiniMax-M3.1-Flash-Preview` | `constellation-minimax-coder` (документационный проход) |
| Ворота | `minimax-code/MiniMax-M3.1-Flash-Preview` | `constellation-minimax-guardian` |

Ревью и верификация объединены в одном агенте на Luna: ревьюер, который не
может собрать проект, ревьюит только код, а не поведение. Писатель работает на
другой модели, поэтому генератор не совпадает с проверяющим.

Агент `constellation-luna-quality-tester` создан 2026-10-28 в
`/Users/explosovebit/Work/ForgePlan/.omp/agents/`. Он требует аудита силы
тестов: для каждого нового теста — сломает ли он реализацию, если поведение
испорчено; слабый тест помечается `WEAK`, непокрытый класс дефектов — `GAP`.

## 6. Ограничения среды

### Лимит Codex — Luna недоступна для ревью

Маршрут `openai-codex` (Luna и Sol) исчерпал лимит подписки ChatGPT:

```
openai-codex/gpt-6-luna    → usage_limit_reached
openai-codex/gpt-5.6-luna  → usage_limit_reached
codex-cli (codex exec)     → «usage limit … try again at Nov 4th, 2026 9:44 AM»
```

Лимит **аккаунтный**, а не модельный: проверены две разные модели Codex, обе
отказали. Он же блокирует агента `scout`, который по умолчанию работает на
Luna, — поэтому инвентаризация, запланированная на Luna, переведена на
MiniMax Flash.

Следствие: **финальное ревью и верификация на Luna выполнить сейчас нельзя.**
Агенты `constellation-luna-quality-tester` и `constellation-luna-probe` созданы
и готовы к запуску; они поднимаются автоматически, как только лимит
восстановится. До этого ревью выполняет `constellation-minimax-reviewer` —
как промежуточная проверка, чтобы дефекты находились рано, но **не** как
финальный ворота. Финальный ворота проходит только на Luna.

Это ограничение не отменяет ни одного требования: гейт остаётся, и он не
объявляется пройденным на основании промежуточного ревью.

### Один build lease

Никаких параллельных `cargo build/test/check/clippy`. Перед тяжёлым прогоном —
`df -h .`, и в той же команде, что отчитывается о результате. Ноль блоков
`test result:` означает, что прогон не состоялся. Код выхода измерять без
трубы.

### Сломанный `protoc` после обновления abseil — обязательное условие сборки

`protoc` 29.3 (единственная установленная версия protobuf) слинкован с
abseil **2407.0.0**. Homebrew 17 октября переключил `/opt/homebrew/opt/abseil`
на `20260817.0` (2608.0.0), поэтому `protoc` перестал запускаться:

```
dyld: Library not loaded: /opt/homebrew/opt/abseil/lib/libabsl_die_if_null.2407.0.0.dylib
```

Сборка падает в build-скрипте `lance-encoding`, а не в коде проекта. Нужная
версия abseil на месте — `20240722.1`. Обход без изменения состояния Homebrew:

```bash
export DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/Cellar/abseil/20240722.1/lib
export PROTOC=/opt/homebrew/Cellar/protobuf/29.3/bin/protoc
export CARGO_NET_OFFLINE=true CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0
export CARGO_TARGET_DIR=<общий target-каталог>
```

Проверено: с `DYLD_FALLBACK_LIBRARY_PATH` `protoc --version` отдаёт
`libprotoc 29.3`, без него — падает с `Library not loaded`. Homebrew не
трогаем: `brew switch abseil` менял бы состояние машины шире этой задачи.

### Место

После `cargo clean -p forgeplan -p forgeplan-core -p forgeplan-mcp` свободно
23 ГБ (`target` сократился с 16 ГБ до 7,5 ГБ).

### ADI-провайдер

`codex-cli` недоступен — исчерпан лимит подписки ChatGPT. `claude-code` и
`devin-cli` без логина, `gemini-cli` с невалидным ключом, `cursor-agent` требует
аутентификации.

Дополнительно: этот сетевой путь завершает TLS в TUN-туннеле. `rustls` (стек
`reqwest`, который линкует forgeplan) получает `tls handshake eof`, тогда как
`curl` и Node завершают хендшейк. Поэтому ADI идёт через локальный relay
`/Users/explosovebit/Work/ForgePlan-Identity-Backup/llm-relay.mjs` на
`http://127.0.0.1:8791/v1`, который проксирует в MiniMax через рабочий
TLS-стек Node. Настройка workspace-local (`.forgeplan/config.yaml`); глобальный
конфиг не менялся.

## 7. Что ещё не сделано

- Волна 1 (фундамент) — в работе: пишет `constellation-minimax-coder`.
- Волна 2 (срезы A–D) — не начата.
- Волна 3 (ревью и проверка на Luna, документация, ворота) — не начата.
- Ремонт 16 групп — не начат; оригиналы не тронуты.
- Артефакты PRD-090 / SPEC-009 / RFC-019 / ADR-030 остаются в draft; активация —
  отдельное действие после прохождения ворот.

## 8. Решения по спорным ссылкам

Полная перепись входящих ссылок на дублирующиеся номера измерена
(`/Users/explosovebit/Work/ForgePlan-Identity-Backup/reference-mapping.md`):
Web — 10 вхождений, Marketplace — 14. Каждое назначение определено по
**содержиму конкретной связи**, а не по номеру, заголовку или времени правки.

Одна ссылка была действительно неоднозначной: `NOTE-019 → PRD-072` в
Marketplace. Решение принято 2026-10-28 по поручению владельца — делать
правильно и логично, даже если придётся чем-то пожертвовать.

**Решение: `NOTE-019` → детерминированный PRD-072.** Улики внутри самой
заметки: заголовок «Research context — deterministic SDD+TDD»; тело «captures
the research that produced PRD-072 / ADR-007 / SPEC-001»; «Both STRUCTURE SDD;
NEITHER provides determinism. That gap is what PRD-072 fills» — пробел в
детерминизме закрывает только детерминированный PRD-072; в теле пин корпуса
`df914f49` и четыре упоминания ADR-007 при полном отсутствии ADR-008; дата
2026-05-28 — до рефрейма (ADR-008 / EVID-121). Упоминание NOTE-019 в EVID-123
слабее: это аудит неполной таблицы Related Artifacts, смешивающий эпохи.

Ошибка в этом решении ничего не теряет: журнал ремонта хранит полные
preimage каждого затронутого файла, откат восстанавливает их одной командой.

## 9. Открытые вопросы владельцу

Вопросов нет. Контракт зафиксирован в PRD-090 / SPEC-009 / RFC-019 / ADR-030,
все решения из плана приняты, варианты не переоткрываются.

## 10. Что уже сделано сверх плана

- Перепись 16 групп и 24 входящих ссылок измерена и обоснована (раздел 8).
- ADI-прогон по всем четырём артефактам (NOTE-055 … NOTE-058) поднял четыре
  неоднозначности в контракте — они закрыты в самих артефактах: правило
  backfill, поведение при дублирующемся canonical key, границы
  source-qualified адресации (FR-019) и идентичность при экспорте; плюс
  четыре в RFC-019 (сохраняются ли байты frontmatter, переназначаются ли все
  кандидаты группы, как карантинируются неоднозначные связи и оценки, где
  стоит schema v5 относительно LanceDB) и уточнение инварианта в ADR-030.
- Обнаружено расхождение с планом: путь `artifact/identity.rs` занят модулем
  идентичности **вызывающего** (`AgentIdentity`, PROB-066). Новый модуль
  переименован в `artifact/canonical_key.rs`; контракт не изменился, изменился
  только путь.
- **Массовая правка мест создания вынесена из Волны 1 в отдельный проход.**
  Измерено: 176 литералов `NewArtifact {` и 63 литерала `ArtifactRecord {`;
  173 + 51 из них перечисляют все поля и не компилируются после добавления
  поля. Это сквозная правка, затрагивающая файлы, принадлежащие срезам A, B и
  D, — держать её в core-владельце означало бы гонку за общие файлы. Проход
  выполняется отдельным агентом в собственном worktree
  (`ForgePlan-Identity-FieldPass`, ветка `chore/canonical-key-field-pass`) и
  интегрируется до заморозки Волны 1. Правило выбора ключа на каждом месте
  задано явно; минтинг на этих местах запрещён.
- **Три инвентаризации подготовлены параллельно** и сохранены в
  `/Users/explosovebit/Work/ForgePlan-Identity-Backup/inventories/`: id-ключевые
  места хранилища (50 КБ), поверхности разрешения в CLI/MCP (62 КБ), примитивы
  ремонта, блокировок и атомарной записи (47 КБ). Это готовые рабочие списки
  для срезов B, C и D.
- Обнаружено, что слой Constellation **уже** хранит по `canonical_key`
  (`publisher.rs`: `MERGE_KEY = ["canonical_key"]`), выводя ключ из slug. То есть
  задача — привести legacy-слой к тому, что Constellation уже делает, а не
  вводить третью идентичность.

## 11. Инцидент при слиянии (зафиксирован честно)

Во время слияния ветки массовой правки в основное дерево была допущена ошибка:
`git checkout <branch> -- <file>` берёт состояние **коммита** ветки, а агент не
коммитил свои правки. В результате 60 файлов были откачены к состоянию HEAD,
то есть потерян накопленный базовый diff P0 в этих файлах.

Обнаружено сразу и полностью восстановлено:

- затронутые файлы восстановлены копированием из `ForgePlan-Constellation-P0`,
  где базовый diff не тронут (P0 — источник истины, 703 изменённых файла на месте);
- правки поля наложены поверх восстановленного базового состояния через
  `patch -p1` (не `git apply`, которому нужен согласованный индекс);
- один файл дал отказы (`git_sync.rs`) — закрыт точечной вставкой;
- итоговая сверка: **0** исходных файлов потеряно, все расхождения с базой —
  ровно те 69 файлов, которые изменены намеренно (60 файлов массовой правки,
  `db/store.rs`, `workspace_read.rs`, `artifact/mod.rs`, новый `canonical_key.rs`
  и четыре R1-правки).

Урок на будущее: сливать рабочее дерево агента в чужое только через
`git diff` + `patch`, либо после явного коммита агента. `git checkout <branch>
-- <file>` без коммита в ветке молча откатывает всё, что было не в HEAD.

Отдельно: первая попытка слить тем же способом дала «успех», который выглядел
правдоподобно — 2 файла отличались от базы вместо 60+. Проверка считала только
файлы со статусом ` M `, а откатанные к HEAD не имеют такого статуса и
выпадали из сравнения. Сверка теперь идёт по всем файлам дерева, а не по
статусу Git.

## 12. Доказательства

| Что | Где |
|---|---|
| План | `local://artifact-identity-plan.md` |
| Снимок состояния и манифест | `/Users/explosovebit/Work/ForgePlan-Identity-Backup/baseline-20261028/` |
| Карта владения файлами | `/Users/explosovebit/Work/ForgePlan-Identity-Backup/ownership-map.md` |
| Relay для ADI | `/Users/explosovebit/Work/ForgePlan-Identity-Backup/llm-relay.mjs` |
| Перепись коллизий | этот файл, раздел 3 |
| ADI-заметки | `.forgeplan/notes/` в рабочем дереве задачи, связаны с PRD-090 и далее |
