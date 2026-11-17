[English](SPEC-SCHEMA.md) · [Русский](SPEC-SCHEMA.ru.md)

# SPEC Schema — Formal Specification

## Когда создавать Spec

- API design (REST, GraphQL, gRPC)
- Data model changes
- Protocol definition
- UI component specification
- Integration contract

**Правило**: SPEC нужен, когда реализацию следует проверить по контракту: структуре API/данных, наблюдаемому поведению или обоим. PRD объясняет потребность пользователя и её причины; SPEC фиксирует, чему должна соответствовать реализация.

## Обязательные секции

Forgeplan **0.38.0** принимает структурную и поведенческую формы SPEC на любой глубине. MUST-проверки runtime:

| Проверка | Допустимые заголовки / поля |
|----------|----------------------------|
| Метаданные | Непустые `id` и `status` во frontmatter |
| Краткое описание | `Summary`, `Executive Summary`, `Overview` |
| Проверяемый контракт (хотя бы один) | Структурный: `API…`, `Data Model…`, `Contract…`; поведенческий: `Requirements…`, `Behavioral Contract…`, `Behavioural Contract…` |
| Связанные артефакты | `Related…` или `Dependencies…` |

Заголовки сопоставляются без учёта регистра по префиксу на любом уровне Markdown. Автор, родительский PRD, тип, scope, правила валидации, события и версия — рекомендации шаблона, а не дополнительные MUST runtime. Сохраняйте английские заголовки и при русском тексте.

### Структурный контракт

Опишите схемы запросов и ответов, ошибки, ограничения и примеры в `## API Contracts`, `## Data Models` или `## Contract`. Для прохождения валидации этой форме **не нужны** искусственные блоки `Requirement`/`Scenario`.

### Поведенческий контракт и тестовый эталон

Публичный `/spec-author` использует наблюдаемые сценарии как основной эталон: ожидаемое поведение, по которому проверяют тесты и код.

```markdown
## Summary
Поведение исполнителя при ошибке шага.

## Requirements
### Requirement: Stop after failure
Исполнитель MUST остановиться, если шаг завершился с ненулевым статусом.
#### Scenario: Second step fails
- GIVEN запуск из трёх шагов
- WHEN второй шаг завершился со статусом 1
- THEN третий шаг не выполняется
- AND отчёт называет второй шаг ошибочным

## Related
У этого самостоятельного примера нет родительского артефакта.
```

`spec-requirement-has-scenario` — правило **SHOULD**. Оно действует только для открытых в тексте блоков `### Requirement…`: внутри каждого нужен `#### Scenario…` до следующего заголовка уровня 3 или выше. Примеры в кодовых блоках и HTML-комментарии это правило игнорирует. Оно проверяет структуру заголовков, а не смысл GIVEN/WHEN/THEN или покрытие тестами. Один заголовок `Requirements` может удовлетворить MUST контракта, но не доказывает наличие пригодного эталона.

Процесс маркетплейса строже core-валидации. `/spec-author` требует проверить и зафиксировать сценарии до реализации; разработчик не должен переписывать их ради зелёных тестов. Отдельный цикл `/tdd` из `agents-tdd` требует **active PRD + active SPEC со сценариями**, составляет план тестов, получает RED на упавшем утверждении и перед GREEN передаёт тесты независимому валидатору. Хуки этого процесса запрещают менять тесты и закрепляют хеш SPEC; один `forgeplan activate` не устанавливает и не обеспечивает такую защиту.

Conformance vectors — опция для чистого алгоритмического ядра или проверки эквивалентности языков, а не общее требование к SPEC. Добавляйте псевдокод, инварианты и API-контракты по необходимости, не подменяя ими наблюдаемые сценарии в сценарном процессе.

### Валидация не проверяет реализацию

Общие проверки плейсхолдеров и заготовок действуют и для SPEC. Нетронутый структурный шаблон — не готовый контракт: множество `{placeholder}` вызывает `no-stub-content` (SHOULD при валидации, блокировка при активации). Содержание и доказательства проверяются отдельно; PASS схемы не подтверждает поведение.

## Spec Types

| Type | Содержит | Example |
|------|----------|---------|
| API | Endpoints, request/response, errors | REST API для user management |
| Data Model | Entities, relationships, constraints | Prisma schema changes |
| Protocol | Message format, sequence | WebSocket protocol |
| UI Spec | Components, states, interactions | Design system component |

## Numbering

| Format | Example |
|--------|---------|
| ID | `SPEC-NNN` |
| File | `SPEC-{NNN}-{kebab-case}.md` |
| Path | `.forgeplan/specs/SPEC-015-oauth2-api.md` |

SPEC использует общий жизненный цикл `draft`, `active`, `stale`, `superseded`, `deprecated`, а не `Approved` / `Implemented`. Меняйте зафиксированный сценарий через явную ревизию/delta SPEC с supersession, затем перезапускайте соответствующий цикл реализации.

## Связанные документы

- [PRD-SCHEMA.ru.md](../schemas/PRD-SCHEMA.ru.md) — Структура и правила валидации PRD
- [EPIC-SCHEMA.ru.md](../schemas/EPIC-SCHEMA.ru.md) — Структура и агрегация Epic
- [ARTIFACT-MODEL.ru.md](../methodology/ARTIFACT-MODEL.ru.md) — Полная иерархия артефактов
