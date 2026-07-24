# Идеи из vibe-kanban → ForgeFarm

> vibe-kanban (BloopAI, **Apache-2.0** — не MIT, как предполагалось; LICENSE в
> корне репо) — ближайший к ForgeFarm shipped-продукт: kanban-issues →
> workspaces (git worktrees + agent session + terminal + dev-server preview),
> где работают 9+ coding-агентов (Claude Code, Codex, Gemini, Copilot, Amp,
> Cursor, OpenCode, Droid, Qwen); diff-review с inline-комментами, уходящими
> агенту следующим промптом; создание и merge PR. Rust workspace из 30+
> крейтов + React frontend + Tauri + npx-cli + relay. ~2070 коммитов, ~8
> человек, июнь 2025 → апрель 2026. **Продукт закрыт** (объявление
> 2026-04-10) — и это одновременно источник проверенных механик и главный
> продуктовый урок. Репо изучен локально: v0.1.44,
> `dev/vibe-kanban/`. Этот документ: что берём кодом, что дизайном, что
> уроком, что отвергаем — и какие дыры в нашей KB вскрыл разбор.

---

## 1. Что такое VK и почему его смерть — аргумент ЗА ForgeFarm

**Операционная модель VK**: человек планирует на канбане → агент выполняет в
workspace (git worktree-контейнер) → человек ревьюит diff и кликает
approve/merge. Никаких quality gates, никакого evidence, никакой автономной
петли: **человек — и планировщик, и верификатор, и шедулер**. При этом
plumbing-слой (executor adapters, worktree lifecycle, log normalization,
live-стриминг в UI) сделан на production-уровне и отлажен на 9 гетерогенных
CLI-агентах.

**Sunsetting: факты и причины.** Закрыт при живом product-market fit:
30 000 MAU, 25 000 GitHub stars, реальная ценность workflow — и ноль
бизнеса. Диагноз основателя (Louis Knight-Webb, дословно):

- *«the vast majority are free users and we couldn't find a business model
  that we could get excited about»*;
- *«Everybody who is making money is selling to enterprise and reselling
  tokens. And we were doing neither of those things»*;
- *«It's a mature market at this point and it's no fun playing for eighth
  place»*;
- ретроспективно: *«I'd hire somebody who's really good at selling to
  Enterprise»*.

Экономика провала: пользователи платят тысячи $/мес вендорам агентов и
$0–30 координатору. **Координационный слой без хранимого суждения (verdicts,
evidence, policy state) не аккумулирует durable-ценность и остаётся
заменяемым UX.** База VK записывает *что произошло* (процессы, коммиты,
логи), но никогда — *хорошо ли это было*. Это ровно тот слой, который
ForgeFarm делает продуктом: contracts, gates, evidence-first close, audit
trail, tiered autonomy. Sunset VK — эмпирическая валидация тезиса, с одной
оговоркой: валидация работает, только если FF реально продаёт governance
(enterprise-история), а не «кокпит получше».

Финальные коммиты говорят сами за себя: `97123d526` «Sunset project routes
to export-only page» (облачный канбан выпилен в export-only страницу) и
`9f1015036` «Add README sunsetting banner». Умерла та часть, что дублировала
Linear/GitHub; часть-«кокпит исполнения агентов» осталась любимой — репо
живёт как community-maintained Apache-2.0.

---

## 2. Архитектурная карта VK одним экраном

| Подсистема VK | Что делает | Аналог в ForgeFarm |
|---|---|---|
| `crates/executors` | 9 адаптеров под одним trait `StandardCodingAgentExecutor` (enum_dispatch); action-chain (`ExecutorAction` linked list); approvals seam; MCP-config injection; log normalization → `NormalizedEntry` JSON-patches | **ExecutorDriver seam** (главный объект добычи) |
| `crates/git` | `GitService`: libgit2 для read-only графа, git CLI для всех мутаций working tree; typed errors (`WorktreeDirty`, `MergeConflicts`, `BranchesDiverged`); in-memory squash merge | **Worktree & Merge Governor** (git-слой) |
| `crates/worktree-manager` | 3-фазная проверка валидности worktree, surgical cleanup, per-path locks, orphan inference | **Worktree Governor** (lifecycle) |
| `crates/workspace-manager` | Workspace = контейнер N worktrees (по одному на repo, одна ветка), rollback при partial creation, orphan sweep, TTL expiry | **Workspace-модель** поверх Governor |
| `crates/db` (SQLite/sqlx) | Workspace → Session → ExecutionProcess → per-repo RepoState (before/after HEAD) + CodingAgentTurn (agent session/message ids) | **Projection DB** (Postgres): Task/Run/Step + git-provenance |
| `crates/services` | ContainerService (workflow-engine на default-методах), exit monitor, EventService (SQLite update hooks → JSON-Patch broadcast), diff_stream, approvals, PR monitor | **Run supervisor + gate hooks** (переписываем поверх Postgres outbox) |
| `crates/local-deployment` | Единственная реализация `Deployment` trait: spawn/kill process groups, dual exit detection, crash recovery | **Runner** |
| `crates/server` | axum REST + WS: workspaces/sessions/execution-processes/approvals/git ops/PR | **Control-plane API** |
| `crates/mcp` | Тонкий MCP-прокси над REST: global mode (~30 tools) и orchestrator mode (7 tools, workspace-scoped) — агент спавнит суб-агентов | **ff CLI/MCP для агентов** (T2→T3 dispatch) |
| `crates/git-host` | PR-операции через vendor CLI (`gh`/`az`), enum_dispatch, typed retryable/terminal errors, backon retry | **git-host seam** (FF: octocrab за тем же trait) |
| `packages/web-core` + `packages/ui` | React: generic WS JSON-Patch hook + immer; NormalizedEntry рендеринг; diff panel; review→prompt loop; триаж «needs attention» | **Board projection + Run Inspector + ff top** |
| `npx-cli` | ~600 LOC thin shim: манифест на CDN + sha256 + versioned cache + exec | **Дистрибуция `npx forgefarm`** |
| 13 relay-крейтов + Tauri + preview-proxy | WebRTC/yamux туннели, SPAKE2 pairing, SSH-over-WebSocket, desktop shell | **Не строим** (prior art в закладки) |
| `crates/review` | Облачный PR-review: аплоад diff + **транскрипта Claude-сессии** генератора → LLM-summary | Идея «transcript = evidence» → **EvidencePack**; модель review отвергаем |

