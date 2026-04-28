---
depth: standard
id: SPEC-003
kind: spec
last_modified_at: 2026-04-28T00:54:42.320304+00:00
last_modified_by: claude-code/2.1.121
links:
- target: PRD-065
  relation: refines
- target: ADR-009
  relation: based_on
- target: SPEC-004
  relation: informs
status: draft
title: Playbook YAML schema
---

---
created: 2026-04-28
depth: standard
id: SPEC-003
kind: spec
title: Playbook YAML schema
status: draft
---

# SPEC-003: Playbook YAML schema

## Summary

Декларативная YAML-схема для Forgeplan playbooks — multi-step оркестрационных workflow, делегирующих шаги внешним plugins/skills/agents и ингестящих их outputs в forge-граф. Schema versioned semver; валидация в load-time через JSON Schema, generated из Rust types (`schemars`). Published at `docs/schemas/playbook.schema.yaml` (PRD-065 FR-1, FR-2). Used by PRD-065 runtime executor; references SPEC-004 mappings via `step.mapping`.

## Contract

### Top-level fields

| Field | Type | Required | Notes |
|---|---|:--:|---|
| `schema_version` | string (semver) | yes | Format version, e.g. `"1.0"` |
| `name` | string (kebab-case) | yes | Unique playbook identifier |
| `title` | string | yes | Human-readable name |
| `description` | string | no | Multi-line summary |
| `triggered_by` | object | no | Project-signal hints для recommendation engine (PRD-067 FR-5) |
| `requires` | object | no | Plugin/skill prerequisites |
| `steps` | array (≥1) | yes | Ordered step objects |

### `triggered_by` (project signals)

```yaml
triggered_by:
  empty_repo: false
  has_git: true
  commit_count_min: 100
  has_docs: false
  has_obsidian: false
  has_cargo_toml: true
```

### `requires`

```yaml
requires:
  plugins:
    - name: c4-architecture
      version: ">=1.0"        # semver range
  skills:
    - name: forge-history-miner
      pack: brownfield-code-pack
```

### Step object

| Field | Type | Required | Notes |
|---|---|:--:|---|
| `id` | string (kebab-case) | yes | Unique within playbook |
| `delegate_to` | object | yes | One of 5 delegate types (см. ниже) |
| `input` | object | no | Step-specific parameters |
| `produces_at` | path | only if mapped | Output location for ingest |
| `mapping` | string | no | Reference to mapping name (SPEC-004) |
| `requires` | array of step IDs | no | DAG ordering |
| `fallback_hint` | string | no | Install command если delegate missing |
| `on_error` | enum | no | `abort` (default) / `continue` |

### `delegate_to` — 5 strict typed variants

```yaml
delegate_to:
  type: plugin              # → invoke external plugin via Task tool
  name: c4-architecture
  target: c4-code

# OR
  type: agent               # → invoke agent via Task tool
  name: c4-component

# OR
  type: skill               # → invoke skill (loaded в agent)
  name: forge-history-miner
  pack: brownfield-code-pack

# OR
  type: command             # → arbitrary shell — opt-in only
  command: ["git", "log", "--oneline"]

# OR
  type: forgeplan_core      # → internal op
  target: ingest            # ingest | new | validate | activate | search
```

## Data Models

Rust types (serde-derived) в `forgeplan-core::playbook::types`:

```rust
pub struct Playbook {
    pub schema_version: SchemaVersion,    // semver
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub triggered_by: Option<TriggeredBy>,
    pub requires: Option<Requirements>,
    pub steps: Vec<Step>,                 // non-empty (validated)
}

pub struct Step {
    pub id: String,
    pub delegate_to: Delegation,
    pub input: Option<serde_yaml::Value>,
    pub produces_at: Option<PathBuf>,
    pub mapping: Option<String>,
    pub requires: Option<Vec<String>>,    // step IDs
    pub fallback_hint: Option<String>,
    pub on_error: OnError,                // default = Abort
}

pub enum Delegation {
    Plugin { name: String, target: String },
    Agent  { name: String },
    Skill  { name: String, pack: Option<String> },
    Command { command: Vec<String> },     // opt-in
    ForgeplanCore { target: ForgeplanOp },
}

pub enum OnError { Abort, Continue }

pub enum ForgeplanOp { Ingest, New, Validate, Activate, Search }
```

## Errors

| Condition | Severity | Action |
|---|---|---|
| Missing required field (name/title/steps) | ERROR | Reject load, exit code 2 |
| Empty `steps` array | ERROR | Reject |
| Unknown `delegate_to.type` | ERROR | Reject, list valid 5 types |
| `requires:` references unknown step ID | ERROR | Reject, list available IDs |
| Cycle in step `requires:` graph | ERROR | Reject, show cycle |
| Plugin in `requires:` not installed locally | WARN | Load OK; step fails at runtime с install hint |
| Unknown YAML field (forward compat) | WARN | Load OK; log unknown |
| `schema_version` > runtime supported | ERROR | Suggest upgrade Forgeplan |
| `schema_version` < runtime minimum | ERROR | Suggest migrate or pin runtime |
| `produces_at` set но `mapping` отсутствует | WARN | Output captured, не ingested |
| `mapping` set но `produces_at` отсутствует | ERROR | Reject — нечего ingest |

FR-1, FR-2, FR-3, FR-5 PRD-065 подтверждены этим контрактом. AC-5 PRD-065 (validation catches malformed) — реализуется этой error matrix.

## Versioning

- `schema_version: "1.0"` — initial published format.
- **Backward compat policy**:
  - **Minor bumps** (1.0 → 1.1) — additive: add fields, deprecation warnings on old fields. Old playbooks load OK на новом runtime.
  - **Major bumps** (1.x → 2.0) — breaking: old runtimes refuse new format with clear migration hint.
- Runtime объявляет supported range (`^1.0`); CI matrix тестит published mappings/playbooks across versions.
- `schema_version` separate от forgeplan binary version: schema может evolve independently.

## Related Artifacts

| Artifact | Type | Relation |
|---|---|---|
| PRD-065 | PRD | refines (this is the contract for FR-1, FR-2, FR-3, FR-5) |
| ADR-009 | ADR | based_on (orchestrator pivot decision) |
| SPEC-004 | SPEC | informs (mapping YAML referenced from `step.mapping`) |
| EPIC-007 | Epic | informs (parent epic) |




