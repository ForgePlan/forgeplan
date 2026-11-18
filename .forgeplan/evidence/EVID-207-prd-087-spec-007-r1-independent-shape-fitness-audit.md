---
depth: tactical
id: EVID-207
kind: evidence
links:
- target: PRD-091
  relation: informs
- target: SPEC-012
  relation: informs
status: draft
title: PRD-091 SPEC-012 r1 independent Shape fitness audit
---

---
assigned_number: 172
created: 2026-10-16
predicted_number: 172
slug: evid-prd-087-spec-007-r1-independent-shape-fitness-audit
updated: 2026-10-16
---

# Независимое ограниченное Shape review PRD-091 / SPEC-012 r1

## Structured Fields

verdict: weakens
congruence_level: 3
evidence_type: audit

## Gate Verdict and Scope

gate_verdict: BLOCKER
bounded_content_assessment: соответствует исходному заданию в проверенных границах; содержательных пропусков не обнаружено
implementation_authorized: false
activation_authorized: false
runtime_verified: false

BLOCKER относится к полному design gate/готовности двигаться дальше, а не к необходимости выдумать дефект требований. Обязательный Deep ADI не подтверждён; стабильный контракт производителя и точный нормативный источник не переданы; представления SPEC расходятся по depth. Это Evidence не объявляет полный gate PASS, завершённую website задачу, active oracle или реализованный API. Structured verdict weakens фиксирует отсутствие оснований для полной готовности, а не опровергает согласованные ограничения поведения.

## Identity and Independence

Reviewer: architect-reviewer, Profile B; fresh independent Sol6.1 High context, не specification/Profile A. Модель контекста: openai-codex/gpt-6.1-sol. High и run-local overlay заданы authoritative dispatch; отдельная проверка аргументов запуска процесса не выполнялась. Никакой fallback/prewalk и делегирования реализации в этом review не было. Дата: 2026-10-16. Product owner: Eli Rum, https://elirum.me.

Owning artifact workspace: /Users/explosovebit/Work/ForgePlan. Проверяется только конечная Shape preparation; родительская сессия владеет tracker. Source/test/doc/config, PRD/SPEC и active parents не изменялись. Единственная созданная запись — этот draft Evidence с informs к обеим кандидатам; без autoactivation.

## Authoritative Inputs

- Approved task: /Users/explosovebit/.omp/agent/sessions/-Work-GertsAi-gertsai_codex/2026-10-16T13-13-31-137Z_01a144d8-a501-7048-9171-61e9652343dd/local/website-onboarding-shape-execution.md:3-24,30-34.
- Core CLAUDE.md:8-35,197-220,502-517,679-685,689-715; website/CLAUDE.md:1-4,198-206; docs/README.md:9-35,80-113; loaded methodology skill agents/forgeplan-methodology.
- Canonical CLI get / get --json для PRD-091, SPEC-012 и contextual active PRD-024/PRD-047; get показывает PRD-091 refines PRD-024 и PRD-047, SPEC-012 based_on PRD-091. Parents остаются active и не пересматриваются этим Evidence.
- PRD projection: .forgeplan/prds/PRD-091-website-onboarding-task-stack-selector-and-revision-bound-public-guide.md.
- SPEC projection: .forgeplan/specs/SPEC-012-website-onboarding-bounded-selection-and-public-guidance-behavior.md.
- Primary vendor source прочитан в этом review: https://code.claude.com/docs/en/memory#agents-md (markdown alternate https://code.claude.com/docs/en/memory.md), 2026-10-16. Секции AGENTS.md / When Claude Code reads AGENTS.md / Choose which instruction files load / When AGENTS.md support is unavailable / Where AGENTS.md differs from CLAUDE.md. Это документация поставщика, не запуск Claude.

## Reviewed Identity and Hashes

Все SHA-256 ниже вычислены независимо в этом review. Canonical full body означает UTF-8 bytes строки body из forgeplan get ID --json, включая внутренние template-derived YAML-поля, без внешних метаданных projection, без нормализации текста. Full projection означает точные bytes Markdown файла. Проверено: после внешнего YAML projection содержит ровно newline + canonical body + newline; различие depth отдельно ниже, не скрыто под body equality.

