---
id: RFC-001
title: "CLI Architecture — модули, data flow, phases"
status: Draft
author: explosovebit
created: 2026-03-21
updated: 2026-03-21
prd: PRD-001
depth: deep
status: Active
---

# RFC-001: CLI Architecture

## Progress

```
Phase A  ████████████████████████  5/5   (100%)  Core CLI                   ✅ DONE
Phase B  ████████████████████████  4/4   (100%)  Search & Score             ✅ DONE
Phase C  ░░░░░░░░░░░░░░░░░░░░░░░░  0/3   (  0%)  Polish & Tests
─────────────────────────────────────────────────
TOTAL                               9/12  ( 75%)
```

---

## Summary

Архитектура CLI `forgeplan` — модульный Rust binary с shared core library. CLI тонкий: парсит аргументы и делегирует в `forgeplan-core`. Все 10 типов артефактов хранятся как Markdown с YAML frontmatter в `.forgeplan/`, config в YAML.

## Motivation

PRD-001 определяет 10 FR для CLI. Нужна архитектура, которая:
- Позволяет переиспользовать core между CLI, Tauri и MCP (Phase 4-5)
- Даёт < 100ms startup (NFR-001) и < 15MB binary (NFR-002)
- Поддерживает 10 типов артефактов с единой логикой CRUD + validation

Без RFC: каждая команда реализуется ad-hoc, логика дублируется, Tauri потребует рефакторинг.

## Goals

- Определить границы `forgeplan-core` vs `forgeplan-cli`
- Описать data flow для каждой команды
- Зафиксировать file layout `.forgeplan/`
- Разбить реализацию на phases с чёткими deliverables

## Non-Goals

- Desktop App архитектура (Phase 4, отдельный RFC)
- MCP server архитектура (Phase 5)
- LanceDB интеграция (Phase 3B, отдельный RFC)
- AI-генерация содержимого

## Options Considered

### Option A: Monolith binary

**Description**: Вся логика в `forgeplan-cli`, без отдельной library crate.

**Pros**: Простота, один Cargo.toml, быстрый старт разработки.

**Cons**: Невозможно переиспользовать в Tauri/MCP. Тесты завязаны на CLI parsing.

### Option B: Core library + thin CLI (выбран)

**Description**: `forgeplan-core` содержит всю бизнес-логику (CRUD, scoring, validation, templates). `forgeplan-cli` — тонкая обёртка с clap parsing и форматированием вывода.

**Pros**: Core тестируется без CLI. Tauri/MCP подключают core напрямую. Чёткое разделение ответственности.

**Cons**: Два crate, чуть больше boilerplate.

### Option C: Plugin architecture

**Description**: Каждая команда — отдельный crate с trait CommandHandler.

**Pros**: Максимальная модульность, easy to extend.

**Cons**: Over-engineering для 10 команд. Сложный DX.

## Trade-off Analysis

| Критерий | Option A: Monolith | Option B: Core+CLI | Option C: Plugins |
|----------|-------------------|-------------------|-------------------|
| Complexity | Low | Medium | High |
| Reusability | None | High | High |
| Binary size | Smallest | Small (+~0) | Larger |
| Testability | Medium | High | High |
| Developer experience | Simple | Good | Complex |
| Future Tauri/MCP | Rewrite needed | Ready | Ready |

## Proposed Direction

**Option B: Core library + thin CLI**. Уже реализован workspace scaffold. Core тестируется изолированно (4 теста проходят). Минимальный overhead, максимальная переиспользуемость.

---

## Architecture

### Module Layout

```
crates/
├── forgeplan-core/src/
│   ├── lib.rs                    # pub mod declarations
│   ├── artifact/
│   │   ├── mod.rs
│   │   ├── types.rs              # ArtifactKind, Meta, Status, Link (DONE)
│   │   ├── frontmatter.rs        # YAML frontmatter parse/write
│   │   └── store.rs              # Filesystem CRUD operations
│   ├── scoring/
│   │   ├── mod.rs
│   │   └── reff.rs               # R_eff scoring (DONE, 4 tests pass)
│   ├── config/
│   │   ├── mod.rs
│   │   └── types.rs              # Config struct + defaults
│   ├── template/
│   │   ├── mod.rs
│   │   └── engine.rs             # Tera engine + embedded templates
│   └── workspace/
│       ├── mod.rs
│       └── init.rs               # .forgeplan/ initialization
│
├── forgeplan-cli/src/
│   ├── main.rs                   # Cli parse + dispatch
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── init.rs               # forgeplan init
│   │   ├── new.rs                # forgeplan new <type> <title>
│   │   ├── list.rs               # forgeplan list [--type] [--status]
│   │   └── status.rs             # forgeplan status (dashboard)
│   └── output.rs                 # Table formatting, progress bars, colors
```