---

## 3. Брать код (Apache-2.0, attribution + NOTICE)

Всё ниже — vendor на уровне файлов/модулей с атрибуцией. Пути — от корня
`dev/vibe-kanban/`.

### 3.1 Git / Worktree (самая высокая ценность на строку)

| Что | Изменения при переносе |
|---|---|
| `crates/git/src/cli.rs` (~950 LOC) — GitCli-обёртка: единая точка `git_impl`, NUL-safe парсинг porcelain, трюк с temp `GIT_INDEX_FILE` (diff с untracked + renames, не трогая реальный index), `classify_cli_error` (AuthFailed/PushRejected), пробы mid-flight операций (`rebase-merge`/`rebase-apply`/`MERGE_HEAD`/`CHERRY_PICK_HEAD`/`REVERT_HEAD`). В шапке файла — **письменная доктрина CLI-over-libgit2**: CLI для всех мутаций (отказывается затирать uncommitted, уважает sparse-checkout, не корраптит WSL-репо), libgit2 только для read-only графа | Vendor почти as-is. Прибить локаль (`LC_ALL=C`) под substring-matching stderr; расширить typed error taxonomy под policy-роутинг HAQ-vs-auto-retry; починить CLI-merge-путь, который течёт сырой строкой «CLI merge failed» вместо typed `MergeConflicts` |
| `crates/worktree-manager/src/worktree_manager.rs` (~580 LOC) — 3-фазная проверка валидности (fs path + back-pointer `<commondir>/worktrees/*/gitdir` + libgit2 `find_worktree`), surgical cleanup (remove --force → delete metadata dir → rm dir → prune), инференс родительского repo из orphan-worktree через `git rev-parse --git-common-dir`, нормализация macOS `/private` и Windows UNC | **Критическая хирургия**: вырезать auto-recreate-on-invalid (`ensure_worktree_exists` молча rm-rf'ит dirty worktree) → трёхсостоянийная модель FF: invalid+clean → recreate разрешён; invalid+dirty → quarantine + HAQ. Process-local `WORKTREE_CREATION_LOCKS` HashMap → Postgres advisory locks/leases. Каждое деструктивное действие — durable audit RunEvent |
| `crates/git/tests/git_ops_safety.rs` (1349 строк) — готовый conformance-набор: rebase сохраняет untracked, rebase abort при uncommitted tracked, merge отказывается при staged на base, конфликт не двигает base ref (текст/binary/rename), libgit2-fallback для orphaned branch, sparse-checkout | Портировать **сценарии** (не harness) как eval-векторы Governor; добавить FF-специфичные: invalid+dirty quarantine, lease fencing под конкурентным доступом, gate-blocked merge |
| `crates/git/src/lib.rs` `perform_squash_merge` (~L1081–1126) — in-memory libgit2 `merge_commits` c `fail_on_conflict`: ref-only squash без касания working tree; плюс gate `BranchesDiverged` (base ahead → блок merge, форсирует rebase-first linear history), post-merge reset task-branch ref (follow-up без конфликтов), `ConflictOp` detect/abort state machine, `reconcile_worktree_to_commit` с dry-run контрактом (`WorktreeResetOptions/Outcome`) | Вставить policy/gate-оценку **перед** любым движением ref (VK двигает безусловно); сделать divergence-check fetch-aware (VK смотрит только локальные refs); унифицировать отчёт о конфликтах между libgit2- и CLI-путями (libgit2-путь возвращает пустой `conflicted_files`) |

### 3.2 Executors: вердикт по крейту

**Selective vendor (file/module-level fork) листового plumbing + reimplement
контрактного слоя. НЕ depend, НЕ форкать целиком.**

- **Не depend**: upstream sunsetting (никто не будет гнать еженедельный
  adapter-treadmill — главную liability крейта), крейты workspace-coupled и
  не опубликованы, в dep-tree личный форк ts-rs
  (`github.com/xazukx/ts-rs`, branch `use-ts-enum`).
- **Не форкать целиком**: унаследуешь ~10k строк рукописных per-agent
  парсеров (codex 2.9k, claude 3.2k, opencode 1.6k…), full-auto default
  профили, дыры Noop-approvals и process-global mutable profile singleton —
  ровно тот субстрат, который дизайн FF отвергает.
- **Что VK обесценил**: риск protocol discovery. Четыре стиля интеграции
  теперь замерены реальной стоимостью: bespoke stdio control protocol
  ~4.5–5k LOC каждый (Claude, Codex); local HTTP+SSE ~4.8k (OpenCode);
  **ACP ~1.9k общих на 3 агента при ~230 marginal LOC на агента**
  (Gemini/Qwen/Copilot через один harness); dumb pipes ~200–1.5k, но без
  approvals и структурных логов. Соотношение bespoke:ACP ≈ 20:1 —
  **эмпирическое подтверждение D-002 (ACP-first)**.
- **Что FF всё равно пишет сам**: сам trait ExecutorDriver — потому что у
  VK-trait нет ровно того, ради чего FF существует: structured **RunOutcome**
  (контракт VK кончается на process Success/Failure), typed error taxonomy
  для автономного retry (VK схлопывает в `Io(string)`), явный
  session-semantics capability contract (resume vs fork vs prompt-stuff —
  VK прячет за одним методом), и policy seam там, где у VK
  human-approval seam.

Конкретные vendor-кандидаты из `crates/executors`:

| Что | Изменения |
|---|---|
| `src/executors/acp/{harness.rs,client.rs}` (~990 LOC) — полный ACP-клиент: !Send connection в карантине current_thread runtime + LocalSet, permission bridging, петля denial-reason → feedback-turn, duplex stdio | База D-002-драйвера. `request_permission` → в policy/gate engine FF вместо human-approval; JSONL pseudo-resume (`session.rs` — вся история заново в промпт) → явная declared capability с token-budget cap или выпилить; stable-ID RunEvents вместо JSONL-файлов; оценить оверхед OS-thread+runtime на ран при fleet-scale |
| `src/executors/claude/{protocol.rs,types.rs,client.rs}` (~830 LOC) — минимальный Rust-клиент stream-json control protocol CC: initialize + PreToolUse hook matchers, `can_use_tool` callback, `set_permission_mode`, `interrupt`, флип ExitPlanMode→BypassPermissions, `--resume-session-at` (message-level rollback — только CC это умеет) | Точка in-run policy enforcement CC-драйвера: `can_use_tool` = синхронный вызов gate engine, возвращающий policy decision record; Stop-hook механизм (VK: commit reminder, `{decision: block, reason}` — не даёт закончить turn пока git dirty) → гейт evidence-first close («нет turn-end без тестов/evidence»); hardcoded model list убрать в Model Gateway; resume-at-message оставить для checkpoint reset |
| `src/executors/codex/jsonrpc.rs` (~300 LOC) — bespoke bidirectional JSON-RPC peer (pending-map, server-initiated requests, shutdown resolution, cancellation) поверх официальных `codex-app-server-protocol` крейтов | Минимальные правки, если держим bespoke Codex-драйвер; error surface → typed retryable/terminal taxonomy FF. Скелет для любого будущего app-server-агента |
| `src/stdout_dup.rs` (~140 LOC) — cross-platform подмена stdout ребёнка на os_pipe (unix fd / windows handle) + dummy-carrier process; краеугольный трюк, дающий protocol-агентам и pipe-агентам один канонический log-путь | Vendor as-is |
| `src/logs/plain_text_processor.rs` + `stderr_processor.rs` + `logs/utils/{patch.rs,entry_index.rs}` — builder-конфигурируемый stream clusterer (time-gap flush, boundary predicates, size-threshold split, partial-update emission), ConversationPatch RFC-6902 helpers, resume-safe EntryIndexProvider | Кластеризацию оставить; позиционную адресацию `/entries/<idx>` → stable event IDs из projection DB; добавить версионирование схемы patch-потока (у VK его нет — новый парсер ретроактивно ломает рендер старых ранов) |
| `crates/utils/src/msg_store.rs` + `log_msg.rs` (~260 LOC) — byte-budgeted ring history + broadcast с `history_plus_stream()` (replay-then-live), сентинелы Ready/Finished | Понизить из source-of-truth до transport-кэша перед durable RunEvents (Postgres append log/outbox); broadcast Lagged drop → run-health event, а не строка в логе; 100MB budget — per-profile config |
| `src/logs/utils/shell_command_parsing.rs` — итеративный unwrap `sh|bash|zsh -c`-обёрток, shlex-детект redirect-target, категоризация первого слова (Read/Search/Edit/Fetch) | Семя risk-классификатора tool-calls в gate engine: 5 UI-категорий → реальные risk tiers с allowlists, path-scope checks и per-tier policy bindings. В VK — display-only |
| `src/mcp_config.rs` — JSONC comment-preserving CST deep-merge (jsonc-parser) + 6 per-agent shape-адаптеров MCP-конфига (Passthrough/Gemini/Cursor/Codex/Opencode/Copilot) — каталог реальной дивергенции конфигов | Взять адаптеры и merge; **инвертировать цель инъекции** — писать per-worktree/ephemeral конфиги (консистентно с executor-sessions.md: «глобальные конфиги не мутируются»), никогда user-global файлы; выкинуть preconfigured-каталог с плейсхолдерами `YOUR_API_KEY` |

### 3.3 Streaming / UI / Дистрибуция

| Что | Изменения |
|---|---|
| `crates/services/src/services/diff_stream.rs` (~840 LOC) — live diff engine: debounced fs watcher + отдельный `.git`-watcher (HEAD ушёл на не-child коммит → полный reset потока) + reconcile-проходы 1s/5s, mtime+size dedup-кэш, 200MB cumulative omit policy, watcher умирает вместе с Drop потока | Портировать для live diffs Run Inspector; эмиссию перенацелить на projection WS keyspace FF; reconcile-проход против пропущенных fs-событий сохранить |
| `packages/web-core/src/shared/hooks/useJsonPatchWsStream.ts` (~250 строк) — generic WS JSON-Patch → immer reducer, протокол Ready/JsonPatch/finished, exponential backoff | Портировать под Board/Inspector transport |
| `packages/web-core/src/features/workspace-chat/ui/DisplayConversationEntry.tsx` + `packages/ui/src/components/Chat*.tsx` — исчерпывающий рендер-switch по NormalizedEntry/ActionType вкл. aggregation groups | Портируется почти дословно, если FF нормализует в тот же словарь |
| `packages/web-core/.../useConversationVirtualizer.ts` — гибридная виртуализация: virtualized head + 8 unvirtualized хвостовых строк (стримящиеся строки не дерутся с ResizeObserver), bottom-lock | Решённая проблема «стриминг в virtual list» — брать |
| `npx-cli/src/{cli.ts,download.ts}` (~600 LOC) — thin shim: platform/Rosetta detection, CDN-манифест + streaming sha256, `.tmp`+rename атомарность, versioned cache с отложенной чисткой старья, update nag, local-dev mode | Ребренд под `npx forgefarm`, свой CDN, заполнить license-поле package.json (у VK пустая строка). Держать рядом с brew/cargo-dist |

---

## 4. Брать дизайн (механика → компонент ForgeFarm)

### VK-D-1. Форма driver-trait, проверенная на 9 CLI

`spawn / spawn_follow_up(session_id, reset_to_message_id) / normalize_logs /
discover_options / availability probe` + `SpawnedChild { child:
AsyncGroupChild, exit_signal: oneshot<Success|Failure>, cancel:
CancellationToken }` — **двунаправленный lifecycle**: executor сигналит
логическое завершение (для never-exiting процессов вроде `codex app-server`),
оркестратор — graceful cancel до kill. → **ExecutorDriver**: форму принять,
добавить отсутствующее: structured RunOutcome (exit + artifacts + verdict
hooks + eval row) и typed error taxonomy вместо `Io(string)`.

### VK-D-2. Двухступенчатый лог-пайплайн + словарь NormalizedEntry

Raw LogMsg transport (replay+live, персистится) строго отвязан от per-driver
нормализаторов; **ре-нормализация = чистый replay сырых логов** — свойство
аудируемости (любой транскрипт восстановим постфактум). Словарь
`NormalizedEntry / ActionType / FileChange / ToolStatus` (unified diffs
первым классом: `FileChange::Edit{unified_diff}`; approval-состояния в
потоке: `ToolStatus::PendingApproval`; `CommandRun{category}`) — готовая
схема, зеркалится в TS через ts-rs. → **RunEvents-схема + Run Inspector**.
Фиксы FF: stable event IDs вместо позиционных индексов, версионирование
raw-потока, per-step token/cost attribution для eval loop (у VK — один
опциональный `TokenUsageInfo` на всё).

### VK-D-3. Approvals: create/wait split = точка вставки HAQ

`create_tool_approval(tool)` возвращает id, который логируется синтетическим
`ApprovalRequested`-событием **до** блокировки в `wait_tool_approval`;
резолюция зеркалится в транскрипт (PendingApproval → Approved /
Denied{reason} / TimedOut); **denial reason становится следующим
feedback-turn агента** (три per-agent механизма — унифицировать как одну
driver capability). → **HAQ + policy engine**: FF ставит policy engine туда,
где у VK человек (а у 4 из 9 агентов — Noop, авто-approve всего);
ApprovalStatus становится durable policy decision record в Postgres с
SLA/эскалацией; человек — эскалационный путь, не дефолтный.

### VK-D-4. Четырёхуровневая модель рана + git-provenance

Workspace (branch + worktree container) → Session (conversation, executor
пиннится первым раном) → ExecutionProcess (один spawn, run_reason,
Running→Completed/Failed/Killed) → **ExecutionProcessRepoState
(before/after HEAD + merge_commit per repo)** + CodingAgentTurn
(agent-native session_id/message_id, соскобленные из потока). →
**Projection DB (Task/Run/Step)**: per-repo before/after HEAD — готовый
provenance-хребет EvidencePack («какой диапазон коммитов породил этот ран» =
один запрос) и для checkpoint reset. Добавить: lease-колонки, gate-state,
verdicts — слой суждения, которого у VK нет.

### VK-D-5. Exit-monitor: упорядоченный teardown

Dual exit detection (250ms try_wait poll наперегонки с exit_signal),
терминальный статус пишется первым, cancel → 5s grace → **process-group
SIGKILL (убивает осиротевших MCP-детей)**, log-flush handshake перед
finalize, захват after-head коммитов, 30s spawn timeout, startup orphan
sweep (running-строки → Failed c best-effort provenance). Место:
`crates/local-deployment/src/container.rs` `spawn_exit_monitor` (L480–813) —
production-отлаженное знание жизненного цикла на 10+ агентах. →
**Run supervisor**: последовательность реимплементировать, каждый
hardcoded-бранч (auto-commit, условный chain, queued message) превратить в
точку gate/policy-оценки с durable audit.

### VK-D-6. Session semantics = явный capability contract

Три несовместимых класса follow-up, скрытых у VK за одним методом:
**resume-in-place + message rollback** (CC `--resume` /
`--resume-session-at`), **fork-on-follow-up** (Codex `thread/fork`, OpenCode
`session/fork` — новый id каждый ход), **prompt-stuffed pseudo-resume** (ACP:
вся JSONL-история заново в промпт, неограниченный token-рост). Разные
следствия для leases, цены retry и checkpoint reset. → **ExecutorDriver
capability declaration + task state machine**: драйвер обязан объявить свой
класс, scheduler прайсит follow-ups соответственно. CC resume-at-message
питает FF «reset to checkpoint».

### VK-D-7. Checkpoint restore

`reset_session_to_process` = per-repo reconcile worktree к захваченному
before_head_commit (dry-run-able, dirty-tree явный) + `drop_at_and_after`
soft-delete поздних процессов (скрыты из timeline, сохранены для аудита) +
`reset_to_message_id` agent-side rewind — полный time-travel
разговор+worktree. → transition «reset to checkpoint» в task state machine;
tombstone-паттерн — правильная форма evidence-preserving хирургии истории.

### VK-D-8. Wire-протокол живой проекции

Per-entity snapshot-as-id-keyed-map + сентинел Ready + фильтрованные live
JSON-Patches (синтез Replace→Add/Remove при смене членства в фильтре), один
generic frontend hook, batch DTO `WorkspaceSummary` для карточек, триаж-
предикат «needs attention» = `hasPendingApproval || (unseen && !running)`.
→ **Board projection + ff top + Run Inspector transport** — но источником
должен быть transactional outbox / LISTEN-NOTIFY над Postgres, **не**
SQLite-update-hook шина VK. Плюс **txid handshake** из `crates/remote`
(`MutationResponse{data, txid=pg_current_xact_id()}` — клиент держит
optimistic state, пока txid не приедет в replication-потоке) — самый чистый
shipped-контракт optimistic-UI-over-replication; принять.

### VK-D-9. Review→prompt loop (любимая механика VK)

Inline diff-комменты `{filePath, lineNumber, side, codeLine, text}` →
markdown `## Review Comments (N)` в следующий agent turn; GitHub PR-комменты
→ единый `UnifiedPrComment` timeline → fenced-блоки ```gh-comment с JSON.
→ **HAQ / Run Inspector review surface**. Изменение FF: персистить комменты
server-side как evidence-linked review artifacts с resolution status, чтобы
независимый verifier мог проверить addressed-ness (VK сплющивает в prose и
забывает — ре-верификация невозможна в принципе).

### VK-D-10. Доктрина «branch durable / worktree cattle»

Workspace = контейнер N worktrees (одна ветка на все repo, per-repo
target_branch), tombstone `worktree_deleted` + ленивое воскрешение из ветки,
TTL expiry c guard «не пока процесс бежит» и pinned-исключением, root
CLAUDE.md/AGENTS.md с @-import'ами per-repo контекста. Доктрина объясняет,
почему auto-repair VK в основном сходил с рук (закоммиченная работа
переживает). → **Worktree Governor**: кодифицировать поправку —
auto-recreate разрешён **iff** invalid+clean; invalid+dirty → quarantine +
HAQ. Политика — в gate engine, не в SQL с localtime-арифметикой.

### VK-D-11. Profile system → autonomy profiles

`ExecutorConfig{executor, variant, model_id, agent_id, reasoning_id,
permission_policy}` — единый identity+overrides объект сквозь API/DB/frontend;
defaults + user-diff persistence c защитой от удаления built-in'ов; per-agent
reverse-mapping пресетов (эффективная политика отображаема). →
**Autonomy profiles + Model Gateway**: хранить версионно в projection DB per
tier/task-class (не process-global mutable JSON singleton, который гоняется с
очередями исполнений); PermissionPolicy Auto/Supervised/Plan — грубое семя
T-tier, заменить реальными tier bindings.

### VK-D-12. MCP-as-orchestration-surface

Scoped tool routers (global ~30 tools vs orchestrator 7 tools), bootstrap
контекста по cwd (prefix-match `container_ref` через
`/api/containers/attempt-context`), отказ self-session-loop, пиннинг
executor per session с mismatch error. **283 из 2070 коммитов самого VK
сделаны через его собственный MCP** — агенты создавали issues и стартовали
workspaces другим агентам; рекурсивный dispatch-loop работает поверх
обычного трекера. → **ff CLI/MCP для агентов (T2→T3)**: петлю доказывать не
надо — надо её гейтить; enforcement обязан жить server-side в control plane
с настоящей authn (у VK scoping — advisory-проверки в MCP-прокси над
неаутентифицированным localhost API, любой агент обходит его curl'ом).

### VK-D-13. Agent-native gate (Stop-hook)

CC Stop-hook, возвращающий `{decision: block, reason}`, форсирует ещё один
turn, пока условие не выполнено (VK: git clean; guard `stop_hook_active` от
петель); эквиваленты у Codex/OpenCode через injected turns. →
**Evidence-first close внутри рана**: «нет turn-end без прогнанных
тестов/приложенного evidence» реализуется этим же механизмом per driver
class — ещё до того, как внешний gate вообще оценивается.

### VK-D-14. Queue-while-running

Single-slot очередь follow-up per session со snapshot executor-config,
typing-cancels-queue-and-restores-draft, consumed только при успешном exit.
→ Run-scoped message queue в projection DB: durable + ordered; поведение
«discard при failure» (VK молча выбрасывает) — явное policy-решение.

---

## 5. Уроки (продукт / дистрибуция / scope / смерть)

1. **PMF без бизнеса — дефолтный исход координационного слоя.** 30k MAU /
   25k stars не спасли: ценность течёт вендорам токенов. Монетизация — либо
   enterprise governance, либо перепродажа токенов. FF позиционируется
   первым; sunset VK — данные в пользу, но только если продавать governance.
2. **Ценность аккумулируется в хранимом суждении, а не в plumbing.**
   Projection DB FF: субстратный слой = схема VK, первичный актив = слой
   суждения (gates, verdicts, EvidencePacks, eval rows).
3. **Экономика adapter-treadmill теперь количественная**: ~10k LOC
   парсеров + еженедельные пины версий (`claude-code@2.1.119`,
   `codex@0.124.0`…) + hardcoded каталоги моделей в Rust-исходниках —
   счёт за обслуживание, вероятно соучастник sunset. ACP 20:1 дешевле →
   D-002 подтверждён: ACP-first, bespoke только для T0/T1 (CC + Codex),
   dumb-pipe адаптеры (Amp/Cursor/Droid-класс) — отказ: не несут ни gates,
   ни структурных логов.
4. **Никогда не связывать локальную работу с hosted-сервисами.** Поздний
   cloud-pivot VK загейтил канбан за sign-in и унёс issues в hosted
   Postgres/ElectricSQL — shutdown стал принудительной 30-дневной миграцией
   с export-only страницами. Projection DB и Board FF обязаны работать
   полностью local/self-hosted с первого дня; cloud — опциональный overlay,
   удаление которого — не-событие.
5. **Misallocation scope убивает**: 13 relay-крейтов (WebRTC, yamux, SPAKE2,
   SSH-over-WebSocket) + Tauri + 6 платформ × 3 бинаря — мирового уровня
   инженерия на фиче удалённого просмотра, ортогональной ядру, при нулевой
   verification-истории в самом ядре. FF: Tailscale/SSH до тех пор, пока
   enterprise не потребует иного; `crates/trusted-key-auth` / `relay-*` —
   отличный Apache-2.0 prior art на тот день.
6. **Не конкурировать с issue-трекерами.** Умерло ровно то, что дублировало
   Linear/GitHub. Board FF — проекция собственных контрактных объектов
   (tasks/leases/gates/runs): колонки = состояния машины, drag = guarded
   transition request, который может быть отклонён. Никогда — general PM.
7. **Каждый shipped DEFAULT-профиль VK включает skip-permissions**
   (`dangerously_skip_permissions` / `--yolo` / `danger-full-access` /
   `skip-permissions-unsafe`). Рынок доказуемо хочет full-auto, а
   альтернатива VK — только поштучный человеческий клик. Белое пятно между
   полюсами — tiered autonomy с policy, budgets, risk classes — **и есть
   продукт FF**: VK доказывает спрос на автономию и не даёт никакой
   безопасности.
8. **npx thin-shim — двигатель роста VK и полностью копируем** (~600 LOC).
9. **«Ре-нормализация = чистый replay» — реальное свойство аудируемости, но
   с failure mode**: неверсионированный raw-поток → новый парсер ломает
   рендер старых ранов. Свойство сохранить, поток версионировать.
10. **Agent-native гигиена репо дёшева и компаундится**: AGENTS.md==CLAUDE.md
    (один источник), scoped per-directory инструкции
    (`crates/remote/AGENTS.md` — полный архитектурный бриф), generated types
    с `--check` CI drift guard. Внедрить в монорепо FF сразу.
11. **Supply-chain порезы компаундятся к закату**: ts-rs на личном форке,
    пустые license-поля npm, приватный billing-крейт за feature flag,
    против которого self-hosters компилируют фантом. FF: никаких пиннов на
    личные форки, метаданные лицензий полные с первого дня.

---

## 6. Не брать (и почему)

| Отвергаем | Почему |
|---|---|
| **SQLite-update-hooks-as-event-bus** (`crates/services/src/services/events.rs`) | Форсирует нетранзакционные записи (задокументировано в `ExecutionProcess::create` — многострочные инварианты неатомарны), неупорядоченный async fan-out, drop при lag, ноль durable/replayable лога. FF: transactional outbox + LISTEN/NOTIFY. JSON-Patch wire-формат взять, шину — нет |
| **Auto-repair invalid worktrees** (`ensure_worktree_exists` / `ensure_container_exists` молча rm-rf + recreate при любом касании) | Антипаттерн, ради которого существует правило never-auto-repair-invalid: uncommitted-работа агента в корраптнутом worktree уничтожается, а не карантинится. Взять только поправку clean/dirty (VK-D-10); рефлекс — нет |
| **Full-auto DEFAULT-профили + PermissionPolicy как вся policy-поверхность** | 3-значный advisory enum, замапленный на per-agent skip-флаги; 4 из 9 агентов молча получают Noop approval service; тихий auto-approve при отсутствующем `tool_use_id` (`claude/client.rs` L309–320). Gate engine FF заменяет это целиком |
| **Generator=verifier review, обе формы** | `spawn_review` ре-промптит **ту же** agent-сессию, что писала код («run `git diff base..HEAD`»); облачный `crates/review` использует транскрипт того же генератора как контекст. Идею transcript-as-evidence взять; модель review — по построению нет |
| **In-memory координация как system of record** | child_store/cancellation_tokens/exit monitors в HashMaps, approvals в DashMaps, one-slot очереди, process-local locks — без leases, без fencing, всё испаряется при рестарте; crash recovery = mark-everything-Failed без re-attach/retry. Допустимо только как process-local кэш под Postgres lease-моделью FF |
| **Весь remote-access стек** (13 relay-крейтов, Tauri shell, preview-proxy click-to-component) | Ортогонально control plane; канонический экспонат misallocation. В закладки как prior art, не строить |
| **MCP-инъекция мутацией user-global конфигов** (`~/.claude.json`, `~/.codex/config.toml`) с плейсхолдерами `YOUR_API_KEY` | Инвазивно, не workspace-scoped, без cleanup; противоречит правилу per-worktree injection нашей KB. Взять только каталог shape-адаптеров |
| **Freeform kanban-семантика** | Drag напрямую ставит status_id, «done» эвристикой по позиции колонки, ноль transition guards, TaskStatus без единой локальной транзиции. Board FF проецирует реальную state machine |
| **mtime-of-auth-file эвристики доступности** («`~/.claude.json` изменён» == LoginDetected) как сигнал диспетчеризации | Гадание. Драйверам FF нужны настоящие health/version handshakes перед dispatch |
| **Hardcoded каталоги моделей и пины npx-версий агентов в Rust-исходниках** | Это и есть treadmill. Место — Model Gateway / конфиг со своим циклом обновления |
| **ACP JSONL pseudo-resume как дефолтный follow-up** | Вся история заново в промпт, неограниченный рост, lossy. Только как явно declared capability с token budget cap |
| **Frontend-антипаттерны**: N WebSockets на историю (по одному на execution process, с 20×500ms retry-заплаткой гонки), позиционная идентичность записей, эфемерные review-комменты, 500ms setTimeout sync-guards вокруг Electric-гонок | FF: один пагинированный timeline на run, stable event IDs из проекции, персистентные review artifacts |
| **Split-brain local-SQLite + cloud-ElectricSQL** с ручным one-way sync (migration_state ledger, synced_at watermarks, delete-orphan-on-404) | Dual-write сложность; у FF один Postgres projection. (txid handshake из облачной половины — единственный кусок, который берём) |
| **Policy-as-SQL**: TTL/expiry в одном localtime-запросе (`find_expired_for_cleanup`), lazy backfill-записи в read-путях, auto-commit failure fail-open («считаем, что изменения были») | Поведенчески ок для тулзы; неаудируемо для control plane |

---

## 7. Пробелы нашей KB, которые VK вскрыл (конкретные дописки)

| # | Пробел | Куда дописать |
|---|---|---|
| G-1 | **Session-semantics дивергенция**: три несовместимых класса follow-up (resume-in-place + message rollback / fork-on-follow-up / prompt-stuffed) и их следствия для leases, цены retry, checkpoint reset. Нужна per-driver capability matrix (аналог VK `BaseAgentCapability`: SessionFork / ContextUsage / reset-to-message) | `architecture/executor-sessions.md` — новый раздел «Follow-up capability classes» |
| G-2 | **Тестируемое определение worktree "invalid"**: KB требует never-auto-repair-invalid, но не определяет invalid. Взять 3-фазную проверку VK (fs path + gitdir back-pointer в `<commondir>/worktrees` + libgit2 registration), инференс orphan-repo изнутри, порядок surgical cleanup, ловушки нормализации путей (macOS `/private`, Windows UNC), per-path locking. Плюс поправка invalid+clean vs invalid+dirty (auto-recreate vs quarantine+HAQ) | `architecture/state-and-truth.md` (worktree-раздел) + отдельная spec Governor'а; поправка к трёхсостоянийной модели |
| G-3 | **Механика merge**: в KB ноль вхождений «squash». Неспецифицированы: стратегия merge per tier, gate base-ahead-blocks-merge, in-memory ref-only merge vs working-tree merge (двойной путь по месту checkout базовой ветки), post-merge reset task-branch ref, conflict-op state machine (rebase/merge/cherry-pick/revert detect + правильный abort), dirty-check-before-rebase. `git_ops_safety.rs` (1349 строк) — готовый conformance-набор для ссылки | Новый документ `architecture/merge-governor.md` (или раздел в Governor-spec) со ссылкой на портированные векторы |
| G-4 | **Словарь нормализованного транскрипта**: KB говорит «контрактный канал (JSON-события) + PTY-эвристика», но схему событий не задаёт. Взять NormalizedEntry/ActionType/FileChange/ToolStatus + фиксы FF (stable IDs, версионирование потока, per-step cost) как письменный RunEvents-контракт | `architecture/state-and-truth.md` или новый `architecture/run-events.md` |
| G-5 | **Process-lifecycle edge cases**: dual exit detection для never-exiting агентов (exit_signal), process-group kill осиротевших MCP-детей, cancel→grace→kill с таймаутами, spawn timeout, порядок terminal-status-write vs log-flush handshake, startup orphan sweep с best-effort provenance | `architecture/executor-sessions.md` — раздел «Run supervisor lifecycle» |
| G-6 | **Per-repo git-provenance схема**: before/after_head_commit + merge_commit per (run, repo), захват при spawn/exit/kill/crash — хребет checkpoint reset и EvidencePack provenance | `architecture/state-and-truth.md` — схема Run/Step |
| G-7 | **HAQ wire-механика per driver class**: точки вставки на уровне протоколов (CC `can_use_tool` control_request через stdin; Codex server-initiated JSON-RPC approvals; ACP `request_permission`; OpenCode HTTP permission-reply с обязательным message при reject), create/wait split, per-protocol timeout-семантика, структурные Q&A-формы, трансляция denial-reason → feedback-turn | `architecture/ui-observability.md` (HAQ) + executor-sessions.md |
| G-8 | **MCP-инъекция в раны**: как каждый harness узнаёт о `forgeplan serve` per worktree — дивергенция форматов (JSON vs TOML vs JSONC, шесть схем server-entry) по каталогу shape-адаптеров VK | `architecture/executor-sessions.md` — spec per-worktree injection per driver |
| G-9 | **Контракт персистентности review-комментов**: data shape человеческого review-входа (file/line/side-anchored), компиляция в промпт, персист как evidence-linked artifacts с resolution status | `architecture/ui-observability.md` + evidence-контракт |
| G-10 | **Board wire protocol**: snapshot+Ready+live JSON-Patch per entity keyspace, синтез членства при фильтрах, batch summary DTO, триаж-предикат needs-attention, txid optimistic-write handshake | `architecture/ui-observability.md` — раздел «Projection transport» |
| G-11 | **Дистрибуция самого ff**: в KB «distribution» — только про skills (skill-forge.md). npx thin-shim канал не описан | `architecture/planes.md` или новый ops-документ; VK `npx-cli/` как референс |
| G-12 | **Семантика очереди follow-up**: судьба queued message при failure рана (VK молча discard'ит), глубина/порядок очереди, snapshot executor-config при enqueue | Task state machine spec |

---

## 8. Дельта к плану

### Phase 0 (фундамент) — изменения

1. **Build vs buy закрыт.** Покупать нечего: ближайший shipped-продукт мёртв,
   и причина смерти (координация не монетизируется вне enterprise
   governance / token resale) означает, что рациональный вендор VK-as-product
   не пересоберёт. Конкурентный риск сместился с «funded VK нас перегонит»
   на «community-fork VK живёт как бесплатный commodity UX» — приемлемо:
   форк наследует no-gates/no-evidence архитектуру, т.е. конкурирует с той
   частью FF, которая никогда не была moat. **Средний путь — форкнуть VK как
   базу FF — отвергнут явно**: субстратные решения (SQLite hook-шина,
   in-memory координация, human-as-scheduler, auto-repair worktrees,
   advisory scoping над неаутентифицированным API) противоречат ядру FF;
   сэкономленное время ушло бы на борьбу с фундаментом.
2. **BUILD коммодити-слоя резко дешевеет**: ~15–20k LOC production-отлаженного
   Apache-2.0 plumbing в карьер (git CLI wrapper + worktree lifecycle +
   merge-safety тесты; референс-драйверы всех четырёх стилей интеграции;
   log clustering / patch transport; live diff engine; npx-дистрибуция) —
   реалистично месяцы отладки, особенно git-safety и process-lifecycle
   edge cases.
3. **Объём ручной работы по драйверам сжимается**: с «написать 3–4 адаптера
   с нуля» до «написать один контракт (trait + RunOutcome + error taxonomy +
   capability declaration) + адаптировать 3 vendored референса»
   (acp/harness, claude/protocol, codex/jsonrpc). Dumb-pipe адаптеры —
   отказ на уровне политики.
4. **Схему projection DB стартовать с четырёхуровневой модели VK**
   (Task/Run/Step ≈ Workspace/Session/ExecutionProcess + RepoState +
   Turn-bookkeeping), сразу добавив lease-колонки, gate-state, verdicts.
5. **С первого дня в монорепо**: AGENTS.md==CLAUDE.md single source,
   scoped per-directory briefs, ts-rs (или аналог) генерация типов с
   `--check` drift-гардом в CI; никаких зависимостей на личные форки.

### Phase 3 (продукт/поверхности) — изменения

1. Run Inspector / Board: портировать транспорт (useJsonPatchWsStream +
   NormalizedEntry rendering + hybrid virtualizer + diff_stream) поверх
   Postgres outbox; принять txid handshake для optimistic UI.
2. HAQ: строить как расширение approvals-петли VK (create/wait split,
   in-band pending в транскрипте, denial→feedback-turn), добавив
   персистентность, SLA/эскалацию и policy-first резолюцию.
3. Review surface: UX петли VK (inline comments → prompt) + персист
   комментов как review artifacts с resolution status для verifier.
4. Дистрибуция: `npx forgefarm` по образцу npx-cli, параллельно
   brew/cargo-dist.
5. **Не строить в Phase 3**: remote access (Tailscale/SSH достаточно),
   desktop shell, preview-proxy инструментацию, любой general-PM функционал.

### Что остаётся write-from-scratch (нигде в landscape нет prior art)

Postgres lease/task state machine; policy/gate engine;
generator≠verifier верификация; EvidencePacks и evidence-first close;
eval loop (model+harness+task → EvidencePacks → routing); tiered autonomy
(T0–T3). VK, gastown, swarm-forge и Herdr — все останавливаются ровно перед
этим слоем.

### Prior-art список (закладки, не зависимости)

| Тема | Где лежит |
|---|---|
| Merge/rebase safety conformance | `dev/vibe-kanban/crates/git/tests/git_ops_safety.rs` |
| Run supervisor lifecycle | `dev/vibe-kanban/crates/local-deployment/src/container.rs` (spawn_exit_monitor L480–813) |
| ACP driver harness | `dev/vibe-kanban/crates/executors/src/executors/acp/` |
| CC stream-json control protocol | `dev/vibe-kanban/crates/executors/src/executors/claude/` |
| Codex app-server JSON-RPC | `dev/vibe-kanban/crates/executors/src/executors/codex/jsonrpc.rs` |
| Server-mode агент (HTTP+SSE) | `dev/vibe-kanban/crates/executors/src/executors/opencode/sdk.rs` |
| Live diff engine | `dev/vibe-kanban/crates/services/src/services/diff_stream.rs` |
| Checkpoint restore | `dev/vibe-kanban/crates/services/src/services/container.rs` (reset_session_to_process L633) |
| Optimistic UI over replication (txid) | `dev/vibe-kanban/crates/remote/src/{response.rs,shape_definition.rs,mutation_definition.rs}` |
| npx-дистрибуция | `dev/vibe-kanban/npx-cli/src/` |
| Device pairing / signed relay (на будущее) | `dev/vibe-kanban/crates/{trusted-key-auth,relay-ws,relay-tunnel-core,relay-webrtc}` |
| SSH-over-WebSocket (на будущее) | `dev/vibe-kanban/crates/{embedded-ssh,desktop-bridge}` |
| Docs-agent style guide | `dev/vibe-kanban/docs/CLAUDE.md` (==AGENTS.md) |

---

*Связанные документы: `synthesis/05-herdr-patterns.md` (наблюдение без
контракта — VK-D-2/D-4 дают контрактный канал, Herdr — эвристический
fallback), `decisions/D-002-acp-first-executor-driver.md` (эмпирически
подтверждён, см. §3.2 и урок 3), `synthesis/02-open-decisions.md` (закрытие
build-vs-buy — §8).*
