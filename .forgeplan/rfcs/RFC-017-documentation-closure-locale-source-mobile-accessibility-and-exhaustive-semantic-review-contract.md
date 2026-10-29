---
depth: standard
id: RFC-017
kind: rfc
last_modified_at: 2026-10-23T18:16:12.524618+00:00
last_modified_by: documentation-walkthrough/1
links:
- target: PRD-089
  relation: based_on
status: draft
title: Documentation closure locale source mobile accessibility and exhaustive semantic review contract
---

---
assigned_number: 17
created: 2026-10-23
depth: standard
id: RFC-017
kind: rfc
last_modified_at: 2026-10-23T15:47:21.868918+00:00
last_modified_by: documentation-walkthrough/1
links:
- target: PRD-089
  relation: based_on
prd: null
predicted_number: 17
slug: rfc-documentation-closure-locale-source-mobile-accessibility-and-exhaustive
status: draft
title: Documentation closure locale source mobile accessibility and exhaustive semantic review contract
updated: 2026-10-23
---

---
assigned_number: 17
created: 2026-10-23
prd: null
predicted_number: 17
slug: rfc-documentation-closure-locale-source-mobile-accessibility-and-exhaustive
updated: 2026-10-23
---

# Documentation closure implementation contract

## Summary

Закрыть FR-001–FR-005 PRD-089: полноценные языковые тела уроков, правильный aria-expanded кнопки и исчерпывающий смысловой аудит документационного корпуса. Существующие проверенные изменения и их EVID остаются историческим снимком; это новый контракт, а не переименование старого PASS.

## Motivation

Прежние механические проверки не доказывают перевод body и смысл всех утверждений. Требуются точное владение источниками и проверяемый построчный ledger, чтобы читатель мог отличить полную выполненную проверку от поиска нескольких опасных шаблонов.

## Goals

- FR-001: выбрать отдельный явный English HTML source для каждого из20lesson slugs и standalone index, оставить существующие RU source paths без silentfallback.
- FR-002: заменить неисправную vendor-реализацию кнопки project-owned Starlight override, сохранив штатный sidebar/body/icon/Escape протокол.
- FR-003/FR-004: закрыть полный корпус в непересекающихся owned cohorts с реальным чтением всех строк, источниками проверенных утверждений и точечными исправлениями.
- FR-005: после технического итога объяснить условияG1/G2/G3 без изменения их носителей.

## Non-Goals

Не менять Rust/Constellation contracts, node_modules/lockfile/версии зависимостей, доступ/настройки настоящих проектов, native agent policies, память и активации. Не превращать исторические планы в текущие обещания. Не запускать Cargo. Не объявлять S1-G/S2.

## Options Considered

### Option A: Explicit English sources and a project-owned component

Выбрано: English standalone HTML в website/public/guides-raw/en/<slug>.html, существующий root raw HTML — RU source. EN route выбирает en path, RU route — существующий root path. Отсутствующий English source падает на build. Обоим языкам сохраняются inline extraction, stylesheet и script contracts. Кнопка переопределяется через поддержанный Starlight components mapping, без патча установленного пакета.

### Option B: Translate text nodes at request/build time

Отклонено: hash/dictionary-подстановка скрывает неполноту перевода, усложняет inline-script strings, canvas/SVG/aria подписи и делает исходник труднее для ручного смыслового review. Смысловые пары должны быть видимы в source.

### Option C: Migrate lessons to structured bilingual Astro/MDX content

Реальная альтернативная стратегия: перенести весь HTML-корпус в новые языковые content collections с общими presentation components и интерактивными islands; меню исправить через project-owned override, смысловой ledger провести полностью. Отклонено: меняет формат всех20интерактивных уроков и их standalone-варианта, требует переноса script/CSS contracts без требования пользователя. Отдельная компонентная альтернатива — dependency upgrade/patch — также отклонена: версия с исправлением не доказана, node_modules patch непереносим.

### Option D: Keep shared RU body and document limitations

Отклонено: пользователь теперь явно потребовал устранить ограничение и выполнить весь смысловой обзор, поэтому прежнего честного предупреждения недостаточно.

## Trade-off Analysis

OptionA дублирует presentation-код HTML в явных языковых парах, но сохраняет существующий формат самостоятельных уроков и не вводит неявный runtime-переводчик. Drift ограничивается структурным и смысловым сопоставлением пар и new ledger. CSS/fonts/design не унифицируются; политика брендинга и учебных объяснений сохраняется. Компонент содержит минимальную локальную адаптацию с указанием upstream версии для следующего upgrade.

