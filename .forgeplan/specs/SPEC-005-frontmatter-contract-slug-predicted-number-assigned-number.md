---
depth: standard
id: SPEC-005
kind: spec
links:
- target: PRD-076
  relation: based_on
status: draft
title: 'Frontmatter contract: slug, predicted_number, assigned_number'
---

---
id: SPEC-005
title: "Frontmatter contract: slug, predicted_number, assigned_number"
status: Draft
author: explosivebit
created: 2026-05-06
updated: 2026-05-06
prd: PRD-076
type: Data Model
depth: deep
---

# SPEC-005: Frontmatter contract: slug, predicted_number, assigned_number

## Summary

Спецификация frontmatter полей `slug`, `predicted_number`, `assigned_number` для двухслойной identity модели Forgeplan-артефактов (PROB-060, PRD-076, ADR-012). Slug — каноничный идентификатор, не меняется. Number — display layer, присваивается на merge.

## Scope

Этот SPEC описывает:
- Новые поля frontmatter (`slug`, `predicted_number`, `assigned_number`) и их semantics
- Derived `id` rendering rules
- Filename format (pre-merge и post-merge)
- Validation regex и rules
- Migration semantics для 73 legacy артефактов
- API contract для MCP `forgeplan_new`, `forgeplan_get`
- Контракт CI-бота (`.github/workflows/assign-id.yml`)

Не входит в scope: implementation details (RFC-009 covers rollout phases), UI rendering details (handled in ForgePlanWeb codebase).

---

## Data Models

### Frontmatter schema (новые поля)

```yaml
---
# Existing fields (unchanged)
kind: prd                    # one of: prd, rfc, adr, epic, spec, problem, solution, evidence, note, refresh
status: draft                # one of: draft, active, deprecated, superseded, stale
title: "Auth System"
created: 2026-05-06
updated: 2026-05-06

# NEW fields (PROB-060 / SPEC-005)
slug: prd-auth-system        # CANONICAL identity. Immutable after creation. Always lowercase.
predicted_number: 74         # Local prediction = max(assigned_number) + 1 in workspace at create time. Hint only.
assigned_number: null        # null until merge to dev. CI bot sets this. Write-once.

# DERIVED at read time (not stored, computed by core)
# id_canonical = slug                                          # always (for refs, search keys, db lookup)
# id_display   = assigned_number ? f"{KIND}-{assigned:03d}"
#                                : f"{KIND}-{predicted}?"      # for CLI/Web/MCP rendering
---
```

### Field constraints

| Field | Type | Required | Mutable | Default | Constraint |
|-------|------|----------|---------|---------|------------|
| `slug` | string | yes | **NO** (immutable) | derived from kind+title | regex (see below) |
| `predicted_number` | uint32 | yes | yes (recompute on rebase) | `local max + 1` | ≥ 1 |
| `assigned_number` | uint32 \| null | yes | **once** (write-once by CI bot) | null | ≥ 1 if set; never reused |

### Slug regex и rules

```regex
^(prd|rfc|adr|epic|spec|prob|sol|evid|note|ref)-[a-z0-9]+(-[a-z0-9]+)*$
```

Правила построения slug из title:
1. Lowercase: `"Auth System"` → `"auth system"`
2. Заменить non-alphanumeric на `-`: `"auth-system"`
3. Collapse повторяющихся `-`: `"auth-system"` (no change)
4. Trim leading/trailing `-`: `"auth-system"`
5. Truncate до 80 chars (с сохранением целого word)
6. Prepend kind prefix: `"prd-auth-system"`

Длина: **3 ≤ len(slug) ≤ 80** (включая prefix)

Запрещённые слаги (reserved):
- `prd-tmp-*` — зарезервировано для test fixtures
- `prd-draft-*`, `prd-pending-*` — зарезервировано для будущих расширений
- `<kind>-<digits-only>` — конфликт с number-based id (например `prd-074` запрещено как user-supplied slug)

### Filename format

**Pre-merge** (assigned_number is null):
```
.forgeplan/<kind_dir>/<slug>.md
Examples:
  .forgeplan/prds/prd-auth-system.md
  .forgeplan/problems/prob-api-panic.md
```