| Object | Bytes | SHA-256 |
|---|---:|---|
| PRD-091 canonical full body | 30278 | 24ba25e45c1f3d5a000cb502ccb67ab6bb329003f6c9c7cec2670dd364ad2d25 |
| PRD-091 full projection | 30593 | fdfcf0da5d175bfdba9a3dd363c9adb485832cbb5e6eb1e1c4e1cc89f828d7c7 |
| SPEC-012 canonical full body | 39292 | 5adc34c6f86ca693580b86c8208b304ff79658b7978b781efb3788850ff60678 |
| SPEC-012 full projection | 39571 | c2a375501395a7836dc4784df3cd27726e65e42c3d112031d5a16f1eab901345 |
| SPEC Requirements block | 25748 | 52c6aba79d2cf5349c4122b6d03e1ffdb29169444a65b7727478c44bce24f6d4 |
| SPEC scenario-only stream S01–S36 | 22873 | 9c2bcdfca3af70c8cd51aca5af22784e875333b77bc86ee87eed428afb8f03c1 |

Requirements bytes: от начала ## Requirements newline до байта перед ## Behavioral Contract newline, UTF-8 LF, с конечной пустой строкой. Совпадает с candidate r1 pin в SPEC:363 и PRD:170. Scenario-only stream: в Requirements в исходном порядке конкатенируются полные блоки от #### Scenario SNN: до следующего heading уровня 1–4 либо конца Requirements; пустые строки сохраняются, заголовки Requirement не включаются. Измерено 12 FR, 10 Requirement, 36 уникальных упорядоченных Scenario IDs S01–S36, без пропусков/дубликатов. Хеш — идентичность текста, не доказательство поведения.

## Executed Verification

1. forgeplan get PRD-091 / SPEC-012 и JSON-формы: оба draft, CLI depth deep; links соответствуют назначению. Canonical get parents: PRD-024 active, PRD-047 active.
2. forgeplan validate PRD-091: PASS, 0 errors, 0 warnings; отдельно один COULD prd-fr-format для русского actor/capability текста. Печатное PASS (with warnings) не превращается здесь в один warning.
3. forgeplan validate SPEC-012: PASS, All checks passed, 0 errors, 0 warnings; CLI depth Deep.
4. У обоих validate есть отказ session routing→coding и hint activate. Hint не даёт разрешения ни coding, ни activation; оба не выполнялись.
5. Независимое вычисление full body/projection/Requirements/scenario hashes и counts; source reading для coverage и vendor cases.
6. forgeplan health: 439 artifacts, 2 at-risk и 10 phase mismatches; исторические health-находки вне этого конечного review не чинились. Это не health PASS проекта.

Сборки, тесты, линтеры, сервер, browser, installs/auth/model probes и disposable local apply НЕ запускались по scope. Никакого runtime/build/browser PASS, code provenance/base_sha/result_sha или фактической проверки загрузки Claude этот Evidence не заявляет.

## Content Fitness and Traceability

Проверены все FR и каждый S01–S36 против authoritative task, а не только наличие заголовков.

| Проверяемое обязательство | Покрытие | Наблюдение |
|---|---|---|
| Все входы; тот же CLI catalog/selection; bounded explained recommendation | PRD:85-86,108-122,193; SPEC:54-100, S01–S07 | Task, language/framework/stack, harness, harness version, scope, model policy, integrations сохранены. S02 допускает изменение либо сохранение результата только по accepted producer examples. Второй registry/full default запрещены. Wire keys/flags не изобретены. |
| Browser declarations != local readiness; installed/discovered/loaded; discovery != isolation | PRD:60-65,87; SPEC:102-122, S08–S10 | Browser never Ready; версии/plugins/MCP/credentials/models/loading непроверены. Каталог не доказывает исполнение либо изоляцию shared global state. |
| Exact supported copy либо non-executable request; injection/quoting/clipboard/invalidation | PRD:88-90; SPEC:124-161, S11–S16 | Нет guessed flags/packages, raw shell interpolation, HTML/script interpretation, secret/auth/shell/paid probes. Ошибка clipboard не success. Изменение входов/source инвалидирует и запоздавший результат; уже внешний clipboard не обещается отозвать. |
| EN root/RU human pages и неизменность descriptions/aliases | PRD:93,100-102; SPEC:96-100,163-181, S07,S17–S19 | /docs и /ru/docs сохраняются, не /en/docs; новой RU landing нет. Commands/IDs не переводятся. EN:/RU:/Triggers: и aliases не мигрируют. |
| Один Marketplace-owned регламент, website explanation/UI/translation | PRD:104-122; SPEC:183-196,323-325, S20–S21,S35 | Точный path/revision — blocker, historical ONBOARDING/CROSS-CLI не объявляются новым регламентом. Только link или revision-bound view, не второй набор правил. |
| Sol PLAN → MiniMax code+tests → fresh Sol review/gates; skills/evidence | PRD:92,106,140-141; SPEC:198-208,299-303, S22–S23,S36 | Точные роли Sol6.1 High / MiniMaxM3 High / FRESH Sol6.1 High сохранены; unavailable role останавливает этап, no fallback/prewalk/self-review. Relevant selection/loading != installed/discovered. Evidence привязано к candidate SHA. |
| Modern/legacy Claude: native AGENTS и CLAUDE compatibility | PRD:94,159; SPEC:210-249, S24–S29 | Сверено с primary vendor source: ≥2.1.277 native threshold, ≥2.1.281 cases, first-upgrade caveat; cwd/ancestor/local triggers, modes, effective user/managed/--settings, plugin ID boundary 2.1.285, prior approval, --add-dir и InstructionsLoaded differences. CLAUDE не deprecated, universal shim не требуется. Документация не выдаётся за live loading proof. |
| Local preflight/preview, отдельное consent/apply, observed readiness; upgrade/rollback/conflicts | PRD:63-69,132,138,185-187; SPEC:251-271, S30–S32 | Owned unchanged boundary; foreign/changed/symlink/shared state не удаляется автоматически, prestate drift требует новый preview/consent. Browser ничего не применяет. |
| Реальный rendered EN/RU keyboard/narrow путь; producer parity; durable revisions | PRD:126-141,177-183; SPEC:273-303,344-346, S33–S36 | Будущая приёмка включает 320 CSS px, labels/focus/error/result announcements, clipboard denial, обе локали, same-revision commands/results, disposable local demonstration, content/link/build и source/translation/regulation revisions. Skipped != PASS; пустой producer example set не даёт 100%. |