## Proposed Direction

OptionA; один интегратор, writer worktrees от проверенного c852f40c baseline с exact current dirty-source seeding только owned paths. Родительский snapshot содержит509существующих docs/artifact изменений; они не перезаписываются и не приписываются новому автору. Независимые HTML cohorts, мобильный override и documentation cohorts используют разные пути. Read-only scout уточняет полный первый-party корпус, не исследует чужие context/credentials, а parent утверждает финальный manifest до source dispatch.

## Source Contract

- website/src/pages/guides/[slug].astro использует public/guides-raw/en/<slug>.html; RU counterpart сохраняет root RU path. Существующие external/static rawRU URLs не являются новым English fallback.
- website/public/guides-raw/en/index.html — самостоятельный EN index; route index остаётся существующим Astro catalog.
- MobileMenuToggle component source принадлежит website/src/components/; mapping в astro.config.mjs. Поддержанный override point подтверждён установленным schemas/components.ts:62 и components/PageFrame.astro:2,12: virtual:starlight/components/MobileMenuToggle. Использовать components.MobileMenuToggle в astro.config.mjs; не вводить запасной DOM observer или Header composition shim.
- На кнопке хранится aria-expanded. Body data-mobile-menu-expanded и icons согласованы; Escape закрывает и возвращает фокус. Для меню никаких изменений поведения сверх этого контракта; отдельные подтверждённые дефекты встраивания уроков перечислены ниже.
- Учебные тексты переводятся целиком, включая user-facing inline-script/SVG/canvas/aria/title strings; кодовые идентификаторы и работающие технические примеры не переводятся механически. Искажённые технические утверждения исправляются в обоих языках с основанием.

## Semantic Review Contract

Manifest включает все first-party contributor docs, EN/RU portal pages, blog, raw lessons/pairs, root entrypoints/CHANGELOG, документирующие templates и найденные документальные first-party дополнительные страницы. .forgeplan artifacts — источники доказательств, а не массово переписываемые документы; node_modules/third-party/caches/.local receipts исключены. Исторические handoffs/plans/releases не исключаются.

На каждый файл: path,sha256-before,lineCount,полные прочитанные диапазоны,класс current/historical/mixed/derived,claim-level authority/path+line или фактический CLI/MCP результат,исход supported/corrected/historically-framed,sha256-after,открытые вопросы. Наличие файла/маркер/mtime/regex/Cyrillic threshold не засчитываются за semantic review. Derived block/copy покрывается через полностью проверенный canonical source и доказанное точное преобразование; ручной текст читается отдельно полностью. Не оставлять unaudited строки/файлы в финальномреестре и не выдавать отсутствие найденного доказательства за доказанную ложность.

## ADI competing-strategy boundary

Сравнивать как взаимоисключающие ПОЛНЫЕ стратегии, не как части одного плана:
- Hypothesis H1 = OptionA: явные полные English HTML pairs + поддержанный project-owned menu override + полный semantic ledger.
- Hypothesis H2 = OptionB: build-time text-node/dictionary translation поверх общего HTML + поддержанный menu override + полный semantic ledger.
- Hypothesis H3 = OptionC: новая bilingual Astro/MDX representation для всех уроков/интерактивности + тот же menu override + полный semantic ledger.
- Hypothesis H4 = OptionD: ничего не исправлять, сохранить общийRUbody/vendor и ограниченный аудит; альтернатива честно отклонена как несовместимая с новым запросом.
ENtranslation, menu и audit НЕ считать тремя альтернативами: это три совместные обязанности любой принимаемой стратегии. Initial adi.json ошибочно дал3complementary implementation hypotheses и сам назвал их неальтернативами; он сохранён как ограниченное историческое наблюдение, не доказательство выбора между3дизайнами.

## Invariants

Нулевая source-мутация вне owned paths. Полный body урока не заменяется summary/placeholder. Текущие гарантии подтверждаются выбранным binary/source, исторические — своими датированными источниками или явной отметкой ограниченности. Шаблонные/project policy правила не меняются для прохождения ранее отложенного formal gate. Нет старого mounted Forgeplan MCP против migrated store.

## Implementation Phases