**Post-merge** (assigned_number is set):
```
.forgeplan/<kind_dir>/<KIND>-<NNN>-<slug-without-prefix>.md
Examples:
  .forgeplan/prds/PRD-074-auth-system.md
  .forgeplan/problems/PROB-061-api-panic.md
```

Backwards compat: filename pattern для legacy артефактов уже соответствует post-merge format. Migration оставляет filename как есть.

### Derived `id` rendering

```rust
// Pseudocode for core resolver
fn render_id(fm: &Frontmatter, mode: RenderMode) -> String {
    match mode {
        RenderMode::Canonical => fm.slug.clone(),
        RenderMode::Display => match fm.assigned_number {
            Some(n) => format!("{}-{:03}", fm.kind.uppercase_prefix(), n),
            None => format!("{}-{}?", fm.kind.uppercase_prefix(), fm.predicted_number),
        },
    }
}
```

Где используется:
- `RenderMode::Canonical` — db keys, search index keys, cross-artifact refs (`Related:`), MCP/API responses field `slug`
- `RenderMode::Display` — CLI output (`forgeplan list`, `forgeplan get`), Web header, graph nodes, Slack-friendly format, MCP response field `id_display`

---

## Validation Rules

| Field | Rule | Error message | Validation owner |
|-------|------|---------------|------------------|
| `slug` | Required, immutable | `slug must not be modified after create` | core/validation |
| `slug` | Matches regex | `invalid slug format: must match ^(prd\|rfc\|...)-[a-z0-9-]+$` | core/validation |
| `slug` | 3-80 chars | `slug length must be 3-80 characters` | core/validation |
| `slug` | Not reserved | `slug uses reserved prefix (tmp/draft/pending or digits-only)` | core/validation |
| `slug` | Unique in workspace | `slug already exists: <existing-path>` | core/validation |
| `slug` | Unique in origin/dev (warn) | `slug already exists in origin/dev: <existing-id>` | pre-commit hook |
| `predicted_number` | Required | `predicted_number is required` | core/validation |
| `predicted_number` | Positive int | `predicted_number must be ≥ 1` | core/validation |
| `assigned_number` | Write-once | `assigned_number is write-once and can only be set by CI bot` | CI workflow + validator |
| `assigned_number` | Unique per kind | `assigned_number conflict: PRD-074 already exists` | CI bot atomic step |
| `assigned_number` | Sequential per kind | `assigned_number must equal max(existing) + 1` | CI bot atomic step |
| Manual `assigned_number` in PR | Forbidden in dev contributors' commits | `assigned_number changes are reserved for CI bot` | CI workflow rule |

---

## API Contracts

### MCP `forgeplan_new`

**Request**:
```json
{
  "kind": "prd",
  "title": "Auth System"
}
```

**Response (201 Created)**:
```json
{
  "slug": "prd-auth-system",
  "predicted_number": 74,
  "assigned_number": null,
  "id_canonical": "prd-auth-system",
  "id_display": "PRD-74?",
  "kind": "prd",
  "title": "Auth System",
  "status": "draft",
  "path": ".forgeplan/prds/prd-auth-system.md",
  "_next_action": "forgeplan validate prd-auth-system",
  "hint": "Use slug 'prd-auth-system' in commit Refs: until merged. Number PRD-074 will be assigned by CI bot at merge to dev."
}
```

**Errors**:
| Status | Code | Description |
|--------|------|-------------|
| 400 | INVALID_TITLE | Empty or non-printable title |
| 400 | INVALID_SLUG | Generated slug fails regex (e.g. all-special-chars title) |
| 409 | SLUG_EXISTS_LOCAL | Slug already exists in current workspace |
| 409 | SLUG_RESERVED | Slug uses reserved prefix |
| 422 | TITLE_TOO_LONG | Title > 200 chars (slug truncation may lose meaning) |

### MCP `forgeplan_get`

**Request**: identifier in any of these forms:
- Slug: `prd-auth-system`
- Display id with number: `PRD-074`
- Display id with predicted: `PRD-74?` (rare; only for local pending artifacts)
- Legacy uppercase: `PRD-018` (resolves via assigned_number lookup)