Пропущенных обязательных scenarios по исходному заданию не обнаружено. Конкретные producer cases/fixtures и поддержанные shell/platform commands ожидаются осознанно; их отсутствие не маскируется выдуманным API.

## Source-Inspection Boundaries

- website/package.json:5-31: build/prebuild/dev/preview есть, npm test/typecheck scripts нет; declared dependency ranges не installed versions.
- website/astro.config.mjs:13-36,120-150: Starlight EN root/RU mirror, docs/guides и docs/marketplace autogenerate. Предлагаемые новые homes: website/src/content/docs/docs/guides/task-stack-onboarding.mdx и website/src/content/docs/ru/docs/guides/task-stack-onboarding.mdx; их создание не выполнялось.
- website/scripts/generate-marketplace-docs.mjs:19-20,80-87,124-142,259-280: MARKETPLACE_DIR/default path, englishOnly, rendering requires, manifest plugins/profiles. Это display generator, не доказанный canonical task resolver или local preflight.
- website/scripts/check-translation-drift.mjs:44-47 использует mtime; check-content-completeness.mjs:16-29 проверяет EN cli/mcp, не новый onboarding guide; check-dead-links.mjs:22-33 пропускает external URLs и не проверяет fragments. Их будущий успешный запуск сам по себе не подтверждает новый guide, normative external URL/anchor или durable translation revision. P4/S36 требуют содержательную проверку ссылок сверх этих ограниченных scripts.
- Existing entry/related sources в обеих локалях найдены и прочитаны целевым поиском: guides/claude-code-setup.md, guides/mcp-setup.md, getting-started/installation.md, marketplace/overview.md. Fixed bundle claims installation EN:18,74-76/RU:18,83-85 и безусловное RU mcp availability утверждение :25-26 уже включены в future reconciliation PRD:102; это не пропущенная область данного Shape.

## Findings and Blockers

### B1 — HIGH, methodology prerequisite; полный design gate blocked

PRD-091:166-170 честно фиксирует mandatory Deep ADI attempt: Request timeout after 30000ms, retryable:no, outcome unknown. CLAUDE.md:203 требует ADI для Deep/Critical; canonical CLI показывает Deep у обоих кандидатов. Завершённого ADI output reviewer не получил. Это reported owner/tool observation, не самостоятельно повторённый failure; reason не перезапускался и процесс не опрашивался. Trigger снятия: owning session предоставляет завершённый ADI с идентичностью рассмотренных текстов/результатом и передаёт кандидата на fresh independent re-review. Таймаут нельзя считать ни успешным reasoning, ни доказанным отсутствием серверного результата.

### F1 — MEDIUM, artifact/tool integrity; metadata disagreement

SPEC-012 projection:2 говорит depth: standard; canonical CLI get --json и validate говорят deep/Deep. PRD-091 projection:2 и CLI согласованы на deep. Canonical bodies совпадают с projection bodies с обычным внешним newline framing, поэтому расхождение по depth нельзя объявить потерей/подменой body. Root cause не установлен. Trigger снятия: owning artifact/tool maintainer объясняет/согласует metadata через scoped canonical operation и повторяет validation/readback; новый projection hash требует обновления identity record. Нельзя молча считать standard и обходить Deep ADI. Здесь ни metadata rewrite, ни global reindex/rebuild не выполнялись.

