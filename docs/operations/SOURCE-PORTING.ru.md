# Source Porting — что откуда портировалось в Forgeplan

Карта происхождения кода: какие концепции и паттерны из reference repositories
(`sources/`, read-only) реализованы в наших Rust crates. Полезно для онбординга
контрибьюторов и для понимания "откуда растут ноги" при рефакторингах.

> Карта влияний, восстановленная 2026-04-17, не доказательство побайтового
> переноса или текущей эквивалентности upstream. Старые пути/LOC — provenance
> той карты. Текущие различия сверены с meaning-authority для `c852f40c`:
> FPF `34a1ae7145309f8c18b4c8864513e3456d43de71`, BMAD
> `d1163f85e1e45840061496531f4bc25bc7639d8c`; для local Quint/OpenSpec Git SHA
> не установлен. Формула ниже перечисляет источники идей, не состав бинаря.

---

## Таблица портирования

| Что портировано | Откуда (source) | Куда (Rust) |
|-----------------|-----------------|-------------|
| **Data model** (`ArtifactKind`, `Meta`, `Link`) | `sources/quint-code/src/mcp/internal/artifact/types.go` | `crates/forgeplan-core/src/artifact/types.rs` |
| **R_eff scoring** (52 LOC — историческая метка исходной карты) | `sources/quint-code/src/mcp/internal/reff/reff.go` | Собственная flat/recursive адаптация в `crates/forgeplan-core/src/scoring/reff.rs`, не текущая формула FPF |
| **SQLite schema** (9 tables) | `sources/quint-code/src/mcp/schema.sql` | Адаптация под LanceDB tables (`crates/forgeplan-core/src/db/`) |
| **CLI patterns** (clap derive) | `sources/git-adr/src/cli/` | `crates/forgeplan-cli/src/commands/` |
| **Template engine** (Tera) | `sources/git-adr/src/core/templates.rs` | `crates/forgeplan-core/src/template/` |
| **PRD workflow / validation inspiration** | BMAD create-prd (путь исходной карты; layout upstream менялся) | Собственные depth-dependent rules в `crates/forgeplan-core/src/validation/`; upstream 13 validation steps не означают 13 обязательных секций PRD |
| **Artifact DAG + delta-specs** | `sources/OpenSpec/src/core/` | `crates/forgeplan-core/src/artifact/` |
| **Slash commands UX** | `sources/quint-code/src/mcp/cmd/commands/*.md` | CLI UX design (help texts) |
| **ADR CLI patterns** (bash original) | `sources/adr-tools/` | Идеи заимствованы в CLI structure |
| **Project management flows** | `sources/ccpm/` (Claude Code PM) | Влияние на Orchestra integration |

---

## Формула Forgeplan

```
Forgeplan = Quint-code (decision engine, R_eff scoring, evidence decay)
          + BMAD (PRD workflow и adversarial review как источники идей)
          + OpenSpec (artifact DAG, delta-specs, custom schemas)
          + FPF (reasoning framework, ADI cycle, trust calculus)
          + git-adr (Rust CLI patterns, clap, templates)
          + LanceDB (embedded DB: tables + vectors в одном)
          + Tauri (desktop app: React UI + shared Rust core) — planned
```

Каждый компонент формулы — отдельная школа мысли с собственными первоисточниками.
Происхождение не доказывает идентичность алгоритмов. Rust FGR — локальные
Formality/Granularity/Reliability и geometric mean; текущий FPF использует
F/ClaimScope/R с иными шкалами. Rust recursive R_eff отличается и от flat
проекции, и от внешних калькуляторов. ADI — Abduction–Deduction–Induction.

---

## Не портировано (сознательно)

- **quint-code SQLite storage** — заменён на LanceDB (vectors + tables в одном)
- **BMAD workflow engine** — упрощён до validation rules в нашем validator
- **OpenSpec full workflow** — не перенесён целиком. Target `DeltaSpec`/`parse_delta` уже поддерживает ADDED/MODIFIED/REMOVED, но не upstream RENAMED; нельзя называть весь delta-spec механизм будущей работой или полной upstream parity.
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
