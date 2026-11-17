---
depth: deep
id: PRD-087
kind: prd
last_modified_at: 2026-10-16T20:18:35.483144+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: PRD-024
  relation: refines
- target: PRD-047
  relation: refines
status: draft
title: Website onboarding — task/stack selector and revision-bound public guide
---

---
assigned_number: 87
created: 2026-10-16
domain: general / healthcare / fintech / govtech / edtech
epic: null
predicted_number: 87
priority: P0 / P1 / P2 / P3
projectType: web_app / api_backend / mobile_app / cli_tool / library
slug: prd-website-onboarding-task-stack-selector-and-revision-bound-public-guide
stepsCompleted: []
updated: 2026-10-16
---

# Website onboarding: выбор по задаче и стеку

## Summary

Автор требований: specification, Profile A; Eli Rum, https://elirum.me. Candidate r1, 2026-10-16. Deep SHAPE ONLY, Smith Row3/SPARC subset. Статус остаётся draft: это не разрешение реализации, не независимая приёмка и не активный тестовый oracle.

## Problem

Посетителю нужен ограниченный набор возможностей под задачу, стек и среду агента, но существующие страницы объясняют преимущественно Claude Code и фиксированные наборы плагинов. Каталог описывает поставку, а не факт загрузки инструкций или готовность проекта. Сайт не имеет доступа к локальному состоянию. Если смешать эти понятия, посетитель получит правдоподобную команду либо ложное Ready. Требуется один контракт выбора с Marketplace CLI и явная граница между рекомендацией и локальной проверкой.

## Target Users

- J1: новый EN/RU посетитель, выбирающий возможности для конкретной задачи и стека.
- J2: владелец существующего проекта, обновляющий настройку или восстанавливающий её после конфликта.
- J3: посетитель с неизвестной, неоднозначной или неподдерживаемой средой.
- J4: пользователь только клавиатуры и узкого экрана.
- J5: сопровождающий двуязычные страницы и принимающий ревизию производителя.

## Goals

На приёмке кандидата: (G1) все семь групп входов J1 имеют явное состояние и влияют на результат ровно по принятому контракту; (G2) 100% согласованного набора положительных/ошибочных примеров совпадают по смыслу с CLI/export одной immutable revision; (G3) ни один browser-only результат не называется Ready; (G4) команды EN/RU идентичны, а семантика выбора одинакова; (G5) весь путь J1–J4 проходится на реально отрисованных EN/RU страницах клавиатурой, включая 320 CSS px; (G6) каждый публичный нормативный фрагмент привязан к ревизии единого регламента, а перевод к ревизии исходной страницы. Это цели будущей приёмки, не измеренные результаты.

## Non-Goals

Нет реализации или выбора архитектуры в этой подготовке; нет RFC/ADR до принятого producer handback. Не менять Runtime, Marketplace, AgentBootstrap, Rust core, инструкции/конфиги проекта или дома, auth, установки, tracker, релизы. Нет второго реестра задач/плагинов/зависимостей, полного набора по умолчанию, выполнения shell в браузере, секретов, provider auth или платных проб. Не мигрировать язык descriptions/aliases: EN:/RU:/Triggers: сохраняются; отложенная языковая задача не зависимость. Не расширять RU landing, не менять маршруты на /en/docs.

## User Journeys

### J1: Новый посетитель EN/RU

1. Из installation или marketplace overview открывает руководство onboarding.
2. Указывает intent задачи, язык/framework/stack, harness и версию, project/global scope, политику моделей и integrations. Заявленные значения помечаются непроверенными; отсутствие и неоднозначность не заменяются угаданными default.
3. Получает объясненный ограниченный набор capabilities: зачем выбран каждый элемент, обязательные зависимости, optional prerequisites, ограничения и исключённые возможности. Установка пакета, enabled/discovered и фактическая загрузка инструкций показаны как разные понятия.
4. При принятом источнике копирует только поддержанную безопасную команду. Пока контракт/команды не приняты, копирует явно НЕИСПОЛНЯЕМЫЙ запрос производителю с выбранными данными и причиной блокировки, без псевдофлагов/пакетов.
5. В локальном проекте, вне сайта, запускает producer preflight и preview; видит фактические версии, scope, план изменений, границы trust/network/auth, конфликты и проверки моделей/skills.
6. Даёт отдельное явное согласие на конкретный apply. Сайт не даёт согласие за пользователя и не применяет изменения.
7. Локальный производитель подтверждает наблюдаемую готовность либо называет блокер. Браузер остаётся рекомендацией/требованием локальной проверки.

