---
depth: standard
id: RFC-016
kind: rfc
last_modified_at: 2026-10-23T11:45:15.492819+00:00
last_modified_by: documentation-walkthrough/1
links:
- target: PRD-088
  relation: based_on
status: draft
title: Constellation documentation contract and audit
---

---
assigned_number: 16
created: 2026-10-23
prd: null
predicted_number: 16
slug: rfc-constellation-documentation-contract-and-audit
updated: 2026-10-23
---

# RFC-016: Constellation documentation contract and audit

## Summary

Документировать существующий Constellation через принятые contributor и пользовательские EN/RU поверхности; проверить общий drift без изменения Rust-поведения и без повышения статуса runtime-приёмки.

## Motivation

Текущие инструкции разнесены между архитектурой, личным пилотом и порталами reference. В справочнике уже обнаружены противоречия моделей, кэша и rebuild-команд. Пользователю нужен один понятный путь настройки, а разработчику — сохранённые результаты проверки, а не неподтверждённая фраза о полной актуальности.

## Goals

- Реализовать FR-001–FR-005 PRD-088 в существующих разделах и навигации.
- Сопоставить reference с фактическим CLI/MCP и явно разделить published/unreleased.
- Сохранить общий аудит с числами, проверенной выборкой и задачами на оставшиеся находки.

## Non-Goals

Не менять API, model/runtime/index semantics, конфигурации настоящих проектов, статусы RFC-015 и S1-G/S2. Не переписывать исторические записи как текущие гарантии.

## Options Considered

### Option A: Дополнить только личный pilot guide

Минимальная правка, но она сохраняет машинные пути, не появляется в пользовательской навигации и не решает расхождения EN/RU/reference. Отклонено как неполное выполнение PRD.

### Option B: How-to + reference в действующей структуре, с отдельным внутренним контрактом

Выбрано. Пользователь находит переносимый сценарий и краткий справочник; maintainer — архитектуру, проверочный runbook и аудит. Автогенерация sidebar и существующие checks переиспользуются, а не создаётся второй сайт.

### Option C: Отдельный портал Constellation

Изолирует текст, но дублирует конфигурацию, навигацию и переводы; создаёт ещё один источник дрейфа. Отклонено как лишняя система для документации одной функции.

### Option D: Ничего не менять

Оставить текущие страницы и генераторы как есть. Отклонено: в исходном корпусе нет публичных Constellation страниц, недостаёт 33 CLI и 30 MCP detail-страниц на каждый язык для выбранного интерфейса; старые генераторы удаляют ручной текст, а текущие рекомендации содержат опасное удаление workspace. Вариант не выполняет FR-001–FR-005. Его плюс — отсутствие дельты, но это сохраняет подтверждённые риски и не отвечает запросу пользователя.

## Trade-off Analysis

| Criterion | A | B | C |
|---|---|---|---|
| Полнота пользовательского пути | Недостаточная | Достаточная при проверке примеров | Возможна, но отдельно от привычной навигации |
| Повторное использование структуры | Только runbook | How-to/reference/architecture по текущим правилам | Новая инфраструктура |
| Риск дрейфа | Сохраняется | Измеряется и исправляется | Дополнительный источник |

## Proposed Direction

Option B. Примеры используют изолированный HOME/workspace и готовую сборку; source/reference вывод берётся из того же снимка реализации. Публичные тексты описывают параметры, состояние и ошибки, а не личную настройку этой машины. Авторитетные утверждения ограничены фактически проверенным поведением.

## Documentation Contract

- Корневые docs — contributor-аудитория; website/src/content/docs — пользовательский портал.
- Новые how-to/reference имеют EN/RU пары. Ссылки и sidebar проверяются в собранном сайте.
- full MCP сохраняет запись; constellation-read — дополнительное чтение handles. Локальный и межпроектный поиски не смешиваются.
- Авто-embed требует явного config.embedding, cached модели, semantic-search build и живого maintenance.
- Секции токен-ограничены, ranges body-absolute, layout v2; ContextPack использует отдельную приблизительную оценку chars/4.
- Markdown остаётся canonical; registry и индекс — разные сущности. Не выполнять незаметную миграцию боевых workspace.
- Руководства отделяют v0.38.0 и unreleased 0.39.0; сборка/узкие probes не объявляются S1-G.
- CLI validate на migrated workspace сейчас даёт named-отказ; full MCP forgeplan_validate на том же PRD прошёл. Это документируется как граница, не обходится старым бинарником и не чинится в Rust в этой задаче.

## Invariants

- Документационная дельта не меняет Rust, публичные CLI/MCP API, текущие настройки проектов или статус S1-G.
- Любое описание команды различает поддержанный путь, named-отказ и историческое наблюдение.
- Публичный текст не зависит от личного каталога владельца и не подменяет source-of-truth Markdown производным индексом.

## Rollback Plan

При ошибке откатываются только принадлежащие этой задаче документационные изменения и их ссылки; чужие рабочие изменения не затрагиваются. Личный pilot log сохраняется отдельно перед заменой публичного руководства. Runtime и настройки проектов не меняются, поэтому rollback документации не требует миграции данных или замены бинарников.

