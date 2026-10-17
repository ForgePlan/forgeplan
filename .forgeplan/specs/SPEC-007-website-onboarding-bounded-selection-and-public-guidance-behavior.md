---
depth: deep
id: SPEC-007
kind: spec
last_modified_at: 2026-10-16T18:01:36.041378+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: PRD-087
  relation: based_on
status: draft
title: Website onboarding — bounded selection and public guidance behavior
---

---
assigned_number: 7
created: 2026-10-16
prd: null
predicted_number: 7
slug: spec-website-onboarding-bounded-selection-and-public-guidance-behavior
type: API / Data Model / Protocol / UI Spec
updated: 2026-10-16
---

# Website onboarding — candidate behavioral specification

## Summary

Автор: specification/Profile A, Eli Rum (https://elirum.me). Candidate r1 от 2026-10-16 для PRD-087, task zWV3XYCwiIeccztMcbhc. Только Shape. Требования и Scenario blocks фиксируются как кандидат независимого design review, НЕ активный SPEC/TDD oracle. Creator не сертифицирует результат. Все MUST/NEVER ниже — обязательства будущей реализации, не утверждения о существующем исполнении.

## Scope

IN: public EN/RU guide/selector, стабильная потребительская семантика, объяснение локальной настройки, safety, revision provenance, accessibility. OUT: producer wire/schema/resolver/adapter design, новый каталог, реализация, runtime proof, изменение core/Marketplace/Runtime/AgentBootstrap, auth/config/installs/tracker. Никаких RFC/ADR до принятого producer handback.

## Contract

Website использует ONE canonical Marketplace selection contract с CLI/export на одной принятой immutable revision. Данные пользователя декларативны и unverified. Browser не проверяет repo/plugins/credentials/MCP/версии/model use/instruction loading, не исполняет shell и никогда не сообщает Ready. Installed package, enabled/discovered capability и actually loaded instruction — три разных факта. Статический каталог доказывает только опубликованную поставку. До принятия producer schema и команд допустим только явно неисполняемый request с причиной блокировки; executable output запрещён. Формы сериализации, ключи JSON, endpoints, flags и имена новых пакетов здесь не назначаются.

## Data Models

Концептуальные сущности, НЕ wire schema:
- Selection intent: task intent; language/framework/stack; harness+version; project/global scope; model-role policy; integrations. Locale — только язык представления, не аргумент канонической семантики выбора. Любая указанная пользователем локальная характеристика остаётся непроверенной.
- Accepted source: owner/workspace/artifact и approved architecture revision; catalog/export version+immutable revision+hash; support matrix; accepted command/quoting contract и verified release; canonical regulation path+revision.
- Recommendation: bounded capabilities, причины включения/исключения, обязательные зависимости, optional prerequisites, ограничения и source identity. Нет произвольного полного default; нет локальных статусов из каталога.
- Handoff: либо exact supported command для объявленных платформы/shell/scope и accepted revision, либо non-executable request. Не secret-bearing payload.
- Page provenance: source page content hash/revision, translated-source revision, producer/export и regulation revisions. Mtime не заменяет ревизии.
- Local evidence: результат producer preflight/preview, explicit consent на конкретный plan/prestate, apply, observed readiness, точные модели/skills и candidate SHA. Website описывает эти факты и границы, не создаёт их из выбранных ответов.

### Conceptual states

Incomplete input; ambiguous input; unsupported version/combination; source missing; source unavailable; source stale; source unapproved; source incompatible/unknown revision; recommendation only; local verification required; copy pending/succeeded/failed. Это семантические состояния, не названия кодов protocol. Причины могут сочетаться: нельзя скрыть unsupported за copy success. При конфликте приоритет безопасности — executable output блокируется при любом source/input blocker; copy failure не меняет recommendation semantics. Ready не browser state.

## Requirements

### Requirement R1: Полный и ограниченный выбор (PRD FR-01,02)

MUST учитывать все семь групп входов, не дополнять неизвестное full default и воспроизводить принятые CLI/export semantics.

#### Scenario S01: Bounded positive selection and dependency parity
- GIVEN принят producer snapshot и положительный пример с task, language/framework/stack, harness/version, scope, model policy и integrations.
- WHEN посетитель получает рекомендацию по тем же данным, что переданы CLI/export этой immutable revision.
- THEN capability set, mandatory dependency closure, optional prerequisites, причины и ограничения совпадают по смыслу с accepted producer example.
- AND каждый выбранный элемент объяснён; unrelated/full catalog не добавлен по умолчанию.

#### Scenario S02: Each selection dimension is respected
- GIVEN accepted pairs примеров, различающиеся ровно task, language/framework/stack, harness/version, scope, model policy или integrations.
- WHEN посетитель последовательно вводит каждую пару.
- THEN результат меняется либо сохраняется ровно как prescribed accepted CLI/export для этой пары, а не по локальному списку сайта.
- AND project/global последствия и необходимые, optional либо неподдержанные integrations явно различимы.

#### Scenario S03: Incomplete or ambiguous declaration
- GIVEN обязательный вход отсутствует либо task/stack допускает несколько несовместимых interpretations по accepted semantics.
- WHEN посетитель запрашивает результат.
- THEN отображаются конкретные незаполненные/неоднозначные поля и допустимое уточнение; current executable command отсутствует.
- AND сайт не угадывает стек/версию/модель, не выбирает full profile.

#### Scenario S04: Unknown or unsupported combination
- GIVEN неизвестный task/stack/harness version либо комбинация вне accepted support matrix.
- WHEN посетитель запрашивает результат.
- THEN видит unknown/unsupported с причиной и действием уточнить либо выбрать поддержанную среду; исполнимого fallback и Ready нет.

### Requirement R2: Единственный принятый источник (FR-02,05,12)

MUST отличать отсутствие, недоступность, устаревание, отсутствие утверждения и несовместимость источника. Нельзя подменять принятый источник манифестом display generator.

#### Scenario S05: Missing or unavailable source
- GIVEN accepted source не предоставлен либо не может быть прочитан.
- WHEN открывается selector.
- THEN причина missing/unavailable и действие получить producer handback видимы; только non-executable request доступен, не предположительная install command.

#### Scenario S06: Draft, stale or incompatible source
- GIVEN snapshot draft/unapproved, stale относительно связанного принятого release/regulation, неизвестной revision/version либо несовместим с consumer contract.
- WHEN строится или обновляется результат.
- THEN executable output блокируется с конкретной причиной и требуемой согласованной ревизией; старый plausible snapshot не маскируется как текущий.
- AND факт существования Marketplace draft PRD-080/SPEC-005 не снимает блокировку.

#### Scenario S07: Display catalog is not another resolver
- GIVEN существующий generator прочитал canonical .claude-plugin manifest либо свой .omp fallback и profiles.
- WHEN посетитель читает reference и открывает selector.
- THEN reference не объявляется selection engine или свидетельством local readiness; selector использует принятые semantics, не второй website task/plugin/dependency registry.
- AND EN:/RU:/Triggers: descriptions и aliases не мигрируют ради этой задачи.

### Requirement R3: Честная граница браузера (FR-03,06)

MUST отличать recommendation от наблюдаемой локальной готовности; browser-only NEVER Ready. Discovery не означает isolation.

#### Scenario S08: Browser declarations never prove readiness
- GIVEN пользователь заявил установленные версии/plugins, enabled integrations и подходящие модели.
- WHEN сайт показал совпадающий catalog result либо успешный copy.
- THEN значения помечены declared/unverified и результат recommendation only/local verification required, не Ready.
- AND сайт не утверждает проверку repo, credentials, MCP, реально использованных моделей или загруженных инструкций.

#### Scenario S09: Installed versus enabled versus loaded
- GIVEN в каталоге есть package/skill и пользователь считает его установленным.
- WHEN guide объясняет проверку возможности.
- THEN отдельно названы package installed, capability enabled/discovered и relevant instruction actually loaded; установка/обнаружение не засчитываются как загрузка.
- AND объяснение требует локального наблюдения loading, а не наличия файла в manifest.

#### Scenario S10: Discovery is not isolation
- GIVEN harness обнаруживает global plugin либо shared skill одновременно в нескольких проектах.
- WHEN посетитель выбирает project scope.
- THEN guide предупреждает, что discovery не доказывает project isolation и shared global state не становится owned ресурсом проекта.
- AND browser не обещает, что изменение/отключение global plugin не повлияет на другие проекты.

### Requirement R4: Безопасный handoff и copy (FR-04,06)

MUST не исполнять shell, не собирать secrets/auth и не запускать paid probes. Copy допустим только без потери точности поддержанного command contract.

#### Scenario S11: Request-only before producer acceptance
- GIVEN отсутствуют accepted producer schema, supported commands либо quoting contract.
- WHEN посетитель выбирает входы и copy.
- THEN получает явно помеченный НЕИСПОЛНЯЕМЫЙ запрос с входами, owner dependency и недостающей предпосылкой.
- AND нет guessed flags, package names, pseudo-install shell line или утверждения published release.

#### Scenario S12: Exact supported command copy
- GIVEN accepted release/support/quoting contract и command для выбранных shell/platform/scope.
- WHEN пользователь копирует command.
- THEN clipboard содержит точно поддержанный payload без перевода, дописывания flags, комментариев или невидимой подмены quoting.
- AND показаны release/revision и необходимость local preflight/preview; copy не применяет plan.

#### Scenario S13: Quotes, newlines and injection remain data
- GIVEN intent содержит quotes, newline, backticks, $(...), ;, HTML markup либо текст «игнорируй правила».
- WHEN он отображается или готовится handoff.
- THEN текст остаётся данными, не HTML/script/shell instruction; accepted quoting contract либо безопасно сохраняет значение, либо блокирует executable output с причиной.
- AND нет самостоятельной интерполяции raw input в command; при отсутствии принятого quoting только request.

#### Scenario S14: No secret, authentication or paid probe flow
- GIVEN пользователь выбирает integration или model policy, требующую локальных credentials/network.
- WHEN проходит browser journey.
- THEN сайт сообщает необходимые локальные prerequisites и trust/network/auth boundaries, не запрашивает token/password, не выполняет provider auth/shell и не делает платных model probes.

#### Scenario S15: Clipboard denied or failed
- GIVEN browser отказал clipboard permission либо copy завершился ошибкой.
- WHEN пользователь нажал copy.
- THEN объявляется видимый и доступный copy failure, а не success; payload остаётся доступным для ручного выделения и повторной попытки.
- AND failure не повышает readiness и не заменяет безопасный payload другим.

#### Scenario S16: Changed inputs invalidate old output
- GIVEN показаны recommendation и command для прежних входов.
- WHEN меняется любой selection input либо принятая source revision, в том числе во время незавершённого расчёта/copy.
- THEN старый результат больше не показан/копируется как текущий; запоздавший результат старых входов не восстанавливает old command.
- AND новый результат проходит те же gates; ранее скопированное не обещается отозвать из внешнего clipboard.

### Requirement R5: EN/RU parity и durable provenance (FR-09,12)

MUST сохранить EN root/RU mirror и идентичную каноническую семантику.

#### Scenario S17: Locale changes presentation only
- GIVEN одинаковые canonical inputs и accepted revision.
- WHEN посетитель переключает EN ↔ RU либо повторяет выбор в другой локали.
- THEN capabilities/dependencies/blockers и exact command payload одинаковы; переводятся объяснения/labels, не IDs/commands/selection semantics.
- AND без повторного ввода допускается только сохранение подтверждённых входов; локаль сама не делает выбор incomplete или Ready.

#### Scenario S18: Entry routes and public page homes
- GIVEN страницы опубликованы после producer acceptance.
- WHEN посетитель открывает installation/marketplace overview или related Claude/MCP setup в любой локали.
- THEN находит onboarding guide и обратные ссылки в своей локали по /docs/... или /ru/docs/..., без /en/docs и без новой RU landing.

#### Scenario S19: Translation and source revision drift
- GIVEN перевод связывался с конкретным source page hash/revision, producer/export revision и regulation revision.
- WHEN хотя бы одна связанная ревизия изменилась без повторной проверки перевода/команд.
- THEN кандидат помечается stale и публикация нового executable advice блокируется до согласования; равные/новые mtimes это не снимают.

### Requirement R6: Единый нормативный регламент и модели (FR-07,08)

Marketplace setup owner владеет ONE canonical regulation. Website не создаёт конкурирующих правил.

#### Scenario S20: Unknown normative home blocks publication
- GIVEN owner не передал точный canonical regulation path и immutable revision либо проверенный release status.
- WHEN кандидат готовят к публикации.
- THEN guide/selector не заявляется нормативно полным и publication blocked; historical ONBOARDING/CROSS-CLI не подставляются как новый регламент.

#### Scenario S21: Revision-bound regulation view
- GIVEN известны принятые canonical regulation path/revision и release.
- WHEN guide объясняет setup.
- THEN он либо ссылается на этот источник, либо показывает provenance-bound view той же ревизии; нет independently authored normative copy.
- AND видимы project/global ownership, task/stack selection, skills selection/loading, evidence/candidate SHA, explicit consent, trust/network/auth boundaries, verified commands, upgrade/migration/rollback/conflict recovery.

#### Scenario S22: Exact observed model roles and fresh review
- GIVEN пользователь читает обязательный model-role process.
- WHEN guide описывает переход от плана к приёмке.
- THEN порядок Sol6.1 High PLAN → MiniMaxM3 High code+tests и начальные проверки → FRESH Sol6.1 High independent review+gates явен, actual model identities и результаты связаны с candidate SHA.
- AND недоступная роль останавливает соответствующий этап с причиной, не вызывает silent fallback, hidden prewalk другой моделью или self-review создателя.

#### Scenario S23: Relevant skills without installation churn
- GIVEN пользователь сменил task при неизменном базовом setup.
- WHEN guide показывает новый релевантный набор.
- THEN описывается selection/loading нужных skills с проверкой actually loaded, а не обязательная переустановка/удаление пакетов при каждом task switch.
- AND необходимая реальная недостающая dependency остаётся явным локальным prerequisite, не скрывается обещанием loaded.

### Requirement R7: Modern Claude и обязательная legacy совместимость (FR-10)

Объяснение MUST соответствовать primary vendor memory документации; документация не runtime proof. Нельзя объявлять CLAUDE.md deprecated или заставлять все проекты принять import-only shim.

#### Scenario S24: Native AGENTS with no project CLAUDE trigger
- GIVEN Claude ≥2.1.281, встроенный AGENTS plugin enabled, mode claude-md-or-agents-md, нет cwd/ancestor CLAUDE.md/.claude/CLAUDE.md/CLAUDE.local.md.
- WHEN guide объясняет project instructions.
- THEN описывает native AGENTS.md/.claude/AGENTS.md cwd и ancestors без обязательного CLAUDE shim; user ~/.claude/CLAUDE.md, managed CLAUDE и .claude/rules не объявляются fallback triggers.
- AND фактическую загрузку нужно подтвердить локально, не вывести из версии.

#### Scenario S25: Ancestor and local precedence
- GIVEN default mode и AGENTS вместе с cwd/ancestor CLAUDE.md, .claude/CLAUDE.md либо CLAUDE.local.md.
- WHEN читатель проверяет, почему AGENTS не загрузился.
- THEN guide объясняет выбор CLAUDE files вместо native AGENTS в default mode, включая эффект личного CLAUDE.local.md, а не обещает оба файла автоматически.
- AND уникальные инструкции не удаляются ради унификации.

#### Scenario S26: Modes, plugin versions and effective settings
- GIVEN читатель проверяет claude-md-or-agents-md, claude-md-and-agents-md, claude-md либо managed-only и built-in plugin availability.
- WHEN guide показывает ожидаемую загрузку.
- THEN объясняет соответственно fallback; оба файла с CLAUDE перед AGENTS в каждой директории и без повторной загрузки already imported/symlinked AGENTS; CLAUDE-only; только managed CLAUDE+auto memory на старте с сохранением documented on-demand nested CLAUDE/rules поведения.
- AND instructionFiles берётся из user/managed/explicit --settings, не project/local settings; disabled built-in plugin лишает native AGENTS поддержки.
- AND до 2.1.285 используется agents-md@builtin, с 2.1.285 читается этот ID либо cc-plugin-agents-md@builtin, без ложного обещания чтения нового ID старой версией.

#### Scenario S27: Legacy and first upgrade session
- GIVEN Claude <2.1.277, отдельная pre2.1.281 CLAUDE-only session либо первая затронутая session после upgrade с ≤2.1.276.
- WHEN guide объясняет AGENTS совместимость.
- THEN native support не обещается: указаны порог 2.1.277, pre2.1.281 случаи и first-upgrade-session caveat с загрузкой со следующей session; при необходимости сохраняется CLAUDE import compatibility.
- AND CLAUDE.md не объявляется deprecated, universal shim не требуется для поддержанного native пути.

#### Scenario S28: External imports need prior approval
- GIVEN native AGENTS содержит @path за пределами working directory, а project external imports ещё не approved.
- WHEN объясняется loading.
- THEN сказано, что native AGENTS external import не загружается и не вызывает новый approval prompt; после prior project approval может загрузиться.
- AND это отличается от CLAUDE external import approval flow, а site сам trust approval не даёт.

#### Scenario S29: Additional directories and hooks differ
- GIVEN --add-dir и CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD включены либо нужны InstructionsLoaded hooks.
- WHEN guide описывает ожидаемые instructions/evidence.
- THEN additional directory CLAUDE loading не переносится автоматически на AGENTS; direct native AGENTS не вызывает InstructionsLoaded, а CLAUDE-imported/symlinked AGENTS вызывает обычный CLAUDE hook flow.
- AND отсутствие hook event не объявляется самостоятельным доказательством отсутствия native AGENTS.

### Requirement R8: Локальный apply, upgrade и rollback (FR-06,07)

MUST объяснять локальную проверку, отдельное consent и owned unchanged recovery, не выдавая локальное выполнение за возможность сайта.

#### Scenario S30: Preview before consent and observed readiness
- GIVEN поддержанная producer command скопирована, но ещё не применена локально.
- WHEN пользователь следует guide.
- THEN сначала получает local preflight/preview с scope/resources/prestate/trust/network/auth и conflicts, затем отдельно разрешает конкретный apply и проверяет observed readiness.
- AND command copied, package installed или plan created не заменяют эти наблюдения; сайт остаётся recommendation/local verification required.

#### Scenario S31: Upgrade preserves instructions and checks drift
- GIVEN существующий проект и целевая revision отличаются от текущей настройки.
- WHEN пользователь следует upgrade/migration.
- THEN guide требует локальный comparison/preview, сохраняет unique instructions и required legacy paths; при prestate drift/конфликте останавливает apply до нового согласованного preview.
- AND не предлагает overwrite чужого состояния или обход проверок для быстрого завершения.

#### Scenario S32: Rollback cannot delete foreign or changed resources
- GIVEN rollback plan затрагивает ресурс, который foreign, изменён после apply, foreign symlink либо shared global plugin state.
- WHEN пользователь читает recovery и локальный plan.
- THEN безусловный rollback этого ресурса запрещён; показан конфликт и канонический recovery с локальной проверкой и отдельным решением.
- AND автоматический rollback ограничен owned unchanged resources; другие проекты не объявляются защищёнными лишь выбором project scope.

### Requirement R9: Доступный реальный путь (FR-11)

MUST обеспечить labels, keyboard, visible focus, errors/output announcements и узкий экран. Проверка только source не достаточна.

#### Scenario S33: Keyboard-only rendered journey
- GIVEN реально отрисованный EN либо RU onboarding с accepted positive example.
- WHEN пользователь только клавиатурой проходит входы, recommendation, regulation link и copy/manual-copy.
- THEN все действия достижимы в логичном порядке, focus видим, у каждого поля programmatic label и нет focus trap; результат и copy status объявляются без обязательного перемещения фокуса.

#### Scenario S34: Accessible errors and narrow viewport
- GIVEN реально отрисованная страница при 320 CSS px с incomplete/unsupported/source/copy error.
- WHEN пользователь обнаруживает ошибку и исправляет входы.
- THEN причина связана с полем либо результатом, объявлена текстом/assistive technology, не только цветом; исправление и command/manual copy доступны без скрытых/обрезанных действий.
- AND повторный валидный результат не оставляет старую ошибку или старую команду текущими.

### Requirement R10: Приёмка и запрет ранней реализации (FR-12)

MUST сохранить единственную D-PRODUCER зависимость и применять independent evidence к конкретному кандидату.

#### Scenario S35: Producer intake gate
- GIVEN Marketplace setup task YWRXqH583HL980CfsVY1 ещё не передал approved stable release handback.
- WHEN bounded design review завершён.
- THEN implementation остаётся blocked; нет RFC/ADR для выдуманного producer, executable guessed output и ожидания deferred language migration.
- AND resume требует handback checklist из Contracts, consistency mapping на этот candidate, architecture/RFC с independent gate, затем отдельно разрешённые MiniMax implementation и verification.

#### Scenario S36: Candidate evidence is not source-inspection PASS
- GIVEN candidate website SHA и принятые producer/export/regulation revisions.
- WHEN fresh Sol6.1 High verifier оценивает acceptance.
- THEN record включает actual rendered EN/RU keyboard/narrow journey, command/result parity одной producer revision, disposable local command demonstration, content/link/build и durable translation/source checks.
- AND точные модели, hosts, failed/skipped/passed и candidate SHA явны; непроверенные hosts/skipped не PASS, creator не self-certifies.

## Behavioral Contract

### Signature
Conceptual input = семь selection групп + accepted source identity + presentation locale. Output = current blocked/recommendation explanation и exact supported command ИЛИ non-executable request; local verification notice всегда. Это не signature реализации или wire API. Ошибки ниже семантические.

### Pseudocode
1. При изменении selection/source инвалидировать текущие recommendation/command, включая запоздалые результаты.
2. Проверить completeness/ambiguity и accepted source identity/support/release/regulation; при blockers показать причины и request-only.
3. Применить ONE accepted producer selection semantics; не назначать локальную альтернативу и не заполнять full default.
4. Отобразить bounded recommendation/dependencies/limitations в выбранной локали, local facts unverified.
5. Разрешить exact command copy только при accepted command+quoting contract; иначе request. No execution/auth/secrets/probes.
6. Объяснить local preflight/preview → explicit consent/apply → observed readiness и recovery. Browser Ready не выводить.
7. Copy success сообщать только после успеха; error сохраняет payload для ручного copy, не меняет уровень готовности.

### Invariants
INV-1 browser never Ready (S08,S30); INV-2 one selection authority (S01,S07); INV-3 invalid source blocks commands (S05,S06,S20); INV-4 locale does not alter semantics/payload (S17); INV-5 no destructive foreign rollback (S32); INV-6 no secret/shell execution (S13,S14); INV-7 unchanged input/source identity required for current output (S16); INV-8 installed/discovered != loaded/isolated (S09,S10); INV-9 model roles observed, fresh reviewer distinct (S22,S36).

## Contracts

D-PRODUCER ONLY = external Marketplace task YWRXqH583HL980CfsVY1. External draft PRD-080/SPEC-005 не являются core graph IDs. Нет зависимости от language migration. Required stable handback: exact owner workspace/artifact + approved architecture revisions; catalog/export version+hash+immutable revision+support matrix; accepted semantics/examples positive/ambiguous/unsupported/conflicts; supported commands/platform-shell limits+quoting contract; diagnostic/preflight/preview/apply/readiness evidence; exact canonical regulation path+revision+verified accepted release status. Serialization, resolver и adapter formats отложены до этого.

Canonical regulation owner = Marketplace setup owner. Exact filename пока не передан; publication blocker. Website owns explanation/UI/translation, links or provenance-bound view only. Предлагаемые new homes (не созданы): `website/src/content/docs/docs/guides/task-stack-onboarding.mdx`, `website/src/content/docs/ru/docs/guides/task-stack-onboarding.mdx`. Existing entry/related homes в обоих locale roots: `getting-started/installation.md`, `marketplace/overview.md`, `guides/claude-code-setup.md`, `guides/mcp-setup.md`. Astro EN root/RU mirror сохраняются.

## Errors

| Семантическая причина | Наблюдаемое поведение / действие |
|---|---|
| incomplete / ambiguous | уточнить связанные поля, без current command |
| unknown / unsupported version or combination | поддержанная среда либо уточнение, без guessed fallback |
| source missing / unavailable | получить accepted handback, request-only |
| source stale / unapproved / incompatible / unknown revision | получить согласованный approved snapshot, executable blocked |
| regulation/page/translation drift | publication/новый executable advice blocked до согласования revisions |
| local state unknown | local verification required, никогда Ready |
| copy denied / failed | объявить error, сохранить manual-copy, не утверждать success |
| local conflict / prestate drift / unsafe rollback | остановить изменения, обратиться к canonical recovery, новый preview/consent |

## Validation Rules

Все R1–R10 имеют Scenario blocks со stable IDs S01–S36. Future acceptance проверяет наблюдаемое поведение, не source text. Contract examples producer ещё не получены; wire fixtures и конкретные supported matrices не выдуманы. Нет обязательного отдельного conformance corpus для обычного web consumer: parity примеры принимаются от producer той же ревизии. Source/hash pin не заменяет независимое review.

## Candidate Acceptance Plan — NOT Executed

P1: Реальный browser EN/RU, keyboard/mouse, 320 CSS px, labels/focus/errors/result/copy announcements (S01–S06,S08,S15–S18,S33,S34). P2: Compare all seven input dimensions, bounded sets, dependency closure, blockers and exact copied command against accepted CLI/export SAME immutable revision (S01–S07,S11–S17). P3: Disposable consumer project command demonstration, preflight/preview/no mutation before consent/apply/readiness, upgrade/drift/foreign symlink/shared global/rollback cases; не shell из browser (S10,S14,S22,S23,S30–S32). P4: Content/link checks + website `npm run build`; check-dead-links.mjs after built dist, check-content-completeness.mjs; check-translation-drift.mjs mtime alone insufficient (S18–S21). P5: Durable producer/regulation/page/translated-source revisions and modern/legacy Claude cases against vendor + accepted release; actual loading only where tested, hosts explicitly skipped otherwise (S19,S24–S29). P6: FRESH Sol6.1 High acceptance record at candidate SHA, exact model/host identities and separate passed/failed/skipped (S22,S35,S36). No npm test/typecheck script is claimed. No baseline page count, runtime PASS or completed implementation is asserted.

## Review Policy

Owning core `CLAUDE.md:516` issues/zero→re-review; `679-685` ≥3 findings per reviewer applies significant code changes ≥5 commits OR new public API. No such code delta exists in this shape. Independent reviewer MUST explicitly state applicability and any no-findings/design-review conflict; unresolved conflict requires owning CORE policy decision, not imported Marketplace same-number ADR/no-quota permission or fabricated findings. Root red line #7 prohibits activation without code/evidence: even independent bounded spec PASS leaves draft. Parent session dispatches fresh independent verifier; creator neither Evidence nor self-PASS.

## Versioning

Candidate r1, 2026-10-16: stable consumer semantics and S01–S36 for independent review. Hash pin covers exact UTF-8 Requirements bytes from `## Requirements` through the byte before `## Behavioral Contract`; computed after authoring. Changes require a new candidate revision/hash and review, not silent alteration for a passing implementation. This freeze is candidate-only; no activation/frozen TDD oracle asserted.

## Related

Core PRD-087 based_on parent; core active PRD-024/PRD-047 contextual ancestors. Authoritative task brief: `/Users/explosovebit/.omp/agent/sessions/-Work-GertsAi-gertsai_codex/2026-10-16T13-13-31-137Z_01a144d8-a501-7048-9171-61e9652343dd/local/website-onboarding-shape-execution.md`. Primary Claude source fetched 2026-10-16: https://code.claude.com/docs/en/memory#agents-md (native support/modes/settings/legacy/approval/hooks/additional dirs); this is vendor documentation, not runtime proof. Source anchors, ownership, policy and handback details: PRD-087.


## Candidate Freeze Record

Candidate r1, 2026-10-16. Requirements SHA-256 (UTF-8, LF; от начала `## Requirements\n` до байта перед `## Behavioral Contract\n`, включая конечную пустую строку): `52c6aba79d2cf5349c4122b6d03e1ffdb29169444a65b7727478c44bce24f6d4`. Требований: 10; Scenario IDs: S01, S02, S03, S04, S05, S06, S07, S08, S09, S10, S11, S12, S13, S14, S15, S16, S17, S18, S19, S20, S21, S22, S23, S24, S25, S26, S27, S28, S29, S30, S31, S32, S33, S34, S35, S36. Pin — идентичность текста для FRESH независимого design review, не review verdict, activation или runtime evidence. При любой правке блока новый candidate revision/hash.




