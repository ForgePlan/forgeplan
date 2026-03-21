---
id: PRD-001
title: "Forgeplan CLI"
status: In Progress
author: explosovebit
created: 2026-03-21
updated: 2026-03-21
epic: EPIC-001
priority: P0
depth: deep
domain: general
projectType: cli_tool
stepsCompleted: []
---

# PRD-001: Forgeplan CLI

## Progress

```
FR-001   ████████████████████████  1/1   (100%)  init workspace       ✅
FR-002   ████████████████████████  1/1   (100%)  new artifact          ✅
FR-003   ████████████████████████  1/1   (100%)  list artifacts        ✅
FR-004   ████████████████████████  1/1   (100%)  status dashboard      ✅
FR-005   ░░░░░░░░░░░░░░░░░░░░░░░░  0/1   (  0%)  validate
FR-006   ░░░░░░░░░░░░░░░░░░░░░░░░  0/1   (  0%)  score R_eff
FR-007   ░░░░░░░░░░░░░░░░░░░░░░░░  0/1   (  0%)  graph
FR-008   ░░░░░░░░░░░░░░░░░░░░░░░░  0/1   (  0%)  search
FR-009   ░░░░░░░░░░░░░░░░░░░░░░░░  0/1   (  0%)  link artifacts
FR-010   ░░░░░░░░░░░░░░░░░░░░░░░░  0/1   (  0%)  stale detection
─────────────────────────────────────────────────
TOTAL                               4/10  ( 40%)
```

---

## Executive Summary

### Vision

Single-binary CLI для создания, валидации и отслеживания структурированных артефактов в любом проекте. Один инструмент заменяет ad-hoc процессы документирования решений.

### Problem

Документы (PRD, RFC, ADR) создаются ad-hoc без стандартов. Нет связей между артефактами --- решение в ADR не привязано к PRD, который его породил. Нет quality gates --- документ считается "готовым" без проверки полноты. Каждый проект изобретает процесс заново, теряя накопленный опыт.

**Impact**: Потеря контекста при смене разработчиков. Дублирование решений. Stale документы без механизма обнаружения. Невозможность оценить качество решений без ручного аудита.

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| Developer | Разработчик, создающий фичи и фиксящий баги | Нет шаблонов, каждый раз документ с нуля; непонятно какой тип документа создать |
| Tech Lead | Отвечает за техническое качество команды | Нет обзора прогресса и качества решений; ручной review каждого документа |
| Architect | Принимает архитектурные решения | Нет поиска по прошлым решениям; нет tracking evidence и decay |

### Differentiators

- R_eff scoring (weakest link) --- качество решения определяется самым слабым evidence, а не средним
- Embedded vector search --- семантический поиск по всем артефактам без внешних зависимостей
- 10 типов артефактов --- от Note до Epic, каждый со своим lifecycle и validation rules
- Depth calibration --- автоматический выбор уровня детализации по сложности задачи
- Local-first --- все данные локально, git для sync, работает offline

---

## Success Criteria

<!-- BMAD: Каждый критерий MUST быть SMART --- Specific, Measurable, Achievable, Relevant, Time-bound. -->
<!-- Запрещены формулировки: "улучшить", "повысить", "ускорить" без конкретных чисел. -->

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-01 | Time to first artifact | seconds | N/A | <30s | Phase 3A | `time forgeplan init && forgeplan new prd test` |
| SC-02 | Validation coverage | % required sections | 0% | 100% | Phase 3C | `forgeplan validate --all` |
| SC-03 | Binary size | MB | 0 | <15MB | Phase 3A | `ls -la target/release/forgeplan` |
| SC-04 | Search latency | ms | N/A | <500ms | Phase 3B | benchmark on 1000 artifacts |
| SC-05 | Test coverage | % | 0% | >80% | Phase 3C | `cargo tarpaulin` |

---

## Product Scope

### MVP (In-Scope)

