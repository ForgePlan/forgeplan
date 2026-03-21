---
id: RFC-002
title: "Validation Engine Architecture"
status: Draft
author: explosovebit
created: 2026-03-21
updated: 2026-03-21
prd: PRD-001
depth: standard
---

# RFC-002: Validation Engine Architecture

## Progress

```
Phase B  ████████████████████████  4/4   (100%)  Validate + Score + Link + Graph  ✅ DONE
─────────────────────────────────────────────────
TOTAL                               4/4   (100%)
```

---

## Summary

Архитектура validation engine для `forgeplan validate` — модуль проверки полноты и корректности артефактов. Engine проверяет обязательные секции по типу артефакта (`kind`) и уровню глубины (`depth`), реализуя автоматизируемые шаги из BMAD 13-Step Validation.

## Motivation

PRD-001 FR-005: "User can validate artifact completeness against schema rules". Без validation engine нет способа автоматически проверить полноту артефактов — review остаётся полностью ручным. Автоматизация 7 из 13 BMAD-шагов позволяет ловить 80% ошибок до ревью.

## Goals

- Определить подход к описанию validation rules (hardcoded vs config)
- Описать rules per kind per depth (PRD, Epic, Spec, RFC, ADR + Quint-code types)
- Определить формат error reporting
- Разделить автоматизируемые и неавтоматизируемые BMAD шаги
- Описать архитектуру модулей validate, score, link, graph

## Non-Goals

- AI-driven validation (шаги 8, 9, 11 из BMAD — domain/project-type/holistic)
- Custom user-defined rules (Phase 4+)
- Real-time validation при редактировании (desktop app)

---

## Options Considered

### Option A: Config-driven rules (YAML/TOML rule files)

**Description**: Правила описаны во внешних файлах. Engine загружает и интерпретирует.

**Pros**: Расширяемость — пользователь может добавлять custom rules. Не нужен перекомпиляция.

**Cons**: Нужен DSL для правил (парсер, интерпретатор). Over-engineering: правила — часть методологии, не настройка пользователя. Runtime errors в конфиге. Увеличение binary size за счёт парсера.

### Option B: Hardcoded rules in Rust (выбран)

**Description**: Правила — это Rust код: `fn rules_for(kind, depth) -> Vec<Rule>`. Каждое правило — struct с check function.

**Pros**: Compile-time гарантии. Нет runtime parsing. Быстрее исполнение. Проще тестирование (unit tests на каждое правило). Rules = часть методологии, не user config.

**Cons**: Новые правила = перекомпиляция. Но: правила меняются редко (привязаны к schemas, которые зафиксированы).

### Option C: Embedded schema files (JSON Schema-like)

**Description**: Schemas встроены в binary, парсятся при старте.

**Pros**: Отделяет data от code. Можно переиспользовать schemas в других инструментах.

**Cons**: JSON Schema плохо описывает markdown-специфичные правила ("секция ## Goals содержит ≥ 1 measurable item"). Overhead без практической пользы.

## Trade-off Analysis

| Критерий | Config-driven | Hardcoded (выбран) | Embedded schemas |
|----------|--------------|-------------------|-----------------|
| Compile safety | None | Full | Partial |
| Extensibility | High | Low (recompile) | Medium |
| Complexity | High | Low | Medium |
| Performance | Medium | Best | Medium |
| Markdown-aware rules | Hard | Easy | Hard |

---

## Proposed Direction

**Option B: Hardcoded rules**. Правила — часть методологии Forgeplan, не user config. Compile-time safety. Markdown-aware checks (regex на секции, word count, link presence) легко выразить в Rust.

---

## Architecture

### Module Layout (новые модули)

```
crates/forgeplan-core/src/
├── validation/
│   ├── mod.rs              # pub mod, ValidationResult, ValidationError
│   ├── rules.rs            # Rule trait + registry: rules_for(kind, depth)
│   ├── checks.rs           # Concrete check functions (section_exists, word_count, etc.)
│   └── kinds/
│       ├── mod.rs
│       ├── prd.rs           # PRD-specific rules per depth
│       ├── epic.rs          # Epic rules
│       ├── spec.rs          # Spec rules
│       ├── rfc.rs           # RFC rules
│       └── adr.rs           # ADR rules
├── graph/
│   ├── mod.rs
│   └── mermaid.rs          # Build dependency graph, render to mermaid
└── link/
    ├── mod.rs
    └── manager.rs          # Add/remove links in frontmatter
```

### Core Types

