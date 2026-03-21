# PRD Process Engine — Implementation Plan

## Phases

```
Phase 0  ████████████████████████  10/10  (100%)  Foundation & Research      ✅ DONE
Phase 1  ████████████████████████  12/12  (100%)  Schemas, Templates & Docs  ✅ DONE
Phase 2  ░░░░░░░░░░░░░░░░░░░░░░░░   0/8   (  0%)  Workflow & Integration (SUPERSEDED by MCP)
Phase 3  ████████████████████████  13/13  (100%)  Rust CLI + LanceDB        ✅ DONE
Phase 4  ░░░░░░░░░░░░░░░░░░░░░░░░   0/7   (  0%)  AI & Automation
─────────────────────────────────────────────────
TOTAL                               35/50  (70.0%)
```

---

## Phase 0 — Foundation & Research (DONE)

- [x] **0.1** Создать каталог `frameworks/` со структурой
- [x] **0.2** README.md с vision и use cases
- [x] **0.3** PLAN.md с фазами
- [x] **0.4** TODO.md с задачами
- [x] **0.5** SOURCES.md — карта всех источников (skills, commands, agents, FPF)
- [x] **0.6** ARTIFACT-MODEL.md — иерархия артефактов (PRD→Spec→RFC→ADR→Epic)
- [x] **0.7** Анализ 10 референсных документов → REF-DOCS-ANALYSIS.md
- [x] **0.8** Аудит 52 skills/plugins по 10 слоям → SKILLS-AUDIT.md
- [x] **0.9** COMPLETENESS-CHECK.md — 62% ready, 10 critical gaps
- [x] **0.10** VISION.md — финальная идея, все слои, модули, data model
- [x] Clone 6 repos: quint-code, OpenSpec, BMAD, git-adr, adr-tools, ccpm
- [x] frameworks/ → .gitignore

## Phase 1 — Schemas, Templates & Documentation

### 1A: Schemas (формальные правила каждого типа)
- [x] **1.1** PRD-SCHEMA.md — обязательные секции, валидация, BMAD 13 steps
- [x] **1.2** EPIC-SCHEMA.md — aggregated progress, dependency graph, children rules
- [x] **1.3** SPEC-SCHEMA.md — API contracts, data models, events, versioning

### 1B: Templates (обогащение существующих)
- [x] **1.4** PRD шаблон — обогатить из BMAD `create-prd/` (validation steps)
- [x] **1.5** Product Brief шаблон — lightweight PRD для Quick Flow (из BMAD)
- [x] **1.6** Problem Card шаблон — из quint-code ProblemCard
- [x] **1.7** Solution Portfolio шаблон — из quint-code (variants + weakest_link)
- [x] **1.8** Decision Record (DDR) шаблон — из quint-code/FPF (invariants + rollback + valid_until)

### 1C: Documentation
- [x] **1.9** PRD-RFC-ADR-FLOW.md — полный workflow guide с decision tree
- [x] **1.10** DEPTH-CALIBRATION.md — когда Tactical, Standard, Deep, Critical
- [x] **1.11** QUALITY-GATES.md — Verification Gate (5-point) + Adversarial Review
- [x] **1.12** GLOSSARY.md — термины: R_eff, CL, DDR, ADI, delta-spec, artifact DAG

## Phase 2 — Workflow & Claude Code Integration

- [ ] **2.1** Расширить `/write-doc` — добавить типы: prd, epic, spec
- [ ] **2.2** Создать `/prd` slash command — быстрый PRD из идеи (Quick Flow)
- [ ] **2.3** PRD-INDEX.md template — индекс всех PRD
- [ ] **2.4** EPIC-INDEX.md template — индекс эпиков
- [ ] **2.5** Интеграция с Hindsight memory (auto-tags для PRD/Epic)
- [ ] **2.6** Verification Gate checklist как quality gate в `/audit`
- [ ] **2.7** Adversarial Review protocol в `/audit`
- [ ] **2.8** Обновить CLAUDE.md — документировать PRD workflow

## Phase 3 — Rust CLI Application (`forgeplan`)

**Language**: Rust
**References**: quint-code (data model), git-adr (Rust CLI), OpenSpec (artifact DAG)