1. Полный inventory, approved manifest/cohorts, новый source admission и role-matched worktrees.
2. Полные EN-переводы+RUsemanticcorrections; mobile source override; полное чтение/исправление корпуса в owned cohorts.
3. Parent integration по точным owned hash/patch; независимый review source и semantic ledger; все дефекты HIGH/Critical исправлены, не padding.
4. Freeze, один tester: build+completeness+links+fragments+locale-source validation; parent реальный browser на EN/RU lessons и mobile menu, без повторного подтверждения старых известных отказов.
5. Постоянный semantic-audit report/ledger и Evidence; затем предметное объяснениеG1/G2/G3 с действительными полномочиями/следующими шагами. Активация не запрашивается.

## Verification

Все40lesson routes получают правильные locale bodies и смысловые пары; обе standalone index версии сохраняют ссылки. MissingEnglish-source negative probe не подменяетсяRU. Buttonclick/Enter/Space/Escape/focus/locale-navigation проверяются в браузере на390px и desktop; vendor-directory fingerprint неизменён. Ledger замыкается по всем manifest paths и1..lineCount; checks не объявляются процентом semantic или codecoverage без основания. Общие ссылки и build проверяются на новом снимке, старые495/496файловые прогоны не переиспользуются как результат новых страниц.

## Rollback Plan

Отменять только новые owned patches/EN files/project override и соответствующие documentary corrections; чужие существующие изменения и прежние frozen evidence не удалять. Данные проектов и индексы не мигрируются, installed packages не патчатся.

## Risks and Open Questions

Полный корпус может содержать дополнительные first-party документы вне прежних четырёх поверхностей: их перечень завершает read-only scope pass, не исключать тихо. Большие исторические файлы разбивать на полные диапазоны и сохранять provenance; невозможную ретроспективную причинность обозначать, не утверждать как факт. Missingnativepolicybinding остаётся отдельным unresolved вопросом после source задачи.

## Affected Files

website/public/guides-raw/*.html and new en/*.html; guide route caller; source-owned mobile component and astro.config.mjs; known documentation corpus paths from approved inventory; docs/operations semantic audit report/ledger and indexes; root CHANGELOG for actual changes. Approved inventory: 911 physical sources (823 documentation/copy sources, 32 first-party prototypes, 56 unpacked historical-package documents), plus 56 documentary ZIP member views, 21 new EN HTML sources, one new mobile component and the final additions/changed-line supplement. Three existing Markdown documentation files under crates/ are included; Rust source changes are not; website-only runtime corrections are explicitly enumerated below. No lockfile, real-project runtime config or native agent cache edits. The only approved Astro configuration change is components.MobileMenuToggle mapping; unrelated configuration changes are excluded. Exact non-overlapping current-root seed ownership is recorded in .local/documentation-closure-20261023/cohorts.json.

## Related Artifacts

PRD-089 — требования нового объёма. PRD-088/RFC-016 и EVID-192–196 — предшествующая история. GitHub#542 — постоянный home.


## Browser-discovered source supplement — 2026-10-23

После исходного dispatch и реальной браузерной проверки обнаружено двойное исполнение script-блоков урока: bodyHtml сохранял оригинальные теги, а обёртка повторно выводила собранные externalScripts/inlineScripts. Ошибки Canvas is already in use наблюдались в EN/RU intro, trust-calculus-overview и trust-calculus. Общий обработчик website/src/lib/embed-html.ts добавлен как отдельный существующий runtime-source supplement (211 исходных строк), не как задним числом включённый документ первоначального корпуса. Удалять script-блоки только из встроенного body; каждый собранный скрипт выводить однократно, standalone HTML не менять. Новых API/пропсов/зависимостей не вводить. Home: #542 issuecomment-6371590875. Первичная узкая правка сделана при диагностике; этот amendment не выдаётся за предварительное согласование той правки.

Также явно разрешена узкая правка Landing.astro: предоставленный head slot заменяет общий LandingMeta; без slot прежний fallback сохраняется. Не дублировать canonical/OG metadata. Все новые границы требуют независимой source-проверки и новой браузерной проверки, не наследуют прежний PASS.

По прямому запросу владельца AGENTS.md дополнен волнами по зависимостям: сначала общие контракты/схемы; готовые независимые участки одной волной до10; зависимые ждут свой вход; один владелец записи на файл и один владелец сборки. Это нормативное изменение разрешено пользователем, не самовольное ослабление gate. Нативные G1/G2/G3, память, активация, Rust, пакеты/lockfile и реальные конфигурации проектов остаются вне разрешённых изменений.
