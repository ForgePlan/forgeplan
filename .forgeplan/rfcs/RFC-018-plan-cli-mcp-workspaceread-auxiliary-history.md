---
assigned_number: 18
created: 2026-10-23
depth: standard
id: RFC-018
kind: rfc
last_modified_at: 2026-10-23T22:32:02.710927+00:00
last_modified_by: compat-implementation-contract/1
links:
- target: SPEC-008
  relation: based_on
- target: PRD-085
  relation: based_on
- target: RFC-015
  relation: refines
- target: ADR-028
  relation: based_on
- target: ADR-029
  relation: based_on
- target: NOTE-054
  relation: based_on
prd: null
predicted_number: 18
slug: rfc-plan-cli-mcp-workspaceread-auxiliary-history
status: draft
title: 'Plan: прежние CLI/MCP через WorkspaceRead и auxiliary history'
updated: 2026-10-23
---




---
assigned_number: 18
created: 2026-10-23
prd: null
predicted_number: 18
slug: rfc-plan-cli-mcp-workspaceread-auxiliary-history
updated: 2026-10-23
---

## Status

draft; автор openai-codex/gpt-6.1-sol. База c852f40c515085d328f02e4b275edda37279a610. NOTE-054: C4 SUFFICIENT; исправленный body pin `b1d302e52e6c2e6c9b6b8d9221f0820e43b89ea465a806246ab85edb0dbc32ba` (UTF-8 от первого `## ` до EOF, без frontmatter). Source writes и behavioural acceptance этим автором не выполнялись.

### Фактическое состояние интеграции после R3

Результат исполнителя зафиксирован в EVID-201 (`informs` RFC-018), а полный неизменяемый отчёт — `.local/constellation-verification/compatibility-20261025/quality-r3-finish-sole/`. На source pin `a83301cb4772ebe61aa08cb66c8b3103667249a058feab1f797683db774ed171`: 320 PASS / 4 FAIL / 0 ignored / 29 непустых binaries; 39 реальных full-host EOF0, 814 вызовов. Четыре process-publication FAIL и тестовый semantic E0106 не закрыты производственными успехами. Настоящая локальная BGE9/9, автоматическое обновление embeddings после правки в обоих full хостах, offline edit/reindex/reopen и отдельные потребительские сценарии прошли. Это не полная приёмка 103/73.

Открыты typed-failure/изоляция: malformed links mapping, startup-X activity при request-Y, абсолютный playbook-Y, corrupt-FPF-KB panic. CLI0.38 уже не создавал receipt для deprecate/supersede и возвращал parsed legacy body: эти базовые ограничения не превращаются в новые функции ради ложного parity. Полные raw наблюдения и последующие классификации сохраняются.

После safe release тестовый lifetime исправлен отдельно; initial94-file pin `c04e78caef3e0bc0c342f7ed56aa5411d643df14ace88f2cc32ce23e2290d200`, остальные93SHA прежние. Эта поправка и следующие адресные правки ещё ждут новой последовательной проверки; R3 результаты не переименовываются в R4. Пользователь разрешил очистку только target; последующий native df допускает новую сборку. Новые writers MiniMax M3.1 Flash max работают в отдельных копиях; независимое ревью, исполнение тестов, docs и gate — Sol6.1.

Фазы и acceptance ниже не отмечаются завершёнными. Успешный внешний provider, инструментальное Rust coverage и G1/G2/G3 остаются неподтверждёнными; PRD/RFC не активируются, настоящие проекты не переключены.

## Summary

Точечно восстановить 103 прежних CLI paths + 73 full MCP tools на новом хранении без переписывания скиллов. Это сумма поверхностей, не декартово произведение.

## Motivation

Оставшиеся store-bound CLI, потеря history/context и nested playbook обходят уже готовые seams; help/schema совместимость не доказывает поведение.

## Context