```rust
/// A single validation rule.
pub struct Rule {
    pub id: &'static str,          // "prd-goals-measurable"
    pub description: &'static str,  // "Each Goal must have a numeric target"
    pub severity: Severity,         // Must | Should | Could
    pub check: fn(&str, &Frontmatter) -> Option<Finding>,
}

pub enum Severity {
    Must,    // Blocks validation (error)
    Should,  // Warning
    Could,   // Suggestion
}

pub struct Finding {
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub section: Option<String>,  // "## Goals", "Meta Header"
    pub line: Option<usize>,
}

pub struct ValidationResult {
    pub artifact_id: String,
    pub findings: Vec<Finding>,
    pub passed: bool,  // true if no Must-severity findings
}
```

### Rule Registry

```rust
/// Returns applicable rules for a given kind and depth.
pub fn rules_for(kind: &ArtifactKind, depth: &Mode) -> Vec<&'static Rule> {
    match kind {
        ArtifactKind::Prd => prd::rules(depth),
        ArtifactKind::Epic => epic::rules(depth),
        ArtifactKind::Spec => spec::rules(depth),
        ArtifactKind::Rfc => rfc::rules(depth),
        ArtifactKind::Adr => adr::rules(depth),
        // Quint-code types: minimal rules (frontmatter presence)
        _ => base_rules(),
    }
}
```

### PRD Validation Rules (пример)

Из PRD-SCHEMA.md, правила по depth:

| Rule ID | Check | Tactical | Standard | Deep | Critical |
|---------|-------|----------|----------|------|----------|
| `meta-header` | Frontmatter has id, status, author | Must | Must | Must | Must |
| `problem-exists` | `## Problem` section exists | Must | Must | Must | Must |
| `problem-density` | Problem ≥ 50 words | — | Should | Must | Must |
| `goals-exist` | `## Goals` section exists | Must | Must | Must | Must |
| `goals-measurable` | Goals contain numbers/metrics | — | Should | Must | Must |
| `non-goals-exist` | `## Non-Goals` section ≥ 1 item | Must | Must | Must | Must |
| `fr-exist` | `## Functional Requirements` section | Must | Must | Must | Must |
| `fr-format` | FRs use "[Actor] can [capability]" | — | Should | Must | Must |
| `fr-no-impl-leakage` | No tech names in FR text | — | Should | Must | Must |
| `success-metrics` | `## Success` section with KPIs | Must | Must | Must | Must |
| `related-artifacts` | Links section exists | Must | Must | Must | Must |
| `target-audience` | Persona section | — | Must | Must | Must |
| `nfr-exist` | Non-Functional Requirements | — | Should | Must | Must |
| `risks-exist` | `## Risks` with ≥ 1 risk | — | Should | Must | Must |
| `timeline` | Milestones with dates | — | — | Must | Must |
| `stakeholders` | Sign-off checkboxes | — | — | Must | Must |
| `acceptance-criteria` | Given/When/Then | — | — | Must | Must |
| `no-placeholders` | No `{{...}}` or `TODO` in content | Should | Must | Must | Must |

### BMAD 13-Step Mapping

| Step | BMAD Name | Автоматизируемо? | Implementation |
|------|-----------|-------------------|----------------|
| 1 | Discovery & Confirmation | Частично | Check `kind` field matches file location |
| 2 | Format Detection & Structure | ✅ Да | Section header matching per schema |
| 3 | Information Density | ✅ Да | Word count per section, filler detection |
| 4 | Product Brief Coverage | ✅ Да | Check problem/audience/goals presence |
| 5 | Measurability | ✅ Да | Regex for numbers/metrics in FR/NFR |
| 6 | Traceability | ✅ Да | FR→Journey links, ID uniqueness |
| 7 | Implementation Leakage | ✅ Да | Blocklist: React, Django, PostgreSQL, etc. |
| 8 | Domain Compliance | ❌ Нет | Requires domain knowledge (AI) |
| 9 | Project-Type Compliance | ❌ Нет | Requires project context (AI) |
| 10 | SMART Requirements | Частично | Check numbers in criteria, but not "Attainable" |
| 11 | Holistic Quality Assessment | ❌ Нет | Requires human/AI judgment |
| 12 | Completeness | ✅ Да | No `{{placeholder}}`, all MUST sections filled |
| 13 | Report Finalization | ✅ Да | Generate summary report from findings |

**Итого**: 7 полностью автоматизируемых + 2 частично = покрытие ~70% BMAD validation.

### Check Functions (primitives)