### J2: Существующий проект

Пользователь выбирает обновление, сверяет прежнюю и целевую ревизии и локальный preview. При изменившемся prestate, чужой symbolic link, несовместимом файле или общей global plugin state получает остановку и ссылку на recovery в каноническом регламенте. Миграция сохраняет уникальные инструкции и нужную совместимость. Rollback допускается только для принадлежащих операции и с тех пор неизменённых ресурсов; чужие/изменённые/разделяемые ресурсы не удаляются автоматически. Смена задачи обновляет релевантный набор и загрузку skills, а не запускает install/uninstall churn.

### J3: Неизвестная или неподдерживаемая среда

Неизвестный task/stack/version, неоднозначные ответы, неподдерживаемая версия или отсутствующий/недоступный/неодобренный/устаревший/несовместимый источник дают конкретную причину и действие: уточнить входы, получить поддержанную версию либо принять согласованный producer snapshot. Нет правдоподобного fallback, Ready или исполнимого обхода.

### J4: Клавиатура и узкий экран

Посетитель проходит те же поля, связанные ошибки, результат и copy без мыши. Каждое поле имеет label, focus видим, порядок логичен, ловушки фокуса отсутствуют. Изменение результата, блокер и copy success/error объявляются вспомогательным технологиям; цвет не единственный носитель смысла. При 320 CSS px поля и команды доступны без обрезанных действий.

### J5: Сопровождение

Сопровождающий получает принятый producer handback, проверяет общность семантики, ревизии каталога/export/регламента и EN/RU source revision. Не публикует команды из draft-контракта. Обновление регламента или источника инвалидирует старое подтверждение; одинаковый mtime не доказательство свежести.

## Functional Requirements

- [ ] FR-01: Посетитель может задать intent, language/framework/stack, harness/version, scope, model policy, integrations; отличить неполные и неоднозначные ответы. Journey: J1,J3,J4.
- [ ] FR-02: Посетитель получает bounded recommendation и объяснение обязательных/optional зависимостей по ЕДИНОМУ принятому selection contract с CLI, без второго справочника и full default. Journey: J1,J3.
- [ ] FR-03: Посетитель видит installed, enabled/discovered и instructions actually loaded как различные состояния; все локальные сведения сайта unverified, browser never Ready. Discovery не доказывает изоляцию ресурсов. Journey: J1,J2.
- [ ] FR-04: Посетитель копирует точную поддержанную команду только принятого релиза и quoting contract; до этого только неисполняемый request. Quotes/newlines/special characters остаются данными. Journey: J1,J3,J4.
- [ ] FR-05: Посетитель видит unsupported/incomplete/ambiguous, missing/unavailable/stale/unapproved/incompatible source, recommendation-only, local verification required и copy error с доступным следующим действием. Изменение любого входа инвалидирует старый результат/команду. Journey: J1,J3,J4.
- [ ] FR-06: Посетитель проходит guide → local preflight/preview → отдельное consent/apply → observed readiness; сайт не запускает shell, не запрашивает secrets/auth и не делает платных probes. Journey: J1,J2.
- [ ] FR-07: Владелец проекта находит project/global ownership, upgrade/migration/rollback/conflict recovery в одном нормативном регламенте Marketplace; website даёт объяснение/ссылку или revision-bound view, не альтернативные правила. Journey: J1,J2,J5.
- [ ] FR-08: Посетитель видит точные роли Sol6.1 High PLAN → MiniMaxM3 High code+tests и начальные проверки → FRESH Sol6.1 High независимое review+gates. Наблюдаемые реальные модели, relevant skill loading и evidence связаны с candidate SHA; без silent fallback/prewalk. Journey: J1,J2,J5.
- [ ] FR-09: Посетитель EN/RU получает одинаковую семантику, идентичные команды и корректные locale links; язык не меняет канонический выбор. descriptions/aliases сохраняются. Journey: J1,J3,J5.
- [ ] FR-10: Посетитель получает актуальное объяснение Claude native AGENTS и legacy совместимости с ancestor/plugin/mode precedence, external approval, additional dirs и hooks; CLAUDE.md не deprecated, универсальный import-only shim не навязывается. Journey: J1,J2.
- [ ] FR-11: Пользователь проходит J1–J4 клавиатурой на реально отрисованной странице; label/focus/error/result announcements и narrow layout проверяются на кандидате. Journey: J4.
- [ ] FR-12: Сопровождающий проверяет pinned producer/export version+hash+revision, regulation revision, page source hash/revision и translated-source revision; mtime недостаточно. Journey: J5.