SPEC-008 / PRD-085; уточнение RFC-015 при ADR-028/ADR-029. PRD-073 — исторический mutation-only контекст, не ограничение reads/FPF. Основание — NOTE-054 и готовые CompatCLIInventory §7 / CompatMCPInventory / CompatSkillConsumers / CompatResearchSynthesis.

## Goals

Сохранить прежние argv/flags/defaults/aliases, exits/stdout/stderr, JSON/types, IDs/resolution, hints, RPC errors, gates и действительные side effects. Не откатывать intentional fake-empty hardening.

## Non-Goals

Нет новых features, generic facade, scoring engine, retired artifact tables, skill rewrite или нового PRD/SPEC/ADR tree. Draft validation + independent checked plan допускают source; active-RFC-before-code НЕ требуется. G1/G2/G3 остаются; фактическое code coverage >=80% — POST-code. Activation только после code/review/evidence, не этим автором.

## Module Breakdown

- CLI/MCP adapters — прежний public contract, существующий read/mutation owner.
- Core feature functions — общие прежние records/relations algorithms.
- Auxiliary metadata — прежняя change_log и anomaly journal, не второй artifact index.
- Native playbook dispatcher — owner-aware прежние операции/DispatchOutcome.

## Component Diagram (prose)

CLI/full MCP разрешают workspace/owner до legacy open; read направляется в существующий Legacy либо один свежий WorkspaceRead, затем в прежний feature algorithm. Mutation под WorkspaceLock пишет canonical Markdown и прежний audit event, unlock предшествует reconcile. Native playbook использует этот canonical/core путь, а не legacy-only open.

## Data Flow

Validation получает WorkspaceRead::frontmatter_map_with_links; scoring — прежний captured assurance. History читает старые и новые rows без artifact store. Git-sync/reindex сохраняют прежние diff/event rules. Anomalies под lock: load → полный AnomalyScan → persist scan.journal, затем прежний filtered report; report не источник journal.

Capture/CURRENT/table/KB/parse failure — не healthy empty. Audit/journal write failure сохраняет прежний warning/primary result, не rollback и не полная записанная history. Startup storage_io unexpected_row ADR-001#s0 сохраняется как index_pending warning; force/repair real workspace запрещены.

## Function Signatures / Component Contracts

Ниже минимальные изменения, не новая generic abstraction. Все Path — разрешённый каталог `.forgeplan`, не его parent.