```rust
// Reusable check primitives
fn section_exists(body: &str, heading: &str) -> bool;
fn section_word_count(body: &str, heading: &str) -> usize;
fn section_has_items(body: &str, heading: &str) -> usize; // count list items or table rows
fn no_placeholders(body: &str) -> Vec<(usize, String)>;  // line, matched text
fn no_tech_names(body: &str) -> Vec<(usize, String)>;    // implementation leakage
fn has_numeric_targets(text: &str) -> bool;               // measurability
fn frontmatter_has(fm: &Frontmatter, key: &str) -> bool;
```

### Error Reporting Format

```
$ forgeplan validate PRD-001

PRD-001 "Forgeplan CLI" (depth: deep)
─────────────────────────────────────

  ✗ [MUST]   meta-header: Missing field "author" in frontmatter
  ✗ [MUST]   acceptance-criteria: Section "## Acceptance Criteria" not found
  ⚠ [SHOULD] problem-density: Problem section has 38 words (expected ≥ 50)
  ✓ [MUST]   goals-exist: Section "## Goals" found
  ✓ [MUST]   fr-exist: 10 functional requirements found
  ...

Result: FAIL — 2 errors, 1 warning, 14 passed
```

### Data Flow

```
forgeplan validate [ID]
  → CLI: parse args, find workspace
  → Core: store::list_artifacts() or store::load_artifact(id)
  → Core: frontmatter::parse_frontmatter(content)
  → Core: determine kind + depth from frontmatter
  → Core: validation::rules_for(kind, depth) → Vec<Rule>
  → Core: for each rule: rule.check(body, &frontmatter)
  → Core: collect Finding[] → ValidationResult
  → CLI: format and print findings
```

### Score Command (FR-006)

`forgeplan score` — обёртка над `scoring::r_eff()`:

```
forgeplan score ADR-001
  → Parse frontmatter of ADR-001
  → Find linked EvidencePack artifacts (relation: informs)
  → Parse each evidence: verdict, CL, valid_until
  → Compute r_eff(evidence_items)
  → Print breakdown + final score
```

Если у артефакта нет привязанного evidence — выводить "No evidence linked, R_eff = 0.0".

### Link Command (FR-009)

```
forgeplan link RFC-001 --based-on PRD-001
  → Load RFC-001 frontmatter
  → Add to links: [{target: "PRD-001", relation: "based_on"}]
  → Write updated frontmatter back
  → Print confirmation
```

Frontmatter links format:
```yaml
links:
  - target: PRD-001
    relation: based_on
  - target: ADR-001
    relation: informs
```

### Graph Command (FR-007)

```
forgeplan graph
  → Load all artifacts
  → Extract links from each frontmatter
  → Build adjacency list
  → Render mermaid:

graph LR
    EPIC-001 --> PRD-001
    PRD-001 -->|based_on| RFC-001
    RFC-001 -->|informs| ADR-001
    RFC-001 -->|informs| ADR-002
```

Output to stdout (user copies to docs or pipes to file).

---

## Risks & Open Questions

- **Risk**: Markdown section parsing fragile — heading level mismatch (## vs ###). Mitigated: normalize heading search to any `#+ <name>`.
- **Risk**: Tech blocklist for leakage detection — false positives (project about "React patterns" будет flagged). Mitigated: blocklist only in FR/NFR sections, not entire body.
- **Open**: Должен ли `validate --fix` автоматически исправлять trivial issues (missing depth в frontmatter)?
- **Open**: Output format — только human-readable или JSON тоже (для CI интеграции)?

## Implementation Phases

### Phase B: Validate + Score + Link + Graph
- [x] **B.1** `forgeplan validate` — validation engine + rules for PRD, Epic, Spec, RFC, ADR
- [x] **B.2** `forgeplan score` — R_eff CLI wrapper with evidence lookup
- [x] **B.3** `forgeplan link` — add/remove typed relationships in frontmatter
- [x] **B.4** `forgeplan graph` — mermaid dependency graph from links

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| PRD-001 | PRD | based_on |
| RFC-001 | RFC | extends (Phase B of same CLI) |
| EPIC-001 | Epic | parent |
| PRD-SCHEMA.md | Schema | informs (PRD validation rules) |
| EPIC-SCHEMA.md | Schema | informs (Epic validation rules) |
| SPEC-SCHEMA.md | Schema | informs (Spec validation rules) |
| QUALITY-GATES.md | Guide | informs (BMAD 13 steps) |

---

> **Next step**: Реализовать B.1 — validation engine в forgeplan-core/validation/, затем CLI команду.