- `forgeplan init` --- инициализация `.forgeplan/` workspace в любой директории
- `forgeplan new <type> <title>` --- создание артефакта из embedded шаблона (10 типов)
- `forgeplan list` --- список всех артефактов с фильтрацией по типу и статусу
- `forgeplan status` --- обзор проекта с progress bars и R_eff scores
- `forgeplan validate` --- проверка полноты артефакта по schema rules
- `forgeplan score <id>` --- вычисление R_eff quality score по evidence
- `forgeplan graph` --- генерация mermaid dependency graph
- `forgeplan search <query>` --- поиск по всем артефактам

### Out of Scope

- Desktop App (Tauri + React) --- Phase 4
- MCP Server --- Phase 5
- AI-генерация содержимого артефактов
- Real-time collaboration
- Cloud sync (только git)

### Growth Vision

- Desktop App с визуальным редактором артефактов (Phase 4)
- MCP интеграция для AI-инструментов (Phase 5)
- AI auto-capture решений из чатов и commits
- Plugin system для custom artifact types

---

## User Journeys

<!-- BMAD: Для каждого типа пользователя (из Target Users) описать минимум один journey. -->
<!-- Каждый journey должен иметь хотя бы один FR в секции Functional Requirements. -->

### Journey 1: Developer --- Создание первого артефакта

**Цель пользователя**: Быстро создать PRD для новой фичи и валидировать его полноту.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan init` | Создаёт `.forgeplan/` с config.yaml | Один раз на проект |
| 2 | `forgeplan new prd "Feature X"` | Создаёт `PRD-001-feature-x.md` из шаблона с auto-incremented ID | Открывается в $EDITOR |
| 3 | Заполняет шаблон в редакторе | --- | Шаблон содержит подсказки и BMAD reminders |
| 4 | `forgeplan validate PRD-001` | Проверяет обязательные секции, выводит список ошибок | Per depth level |
| 5 | `forgeplan new rfc "Feature X Architecture"` | Создаёт RFC с автоматической ссылкой на PRD-001 | Link type: based_on |

**Результат**: Разработчик создал PRD и RFC за 5 минут с гарантией полноты.

### Journey 2: Tech Lead --- Обзор прогресса и качества

**Цель пользователя**: Оценить текущий статус проекта и качество принятых решений.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan status` | Выводит progress bars по всем артефактам, общий % | Aggregated view |
| 2 | `forgeplan list --type adr --status accepted` | Список принятых ADR | Фильтрация |
| 3 | `forgeplan score ADR-001` | R_eff = 0.7 (weakest: benchmark evidence expired) | Показывает каждый evidence score |
| 4 | `forgeplan graph` | Mermaid-граф зависимостей между артефактами | Копируется в документацию |

**Результат**: Tech Lead видит полную картину проекта с объективными метриками качества.

### Journey 3: Architect --- Поиск и создание архитектурного решения