## Public Content Ownership and Homes

Предлагаемые НОВЫЕ, пока не созданные source homes: `website/src/content/docs/docs/guides/task-stack-onboarding.mdx` и `website/src/content/docs/ru/docs/guides/task-stack-onboarding.mdx`. Предлагаемые routes: `/docs/guides/task-stack-onboarding/`, `/ru/docs/guides/task-stack-onboarding/`. Здесь единое объяснение и selector; не отдельный standalone HTML guide и не RU landing. Выбор компонентов/адаптера отложен до RFC.

Существующие страницы для входящих/обратных ссылок и согласования затронутых утверждений, в ОБОИХ корнях `website/src/content/docs/docs/` и `website/src/content/docs/ru/docs/`: `getting-started/installation.md`, `marketplace/overview.md`, `guides/claude-code-setup.md`, `guides/mcp-setup.md`. Installation нельзя оставлять с противоречащим безусловным fixed plugin default; MCP installation нельзя приравнивать к проверенной readiness. Это целевая область будущего изменения, не выполненные правки.

Website owner отвечает за человеческое объяснение, UI и перевод. Marketplace setup owner, task `YWRXqH583HL980CfsVY1`, отвечает за ONE canonical operations regulation/runbook, каталог и selection engine. Точный canonical filename регламента НЕ передан: это publication blocker, путь не изобретается. Старые Marketplace ONBOARDING/CROSS-CLI не доказательство нового регламента.

Регламент MUST охватить: project/global ownership; task/stack selection; фактическую политику моделей FR-08; relevant skills selected/loaded vs installed/discovered; evidence и candidate SHA; explicit consent; trust/network/auth boundaries; команды проверенного принятого релиза; upgrades/migrations; rollback только owned unchanged resources; prestate drift, conflicts, foreign symlinks и shared global plugin recovery. Website обязан ссылаться на точный путь+immutable revision либо показывать привязанное к ним представление.

## Stable Consumer Contract

Фиксируются значения и ограничения поведения, не JSON keys/endpoints/CLI flags. Контракт сериализации, resolver/adapter, support matrix и executable command forms остаются за производителем. Browser не инспектирует repo/plugins/credentials/MCP/версии/модели. Каталог не является свидетельством исполнения. Принятый snapshot должен однозначно связывать рекомендацию, команды, release/support matrix, регламент и перевод; неизвестная/несовместимая ревизия блокирует executable output.

## Dependencies

Единственная внешняя блокирующая зависимость D-PRODUCER = Marketplace setup task `YWRXqH583HL980CfsVY1`. Marketplace PRD-080 / SPEC-005 сообщены как DRAFT logical contracts во ВНЕШНЕМ workspace; это не core IDs, не утверждённый API и не graph edges. Отложенная language migration не dependency.

Условие снятия блокера: owner передал approved stable handback, согласованный с release, а не сообщение «draft существует». Checklist приёмки handback:
- точный workspace/owner artifact и approved architecture revisions;
- catalog/export version, immutable revision/hash и support matrix;
- принятые selection semantics и примеры positive/ambiguous/unsupported/conflicts;
- поддержанные команды, platform/shell ограничения и quoting contract;
- diagnostic/preflight/preview/apply/readiness evidence;
- точный canonical regulation path+revision и подтверждение проверки команд против принятого релиза.

Resume: contract intake → consistency check + mapping на frozen candidate scenarios → architecture/RFC + независимый gate → отдельно разрешённая MiniMax implementation → candidate verification. Сейчас ни implementation authorization, ни ожидание/poll Marketplace не допускаются.

## Acceptance Criteria