### 3A: Foundation
- [x] **3.1** `cargo init` + Cargo.toml с dependencies (clap, serde, chrono, anyhow, thiserror)
- [x] **3.2** CLI scaffold (clap derive) — `forgeplan init|new|list|status|validate|score`
- [x] **3.3** Config module — `.forgeplan/config.yaml` loader (serde_yaml)
- [x] **3.4** LanceDB tables schema — 3 tables (artifacts, evidence, relations) via Phase 3D

### 3B: Core
- [x] **3.5** Artifact model — port quint-code `types.go` → Rust (ArtifactKind, Meta, Link, Status)
- [x] **3.6** Parser — YAML frontmatter + markdown body
- [x] **3.7** Writer — create artifact in `.forgeplan/` directory with auto-ID
- [x] **3.8** Template engine — embedded templates (include_str!) + placeholder substitution

### 3C: Features
- [x] **3.9** Validator — required sections check per kind (BMAD rules) — RFC-002
- [x] **3.10** Progress tracker — checkbox parser + ASCII progress bars (forgeplan progress)
- [x] **3.11** Graph builder — dependency graph → mermaid output
- [x] **3.12** R_eff scoring — port quint-code `reff.go` → Rust (done in 3A, CLI wrapper in 3B)
- [x] **3.13** Link manager — typed relationships in frontmatter

## Phase 4 — AI & Automation

- [ ] **4.1** MCP server mode — `forgeplan serve` (rmcp crate)
- [ ] **4.2** LLM integration — generate PRD from description
- [ ] **4.3** FPF ADI cycle — Abduction→Deduction→Induction for decisions
- [ ] **4.4** Auto-decompose — PRD → RFC tasks (contextual chain)
- [ ] **4.5** Evidence Decay — valid_until TTL + stale detection + refresh alerts
- [ ] **4.6** Depth calibration — auto-suggest Tactical/Standard/Deep/Critical
- [ ] **4.7** Auto-capture — agent records decisions from conversation context

---

## Architecture (Rust)

```
forgeplan/
├── Cargo.toml                    ← workspace root
├── crates/
│   ├── forgeplan-core/           ← SHARED LIBRARY (вся логика)
│   │   ├── artifact/             ← types, parser, writer, store
│   │   ├── scoring/              ← R_eff = min(evidence_scores)
│   │   ├── validation/           ← BMAD 13-step rules
│   │   ├── search/               ← LanceDB vectors + Tantivy text
│   │   ├── progress/             ← checkbox parser + progress bars
│   │   ├── graph/                ← mermaid dependency graph
│   │   ├── embed/                ← ONNX local embeddings (BGE-M3)
│   │   ├── db/                   ← LanceDB operations
│   │   ├── template/             ← tera engine
│   │   └── config/               ← .forgeplan/config.yaml
│   ├── forgeplan-cli/            ← CLI binary (clap derive)
│   ├── forgeplan-tauri/          ← Desktop app backend (Tauri 2.0 + core)
│   └── forgeplan-mcp/            ← MCP server (Phase 5, rmcp)
├── apps/desktop/                 ← React frontend (Tauri UI)
└── templates/                    ← .md.tera шаблоны (embedded in binary)
```

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Language | **Rust** | Type safety, single binary, git-adr reference, pulldown-cmark |
| CLI | clap (derive) | Auto-completions, typed args, man pages |
| Templates | tera | Jinja2-compatible, powerful filters |
| Markdown | pulldown-cmark | Fast, compliant, pure Rust |
| YAML | serde_yaml | serde ecosystem integration |
| Database | LanceDB | Embedded, zero-config, structured tables + vector embeddings |
| Config | YAML | Human-readable |
| Diagrams | Mermaid | GitHub/GitLab renderable |
| MCP | rmcp | Official Rust MCP SDK |

## Key Files

| File | Purpose |
|------|---------|
| `VISION.md` | Финальная идея — все слои, модули, data model, patterns |
| `PLAN.md` | Этот файл — implementation plan |
| `COMPLETENESS-CHECK.md` | Gap analysis — что есть, чего нет |
| `SOURCES.md` | Карта всех источников |
| `docs/references/REF-DOCS-ANALYSIS.md` | Анализ 10 документов |
| `docs/references/SKILLS-AUDIT.md` | Аудит 52 skills |
| `docs/guides/ARTIFACT-MODEL.md` | Иерархия артефактов |