**Цель пользователя**: Найти связанные решения и создать новый ADR с полным контекстом.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan search "authentication"` | Находит связанные артефакты по семантике | Vector + keyword search |
| 2 | Изучает найденные решения | --- | Контекст для нового решения |
| 3 | `forgeplan new adr "Choose Auth0" --depth deep` | Создаёт ADR с полным DDR шаблоном (invariants, rollback, valid_until) | Deep = все секции |

**Результат**: Architect принял решение с полным контекстом и evidence tracking.

---

## Functional Requirements

<!-- ============================================================ -->
<!-- BMAD QUALITY REMINDERS (НЕ УДАЛЯТЬ):                        -->
<!--                                                              -->
<!-- FORMAT: "[Actor] can [capability]"                            -->
<!--   OK:    "User can filter projects by status"                -->
<!--   BAD:   "Filter component renders project list"             -->
<!--                                                              -->
<!-- NO IMPLEMENTATION LEAKAGE:                                   -->
<!--   Запрещены названия технологий (React, Django, PostgreSQL,  -->
<!--   Redis, AWS, Docker, etc.) ЕСЛИ они не являются частью      -->
<!--   capability. PRD описывает ЧТО, не КАК.                    -->
<!--   OK:    "API consumer can retrieve data via REST endpoint"  -->
<!--   BAD:   "React component fetches data using Redux store"    -->
<!--                                                              -->
<!-- NO SUBJECTIVE ADJECTIVES:                                    -->
<!--   Запрещены: "быстро", "удобно", "интуитивно", "легко",     -->
<!--   "просто", "эффективно" --- без конкретных метрик.            -->
<!--                                                              -->
<!-- TRACEABILITY:                                                -->
<!--   Каждый FR MUST traceably link to a User Journey.           -->
<!--   Orphan FR (без связи с journey) = validation failure.      -->
<!-- ============================================================ -->

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Core | Must | User can initialize a `.forgeplan/` workspace in any project directory | Journey 1 |
| FR-002 | Core | Must | User can create a new artifact of any supported type from embedded template | Journey 1 |
| FR-003 | Core | Must | User can list all artifacts filtered by type and status | Journey 2 |
| FR-004 | Core | Must | User can view project status with progress bars and R_eff scores | Journey 2 |
| FR-005 | Validation | Must | User can validate artifact completeness against schema rules | Journey 1 |
| FR-006 | Scoring | Should | User can compute R_eff quality score for decisions with evidence | Journey 2 |
| FR-007 | Graph | Should | User can generate a mermaid dependency graph of linked artifacts | Journey 2 |
| FR-008 | Search | Should | User can search artifacts by keyword across all content | Journey 3 |
| FR-009 | Core | Must | User can link artifacts with typed relationships (informs, based_on, supersedes) | Journey 1 |
| FR-010 | Lifecycle | Should | User can detect stale artifacts where valid_until has expired | Journey 2 |

---

## Non-Functional Requirements

<!-- ============================================================ -->
<!-- BMAD QUALITY REMINDERS (НЕ УДАЛЯТЬ):                        -->
<!--                                                              -->
<!-- FORMAT: "System shall [metric] [condition] [measurement]"    -->
<!--   OK:    "System shall respond within 200ms at p95 under     -->
<!--           1000 concurrent users, measured by APM"            -->
<!--   BAD:   "System should be fast and responsive"              -->
<!--                                                              -->
<!-- MEASURABILITY:                                               -->
<!--   Каждый NFR MUST содержать конкретное число и метод         -->
<!--   измерения. Запрещены: "быстрый", "отзывчивый",            -->
<!--   "масштабируемый", "надёжный" без цифр.                     -->
<!--                                                              -->
<!-- TEMPLATE: criterion + metric + condition + measurement       -->
<!-- ============================================================ -->

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Performance | CLI shall start and execute command | < 100ms | Любая команда кроме search | `hyperfine forgeplan status` |
| NFR-002 | Size | Binary shall not exceed size limit (without ONNX) | < 15MB | Release build, stripped | `ls -la target/release/forgeplan` |
| NFR-003 | Availability | CLI shall work without network connection | 100% core features | Все команды кроме AI-генерации | Тест в airplane mode |
| NFR-004 | Portability | CLI shall run on all major platforms | macOS + Linux + Windows | CI matrix | GitHub Actions CI |

---

## Acceptance Criteria

<!-- Обязательно для depth: deep / critical. Опционально для standard. -->
<!-- Формат: Given / When / Then (Gherkin-style) -->

### AC-1: Инициализация workspace

```gherkin
Given пустая директория без .forgeplan/
When  пользователь выполняет `forgeplan init`
Then  создаётся .forgeplan/ с config.yaml
And   создаются поддиректории для всех типов артефактов
```

### AC-2: Создание артефакта из шаблона

```gherkin
Given .forgeplan/ существует в текущей директории
When  пользователь выполняет `forgeplan new prd "Test Feature"`
Then  создаётся файл PRD-001-test-feature.md из embedded шаблона
And   ID автоматически инкрементируется (следующий будет PRD-002)
And   frontmatter заполнен с текущей датой и ссылкой на epic (если указан)
```

### AC-3: Валидация артефакта

```gherkin
Given артефакты существуют в .forgeplan/
When  пользователь выполняет `forgeplan validate`
Then  все обязательные секции проверяются в соответствии с depth level
And   выводится список ошибок с указанием секции и правила
```

### AC-4: R_eff scoring

```gherkin
Given ADR с привязанным evidence (benchmarks, tests)
When  пользователь выполняет `forgeplan score ADR-001`
Then  R_eff вычисляется как min(evidence_scores)
And   expired evidence (valid_until < now) получает score 0.1
And   выводится breakdown по каждому evidence
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| ADR-001: Rust вместо Go | Architectural | Accepted | explosovebit |
| ADR-002: LanceDB вместо SQLite | Architectural | Accepted | explosovebit |
| ADR-003: DEC merged into ADR | Architectural | Accepted | explosovebit |
| Schemas & Templates (Phase 1) | Technical | Done | explosovebit |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-001 | LanceDB Rust SDK breaking changes | Medium | High | Pin версии, fallback to file-based search, абстракция storage layer | explosovebit |
| R-002 | Изменение формата шаблонов ломает существующие артефакты | Low | High | Versioned frontmatter, migration tool при обновлении | explosovebit |
| R-003 | ONNX Runtime увеличивает binary size > 15MB | High | Medium | Optional feature flag, download ONNX model on first use | explosovebit |

