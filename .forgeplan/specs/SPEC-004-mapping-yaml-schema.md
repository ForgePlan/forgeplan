---
depth: standard
id: SPEC-004
kind: spec
last_modified_at: 2026-04-28T00:55:13.946771+00:00
last_modified_by: claude-code/2.1.121
links:
- target: PRD-066
  relation: refines
- target: ADR-009
  relation: based_on
status: draft
title: Mapping YAML schema
---

---
created: 2026-04-28
depth: standard
id: SPEC-004
kind: spec
title: Mapping YAML schema
status: draft
---

# SPEC-004: Mapping YAML schema

## Summary

Декларативная YAML-схема для Forgeplan mapping files — translation rules от output внешнего плагина (C4 docs, autoresearch summaries, git logs, DDD models, SPARC specs) к forge artifacts (PRD/ADR/Epic/Note/Spec). Schema versioned semver; per-mapping `compat_spec_version` для upstream plugin output stability. **Hallucination-proof invariant**: каждый ingested artifact имеет mandatory `## Sources` section с file:line refs (ADR-009 invariant). Published at `docs/schemas/mapping.schema.yaml` (PRD-066 FR-1, FR-2). Used by PRD-066 ingest engine; referenced from SPEC-003 playbook step.

## Contract

### Top-level fields

| Field | Type | Required | Notes |
|---|---|:--:|---|
| `schema_version` | string (semver) | yes | This schema format version |
| `name` | string (kebab-case) | yes | Unique mapping identifier |
| `title` | string | yes | Human-readable name |
| `compat_spec_version` | string | yes | Upstream plugin output compat range |
| `source_kind` | enum | yes | `c4-documentation` / `autoresearch` / `git-log` / `ddd-model` / `sparc-spec` |
| `target_kind` | string | yes | Currently always `forge` |
| `sources` | array (≥1) | yes | Input file discovery rules |
| `rules` | array (≥1) | yes | Transformation rules |
| `guards` | object | no | Invariants & safety limits |
| `errors` | object | no | Per-error policy overrides |

### `sources` (input discovery)

```yaml
sources:
  - pattern: "C4-Documentation/**/*.md"
    type: markdown
    parser: front_matter_plus_sections
  - pattern: ".git/log"
    type: git_log
    parser: log_with_blame
```

`type` → `parser` binding declarative — нет embedded code. Allowed parsers:
- `front_matter_plus_sections` — YAML frontmatter + ## sections
- `markdown_only` — без frontmatter
- `log_with_blame` — git log + git blame for ADR inference
- `json` — JSON file
- `yaml` — YAML file

### Rule object (single transformation unit)

| Field | Type | Required | Notes |
|---|---|:--:|---|
| `id` | string (kebab-case) | yes | Unique в mapping |
| `when` | selector object | yes | Match conditions |
| `target.kind` | enum | yes | `prd`/`adr`/`epic`/`note`/`spec`/`problem` |
| `fields` | object | yes | Forge artifact field templates |
| `sources_section` | object | yes | `## Sources` config (invariant) |
| `links` | array | no | Auto-create typed links |

### `when` selector (declarative match)

```yaml
when:
  file_glob: "C4-Documentation/components/*.md"
  front_matter:                # все keys must match
    kind: component
  contains_section: "## Purpose"   # optional content check
```

Multiple selectors AND-combined. Selector failure → rule skipped, не error.

### `fields` (templated mappings)

Template — Tera-style placeholders, **только path lookups + whitelist filters** (no embedded code):

```yaml
fields:
  title: "{{front_matter.name | trim}}"
  problem: "{{section.purpose}}"
  goals: "{{section.responsibilities | bullet_list}}"
  target_users: "{{section.consumers | comma_list}}"
```

**Whitelist filters**: `trim`, `lower`, `upper`, `bullet_list`, `comma_list`, `slugify`, `truncate(n)`, `default(value)`, `replace(from,to)`. **Arbitrary Tera filters → load error** (security invariant).

### `sources_section` (hallucination-proof — INVARIANT)

```yaml
sources_section:
  include: true                # MUST be true — ADR-009 invariant
  format: "{path}:{line_start}-{line_end}"
  precision: line              # line | block | file
  source_hash: true            # для idempotency (PRD-066 AC-3, FR-5)
```