AC-1: Given принятый snapshot и семь входных групп, When пользователь выбирает задачу, Then объяснение, capabilities и зависимости совпадают с CLI/export той же ревизии; unsupported/ambiguous не подменяются full default.
AC-2: Given только браузер и заявленные данные, When показан результат/copy, Then отсутствует Ready и исполнимая неподтверждённая команда; local verification явно требуется.
AC-3: Given любая смена входа/ревизии, When старый результат уже виден, Then старые recommendation/command больше нельзя использовать как текущие.
AC-4: Given EN/RU страницы, When повторён J1–J4, Then одинаковы выбор, зависимости, команды и блокеры; клавиатурный путь и narrow layout доступны.
AC-5: Given конфликт существующего проекта, When читается recovery, Then нет безусловного удаления чужих/изменённых/shared ресурсов; требуются локальный preview и согласие.

## Candidate Acceptance Plan — NOT Executed

1. На candidate SHA отрисовать обе локали в реальном браузере; пройти J1–J4 мышью и только клавиатурой, включая 320 CSS px, labels/focus/error/result announcements и отказ clipboard. Source inspection не runtime PASS.
2. Сравнить capabilities, mandatory/optional dependencies, причины, blockers и exact copied commands с принятым CLI/export на ТОЙ ЖЕ immutable revision для всего согласованного набора примеров и всех входных групп.
3. Продемонстрировать поддержанную команду в disposable consumer project: preflight/preview без мутаций, отдельное consent/apply, наблюдаемая readiness; отдельно upgrade/drift/foreign symlink/shared global/rollback conflict. Secrets и paid probes не нужны; непроверенные hosts явно skipped, не PASS.
4. Выполнить content/link checks и `npm run build` в website. Existing `check-dead-links.mjs` требует built dist; `check-translation-drift.mjs` проверяет mtime и НЕ заменяет durable revisions; `check-content-completeness.mjs` только часть проверки. В package.json нет npm test/typecheck; не выдумывать их. Page-count baseline здесь не утверждается.
5. Проверить durable producer/export/regulation/page/translation revisions, modern+legacy Claude cases по SPEC; нормативные ссылки ведут к единому источнику.
6. FRESH Sol6.1 High verifier связывает наблюдения, точные модели/hosts/commands, skipped/failed/passed и evidence с candidate SHA. Создатель не принимает свою работу.

## Risks and Mitigations

R1: guessed producer API → request-only до D-PRODUCER; R2: двойная authority регламента → exact owner path/revision; R3: stale translation/snapshot → durable revision gate; R4: install=loaded/Ready → три отдельных понятия и локальное evidence; R5: разрушительный recovery → owned unchanged boundary и блокировка drift; R6: локальная квота findings → отдельное решение owning core policy, не выдуманные дефекты.

## Review Policy and Lifecycle

Core `CLAUDE.md:516` требует issues и re-review при zero findings после significant changes. `CLAUDE.md:679-685` задаёт ≥3 issues каждому из двух reviewers для significant code changes (≥5 commits OR new public API). Здесь нет code changes/commits/new API, есть только draft shape. Fresh verifier MUST явно определить применимость обеих норм к design-only и назвать конфликт с честным no-findings, если он возникает; Marketplace same-number ADR/no-quota решение НЕ применяется к core. Если политика мешает честному design approval, нужен точный owning core policy decision о zero-defect design review; fabricated findings запрещены. Creator не выпускает review Evidence/self-PASS. Red line #7 запрещает activation без code/evidence: bounded design PASS всё равно оставляет PRD/SPEC draft.

## Timeline

Текущий этап finite Shape, 2026-10-16. Следующий этап определяется получением D-PRODUCER, не календарной датой. Author: specification/Profile A. Независимый verifier: fresh Sol6.1 High, назначается родительской сессией. Product owner: Eli Rum. Release/implementation approval сейчас отсутствует.

## Source Anchors

Проверены чтением 2026-10-16: `website/package.json:5-31`; `website/astro.config.mjs:13-36,120-150`; `website/scripts/generate-marketplace-docs.mjs:19-20,80-87,124-142,259-280`; `website/scripts/check-translation-drift.mjs:44-47`; восемь EN/RU страниц из Public Content Homes; `CLAUDE.md:16,516,679-685`. Generator читает .claude-plugin/marketplace.json с .omp fallback, plugins и manifest.profiles, но не выполняет canonical selection или local readiness. `englishOnly` сохраняется. Dependencies Astro ^6.0.1/Starlight ^0.38.2 — объявленные ranges, не измеренные installed versions.

