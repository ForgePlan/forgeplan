# Source Porting — что откуда портировалось в Forgeplan

Карта происхождения кода: какие концепции и паттерны из reference repositories
(`sources/`, read-only) реализованы в наших Rust crates. Полезно для онбординга
контрибьюторов и для понимания "откуда растут ноги" при рефакторингах.

---

## Таблица портирования

| Что портировано | Откуда (source) | Куда (Rust) |
|-----------------|-----------------|-------------|
| **Data model** (`ArtifactKind`, `Meta`, `Link`) | `sources/quint-code/src/mcp/internal/artifact/types.go` | `crates/forgeplan-core/src/artifact/types.rs` |
| **R_eff scoring** (52 LOC, weakest-link formula) | `sources/quint-code/src/mcp/internal/reff/reff.go` | `crates/forgeplan-core/src/scoring/reff.rs` |
| **SQLite schema** (9 tables) | `sources/quint-code/src/mcp/schema.sql` | Адаптация под LanceDB tables (`crates/forgeplan-core/src/db/`) |
| **CLI patterns** (clap derive) | `sources/git-adr/src/cli/` | `crates/forgeplan-cli/src/commands/` |
| **Template engine** (Tera) | `sources/git-adr/src/core/templates.rs` | `crates/forgeplan-core/src/template/` |
| **PRD 13-step validation** | `sources/BMAD-METHOD/src/bmm-skills/2-plan-workflows/create-prd/` | `crates/forgeplan-core/src/validation/` |
| **Artifact DAG + delta-specs** | `sources/OpenSpec/src/core/` | `crates/forgeplan-core/src/artifact/` |
| **Slash commands UX** | `sources/quint-code/src/mcp/cmd/commands/*.md` | CLI UX design (help texts) |
| **ADR CLI patterns** (bash original) | `sources/adr-tools/` | Идеи заимствованы в CLI structure |
| **Project management flows** | `sources/ccpm/` (Claude Code PM) | Влияние на Orchestra integration |

---

## Формула Forgeplan

```
Forgeplan = Quint-code (decision engine, R_eff scoring, evidence decay)
          + BMAD (PRD workflow, 13-step validation, adversarial review)
          + OpenSpec (artifact DAG, delta-specs, custom schemas)
          + FPF (reasoning framework, ADI cycle, trust calculus)
          + git-adr (Rust CLI patterns, clap, templates)
          + LanceDB (embedded DB: tables + vectors в одном)
          + Tauri (desktop app: React UI + shared Rust core) — planned
```

Каждый компонент формулы — отдельная школа мысли с собственными первоисточниками.
R_eff scoring из Quint-code — самый "дорогой" перенос (core trust calculus).
BMAD дал структуру PRD. OpenSpec — идею DAG зависимостей между артефактами.
FPF — рамку рассуждения (ADI cycle). git-adr — CLI ergonomics.

---

## Не портировано (сознательно)

- **quint-code SQLite storage** — заменён на LanceDB (vectors + tables в одном)
- **BMAD workflow engine** — упрощён до validation rules в нашем validator
- **OpenSpec change-spec generator** — идея взята, реализация deferred (delta-specs — будущая работа)
- **adr-tools bash** — только inspiration, не прямой порт

---

## Правила работы с `sources/`

- **READ-ONLY** — не редактировать upstream repos
- **Не копипастить** — читать как reference, писать свой Rust
- **Не vendor'ить** — git submodule / clone, не включать в наш binary
- **Обновление reference**: при значимых изменениях upstream — `git pull` в `sources/<repo>/`, пересмотр наших портов

---

## Связанные артефакты

- `ADR-003` — Markdown files as source of truth, LanceDB as index layer
- `PRD-001` — Foundation (Phase 0-3 scaffold)
- `EPIC-001` — Core foundation (closed)
- `EPIC-002` — v2.0 vision (closed)

---

*Восстановлено из CLAUDE.md audit 2026-04-17 (до audit размер CLAUDE.md был 816 строк,
после — 324; Reference Code таблица была удалена без миграции).*
