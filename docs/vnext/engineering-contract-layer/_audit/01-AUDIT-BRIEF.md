# ФОРДЖПЛАН vNEXT — РЕШЕНИЕ ПО ПРОГРАММЕ

Сводный вердикт главного ревьюера по итогам 10 независимых аудитов + верификация ключевых утверждений своими руками (все команды ниже прогонялись в `/Users/explosovebit/Work/ForgePlan` на ветке `docs/ddr-terminology-alignment`).

---

## 1. ВЕРДИКТ ПО ИДЕЕ

# **ПЕРЕРАБОТАТЬ**

Ядро идеи верное и подтверждается измерением, а не вкусом: сегодня `R_eff` считается по прозе, которую агент написал сам про себя — `ls .forgeplan/evidence | wc -l` → **148**, а `rg -l 'base_sha|result_sha' .forgeplan/evidence/` → **0**, и в `crates/forgeplan-core/src/scoring/` нет ни одной ссылки на git-провенанс. Проверка git-дельты вместо доверия к утверждению — это новая способность, которую текущая архитектура структурно не может дать, и её стоит строить.

Но пакет — это позиционный документ в одежде архитектурного. Три механизма, на которых держится вся программа, не специфицированы нигде: **неподделываемая идентичность актора** (07-AUTHORITY-POLICY.md:47 требует «builder ≠ verifier», FPV-07 при этом выносит identity в non-goals), **место хранения** четырёх новых классов объектов (`grep -rni 'markdown|lancedb|source of truth' docs/vnext/` → 0 попаданий при ADR-003 `status: active`), и **независимый пересчёт git-дельты** (05-EXECUTION-RECEIPT.md:30 — «changed paths reported by provider», при том что 06 объявляет «Verify the artifact, not the claim»). Без них инварианты 5/6/7 из 02-TARGET-ARCHITECTURE.md — это обещания, а не гарантии.

Второе: пакет разворачивает четыре **активных** ADR, ни разу их не назвав. Я проверил лично: `ADR-001` `status: active`, `ADR-003` `status: active`, `ADR-009` `status: active`, `ADR-011` `status: active`, при `grep -rnoE 'ADR-[0-9]{3}' docs/vnext/ | wc -l` → **0**. Правило проекта (`CLAUDE.md`, раздел про артефакты) — «supersede, do not delete»; здесь ни supersede, ни даже упоминания.

Третье: пакет не выполняет собственный контракт. `ISSUE-GOVERNANCE.md:23-32` требует восемь секций; ни одна из 16 issue не содержит все восемь (Product boundary — 0/16, Rollback/migration — 0/16, Dependencies в теле — 0/16), а `validate_pack.py:5-26` печатает «OK: 16 issues, 5 schemas», потому что проверяет только существование файлов и парсабельность JSON.

Почему не **ОТКЛОНИТЬ**: ядро (FPV-05 + FPV-06) закрывает измеримый разрыв 0/148 и не требует ни протокола, ни адаптеров, ни сервера. Почему не **ПРИНЯТЬ С ПРАВКАМИ**: три BLOCKER'а — это не правки текста, а решения, которых пока не существует, и одно из них (ADR-001 ↔ ADR-009) требует человека.

---

## 2. КОЛЛИЗИИ С СУЩЕСТВУЮЩИМИ РЕШЕНИЯМИ

Всё, что должно быть закрыто **до старта FPV-01**. Проверено: `status:` каждой ADR прочитан напрямую.