- Существующие common::ReadSource/open_read_source, WorkspaceRead::records/resolve_id/get_all_relations/frontmatter_map_with_links остаются без изменений, кроме доказанно необходимой локальной правки.
- `AuxiliaryStore::open_history_existing(workspace: &Path) -> anyhow::Result<Self>` async — открывает только connection/change_log, ничего не создаёт и не мигрирует; missing history допустима, corrupt/unreadable существующая table — Err.
- `AuxiliaryStore::open_history_or_create(workspace: &Path) -> anyhow::Result<Self>` async — caller уже держит WorkspaceLock; создаёт только отсутствующую change_log по существующему schema::change_log_schema, сохраняет все прежние rows; не создаёт FPF или artifact tables.
- `AuxiliaryStore::log_change(&self, entry: &ChangeLogEntry) -> anyhow::Result<()>` async — append прежнего восьмиколоночного формата; missing handle — Err, не silent drop.
- `AuxiliaryStore::get_change_log(&self, artifact_id: Option<&str>, source: Option<&str>, limit: usize) -> anyhow::Result<Vec<ChangeLogEntry>>` async — отсутствие history возвращает пустой список только после подтверждённого отсутствия table; иначе прежний escaping/filter/timestamp-desc/truncate.
- `db::auxiliary::change_log_append(table: &Table, entry: &ChangeLogEntry) -> anyhow::Result<()>` и `change_log_query(table: &Table, artifact_id: Option<&str>, source: Option<&str>, limit: usize) -> anyhow::Result<Vec<ChangeLogEntry>>` async, crate-visible — вынести существующие query bodies без изменения алгоритма. LanceStore::log_change/get_change_log делегируют им, сохраняя прежнюю ошибку отсутствующего handle. schema.rs не меняется.
- `common::log_change(workspace: &Path, store: Option<&LanceStore>, artifact_id: &str, action: &str, source: &str)` и `common::log_change_field(workspace: &Path, store: Option<&LanceStore>, artifact_id: &str, action: &str, (field, old_value, new_value): (&str, Option<&str>, Option<&str>), source: &str)` async возвращают `()`; legacy пишет один раз через прежний sink, captured — через history handle. Их best-effort warning остаётся прежним. Caller держит текущий lock; повторно lock не брать. Формирование entry использует существующий ChangeLogEntry builder.
- Уже есть `detect_anomalies_with_journal(store, workspace, filter) -> Result<AnomalyScan>`, `detect_anomalies_captured_with_journal(read, workspace, filter) -> Result<AnomalyScan>` и `persist_anomaly_journal(workspace, &scan.journal) -> Result<()>`, все async. Переиспользовать, не придумывать второй journal API. CLI/MCP lock охватывает load → scan → persist; прежние anomaly keys/timestamps/filter/resolution GC не менять.
- FPF owner добавляет `dashboard_captured(read: &WorkspaceRead, config: Option<&FpfConfig>) -> Result<FpfDashboard>` async, `contexts::detect_for_artifact_captured(read: &WorkspaceRead, id: &str) -> Result<Option<(String, usize, f64)>>` async и `llm::reason::build_fpf_context_auxiliary(store: &AuxiliaryStore, title: &str, body: &str) -> Result<Option<String>>` async. Legacy wrappers и эти адаптеры используют общие прежние computations/keyword-selection, без второго scoring/context engine. Existing captured fpf_check уже пригоден. Наличие KB проверяется через AuxiliaryStore, не artifact store; `.ok()` не маскирует corrupt table как отсутствующую.

## Affected Files

### Owned grid / dependencies

Обозначения: C = crates/forgeplan-cli/src/commands; K = crates/forgeplan-core/src; M = crates/forgeplan-mcp/src. Каждый список ниже — исключительное владение. Один интегратор/Main и один tester; авторы не запускают Cargo/build/test/lint/fmt.

FROZEN сейчас: существующие common::ReadSource/open_read_source и WorkspaceRead interfaces, включая variants/signatures. W0 metadata additions в common.rs их не меняют. Ready read groups могут начинать параллельно W0 после source admission и подготовки worktrees Main; общего барьера завершения metadata нет. Ожидание касается только вызываемого нового API, не всей группы или волны.