### B2 — HIGH, reported cross-surface integrity limitation

Authoritative reviewer assignment сообщает: mounted MCP validate с workspace=/Users/explosovebit/Work/ForgePlan вернул Artifact not found для обеих новых IDs, при доступности через CLI; parent уже сообщил tool issue. В этом review CLI доступность и links подтверждены, MCP failure НЕ воспроизводился заново. Scope evidence/write использует canonical CLI, не недоступный MCP parent. Причина расхождения неизвестна; успешный CLI validation не доказывает исправность MCP visibility. Trigger снятия: owner получает согласованное read/validate обоих IDs через нужную поверхность либо явное owning решение о допустимой изоляции этой неисправности до следующего gate. Никакой глобальной переиндексации по догадке.

### B3 — external dependency, expected publication/implementation blocker, не дефект требований

PRD:104,114-124 и SPEC:187-196,293-297,323-325 требуют handback только от Marketplace setup task YWRXqH583HL980CfsVY1. Draft Marketplace PRD-080/SPEC-005 — внешние квалифицированные references, не core graph IDs/активная архитектура/released API. Exact catalog/export schema/version/hash/revision, approved architecture, selection examples, support/quoting commands, release evidence и canonical regulation path/revision ещё не переданы. Language migration явно deferred и не добавляется как dependency. Trigger снятия: parent предоставляет полный принятый handback checklist; сайт не публикует guessed executable advice. Producer process не опрашивался.

## Local Review Policy Determination

CLAUDE.md:516 находится внутри Rust coding rules (:502), сразу после new pub fn и pre-commit cargo правил (:505-515). After significant changes в этом разделе относится к code-change audit, не создаёт общего defect quota для draft requirements review. CLAUDE.md:679-685 отдельно задаёт significant changes как ≥5 commits OR new public API и назначает security/code reviewers с ≥3 issues каждому. В данном конечном review нет code delta, commits или нового public API; концептуальная consumer semantics прямо не wire API (SPEC:32,36,40,307). Поэтому ни двухагентный code/security audit, ни zero→re-review, ни ≥3 issues не применяются к этой Shape подготовке. Это толкование области действия owning CORE текста, не импорт Marketplace no-quota ADR и не изменение core policy.

Исторический active PRD-024 projection:333-348 содержит template HTML comment BMAD VALIDATION CHECKLIST с требованием ≥1 issue (:345-347). Этот footer относится к проверке самого старого parent artifact; refines не превращает его в repository-level operative policy для всех будущих детей. Родительское тело и его historical metadata не переписаны. Methodology skill фраза zero findings suspicious — повод проверить тщательность, а не разрешение фабриковать finding или расширять scope.

Неразрешённого policy blocker для честного конечного review здесь нет; отдельное owning CORE решение о zero-defect design review сейчас не требуется. Честный результат остаётся 0 содержательных пропусков и конкретные B1/F1/B2/B3, а не искусственная квота. Для будущего significant code delta нормы core сохраняют действие; это review их не отменяет.

## Resume and Exit

Finite independent review завершён этим draft Evidence, но полный design gate BLOCKER. PRD-091 и SPEC-012 остаются draft, родители untouched. Не выполнены reasoning retry, producer polling, source implementation, RFC/ADR creation, tracking writes, release/activation.

Owning session: сначала получить подтверждённый mandatory ADI и разрешить artifact/tool identity limitations; от Marketplace parent принять handback catalog/export/commands/regulation на immutable revision. Затем consistency mapping на S01–S36 и все семь входов; при изменении текста новый candidate/hash и fresh independent review. Только после принятых предпосылок — architecture/RFC с независимым gate и отдельно разрешённая MiniMax implementation. Runtime acceptance остаётся будущей работой по S33–S36, не текущим PASS. При изменении любой pinned body/source revision этот audit не переносится автоматически.

## Evidence Persistence Verification

Создан EVID-207 в draft через canonical CLI new/update. Первая informs-связь с PRD-091 записалась; первая попытка связи с SPEC-012 завершилась LanceStore transient error No space left on device (os error 28). Readback после ошибки показал только PRD-091 link; forgeplan validate EVID-207 тогда PASS, 0 errors/0 warnings. df -h . показал 927G size, 927G used, 120M available, 100% use. Без удаления/чистки/изменения чужих данных одна повторная canonical link операция затем успешно записала SPEC-012 informs. Этот сбой не скрывается как непрерывно успешная запись; малый остаток диска — риск следующих мутаций, не причина изобретать обход canonical API. Финальный readback/validation выполняются отдельно после сохранения этой записи.