`include: false` → schema validation **fails** (invariant violation).

### `links` (auto-graph creation)

```yaml
links:
  - target: "{{front_matter.parent_container}}"
    relation: refines           # informs/based_on/refines/contradicts/supersedes
    if_exists: skip             # skip | warn | error
  - target_artifact_id: "EPIC-006"   # static link
    relation: based_on
```

### `guards` (safety limits)

```yaml
guards:
  max_artifacts: 100             # abort если mapping создаст больше
  require_section: ["Sources"]   # все artifacts must have these sections
  forbid_overwrite_active: true  # никогда не update active artifact (только draft)
```

## Data Models

Rust types (serde-derived) в `forgeplan-core::ingest::types`:

```rust
pub struct Mapping {
    pub schema_version: SchemaVersion,
    pub name: String,
    pub title: String,
    pub compat_spec_version: VersionReq,    // semver
    pub source_kind: SourceKind,
    pub target_kind: TargetKind,            // currently only Forge
    pub sources: Vec<SourceSpec>,           // non-empty
    pub rules: Vec<Rule>,                   // non-empty
    pub guards: Guards,
    pub errors: ErrorPolicy,
}

pub struct Rule {
    pub id: String,
    pub when: Selector,
    pub target: TargetSpec,
    pub fields: HashMap<String, Template>,
    pub sources_section: SourcesSectionSpec,    // include MUST be true
    pub links: Vec<LinkSpec>,
}

pub struct Template(String);   // parsed at load via Tera with whitelisted filters

pub enum SourceKind {
    C4Documentation,
    Autoresearch,
    GitLog,
    DddModel,
    SparcSpec,
}

pub struct Guards {
    pub max_artifacts: Option<usize>,           // safety limit
    pub require_section: Vec<String>,
    pub forbid_overwrite_active: bool,          // default true
}
```

## Errors

| Condition | Severity | Action |
|---|---|---|
| Missing required top-level field | ERROR | Reject load |
| Unknown `source_kind` | ERROR | List valid 5 kinds |
| Rule `target.kind` invalid | ERROR | List valid 6 kinds |
| Template uses non-whitelisted filter | ERROR | List allowed filters |
| **`sources_section.include: false`** | ERROR | **Invariant violation (hallucination-proof)** |
| Empty `sources` или `rules` array | ERROR | Reject |
| Source `pattern` matches 0 files | WARN | Log, continue с next rule |
| Generated artifact field validation fails | ERROR per artifact | Skip artifact, continue mapping, summary at end |
| Idempotent re-run: existing artifact, same `source_hash` | INFO | Skip (no-op) |
| Idempotent re-run: existing artifact, **different content** + same source path | WARN | Update artifact, log diff |
| `guards.max_artifacts` exceeded | ERROR | Abort mapping, partial-state report |
| `guards.forbid_overwrite_active` violated | ERROR | Skip artifact, log |
| Source path не существует at runtime | WARN | Mark artifact stale, не delete |

FR-1, FR-2, FR-3, FR-4, FR-5, FR-7 PRD-066 подтверждены этим контрактом. AC-2 (Sources section), AC-3 (idempotency), AC-4 (doctor --sources) — реализуются guards + invariants.

## Versioning

- `schema_version: "1.0"` — initial mapping format.
- `compat_spec_version` per mapping — **separate semver** pinning upstream plugin output format (e.g., `c4-architecture: "^1.0"`). Mapping author sets; CI matrix тестит upstream releases.
- **Failure mode**: upstream plugin breaking change → published mapping `compat_spec_version` no longer satisfied → runtime emits hint "mapping X needs update for upstream version Y", не runs.
- Mapping author публикует new version (`c4-to-forge-2.0.yaml`); runtime selects highest compatible.

## Related Artifacts

| Artifact | Type | Relation |
|---|---|---|
| PRD-066 | PRD | refines (contract for FR-1, FR-2, FR-4) |
| ADR-009 | ADR | based_on (mapping primitive defined here, hallucination-proof invariant) |
| ADR-003 | ADR | informs (markdown source of truth — mappings produce markdown) |
| SPEC-003 | SPEC | informs (playbook step.mapping references this) |
| EPIC-007 | Epic | informs (parent epic) |