### Data Flow

```
User input                      CLI layer                  Core layer              Filesystem
─────────────────────────────────────────────────────────────────────────────────────────────

forgeplan init          →  commands/init.rs        →  workspace::init()      →  .forgeplan/
                            parse args                 create dirs               config.yaml
                            call core                  write config              subdirs/

forgeplan new prd "X"   →  commands/new.rs         →  template::render()     →  .forgeplan/prds/
                            parse kind+title           load template             PRD-001-x.md
                            call core                  inject frontmatter
                                                       auto-increment ID
                                                       store::write()

forgeplan list          →  commands/list.rs        →  store::list_all()      ←  .forgeplan/*/
                            parse filters              frontmatter::parse()      *.md files
                            format table               filter by kind/status

forgeplan status        →  commands/status.rs      →  store::list_all()      ←  .forgeplan/*/
                            format dashboard           count by kind/status
                            render progress bars       scoring::r_eff()
```

### File Layout: `.forgeplan/`

```
.forgeplan/
├── config.yaml          # Project config (name, default_depth, template_dir)
├── prds/                # PRD artifacts
├── epics/               # Epic artifacts
├── specs/               # Spec artifacts
├── rfcs/                # RFC artifacts
├── adrs/                # ADR artifacts
├── problems/            # ProblemCard artifacts
├── solutions/           # SolutionPortfolio artifacts
├── evidence/            # EvidencePack artifacts
├── notes/               # Note artifacts
└── refresh/             # RefreshReport artifacts
```

### Config Schema

```yaml
# .forgeplan/config.yaml
version: 1
project_name: "my-project"
default_depth: standard          # tactical | standard | deep | critical
id_digits: 3                     # PRD-001 vs PRD-0001
created_at: "2026-03-21T12:00:00"
```

### Auto-ID Generation

Алгоритм для `forgeplan new <type> <title>`:

1. Определить директорию по типу: `prds/` для prd, `epics/` для epic, etc.
2. Прочитать существующие файлы, извлечь максимальный ID
3. Инкрементировать: `max_id + 1`, pad нулями до `id_digits`
4. Slug из title: lowercase, пробелы → дефисы, убрать спецсимволы
5. Filename: `{PREFIX}{NNN}-{slug}.md` → `PRD-001-test-feature.md`

### Frontmatter Format

```yaml
---
id: PRD-001
title: "Test Feature"
status: Draft
author: ""
created: 2026-03-21
updated: 2026-03-21
depth: standard
kind: prd
---
```

---

## Risks & Open Questions

- **Risk**: Tera compile time — tera добавляет ~2s к компиляции. Mitigated: включаем только в core.
- **Risk**: Edition 2024 — некоторые crates могут не поддерживать. Mitigated: fallback to 2021.
- **Open**: Нужен ли `--editor` flag в `forgeplan new` для автоматического открытия в $EDITOR?
- **Open**: Формат вывода `list` — только table или JSON/CSV тоже?

## Implementation Phases

### Phase A: Core CLI (этот чат)
- [x] **A.1** Workspace scaffold (Cargo.toml, crates/)
- [x] **A.2** Types + R_eff scoring (artifact/types.rs, scoring/reff.rs)
- [x] **A.3** `forgeplan init` — workspace initialization
- [x] **A.4** `forgeplan new` — template engine + auto-ID
- [x] **A.5** `forgeplan list` + `forgeplan status` — frontmatter parser + dashboard

### Phase B: Search & Score
- [x] **B.1** `forgeplan validate` — schema rules engine (RFC-002)
- [x] **B.2** `forgeplan score` — R_eff CLI wrapper
- [x] **B.3** `forgeplan link` — typed relationships in frontmatter
- [x] **B.4** `forgeplan graph` — mermaid dependency graph

### Phase C: Polish & Tests
- [ ] **C.1** Error handling refinement (thiserror enum)
- [ ] **C.2** Integration tests (assert_cmd + tempdir)
- [ ] **C.3** >80% test coverage + CI

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| PRD-001 | PRD | based_on |
| EPIC-001 | Epic | parent |
| ADR-001 | ADR | informs (Rust вместо Go) |
| ADR-002 | ADR | informs (LanceDB вместо SQLite) |

---

> **Next step**: Реализовать Phase A.3-A.5 в этом чате. После завершения — создать ADR для принятых решений.