---

## Timeline

<!-- Обязательно для depth: deep / critical. -->

| Milestone | Target Date | Description |
|-----------|-------------|-------------|
| PRD Approved | 2026-04-01 | Requirements locked, ready for Spec |
| Spec Complete | 2026-04-15 | API contracts и data model определены |
| RFC Approved | 2026-04-30 | CLI architecture решена |
| Phase 3A: Core CLI | 2026-05-31 | init, new, list, status |
| Phase 3B: Search & Score | 2026-06-15 | validate, score, search, graph |
| Phase 3C: Polish & Tests | 2026-06-30 | >80% coverage, release binary |

---

## Stakeholders

<!-- Обязательно для depth: deep / critical. -->

| Role | Name | Sign-off |
|------|------|----------|
| Product Owner | explosovebit | [ ] |
| Engineering Lead | explosovebit | [ ] |

---

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-001 | Parent epic | Active |
| ADR-001 | Informs (Rust вместо Go) | Accepted |
| ADR-002 | Informs (LanceDB вместо SQLite) | Accepted |
| ADR-003 | Informs (DEC merged into ADR) | Accepted |

---

<!-- ============================================================ -->
<!-- BMAD VALIDATION CHECKLIST (для автора и ревьюера):           -->
<!--                                                              -->
<!-- [x] Executive Summary содержит vision + problem + users      -->
<!-- [x] Success Criteria --- все SMART с числами                   -->
<!-- [x] Product Scope --- MVP чётко отделён от out-of-scope        -->
<!-- [x] User Journeys --- минимум 1 на каждую персону              -->
<!-- [x] FR --- формат "[Actor] can [capability]", нет impl leakage -->
<!-- [x] NFR --- конкретные метрики, метод измерения                -->
<!-- [x] Traceability --- каждый FR ссылается на journey            -->
<!-- [x] Acceptance Criteria --- Given/When/Then (deep/critical)    -->
<!-- [x] Risks --- минимум 1 риск с mitigation                      -->
<!-- [x] Related Artifacts --- ссылки на SPEC/RFC/ADR если есть     -->
<!--                                                              -->
<!-- ADVERSARIAL REVIEW (BMAD):                                   -->
<!-- Ревьюер ОБЯЗАН найти минимум 1 проблему.                     -->
<!-- 0 найденных проблем = недостаточно тщательный review.        -->
<!-- ============================================================ -->

> **Next step**: После approve -> создать SPEC (контракты) и/или RFC (архитектура).