Claude primary source: https://code.claude.com/docs/en/memory#agents-md, fetched 2026-10-16; описание vendor, не запуск Claude. Полные современные/legacy семантические случаи фиксирует дочерний SPEC.

## Related Artifacts

Core PRD-024 (active site) и PRD-047 (active locale) — parents по refines, исторические тела не переписываются. Task `zWV3XYCwiIeccztMcbhc`; Development `ZB8WYUpwNskvAZdRK8Jh`; tracker workspace `kgYnhWodYw7kNbJMAK5q`; artifact workspace `/Users/explosovebit/Work/ForgePlan`. Child SPEC связывается based_on с этим PRD. Внешний Marketplace task записан только квалифицированным текстом.


## ADI Attempt and Design Handoff

2026-10-16: обязательный Deep `forgeplan_reason PRD-087` вызван с workspace `/Users/explosovebit/Work/ForgePlan`. Наблюдаемый результат: `MCP failure; server: forgeplan; tool: forgeplan_reason; transport: stdio; stage: receive; failure: timeout; retryable: no; Request timeout after 30000ms`. Tool сообщает outcome unknown. ADI completion НЕ подтверждён, output не получен; повтор/скрытый fallback не выполнялся. Даже успешный будущий automated advisory не становится принятым producer wire API либо независимым review. Нужны доступный завершённый ADI и последующий независимый review, а не self-PASS.

Дочерний core SPEC-007 (draft, deep) содержит 10 требований и 36 candidate Scenario blocks S01–S36. Candidate r1 Requirements SHA-256: `52c6aba79d2cf5349c4122b6d03e1ffdb29169444a65b7727478c44bce24f6d4`; определение bytes — SPEC Candidate Freeze Record. Independent verifier должен отдельно решить применимость CLAUDE.md:516 и 679–685 к design-only, назвать настоящий конфликт, если zero findings мешает честному verdict, и запросить owning CORE policy decision без выдуманных дефектов. Ни Marketplace ADR, ни creator validation это решение не подменяют. Parent session владеет dispatch независимого fresh Sol6.1 High verifier и tracking; в этом creator assignment review/Evidence не выполняются.


### Resolution 2026-10-16 — terminal status и новый авторский ADI

Предыдущие абзацы сохраняют наблюдение клиента на момент первого вызова, а не текущий итог расследования. Из прежнего transcript подтверждён вызов forgeplan_reason(PRD-087, workspace=/Users/explosovebit/Work/ForgePlan) в 17:57:42.357Z и nonretryable timeout клиента в 17:58:12.367Z. Прочитанные forgeplan_activity и tools-2026-10-16.jsonl:138 подтверждают terminal SERVER status ok в 17:58:42.621Z, duration_ms=60260, args_hash=ec08d4f02ead. Следовательно, сервер завершил тот запрос после таймаута клиента; прежний analysis OUTPUT не восстановлен, его содержание/качество и соответствие окончательному candidate r1 не установлены. Чужие reason tool_err с другими args_hash к этому запросу не относятся.

Полный НОВЫЙ ограниченный авторский Abduction → Deduction → Induction, диагностическое происхождение, pins входов и handback checklist записаны в [NOTE-053](../notes/NOTE-053-prd-087-spec-007-bounded-authored-adi-and-factual-shape-handoff-resolution.md), draft, informs PRD-087 и SPEC-007. Автор — specification/Profile A в явном OMP openai-codex/gpt-6.1-sol High по authoritative run-local dispatch; не независимый reviewer. Это новый анализ на текущих канонических PRD/SPEC, НЕ восстановленный ответ forgeplan_reason, НЕ второй engine invocation и НЕ self-PASS. Выбрано направление сохранения consumer semantics с accepted version-pinned producer и request-only до handback; wire/schema/flags/adapter не назначены.