| # | Что сталкивается | Вердикт | Требуемое действие |
|---|---|---|---|
| **B1** | `ADR-001` (**active**, `:30` «Отвергаем adapter traits. Forgeplan НЕ интегрируется напрямую с внешними системами») ↔ `09-ADAPTER-ARCHITECTURE.md:52-61` (ports: ContractReader / ExecutionRegistrar / EvidenceSubmitter / VerdictReader / PolicyEvaluator) + `11-ORCHESTRATOR-INTEGRATIONS.md:13-27` | **UNDECLARED_SUPERSESSION** | Либо `forgeplan supersede ADR-001 --by <new-adr>` с явным разбором, какое из пяти обоснований ADR-001 перестало держаться, либо сузить ports так, чтобы они доказуемо не были external-system traits. Третьего нет. |
| **B2** | `ADR-001` (**active**, «AI agent is the orchestrator, not Forgeplan») ↔ `ADR-009` (**active**, `:45` «Forgeplan-core становится оркестратором»). Ни один не объявляет supersede другого. | **PRE-EXISTING CONTRADICTION** — существует независимо от vNext | FPV-01 AC «One canonical ADR is active» **недостижим**, пока человек не выберет сторону. Это отдельный PROB (см. §5), а не часть boundary-ADR. |
| **B3** | `ADR-003` (**active**, `:25` «Markdown файлы = единственный source of truth») + RED LINE #8/#11 ↔ пакет нигде не говорит, где живут WorkContract / ExecutionReceipt / EvidenceBundle / VerificationVerdict + append-only authority log. `grep -rn '\.forgeplan' docs/vnext/` → 1 попадание, и то read-only. | **NEEDS_HUMAN_DECISION**, блокирует FPV-03/04/05 | SPEC до FPV-03: по каждому объекту — git-tracked markdown/JSON в `.forgeplan/`, gitignored derived index, или внешний стор. Машинно-пишущиеся объекты (receipts, verdicts, audit events) в git-tracked дерево артефактов не кладутся без поправки к ADR-003, иначе RED LINE #11 становится неисполнимым. `ADR-018:38-42` уже отказал «второму authoritative не-markdown стору» — переиспользовать это рассуждение, не переигрывать. |
| **B4** | `ADR-011` (**active**, «Plugin/Agent dispatchers invoke `claude --print` directly») + `crates/forgeplan-core/src/playbook/dispatch/agent_dispatcher.rs:69` ↔ `01-PRODUCT-BOUNDARY.md:36,40` («не является coding agent / general-purpose workflow engine»), FPV-15 non-goal «launching agent processes» | **CONTRADICTION**, шипнутый код против объявленной границы | FPV-01 обязан назвать судьбу playbook-runtime (5 диспетчеров + CLI `playbook.rs`/`ingest.rs`): KEEP / MOVE-TO-EXTENSION / DEPRECATE. Пакет упоминает `dispatch` (безобидный, pure-read) и **не упоминает** playbook вообще. |
| M5 | `ADR-006` (**active**, `:57` «R_eff = min(evidence_scores) — НИКОГДА не average») + `ADR-002` (**active**, «R_eff пропускает non-active зависимости») ↔ `06-EVIDENCE:66` «R_eff = min(required_claim_scores)» | UNDECLARED_AMENDMENT | Поправка к ADR-006 в стиле «Amendment 1/2» ADR-003; явный ответ, выживает ли правило ADR-002; синхронное обновление `CLAUDE.md` §Key formulas. Плюс dry-run, печатающий старый/новый score для 65 существующих PRD до переключения. |
| M6 | `ADR-005` (**active**) lifecycle-словарь ↔ `06-EVIDENCE:44-50` verdict `stale` и `05-RECEIPT:36-46` receipt-status `superseded` | CONTRADICTION (словарь) | Разнести namespace в схемах Protocol v1 (`verdict.stale`, `receipt.superseded`), иначе JSON-потребитель спутает их с терминальными статусами артефакта. Плюс поправка к ADR-005 про authority-precondition на `activate` (07:31,47,51). |
| M7 | `ADR-008` (**active**, `:32` «New command `forgeplan agent-manifest`… Versioned schema (semver)») ↔ FPV-09 CapabilityManifest | DUPLICATE | Слить: FPV-09 реализует `agent-manifest` под именем Protocol v1 и фиксирует переименование, либо supersede ADR-008 §2. Две конкурирующие manifest-схемы — постоянный источник drift. |
| M8 | `ADR-009` (**active**, `:51-58`, 4 примитива + Pack) ↔ `12-MARKETPLACE-V2.md:9-17` (7 категорий, без Playbook и Mapping) | UNDECLARED_SUPERSESSION | FPV-11 обязан дать явную карту old→new и судьбу `EPIC-007` (5 packs) и PRD-065/066/067. |
| M9 | `ADR-012` (**active**, slug canonical / `assigned_number` выставляется CI на merge) ↔ FPV-03 «identical canonical digest» при `source.artifacts[].id` как голой строке | NEEDS_DECISION, блокирует FPV-02/03 | Protocol v1 обязан зафиксировать: `ArtifactReference.id` = **slug**, display number — опциональное недайджестируемое поле. Golden-тест: контракт, скомпилированный до и после merge на неизменном графе, даёт один digest. |
| M10 | Marketplace `ADR-014` (**active**, `:126` «DD-5 DEMAND SIGNAL — ABSENT… нет подтверждённого спроса ни на один non-Claude-Code CLI») ↔ FPV-12 (Cursor+Codex+OpenCode, Tier 1+2 сразу) | CONTRADICTION | Либо supersede ADR-014 с указанием, что изменило demand-evidence, либо урезать FPV-12 до Tier 0 + один reference-адаптер. *Замечание:* в маркетплейсе **два** файла `ADR-014-*` (один `draft`, один `active`) — живой экземпляр дефекта #394 (duplicate id silently overwrites on reindex). |
| M11 | `ADR-018` (`draft`, `:71` «H2 SpaceBus — встроенный брокер… **не выживает**», `:243` «до тех пор отложено») + `PRD-081` + `SPEC-006` ↔ FPV-15 (HTTP MCP, event ingestion, webhooks, multi-repo registry, replay) | DUPLICATE отвергнутой формы | FPV-15 либо объявляет себя multi-machine продолжением, которое ADR-018 уже зарезервировал как будущее (и цитирует его), либо поглощается фазой F2 ADR-018. Также конфликт с `CLAUDE.md` §Non-Goals: «NOT SaaS… Local-first, single binary». |
| M12 | ForgePlanWeb `ADR-002` (**active**, «Mandatory dispatch → claim → execute → release для любого sub-агента») ↔ FPV-01 «решить судьбу `forgeplan_dispatch`» | CROSS-REPO IMPACT не учтён | Добавить web-ADR-002 и PRD-057 в impact-лист FPV-01. Если dispatch демотируется — web-ADR-002 supersede в той же волне. |
| M13 | `CLAUDE.md:174` («Решения — `.forgeplan/adrs/`, **единственное** место; `docs/` держит гайды и схемы, не решения») ↔ FPV-01 AC «`docs/architecture/product-boundary.md` exists and includes ownership matrix» | METHODOLOGY VIOLATION | Переписать AC: канон — ADR; `docs/architecture/product-boundary.md` допустим только как производный читательский гайд, ссылающийся на ID ADR. Две рукописные копии ownership-матрицы — ровно тот класс drift, который призван убить FPV-10. |

**Разногласие между аудиторами и моё решение.** Агент 6 (F6) утверждает: премисса FPV-01 про «dispatch scheduling» **опровергнута** — `dispatch.rs:4-8` объявлен pure-read, `CLAUDE.md:724` прямо пишет «НЕ спавнер». Агент 9 (PB-07) утверждает: «не scheduler» бессвязно, потому что в core есть job-runner. Противоречия нет, оба правы про разные поверхности: **FPV-01 пометил безобидную поверхность (`dispatch`) и пропустил нарушающую (`playbook::dispatch` с `Delegation::Command`/`Delegation::Agent`, `timeout_seconds`, `budget_usd`)**. Требование B4 сформулировано под это.

---

## 3. ВЕРДИКТ ПО ЗАДАЧАМ (FPV-00 … FPV-15)