**Response (200 OK)** — same shape regardless of input form:
```json
{
  "slug": "prd-auth-system",
  "predicted_number": 74,
  "assigned_number": 74,
  "id_canonical": "prd-auth-system",
  "id_display": "PRD-074",
  "kind": "prd",
  "title": "Auth System",
  "status": "active",
  "frontmatter": { /* all fields */ },
  "body": "# PRD-074: Auth System\n\n..."
}
```

**Errors**:
| Status | Code | Description |
|--------|------|-------------|
| 404 | NOT_FOUND | No artifact matches the given identifier |
| 409 | AMBIGUOUS | Identifier matches multiple artifacts (should not happen with proper invariants) |

### CI Workflow contract — `.github/workflows/assign-id.yml`

```yaml
name: Assign artifact ID
on:
  pull_request:
    types: [labeled]   # Triggered by 'ready-to-merge' label

concurrency:
  group: forgeplan-id-assign
  cancel-in-progress: false   # Serialize, do not cancel

jobs:
  assign:
    runs-on: ubuntu-latest
    if: github.event.label.name == 'ready-to-merge'
    steps:
      - uses: actions/checkout@v4
        with: { ref: ${{ github.head_ref }} }
      - run: cargo run --bin forgeplan -- ci-assign-id --pr ${{ github.event.pull_request.number }}
      - run: |
          git config user.name "forgeplan-bot"
          git config user.email "bot@forgeplan.dev"
          git commit -am "chore: assign $(forgeplan ci-last-assigned)"
          git push
```

Behavior:
1. Triggered when 'ready-to-merge' label added to PR
2. `concurrency` group `forgeplan-id-assign` ensures atomic serialization
3. `forgeplan ci-assign-id` command: scans new artifacts in PR, finds next free number per kind in origin/dev, sets `assigned_number`, renames files
4. Auto-commit and push back to PR branch
5. PR can then be merged normally

### Slug collision handling (rare)

When CI bot detects two PRs in merge queue with same slug:
1. First PR (chronologically merged) keeps original slug
2. Second PR's slug auto-suffixed: `prd-auth` → `prd-auth-2`
3. Frontmatter updated, filename renamed
4. Same-PR refs in body of OTHER artifacts (via `Related:`, `Refs:`) automatically rewritten by `forgeplan reconcile-ids` step
5. PR comment posted notifying author of suffix

**Cross-PR refs** to the suffixed slug remain pointing to original slug — manual fix required (rare). `forgeplan reconcile-ids --report-cross-pr` detects these.

---

## Migration Semantics (legacy 73 artifacts)

For each existing artifact:

```python
# Pseudo-migration script (RFC-009 covers full rollout)
for artifact in scan_workspace():
    if 'assigned_number' not in artifact.frontmatter:
        # Generate slug from existing title
        slug = generate_slug(artifact.kind, artifact.title)
        # Use existing numeric prefix as assigned_number
        assigned = parse_existing_id(artifact.path)  # e.g. PRD-018 → 18
        # Update frontmatter (additive — do NOT modify other fields)
        artifact.frontmatter['slug'] = slug
        artifact.frontmatter['predicted_number'] = assigned
        artifact.frontmatter['assigned_number'] = assigned
        artifact.save()  # filename unchanged
```

**Invariants preserved**:
- Existing IDs (`PRD-018`) continue to work in all refs and search
- Filenames are not renamed for legacy artifacts
- No content changes — only frontmatter additions

**Slug collisions in legacy**: if two legacy artifacts produce same slug from titles (e.g. two PRDs both titled "Authentication"), the script logs a warning and uses `<slug>-<number>` for the second one.

---

## Versioning

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-05-06 | Initial specification — slug, predicted_number, assigned_number contract |

Future versions reserved for: namespace prefixes (multi-tenant), translation between conventions (if ever needed), bulk operations API.

---

## Related

- PROB-060: Distributed artifact ID assignment — collisions in parallel branches
- PRD-076: Product requirements (this SPEC implements)
- ADR-012: Decision record (slug-canonical, number-display)
- RFC-009: Migration rollout plan
- PRD-057: Multi-agent dispatch (consumer of this contract)
- PRD-071: Hint protocol (forgeplan_new response complies)
- ADR-003: Markdown source of truth (invariant preserved by this design)

---

> **Next step**: After approve → RFC-009 covers implementation phases.