Текущее CLI и mounted MCP get/validate согласованы: PRD-087 и SPEC-007 draft/deep, тела совпадают с исходными pins; projection обоих deep. F1 сейчас не воспроизводится, B2 на доступной mounted поверхности сейчас не воспроизводится; причина старых расхождений не доказана, metadata repair/reindex/restart не выполнялись. EVID-172 остаётся неизменным историческим Evidence, его BLOCKER здесь не переписывается и не считается закрытым автором. Новый PRD hash после этой фактической вставки фиксируется в NOTE-053; SPEC full body и Requirements/Scenario pins остаются прежними. Все FR/goals/journeys вне этой секции неизменны.

B1 awaiting independent verification: parent передаёт NOTE-053 и точный обновлённый PRD + неизменённый SPEC FRESH Sol6.1 High verifier; только он решает достаточность нового авторского ADI и текущего согласованного readback. Если owning policy требует ещё один engine-delivered High analysis, нужны точная норма и поддержанный effort/output route; прямое авторство не выдаётся за engine output. Publication/implementation по-прежнему BLOCKED единственной внешней D-PRODUCER — Marketplace task YWRXqH583HL980CfsVY1. Deferred language migration не dependency. Resume: accepted handback → consistency mapping S01–S36 → отдельно разрешённая architecture/RFC с independent gate → отдельно разрешённые MiniMax code+tests → fresh candidate verification. Нет producer polling, runtime/build/browser proof, lifecycle activation или implementation authorization.


## Stakeholders

Eli Rum — product owner и решение о публикации/реализации. specification/Profile A — автор draft требований, не reviewer. Website owner — объяснение/UI/перевод. Marketplace setup owner — D-PRODUCER, canonical catalog/selection и нормативный регламент. FRESH Sol6.1 High verifier — независимая design/candidate оценка; parent session назначает verifier и ведёт tracker. Сейчас никто не подтвердил implementation/release approval.

## Success Criteria

При candidate acceptance G1–G6 из Goals измеряются по SPEC-007: G1/G2 — S01–S07, 100% принятого набора producer examples; G3 — S08–S16,S30, zero browser Ready/unsafe executable output; G4 — S17–S19, exact EN/RU command parity; G5 — S33–S34, обе локали и 320 CSS px keyboard journey; G6 — S19–S21,S36, все pinned revisions и candidate SHA. Отсутствие producer examples/ревизии означает blocked, а не 100% от пустого набора. Это будущие критерии, не результат прогона.

## Non-Functional Requirements

NFR-1: Security/privacy — zero collected secrets/provider auth/browser shell/paid model probes; наблюдать S13–S14 на кандидате. NFR-2: Accessibility — labels, visible focus, keyboard, announced output/errors, доступные действия при 320 CSS px в EN и RU; S33–S34. NFR-3: Integrity — executable advice не публикуется с unapproved/stale/incompatible producer/regulation/translation revision; S06,S19–S21. Новые performance/uptime бюджеты в этом feature не вводятся; существующие site требования остаются вне пересмотра.

## Rollback Plan

Website publication candidate может откатываться только на ранее принятый согласованный комплект page/translation/producer/regulation revisions. Если такой комплект больше не поддержан, executable advice блокируется, а не возвращается устаревшая команда. Это rollback consumer публикации; не разрешение менять или удалять пользовательский setup. Для локальной настройки J2/S31–S32 применяется только canonical producer recovery: owned unchanged resources, без foreign/changed/shared deletion и без обхода consent/prestate checks. В текущем Shape ничего не развёрнуто и source rollback не выполнялся.

## Traceability

FR-01 → J1/J3/J4, SPEC S01–S04; FR-02 → J1/J3, S01–S07; FR-03 → J1/J2, S08–S10; FR-04 → J1/J3/J4, S11–S16; FR-05 → J1/J3/J4, S03–S06,S15–S16; FR-06 → J1/J2, S14,S30; FR-07 → J1/J2/J5, S20–S21,S31–S32; FR-08 → J1/J2/J5, S22–S23,S36; FR-09 → J1/J3/J5, S17–S19; FR-10 → J1/J2, S24–S29; FR-11 → J4, S33–S34; FR-12 → J5, S19,S35–S36.

Семь логических входов: 1 task intent; 2 language/framework/stack; 3 harness; 4 harness version; 5 scope; 6 model-role policy; 7 integrations. Запись harness/version группирует два связанных входа для краткости, не исключает отдельную проверку каждого.