| Issue | Вердикт | Причина (одной строкой) |
|---|---|---|
| **FPV-00** | **NEEDS_FIX** | Нет ни одного `- [ ]` AC (только проза «Definition of Done»), а его «Delivery phases» — четвёртый взаимно противоречащий источник порядка наряду с `manifest.json`, `EXECUTION-ORDER.md` и `17-ROADMAP.md`. |
| **FPV-01** | **NEEDS_FIX (gate)** | Ложная премисса про «project-management layer», отсутствует ADR-collision register (B1–B4), AC гонит решение в `docs/` вопреки `CLAUDE.md:174`; помечает `dispatch` вместо `playbook::dispatch`. |
| **FPV-02** | **NEEDS_FIX** | 7 из 11 объявленных типов не имеют схемы (AuthorityPolicy, ErrorEnvelope, ActorIdentity, CapabilityManifest, ArtifactReference, Claim/Lease, ExternalReference); AC «unknown optional fields survive round-trips» опровергается собственными схемами — я насчитал **25** `"additionalProperties": false` (WC 10, EB 6, ER 5, VV 2, EM 2). |
| **FPV-03** | **NEEDS_FIX** | Компилятор обязан собирать поля, которых в корпусе нет: `rg -ril 'forbidden.?path' .forgeplan` → 0, per-criterion evidence → 0/65 PRD; per-field provenance (04:46-55) невыразим в закрытой схеме. Нужен backfill шаблонов **до** FPV-03. |
| **FPV-04** | **NEEDS_FIX** | 9-статусное зеркало внешнего runtime нарушает собственный инвариант 7 (02:109) — нет владельца переходов, нет TTL; у receipt нет top-level `digest`, хотя 05:32 его требует, а `additionalProperties:false` запрещает добавить. |
| **FPV-05** | **NEEDS_FIX** — но **самая ценная** | Ядро программы (git-delta provenance gate, #360). Блокеры: #328 заявлен дважды (см. FPV-06), не сказано, что core **сам** пересчитывает `changed_paths` и считает данные провайдера недоверенными, и не сказано, как result SHA становится достижим локально. Слайс «#360 как отдельный PRD против текущей границы» — единственное, что можно шипить сейчас. |
| **FPV-06** | **NEEDS_FIX** | `claim_score = evaluate(...)` — имя функции без формулы, весов и диапазонов, при этом `R_eff = min(required_claim_scores)` без него бессмысленен; не цитирует **#392** (52 false-zero артефакта, 185 аномалий — доминирующий открытый дефект R_eff) и #393; молча правит ADR-006/ADR-002. |
| **FPV-07** | **NEEDS_FIX** | Правило builder ≠ verifier опирается на самозаявленную идентичность, которую сам FPV-07 выносит в non-goals; `producer`/`verifier`/`decided_by` — голые строки, `instance_id` не переносится, значит инвариант 6 неразрешим по протоколу. |
| **FPV-08** | **NEEDS_FIX** | Премисса протухла: все 10 команд PRD-070 существуют (`ls crates/forgeplan-cli/src/commands/` → 79 файлов), `@file` закрыт в v0.33.0 (#350 CLOSED), `forgeplan decay` уже шипнут; «reduce tool surface» противоречит **открытому** Epic #287 (+9 MCP tools) и уронит `scripts/check-mcp-tool-count.sh:53-58` (`-lt 30` → FAIL с вводящим в заблуждение сообщением). |
| **FPV-09** | **MERGE_WITH_OTHER** | CapabilityManifest = `agent-manifest` из активного ADR-008 (M7); плюс conformance-набор (16:27) не определяет, что значит «pass» при разных enforcement-уровнях, а 18:27 делает это релизным гейтом. |
| **FPV-10** | **NEEDS_FIX** | Phase 1 — на фазу раньше, чем существуют описываемые возможности (`grep -rn 'WorkContract' crates/` → **0**), при собственном правиле «Do not advertise unshipped features»; не покрывает 22 блог-поста с конфликтующим позиционированием; не говорит, что делать с шипнутым drift-детектором. |
| **FPV-11** | **NEEDS_FIX** | Заменяет активную модель ADR-009 (4 примитива + Pack) на 7 категорий без supersede и без места для Playbook/Mapping и шипнутых `playbook.rs`/`ingest.rs`. |
| **FPV-12** | **MERGE_WITH_OTHER + SPLIT** | Противоречит активной marketplace ADR-014 (demand-gate, M10); три хоста + Claude Code в одной issue против собственного правила «одна issue — один PR»; пересекается с открытым #363 (кто владеет `forgeplan mcp install --cli`). Разбить по одному хосту, начать с OpenCode как reference. |
| **FPV-13** | **DROP** (вернуть после B1) | Четыре оркестратора + generic SDK в одном теле; напрямую упирается в нерешённый ADR-001; в корпусе из 77 PROB нет ни одной заявки на кросс-хостовую переносимость контрактов. |
| **FPV-14** | **NEEDS_FIX** | Требует данные, которые поставляет только Phase 6 (инверсия последовательности), doc 13 не называет ни одного источника данных; «Web remains read-only» ложно — `@forgeplan/web-agent` (Claude Agent SDK daemon) уже шипнут в forgeplan-web. |
| **FPV-15** | **DROP** | Дублирует ADR-018/PRD-081 и воспроизводит именно ту брокер/демон-топологию, которую ADR-018 адверсариально отверг (5/10, «не выживает»); противоречит `CLAUDE.md` §Non-Goals (local-first, not SaaS). |

Сводка: **0 READY**, 11 NEEDS_FIX, 2 MERGE_WITH_OTHER, 2 DROP, 1 gate-issue (FPV-01) в NEEDS_FIX. Ни одну issue нельзя создавать на GitHub в текущем виде.

---

## 4. ЛОЖНЫЕ ПРЕДПОСЫЛКИ

| Утверждение пакета | Реальность | Что это обесценивает |
|---|---|---|
| FPV-01: «ForgePlan is currently described as… **project-management layer**» | `docs/methodology/FORGEPLAN-GUIDE.md:14` «**Not Jira.** Not project management. Not a task tracker.»; `METHODOLOGY-COURSE.md:22`; `CLAUDE.md` §Non-Goals «NOT project management» | AC «README… no longer call ForgePlan a task/project manager» выполняется в день ноль и даёт ложную уверенность, что работа по позиционированию сделана. Реальный конфликт — «agent harness» — живёт в 22 блог-постах, которых пакет не касается (`grep -rn -i 'blog' docs/vnext/` → 0). |
| FPV-08 AC: «No `@file` … asymmetry remains» | #350 **CLOSED** 2026-06-02, фикс в `CHANGELOG.md:26`, хелпер в `server.rs:923` | Один из четырёх пунктов FPV-08 уже закрыт; оставшийся #353 — про валидацию agent-id, где два валидатора (`validate_agent_id` strict / `_relaxed`) существуют **намеренно**, и наивная «унификация» снимет CLI-защиту от path-separator. |
| FPV-01: «dispatch scheduling может понадобиться вынести из core» | `dispatch.rs:4-8` «pure read — does not mutate»; `CLAUDE.md:724` «планер… **НЕ спавнер!**» | Помечена безобидная поверхность. Настоящий шедулер — `playbook::dispatch` (`Delegation::Command`, `budget_usd`, `timeout_seconds`) — в пакете не упоминается ни разу. |
| FPV-14: «Web remains read-only» | `agent/package.json` → `@forgeplan/web-agent`, «persistent Claude Agent SDK session… spawned as a separate process», браузер говорит с ним по `ws://127.0.0.1` в обход `/api/*` | Узкое утверждение (SvelteKit-прокси только read-only, rule 22) — верно. Широкое — ложно. FPV-14 либо противоречит шипнутому коду, либо молча регрессирует фичу. |
| FPV-08: CLI/MCP parity gap как открытая проблема | Все 10 команд PRD-070 на диске (`activity, activity_stats, claim, claims, dispatch, phase, phase_advance, release, restore, undo_last`); PRD-071 hint-контракт шипнут | Проблемное утверждение FPV-08 надо переписать против текущей поверхности; остаются только role profiles, context bundle, версионированные JSON-схемы и latency budget. |
| #328: «нужен новый CLI/MCP примитив `forgeplan_decay`» | `commands/decay.rs` существует, `main.rs:340,1242` диспетчит, `server.rs:5909` объявляет MCP-инструмент | Реальный остаток #328 — парсер `## Revisit Trigger` в активных ADR. Мелкий, шипабельный сегодня фикс, который FPV-05/FPV-06 хотят утопить в Wave-2 переписывании. |
| Координационный список 8 issue (`ISSUE-GOVERNANCE.md:48`) актуален | Список обрывается на #397; **#392** (52 ложных нуля R_eff, 185 аномалий) отсутствует во всём пакете, как и #393, #394, #410, #411 | FPV-06 не цитирует главный дефект, который якобы чинит. Список надо перевыводить из `gh issue list` в день создания issue, а не из снимка. |
| `CLAUDE.md:101` «Epic #287 ✅ (brownfield)» | `gh issue view 287` → **OPEN**, +9 MCP tools и 3 extensions, дочерняя marketplace #79 тоже open | Базовая линия «что шипнуто» ненадёжна в обе стороны; FPV-08 «reduce tool surface» и #287 «add 9 tools» идут навстречу друг другу. |
| `03-PROTOCOL-V1.md:20-24` «unknown optional fields должны сохраняться при round-trip» | 25 вхождений `"additionalProperties": false` во всех 5 схемах; `protocol_version` pattern `^1\.` принимает 1.1, который закрытая схема тут же отвергает | Минорная эволюция протокола ломающая по построению; AC FPV-02:42 недостижим без решения про совместимость. |
| `16-CONFORMANCE.md:27-34` семантическая переносимость проверяема | 07:55-59 и 10:57 разрешают уровням расходиться от `full` до `advisory` — один и тот же контракт даёт разный changed-path set и разный verdict **по замыслу** | Релизный гейт 18:27 нефальсифицируем. Разделить на (а) паритет digest+verdict при **одинаковой** наблюдаемой дельте и (б) поведенческие тесты, оцениваемые против заявленного capability-уровня. |
| `01-PRODUCT-BOUNDARY.md:85` «O — One owner per state» | В той же таблице: `:50` «Paperclip **или** бизнес-система», `:52` «Agent host **или** orchestrator»; далее 10:17 Cursor владеет worktree, 11:19/11:23 Vibe/Conductor владеют тем же | Принцип опровергнут внутри документа, который его вводит. Нужно правило приоритета для вложенного случая (Conductor запускает Cursor). |
| **Опровергнутая гипотеза (в пользу пакета)** — что один из 8 координационных issue «потерян» | Все 8 упомянуты поимённо в FPV-05/06/08 и в arch-доках 06:75-78, 08:47-50 | Механизм координации правильной формы; ломается он в другом — устаревание списка, двойное владение #328, отсутствие обратной записи в старые issue. |
| **Опровергнутая гипотеза (в пользу пакета)** — что `forgeplan scan-import` затянет 60 файлов пакета как PRD (PROB-047) | Митигация реализована: `crates/forgeplan-core/src/scan/detect.rs:80-85` глушит Tier 3 под `docs/`; ни один файл пакета не триггерит предикат PRD | Риск остаточный и узкий: `ref/` **не** под `docs/`, там защиты нет — ещё один довод удалить дубликат, а не оставлять его рядом. |

---

## 5. КУДА КЛАСТЬ МАТЕРИАЛ

Прецедент — `99d0cf0` / `a4af8e6` (space-mesh): **сырьё и рассуждение → `docs/`, связывающие решения → `.forgeplan/`, взаимные ссылки в обе стороны, ничего не активируется без кода и evidence.**

### 5.1 Что остаётся на месте (файлы не двигаем)

| Компонент | Решение | Обоснование |
|---|---|---|
| `docs/vnext/engineering-contract-layer/architecture/03…18` (16 не-решенческих док) | **KEEP** | Прямой аналог `docs/space-mesh/…handoff.md`: материал, который будущие ADR/PRD/SPEC будут цитировать. `docs/` уже держит неиндексированные рабочие каталоги (`architecture/`, `space-mesh/`, `audit/`, `sessions/`). |
| `architecture/01-PRODUCT-BOUNDARY.md`, `02-TARGET-ARCHITECTURE.md` | **SPLIT** — файл остаётся аннексом, связывающее решение уходит в ADR внутри FPV-01 **после** снятия B1–B4 | Их собственные заголовки («Каноническое определение», «Владение состояниями», «Неизменяемые инварианты») — это решения по `CLAUDE.md:174`. Но выпустить ADR сегодня — значит молча переехать четыре активных ADR. |
| `protocol/schemas/*.json` | **KEEP** (НЕ в `docs/schemas/`) | `docs/schemas/` содержит только 6 markdown-файлов — человеческую документацию **уже действующих** контрактов валидатора. Эти пять описывают ноль шипнутого кода (`grep -rn 'WorkContract' crates/` → 0) и покрывают 5 из 11 объявленных типов. Триггер переезда: FPV-02 сдал Rust DTO + fixture-корпус, на котором Rust и JSON-Schema согласны → публикация в `website/public/schemas/` под объявленный `$id`. |
| `scripts/*.py` | **KEEP** | `git ls-files "*.py"` → пусто: в репо нет Python вообще. `/scripts/` — шесть `.sh`, работающих как CI-гейты на каждом PR. `validate_pack.py:4` резолвит корень как `parents[1]` — переезд его сломает. |
| `governance/`, `prompts/`, `issues/`, `README.md` | **KEEP** | Процессные правила и task-слой программы. `.claude/` — gitignored, туда нельзя. |
| `.github/PULL_REQUEST_TEMPLATE/forgeplan-vnext.md` | **KEEP** в директорной форме | Проверено: `.github/pull_request_template.md` нет ни на одном уровне — шаблон ничего не затеняет. Промоушен в дефолт навязал бы 43-строчную vNext-форму каждому docs/chore/release PR. Но директорный шаблон **никогда не применяется автоматически** — обязателен фикс в `AGENTS-VNEXT.md`: `gh pr create --base dev --template forgeplan-vnext.md`. |
| `docs/README.md` / `docs/README.ru.md` | **Индекс не добавляем сейчас** | `grep -n 'space-mesh\|vnext' docs/README*.md` → пусто; индексация нешипнутой программы = реклама планируемого как готового. Явный триггер: первый vNext-релиз, добавляющий user-facing CLI-флаг или MCP-инструмент, добавляет запись в оба индекса тем же PR. |

### 5.2 Что попадает в граф (4 артефакта, только через MCP — RED LINE #11)

Предсказанные номера подтверждены: сейчас максимум `EPIC-008`, `PROB-081`, `EVID-148`. В `Refs:` до merge — **слуги**, не номера (`CLAUDE.md` §Working with artifact IDs).

```
# 1. Якорь программы
mcp__forgeplan__forgeplan_new(kind="epic",
  title="ForgePlan vNext: engineering contract and verification layer")
mcp__forgeplan__forgeplan_update(id="EPIC-009", body="<Vision/Problem/Goals/Non-Goals/
  Target Users/Success Criteria по образцу EPIC-007. Success Criteria ОБЯЗАНЫ содержать
  гейт: 'Ни одна FPV issue не создаётся на GitHub, пока не закрыт
  prob-vnext-boundary-contradicts-active-adrs'. Non-Goals: публикация Protocol v1
  копии до посадки FPV-03/04/05; создание 16 PRD авансом. Тело ССЫЛАЕТСЯ на
  docs/vnext/... путями, НЕ копирует их>")
mcp__forgeplan__forgeplan_validate(id="EPIC-009")   # 0 MUST errors

# 2. Collision register — BLOCKER, гейт FPV-01 (сошлись 5 из 10 аудиторов)
mcp__forgeplan__forgeplan_new(kind="problem",
  title="vNext boundary contradicts four active ADRs with no supersession path")
mcp__forgeplan__forgeplan_update(id="PROB-082", body="<таблица по ADR: ADR-001 (active,
  :30) vs 09-ADAPTER-ARCHITECTURE.md:52-61 | ADR-003 (active, :25) vs неопределённая
  персистентность 4 классов | ADR-009 (active, :45) vs 01-PRODUCT-BOUNDARY.md:39 |
  ADR-011 (active) + playbook/dispatch/agent_dispatcher.rs:69 vs 01:36,40.
  Колонка вердикта: unaffected | amended | superseded-by-new-ADR.
  Репро: grep -rnoE 'ADR-[0-9]{3}' docs/vnext/ | wc -l -> 0>")
mcp__forgeplan__forgeplan_link(source="PROB-082", target="ADR-001", relation="contradicts")
mcp__forgeplan__forgeplan_link(source="PROB-082", target="ADR-009", relation="contradicts")
mcp__forgeplan__forgeplan_link(source="PROB-082", target="ADR-011", relation="contradicts")
mcp__forgeplan__forgeplan_link(source="PROB-082", target="ADR-003", relation="based_on")
mcp__forgeplan__forgeplan_link(source="PROB-082", target="EPIC-009", relation="informs")

# 3. Пред-существующее противоречие — переживает отмену vNext, потому отдельный артефакт
mcp__forgeplan__forgeplan_new(kind="problem",
  title="ADR-001 and ADR-009 are both active and assert opposite orchestration ownership")
mcp__forgeplan__forgeplan_update(id="PROB-083", body="<ADR-001:10 active, :30 vs
  ADR-009:11 active, :45. Ни один frontmatter links не объявляет supersession.
  Блокирует FPV-01 AC 'One canonical ADR is active'.
  Предлагаемый путь: forgeplan reason по обоим -> человек выбирает -> supersede>")
mcp__forgeplan__forgeplan_link(source="PROB-083", target="ADR-001", relation="contradicts")
mcp__forgeplan__forgeplan_link(source="PROB-083", target="ADR-009", relation="contradicts")
mcp__forgeplan__forgeplan_link(source="PROB-082", target="PROB-083", relation="based_on")

# 4. Запись аудита (форма EVID-148)
mcp__forgeplan__forgeplan_new(kind="evidence",
  title="vNext pack 10-reviewer adversarial audit: NEEDS_REWORK across all areas")
mcp__forgeplan__forgeplan_update(id="EVID-149", body="<находки по областям с file:line.
  ОБЯЗАТЕЛЬНО:

  ## Structured Fields

  verdict: weakens
  congruence_level: 3
  evidence_type: audit

  CL3 — ревьюеры гоняли команды против ЭТОГО workspace (в отличие от CL2 у EVID-148).
  weakens — аудит ослабляет утверждение, что пакет готов к исполнению.>")
mcp__forgeplan__forgeplan_link(source="EVID-149", target="EPIC-009", relation="informs")
mcp__forgeplan__forgeplan_link(source="EVID-149", target="PROB-082", relation="informs")
mcp__forgeplan__forgeplan_link(source="EVID-149", target="PROB-083", relation="informs")
mcp__forgeplan__forgeplan_score(id="EPIC-009")      # R_eff > 0 подтверждает, что поля распарсились
```

Все четыре остаются в `draft` — RED LINE #7 запрещает активацию без кода и evidence (space-mesh оставил все четыре в draft и сказал об этом прямо). Проверено: `contradicts` — валидная связь (`crates/forgeplan-core/src/link/mod.rs:101`).

### 5.3 `ref/` — удалить дубликат, сохранить провенанс

**Расхождение между входными агентами.** Placement plan перечисляет **8** уникальных файлов для переноса; loss inventory — **9**. Я проверил своими руками:

```
find ref -maxdepth 3 -type f -not -path '*/payload/*'
→ ref/.DS_Store
  ref/ForgePlan Agent Pack.zip
  ref/forgeplan-vnext-agent-ready-pack/{00-START-HERE.md, 01-IMPORT-MAP.md,
      02-EXECUTION-PLAN.md, CHECKSUMS.sha256, FILE-INVENTORY.md, MANIFEST.json,
      PROMPT-START-AGENT.md}
  ref/forgeplan-vnext-agent-ready-pack/scripts/{install.sh, validate.sh}
```

**Адъюдикация: переносим 9, не 8** — placement plan пропустил `scripts/validate.sh`. Payload при этом байт-в-байт совпадает (`diff -rq ref/.../payload/docs/vnext/ docs/vnext/` → пустой вывод, проверено лично), поэтому 60 файлов payload — чистый дубликат, и держать вторую редактируемую копию значит воспроизводить ровно тот класс drift, о котором весь этот аудит.

`ref/` **не** подпадает под правило `.gitignore` для `dev/` — то правило гласит «reference clones — external repos studied as prior art, **not ours to track**» и перечисляет пять сторонних репозиториев. vNext-пакет — собственный дизайн ForgePlan, тест «external, not ours» не проходит. Игнорировать его — оставить единственную копию `install.sh` и `CHECKSUMS.sha256` под `git clean` с видом преднамеренности.

```bash
mkdir -p /Users/explosovebit/Work/ForgePlan/docs/vnext/engineering-contract-layer/_import
cd /Users/explosovebit/Work/ForgePlan
mv ref/forgeplan-vnext-agent-ready-pack/00-START-HERE.md \
   ref/forgeplan-vnext-agent-ready-pack/01-IMPORT-MAP.md \
   ref/forgeplan-vnext-agent-ready-pack/02-EXECUTION-PLAN.md \
   ref/forgeplan-vnext-agent-ready-pack/CHECKSUMS.sha256 \
   ref/forgeplan-vnext-agent-ready-pack/FILE-INVENTORY.md \
   ref/forgeplan-vnext-agent-ready-pack/MANIFEST.json \
   ref/forgeplan-vnext-agent-ready-pack/PROMPT-START-AGENT.md \
   docs/vnext/engineering-contract-layer/_import/
mv ref/forgeplan-vnext-agent-ready-pack/scripts/install.sh \
   ref/forgeplan-vnext-agent-ready-pack/scripts/validate.sh \
   docs/vnext/engineering-contract-layer/_import/
# перепроверить идентичность НЕПОСРЕДСТВЕННО перед удалением
diff -rq ref/forgeplan-vnext-agent-ready-pack/payload/docs/vnext/ docs/vnext/ && \
  rm -rf "/Users/explosovebit/Work/ForgePlan/ref/"
# ref/ в .gitignore НЕ добавлять — это не внешний reference clone
```

После удаления payload строки `payload/*` в `CHECKSUMS.sha256` перестанут верифицироваться — это допустимо и должно быть отмечено в commit-сообщении: файл становится записью провенанса импорта, а не живой проверкой.

### 5.4 Мелкие размещения — решаю сам, не спрашиваю

| Компонент | Решение | Команда |
|---|---|---|
| `.created-issues.json` (пишется `create_github_issues.py:25` внутрь трекаемого дерева) | **GITIGNORE** — тот же класс, что `.forgeplan/session.yaml` | `printf '\n# vNext pack: per-machine GitHub issue-creation state\ndocs/vnext/engineering-contract-layer/.created-issues.json\n' >> .gitignore` |
| `.serena/` (untracked И не игнорируется) | **GITIGNORE** — прецедент `.claude/`; ценность нулевая (сток-шаблон + регенерируемый кэш), а шум в `git status` приучает игнорировать `??`-строки, что и позволило `docs/vnext/` проскользнуть | `printf '\n# Serena tool state (per-developer)\n.serena/\n' >> .gitignore` |
| `ref/.DS_Store` | уходит вместе с `rm -rf ref/` | — |
| dry-run 16 тел issue в scratchpad | **скопировать в пакет** — `create_github_issues.py` не имеет `--dry-run`, это единственная отрендеренная форма для человеческого ревью | `cp "/private/tmp/claude-501/.../scratchpad/FPV-issues-dryrun.md" docs/vnext/engineering-contract-layer/issues/DRY-RUN-REVIEW.md` |

---

## 6. ЧТОБЫ НИЧЕГО НЕ ПОТЕРЯЛОСЬ

Проверено лично: `git log --all --oneline -- docs/vnext ref/` → **пусто**. Ни `docs/vnext/`, ни `ref/` никогда не коммитились ни на одной ветке, ни в одном стэше. Их нет в object store — значит ни reflog, ни `git fsck --lost-found`, ни stash не вернут их после `git clean -fd`. Порядок ниже — по убыванию срочности; шаги 1–4 выполнить до любых других операций.

**Ловушка топологии, действует на все шаги:** `git rev-list --left-right --count origin/dev...HEAD` → `12  0`. Текущая ветка — строгий предок `origin/dev`, её работа уже влита (PR #406). Ветку под пакет резать **только от `origin/dev`**, иначе она (а) потеряет 12 коммитов dev, включая влитую работу Memory-graph-citizen (#412), которая чинит дефект, описанный в PROB-080, и (б) утащит в PR несвязанные `CLAUDE.md +86` и `PROB-047 +1`.

| № | Действие | Команда |
|---|---|---|
| **1** | Закоммитить пакет + PR-шаблон на ветке от `origin/dev`. **Первое действие сессии.** | `cd /Users/explosovebit/Work/ForgePlan && git fetch origin && git checkout -b bootstrap/vnext-pack origin/dev && git add docs/vnext .github/PULL_REQUEST_TEMPLATE && git commit -m "docs(vnext): import engineering contract layer implementation pack"` |
| **2** | Сохранить 9 уникальных файлов `ref/` (см. §5.3) — `PROMPT-START-AGENT.md` по признанию `01-IMPORT-MAP.md` «Не копируется» и существует только там | блок `mkdir`/`mv` из §5.3, затем `git add docs/vnext/engineering-contract-layer/_import/ && git commit -m "docs(vnext): preserve import manifest, checksums and install scripts"` |
| **3** | Закоммитить PROB-080 и PROB-081 (186 и 124 строки; `git log --all -- '.forgeplan/problems/PROB-08*'` → пусто). LanceDB **не** бэкап: индекс gitignored и производный по ADR-003, восстановление идёт только markdown → индекс, никогда обратно | `git add .forgeplan/problems/PROB-080-2-1.md .forgeplan/problems/PROB-081-kind-refresh-0-372-renew-reopen.md && git commit -m "docs(prob): add PROB-080 kind-catalog drift and PROB-081 unused Refresh kind"` |
| **4** | Вынести `CLAUDE.md +86` («Task List Discipline») на **отдельную** ветку от `origin/dev` — это законченная доктрина «ничего не теряется», не имеющая отношения к DDR-ветке, на которой лежит | `git checkout -b docs/task-list-discipline origin/dev && git checkout docs/ddr-terminology-alignment -- CLAUDE.md && git add CLAUDE.md && git commit -m "docs(claude): add Task List Discipline section"` |
| **5** | Сохранить dry-run 16 тел issue из scratchpad (единственная отрендеренная форма — у скрипта нет `--dry-run`) | `cp "/private/tmp/claude-501/-Users-explosovebit-Work-ForgePlan/c1dd710a-e1c9-406a-b746-ebcb60c4ef46/scratchpad/FPV-issues-dryrun.md" docs/vnext/engineering-contract-layer/issues/DRY-RUN-REVIEW.md && git add docs/vnext/engineering-contract-layer/issues/DRY-RUN-REVIEW.md` |
| **6** | Закрыть два `.gitignore`-пробела **до** первого запуска `create_github_issues.py` (иначе первый прогон закоммитит машинно-локальный маппинг) | обе команды из §5.4 |
| **7** | Положить 4 артефакта в граф (§5.2) — иначе `forgeplan_health`, `forgeplan_order` и граф не видят 16-issue программу, а это, по `CLAUDE.md`, «дефект процесса, а не экономия» | блок MCP-вызовов из §5.2 |
| **8** | Триаж `stash@{0}`: часть уже влита (`d855abc` config), часть — нет (строки атрибуции BMAD в `VISION.md`). Стэшей 14, часть протухла месяцами; реакция «попнул → конфликт → дропнул» унесёт невлитое | `git stash show -p 'stash@{0}' -- VISION.md && git diff HEAD 'stash@{0}' -- VISION.md` — извлечь hunk обычным коммитом, дропать стэши поштучно с записанной причиной, **не** `git stash clear` |
| **9** | Зафиксировать четыре процессных дефекта, найденных этим аудитом, как артефакты (иначе умрут с сессией): inert CI-гейт frontmatter (`is_new` считается против PR-checkout), никем не исполняемое требование координации 8 issue, противоречивый вердикт `forgeplan health` в одном прогоне, деградировавший slug `PROB-080-2-1.md` из кириллического заголовка | `forgeplan new problem "..."` / `forgeplan new note "..."` — через MCP/CLI, никогда `Edit`/`Write` по `.forgeplan/*.md` |

---

## 7. ПОРЯДОК ДЕЙСТВИЙ

### (a) Безопасно, локально, обратимо — делать сразу

1. `git fetch origin` и проверить топологию: `git rev-list --left-right --count origin/dev...HEAD`.
2. Шаги 1–6 чек-листа §6 (коммиты пакета, `_import/`, PROB-080/081, `CLAUDE.md`, dry-run, `.gitignore`). Всё локально, ничего не пушится.
3. Перепроверить идентичность payload **непосредственно перед** удалением, затем `rm -rf ref/` (§5.3).
4. Создать 4 артефакта в графе через MCP (§5.2), прогнать `forgeplan_validate` на EPIC-009 и `forgeplan_score` на EPIC-009 после линковки EVID-149 (подтверждение, что три structured fields распарсились и `R_eff > 0`).
5. Прогнать `forgeplan health` и убедиться, что новые `contradicts`-рёбра проявились в `forgeplan_contradictions` (это цель — сделать коллизию машинно-видимой), а не сломали health-вывод.
6. Контент-фиксы в пакете (правки текста, не решения):
   - `AGENTS-VNEXT.md` — открывающий пункт о приоритете: «CLAUDE.md и AGENTS.md авторитетны; этот файл добавляет vNext-правила и никогда их не переопределяет»; ссылка на него из корневого `AGENTS.md`.
   - `AGENTS-VNEXT.md` + `ISSUE-BUILDER.md` — контракт тела Evidence (`## Structured Fields` с `verdict:`/`congruence_level:`/`evidence_type:`); `--base dev`; `--template forgeplan-vnext.md`; человеческий гейт перед любым `git push`/`gh pr create`; pre-PR пайплайн `cargo fmt && cargo fmt --check && cargo check && cargo test && cargo clippy --workspace --all-targets -- -D warnings`; удалить «or equivalent receipt» из `ISSUE-GOVERNANCE.md:41`; правило merge commit (не squash).
   - `AGENTS-VNEXT.md:11` / `ISSUE-BUILDER.md:9` — `forgeplan context` требует позиционный `<ID>` (`main.rs:409-412`); переписать как `forgeplan health`, `forgeplan route "<title>"`, `forgeplan context <ID> --json`.
   - `PROGRAM-COORDINATOR.md:13-14` — после переноса: «пакет уже установлен в `docs/vnext/engineering-contract-layer`; перейти к `validate_pack.py`».
   - `README.md:3` — «канонический» переформулировать: связывающие решения живут в `.forgeplan/adrs/`, пакет — аннекс рассуждения; добавить ID EPIC для перехода пакет → граф.
7. Укрепить `validate_pack.py` (сейчас он зелёный при 0/16 соответствии собственному контракту): проверка обязательных секций по `bodies.json`, детект циклов, `dep.phase < issue.phase`, байтовый diff `.md` против `bodies.json`, сверка `EXECUTION-ORDER.md` с фазами манифеста. Починить `next_issue.py` — обе ветки `return False` при отсутствующем/неавторизованном `gh` заставляют его уверенно печатать FPV-01 как следующий; должен возвращать явную ошибку.
8. Объявить **один** источник порядка (`manifest.json`), перегенерировать `EXECUTION-ORDER.md` и раздел фаз `17-ROADMAP.md` из него, переписать «Delivery phases» в FPV-00 как генерируемую секцию.

### (b) Требует одобрения владельца — не делать без явного «да»

9. **Любой `git push`** — RED LINE #2 (после ревью владельцем), в том числе bootstrap-ветки с пакетом.
10. **Создание 16 GitHub issue.** Рекомендация: не запускать `create_github_issues.py` в текущем виде. Арифметика бэклога: 32 открытых issue, все одного автора, **ноль** assignee, закрытий 2–4 в месяц (2026-05 → 13, 06 → 2, 07 → 4), против 120 новых непроставленных AC. Плюс `create_github_issues.py:128-131` вписывает в тело epic путь `docs/vnext/…`, который на момент запуска не существует ни на одной ветке. Предлагаемый компромисс: создать **только FPV-00 (epic) + FPV-01 (boundary)**, остальное держать в пакете как планировочную поверхность до тех пор, пока открытый бэклог два месяца подряд не пойдёт вниз.
11. **Carve-out «Wave 0 — корректность»** мимо всей vNext-конструкции: восемь названных дефектов (#325, #328-остаток, #329, #392, #393, #304, #353, #374, #397) имеют живые репродьюсеры и должны чиниться против **текущей** схемы обычным циклом PRD→RFC→ADR, а не ждать несуществующего протокола. `17-ROADMAP.md:3-8` сам называет их Phase 0, а `EXECUTION-ORDER.md:12` запирает их за FPV-02 — это внутреннее противоречие пакета, и разрешать его надо в пользу ROADMAP.
12. **Отдельный ранний слайс FPV-05 (#360, git-delta provenance gate)** как самостоятельный PRD против сегодняшней границы: `git diff --name-only base..result` + scope-проверка + три поля во frontmatter Evidence. Не требует ни Protocol v1, ни адаптеров, ни сервера, и закрывает разрыв 0/148 — единственная часть программы, которую можно шипить прямо сейчас.
13. Ужесточение `dev`-ruleset: `gh api repos/ForgePlan/forgeplan/rulesets/14715790` показывает `required_approving_review_count: 0`, `allowed_merge_methods: ["merge","squash","rebase"]`, CODEOWNERS отсутствует. «Independent verification» пакета — свободный текст в шаблоне, который никогда не применяется. Это изменение процесса репозитория, решает владелец.

### (c) Заблокировано до разрешения — не начинать

14. **FPV-01** — до закрытия PROB-082 (ADR-collision register) и PROB-083 (ADR-001 ↔ ADR-009). Иначе первым действием программы станет молчаливое переопределение четырёх активных решений.
15. **FPV-02 / FPV-03** — до решения по персистентности (B3) и по тому, что `ArtifactReference.id` = slug (M9). Rust DTO, положенные на текущие формы схем, дороже переделывать потом.
16. **FPV-07** — до решения по attestation-субстрату актора. Пока его нет, инвариант 6 и Deep/Critical-профили должны быть в документах помечены как **advisory**, а не рекламироваться как гарантии.
17. **FPV-09** — до слияния с ADR-008 `agent-manifest` (M7).
18. **FPV-11 / FPV-12 / FPV-13** — до supersede ADR-009 (marketplace-модель) и marketplace ADR-014 (demand-gate) либо до сужения области.
19. **FPV-14** — до решения по источнику данных (Phase 5 против Phase 6) и по судьбе `@forgeplan/web-agent`.
20. **FPV-15** — до решения по ADR-018/PRD-081. По умолчанию — DROP.

---

## 8. ОТКРЫТЫЕ ВОПРОСЫ К ВЛАДЕЛЬЦУ

Пять вопросов, на которые я не смог ответить из репозитория. Всё, что имело очевидный правильный ответ (`.serena/` → gitignore, `ref/` → удалить после сохранения 9 файлов, схемы и скрипты → остаются в пакете, PR-шаблон → остаётся в директорной форме, `docs/README` → индексируем на FPV-10), решено выше без вопроса.

1. **Кто оркестрирует?** `ADR-001` («AI agent is the orchestrator, not Forgeplan») и `ADR-009` («Forgeplan-core становится оркестратором») оба `status: active` и утверждают противоположное; ни один не объявляет supersession. Это предшествует vNext и переживёт его отмену. Какой выживает? От ответа зависят B1, B4, FPV-01, FPV-11 и судьба playbook-runtime.

2. **Что делать с playbook-runtime (ADR-011, `claude --print` из core)?** Пять диспетчеров + `Delegation::Command` с `budget_usd` и `timeout_seconds` + CLI `playbook.rs`/`ingest.rs` — это шипнутая с v0.27.0 поверхность, которая по объявленной границе становится нелегальной. KEEP (и тогда граница ложна в день ноль), MOVE-TO-EXTENSION, или DEPRECATE с окном? Стоимость нигде в пакете не забюджетирована.

3. **Где живут WorkContract / ExecutionReceipt / EvidenceBundle / VerificationVerdict + append-only authority log?** Git-tracked markdown/JSON в `.forgeplan/` (тогда каждое исполнение агента мутирует source of truth и RED LINE #11 становится неисполнимым), gitignored derived index (тогда verdict'ы не переживают clone), или внешний стор (тогда нужна явная поправка к ADR-003, а `ADR-018:38-42` уже отказал «второму authoritative не-markdown стору»)?

4. **Идёт ли позиционная половина (FPV-09…FPV-15) без единой заявки?** В корпусе из 77 PROB `rg -il 'cursor|codex|opencode|work.?contract|execution.?receipt|provenance'` даёт 4 файла, и все четыре — про другое (installer-таргеты, harness-совместимость, wish-list, module-placement). Реальная свежая боль — PROB-072 (worktree drift), PROB-073 (медленно), PROB-077 (silent data loss), PROB-078 — вся локальная и репозиционированием не лечится. Требуем ли хотя бы один заведённый внешним пользователем PROB про кросс-хостовую переносимость контрактов как условие старта этой половины?

5. **Есть ли субстрат для attestation актора?** Подписанные git-коммиты, OIDC-токен от CI, host-signed receipts — или ничего? Если ничего, инвариант 6 (02:108) и правило builder ≠ verifier (07:47) должны быть понижены до advisory в документах и в CapabilityManifest, потому что `validate_agent_id` (`claim/mod.rs:178-190`) проверяет только непустоту, длину и класс символов — любой вызывающий передаёт любой `--agent`.