## Risks & Open Questions

- Общий корпус велик: механический инвентарь и ручной semantic review должны быть различимы в отчёте.
- Генераторы могут покрывать только корневые команды или full MCP; выявленные ограничения фиксируются, а не выдаются за полное покрытие.
- Исторические claims и приватный pilot log должны быть помечены/сохранены отдельно, не потеряны при замене публичного текста.

## Implementation Phases

### Phase 1: Inventory and source contract

- [x] Составить карту корпуса, языковых пар и источников reference.
- [x] Сверить новые команды, профили, модель и рискованные текущие примеры с кодом/help/runtime.

### Phase 2: Documentation

- [x] Оформить how-to/reference EN/RU и переносимый contributor runbook.
- [x] Исправить подтверждённые противоречия в затронутых текущих страницах и навигации.

### Phase 3: Verification and evidence

- [ ] Выполнить checks ссылок, coverage и переводов; различить исходные и новые отказы.
- [ ] Собрать сайт, проверить новые страницы в браузере, сохранить Evidence и оставшиеся находки.

## Observed context for ADI

- Полный инвентарь этой ветки до изменений: 181 contributor Markdown, EN171/RU170 страниц портала; changelog намеренно EN-only.
- Реальный c852f40c binary:120 CLI paths,84 roots; full MCP76 и read9. До изменений были87 CLI detail и55 MCP detail на язык:33+30 отсутствующих detail-страниц, включая весь read-profile.
- Scope: документы и Node generators/checkers, production Rust/API и реальные настройки проектов не меняются.
- FR-001: навигация EN/RU на how-to/reference; FR-002: full writes vs read handles и явные условия model/cache/maintenance; FR-003: воспроизводимая проверка get/search/coverage/errors; FR-004: повторяемые corpus/interface/link/locale checks; FR-005: сохранённый реестр findings и limits.
- Варианты действительно различны: D ничего не менять, A только личный pilot, B действующая how-to/reference структура, C отдельный портал. Выбран B.
- Scratch runtime: two source search/ambiguity/qualified get, exact canonical CLI get, missing write tool on read profile, snapshot-specific identities, queued auto embedding5/5 после MCP update без ручного embed. Это ограниченные наблюдения, не S1-G/S2.

## Verification-driven corrections

- Первое независимое ревью выявило два входа чужого workspace: поиск родителей от TMPDIR и восстановление FORGEPLAN_WORKSPACE из ancestor .env до CLI parse. Metadata discovery проверяет канонических родителей до spawn и явно передаёт пустую переменную; EN/RU shell-сценарии прекращают работу до artifact writes при отсутствии локального config. Приоритеты resolver не меняются. Ошибка и условие повторной проверки сохранены в GitHub #541; прежняя политика обсуждена в #303.
- Реальный первый build текущего дерева прошёл:566 страниц,22.89s, два известных baseline warning. При этом отдельный link-check нашёл94 неработающие ссылки/30 уникальных targets; contributor-check выявил21 исходную неработающую ссылку. Это не выдаётся за final PASS.
- В той же выбранной архитектуре исправляются источники ссылок: locale-aware href нормализация только известных lessons из catalog при inlining, все два extractEmbed callers без shim, шесть blog MDX и prebuild copy-changelog для существующих contributor-файлов. DocsHeader использует стандартный _top и не создаёт отсутствующую RU404-ссылку. Нового портала, iframe или маршрутов-заглушек нет.
- Оба новых link-source slices реализованы coder в отдельных worktrees; integration owner копирует проверенные полные тексты. Все финальные проверки выполняет один tester после нового freeze; browser — parent. Исторические late gates/parent-written Node сохраняются как deviations, не исправляются задним числом.
- Расширенные affected files: website/scripts/{reference-runtime,generate-cli-docs,generate-mcp-docs,check-content-completeness,check-translation-drift,copy-changelog}.mjs; website/src/{lib/embed-html.ts,data/guides.ts,components/DocsHeader.astro,pages/guides/[slug].astro,pages/ru/guides/[slug].astro}; EN/RU blog {how-to-use-fpf,what-is-fpf,where-fpf-helps}; historical brownfield example links. Production Rust и настоящие project configs не меняются.

## Verification

Фактические CLI --help и MCP tools/list; повторяемые изолированные CLI/stdio сценарии; существующие site checks и npm run build; браузерная проверка EN/RU новых маршрутов. Cargo не нужен: production Rust не меняется.

## Affected Files

README.md и соответствующие RU entrypoints; docs/README*; docs/architecture/constellation.md; docs/operations/CONSTELLATION-PILOT.ru.md; текущие schema/operations страницы с подтверждённым drift; website/src/content/docs/{docs,ru/docs}/{getting-started,guides,reference,cli,mcp}; website/README.md. Генераторы/checks меняются только если это необходимо для подтверждённого документационного контракта.

## Related Artifacts

PRD-088 — продуктовая задача документации. RFC-015 — функция, которую описываем, без изменения её приёмки.