| Волна/owner | Точные owned paths | Dependency / результат |
|---|---|---|
| W0 common metadata | C/common.rs только при необходимости; K/db/auxiliary.rs; K/changelog/mod.rs; K/anomalies.rs; K/db/store.rs только делегация history queries | Фиксирует новые history API/locking для их потребителей; не блокирует readers на FROZEN interfaces |
| W1-1 validation | C/validate.rs; C/gaps.rs; K/gaps/mod.rs | Готова сразу на FROZEN read contract, параллельно W0; validation/gaps используют существующие records/links computations |
| W1-2 listing | C/list.rs; C/stale.rs; C/progress.rs; C/calibrate.rs | Готова сразу на FROZEN read contract, параллельно W0; прежние фильтры/output |
| W1-3 graph | C/graph.rs; C/blocked.rs; C/order.rs; C/tree.rs; C/dispatch.rs | Готова сразу на FROZEN read contract, параллельно W0; прежние edges/parent_epic/blockers/cycles/FM hydration |
| W1-4 composite | C/context.rs; C/estimate.rs; C/calibrate_estimate.rs; C/fgr.rs | Reads готовы сразу; только конкретный context-dependent caller ждёт используемый новый W1-7 API; outputs/LLM opt-ins прежние |
| W1-5 memory | C/recall.rs; C/remember.rs | Reads готовы сразу; file-first remember/forget сохранить; только audit wiring ждёт history API и release remember.rs |
| W1-6 phase | C/phase.rs; C/phase_advance.rs | Готова сразу: существующие resolver/state API, canonical ID/advisory fallback и WorkspaceLock, без зависимости от history |
| W1-7 FPF/LLM core | весь C/fpf.rs; K/fpf/mod.rs; K/fpf/contexts.rs; K/llm/reason.rs | Работа на существующем AuxiliaryStore/read API готова сразу; единственный owner новых core context APIs, без общего W0 барьера |
| W1-8 export/history | C/export.rs; C/log_cmd.rs | C/export.rs готов сразу на FROZEN read contract; только C/log_cmd.rs ждёт W0 history API; прежние outputs |
| W1-9 maintenance | C/migrate.rs; C/watch.rs | Готова сразу на existing owner-aware APIs; прежний watch argv выбирает owner без нового обязательного flag; migrate без retired tables |
| W1-10 native playbook | K/playbook/dispatch/forgeplan_core_dispatcher.rs; K/playbook/dispatch/routing.rs | Начинает на existing read/canonical APIs сразу; только вызываемый новый core helper ждёт его owner. Audit integration ждёт W0; dry-run/yes/outcomes прежние |
| W1-R read integrity | K/constellation/facade.rs; K/constellation/storage.rs; K/constellation/view.rs; K/constellation/workspace_read.rs только если необходим | Существующие SPEC-008 checks: active+missing CURRENT, corrupt CURRENT без marker, checksum и физическая доступность каждой exact pinned version; без нового read abstraction |
| W2 MCP sole owner | M/server.rs; M/types.rs и M/convert.rs только если необходимо для прежнего payload | Один owner начинает независимое wiring сразу. Только history caller ждёт W0 history API, route/reason context — новые W1-7 APIs; existing journal API уже доступен. Не ждёт независимые CLI groups; server.rs не делится |
| W2 audit sole owner = W0 author | C/new.rs; C/update.rs; C/activate.rs; C/supersede.rs; C/deprecate.rs; C/renew.rs; C/reopen.rs; C/link.rs; C/ingest.rs; C/git_sync.rs; C/reindex.rs; C/remember.rs только если вызывается изменённый helper | Только после release затронутых файлов; все прежние logging callsites и пропущенные captured counterparts, без новых придуманных event classes. Playbook/MCP logging, если baseline его содержит, подключают их единственные владельцы |
| Integrator/Main | crates/forgeplan-cli/src/main.rs только при необходимости; CHANGELOG.md; общие CLI/MCP integration tests и harness (владение новыми проверками — по local://compatibility-writer-test-ownership.json; авторы пишут свои файлы, интегратор только собирает и запускает); Cargo manifests только при доказанной необходимости | Интеграция всех slices, сохранение чужой docs delta. M/lib.rs и M/main.rs не меняются, кроме конкретного провала host acceptance с явным передачей ownership |
| Один tester | исполнение интегрированной проверки, raw receipts, fingerprint-linked EVIDENCE | После landing всех slices и независимого review; только он строит и запускает проверки |

Историческая запись исходного планирования, не текущая инструкция dispatch: подготовлена только W0 копия /Users/explosovebit/Work/ForgePlan-Compat-Common-20261024, branch fix/compat-common-20261024, base c852; source writes ещё не было. Остальные writer worktrees готовит Main после проверки плана. Точное владение новыми файлами проверок — local://compatibility-writer-test-ownership.json: один автор на файл, без изменения общих наборов. Любой дополнительный файл требует передачи конкретного ownership Main до редактирования, не самовольного расширения slice. Общие integration suites правит только интегратор; in-file unit tests принадлежат owner файла.

## Options Considered

### Trade-offs

- A, переписать скиллы на constellation-only: меньше adapter work, но меняет вызовы; девять read-profile tools не заменяют full workflow.
- B, снять guard/создать legacy tables: меньше локальных правок, но второй owner, migrations/shadow index и неверные данные.
- C, существующий read seam + canonical writers + auxiliary metadata: сохраняет вызовы/алгоритмы/history; требует точечного переноса и явного metadata writer.

## Trade-off Analysis

A нарушает неизменность потребителей; B нарушает single-owner; C сохраняет обе границы ценой adapter/callsite parity work.

## Proposed Direction

C задан пользователем/ready reports. Обязательный native reason SPEC-008 выполнен: ranking H4 → H1 → H2 → H3, то есть evidence discipline и один WorkspaceRead, не отдельный анализ вариантов A/B/C. Это НЕ ADI implementation RFC. Отдельный native reason RFC-018 уже вернул анализ через `codex-cli`: receipt `local://compatibility-rfc018-native-adi.json`. Поле model в receipt пустое; оно не доказывает конкретную модель вызова. Следующий шаг receipt — повторная validation, не code/runtime acceptance. Independent checked plan остаётся условием source admission; активация до кода не требуется.

## Implementation Phases

- [ ] W0 и ready W1 readers параллельно: новые history API отдельно от уже FROZEN read contract; no retired artifact tables.
- [ ] W1: file-disjoint adapters/core/read integrity; ждать только фактически вызываемые новые APIs, затем release owned files.
- [ ] W2: один MCP owner и последовательное подключение audit callsites.
- [ ] Main: интегрировать, независимый review, обновить фактический CHANGELOG.
- [ ] Один tester: выполнить полный acceptance ниже, EVIDENCE; activation отдельно после code/review/evidence.

## Test Strategy Hooks / Acceptance

NOT_RUN этим автором. Один tester фиксирует case → command → expected/observed → exit → fingerprint; 103 прежних CLI paths (98 leaves содержательно, 5 groups dispatch/help) + 73 full MCP tools, не 103×73. Schema/help или MCP-аналог не закрывают поведение CLI.

1. Paired disposable nonempty legacy/post-index corpora, empty/negative fixtures; 0.38 открывает только legacy fixture. Проверить прежние flags/defaults/aliases (включая guard.target_phase), ID/slug/legacy refs, exact Unicode/body/unknown FM/timestamps/links, counts/verdicts/evidence/score/hints, stdout/stderr/exit, JSON types/envelopes и RPC/isError. CLI list/tree bare arrays и validate single/all/CI/adversarial/phase side effects — отдельно.
2. Потребительские gate-check и claim→get→update→link→validate→release, TTL/conflicts; lifecycle happy/blocked с прежними predicates без force; supersede/deprecate/delete/restore/undo, receipt bytes/consumption/collision/missing. Baseline planning-only/stubs не объявлять реализованными writers.
3. History: старые change_log rows + все прежние новые mutation events, включая ingest/git-sync/reindex, source/filter/limit/order/commit после reopen/restart. Append/save failure сохраняет старые warning/primary result, не выдаётся за полную history; failed canonical mutation не создаёт success event. Anomalies: scan → cutoff → scan → restart; old excluded/new included, resolved GC; filtered scan не стирает прочие entries; CLI/MCP cross-poll под lock.
4. Native playbook через CLI и full MCP: каждый new/validate/activate/search/ingest-apply; отдельно dry-run без записей, yes/consent и gate rejection. Успех внешнего handler не закрывает nested path.
5. Workspace: request > env > default > startup; все 55 старых workspace-capable tools из X в Y при неизменном X; 13 parameterless — target-rooted processes; пять parameterized без workspace — прежние CWD/HOME. Оба full hosts (CLI и standalone): initialize/list/calls/EOF exit 0, external edit/live migration/read-after-write/restart. Startup effects и query effects раздельны.
6. Отдельные fixtures: active+missing CURRENT, corrupt CURRENT с/без marker, manifest checksum, отсутствие каждой physical pinned version, invalid/unreadable source/YAML/links, corrupt против absent KB. Ошибка не empty-success. Baseline table-set diff: surviving evidence/change_log допустимы; retired artifact tables не создаются вновь.
7. Route получает прежний FPF context, reason — graph-derived bounded context. Keyword/smart/semantic, feature on/off, cache present/absent/provider branches проверяются по meaningful values и честному fallback; реальное новое semantic ranking не обещает byte parity со старым unwired path. Mock не заменяет обязательный real-provider/model case; недоступное — NOT_RUN с blocker.
8. Расширить existing CLI local-compat/migrated-write/resolver/playbook/hint и MCP local-compat/migrated-e2e/process/worktree suites из reports. Один последовательный integrated run и независимый review; фактическое code coverage >=80% POST-code. Evidence содержит source/binary/model hashes, flags/platform limits. FAIL/NOT_RUN не закрывают acceptance; G1/G2/G3 остаются.

## Risks & Open Questions

| Risk | Likelihood / impact | Mitigation |
|---|---|---|
| Второй artifact owner | medium / high | owner check до open; baseline table-set assertions, guard сохраняется |
| Потеря старой/новой history | high / high | прежняя schema/query, один metadata writer, warn при failure, restart tests |
| Гонка монолитного adapter/общих helpers | high / high | один server.rs owner; FROZEN read APIs, точечные API dependencies; audit после release |
| Ложный parity PASS | high / high | отдельная строка каждой старой поверхности и required режима, meaningful values, native nested ops |
| Известный startup index_pending | observed / high | не repair/force; сохранить warning и отделить artifact write от publication ACK |

## Related Artifacts

- SPEC-008: parent, based_on.
- PRD-085: goal context, based_on.
- RFC-015: existing implementation context, refines.
- ADR-028 и ADR-029: existing read/owner constraints, based_on.
- NOTE-054: C4 SUFFICIENT research, based_on; исправленный C6 pin закреплён в Status.
- Новые EVIDENCE создаются после code/test, их IDs здесь не выдумываются.

## References

agent://CompatCLIInventory; agent://CompatMCPInventory; agent://CompatSkillConsumers; agent://CompatResearchSynthesis; agent://CompatResearchC4.
local://compatibility-v038-cli-help-20261023.json; local://compatibility-v038-mcp-schema-20261023.json; local://compatibility-v038-v039-mcp-schema-diff-20261023.json.


### R5: кандидат для пробного использования, не выпуск

На снимке `5a7523e32a66bdf9de7d80205619c197dae688a726b4a09ca45cd3c310e02785` (442 входа) выполнен сокращённый по прямому приоритету пользователя прогон. EVID-202 фиксирует 22 PASS/0 FAIL/0 ignored в полном MCP-target, fmt/check exit0, свежие semantic-capable CLI и standalone MCP без test-helpers. На обоих full-хостах проверены реальный поиск playbook в Y, одинаковый PRD-001 для claim/conflict/release, отказы unsafe/без consent, журнал Y и неизменность всех байтов X после завершения стартового сканирования. Проверка init отдельно покрыла envY и неподдержанный raw workspaceY: действие и журнал остаются в фактическом корне X. Всего 6 достоверных full-host сессий, 32 RPC-запроса, 20 tool calls, EOF/exit0. Два неверных измерения сохранены с объяснением, не объявлены дефектами продукта.

R4: 68 PASS на прежнем 425-input снимке; R3: реальная BGE и автоматическое embedding на прежних бинарниках. Эти результаты не переименованы в R5. У текущего кандидата включён semantic-search, но новое runtime-доказательство autoembedding не выполнялось. Полные unit/doc/integration/schema проверки, strict clippy и BGE/reindex-матрица отложены до выпуска по запросу пользователя. Coverage >=80%, внешний provider и G1/G2/G3 не измерены/не разрешены этим сокращением. RFC/PRD не активированы; реальные проекты и MCP-клиенты не переключены.

Кандидат: `.local/constellation-verification/compatibility-20261025/quality-r5-final-sole/production-semantic/`. Точные SHA/профили, команды и ограничения: `candidate-result.json`, `SAFELEASE_RELEASE.json` и EVID-202. Допустим следующий шаг на disposable-копиях, не заявление полной приёмки.
