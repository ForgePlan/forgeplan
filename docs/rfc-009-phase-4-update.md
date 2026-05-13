# RFC-009 Phase 4 update — workaround note (Phase 2.3 audit)

**Status**: workaround pending lead-applied MCP `forgeplan_update`
**Reason**: RED-LINE #11 forbids direct `Edit`/`Write` on
`.forgeplan/rfcs/RFC-009-*.md`. This note captures the exact text changes for the
team lead to apply via `mcp__forgeplan__forgeplan_update id=RFC-009 body=...`
after merge of `feat/prob-060-phase-2-3-fix-rfc-docs`.
**Author**: Fixer 2.3-C (FIXER 2.3-C sprint)
**Date**: 2026-05-08
**Cross-refs**: PROB-060, PRD-076, RFC-009, ADR-012, SPEC-005; CLAUDE.md
«Working with artifact IDs»; `docs/methodology/ID-ASSIGNMENT.ru.md`.

---

## Why this note exists

PR #268 dev-sync CI revealed что 8 legacy PROB-060-related artifacts работают
без миграции через resolver fallback paths. User signal: «обновлять столько
артефактов это плохо». Phase 2.3 audit (Fixer 2.3-A) подтвердил через E2E suite
(`crates/forgeplan-cli/tests/legacy_compat_e2e.rs`) что все 73 legacy
artifacts (PRD-001..073, ADR-001..011, RFC-001..008, и т.д.) работают
first-class через display id path без `slug` field в frontmatter.

Result: migration становится cosmetic/optional housekeeping, **не required**
для функциональности. RFC-009 §Phase 4 акценты надо сместить — миграция
больше не gating activity.

CLAUDE.md и `docs/methodology/ID-ASSIGNMENT.ru.md` уже обновлены этим sprint
(Fixer 2.3-C). RFC-009 обновляется лидом после merge через MCP.

---

## Apply via MCP after merge

```python
# 1. Capture current body (excludes YAML frontmatter)
body = mcp__forgeplan__forgeplan_get(id="RFC-009")["body"]

# 2. Apply changes per «Diff sections» below
new_body = apply_changes(body)  # see below

# 3. Push update
mcp__forgeplan__forgeplan_update(id="RFC-009", body=new_body)
```

Last-resort fallback: `forgeplan scan-import` пересоберёт LanceDB index из
markdown.

---

## Diff sections

### Diff 1 — Update §Status banner

**Find** (around line 37):

```markdown
**Status 2026-05-07 (Phase 0b complete)** — Phase 0b shipped via integration branch `feat/prob-060-phase-0b-integration`: EVID-114 (Variant B stress-test, CL2 — Variant A pre-Phase-2-GA gate documented в `docs/operations/EVID-A-real-stress-test.md`); EVID-115 (real-workspace migration dry-run, CL3 — 305 artifacts, 6 dogfooding collisions, all `--auto-suffix`-resolvable). ADR-012 R_eff = 0.90 после EVID linking. 14 audit findings closed (9 fixed across 2 fix rounds, 5 deferred с rationale per PR description). Phase 2 unblocked; ready for next sprint cycle.
```

**Replace with**:

```markdown
**Status 2026-05-07 (Phase 0b complete)** — Phase 0b shipped via integration branch `feat/prob-060-phase-0b-integration`: EVID-114 (Variant B stress-test, CL2 — Variant A pre-Phase-2-GA gate documented в `docs/operations/EVID-A-real-stress-test.md`); EVID-115 (real-workspace migration dry-run, CL3 — 305 artifacts, 6 dogfooding collisions, all `--auto-suffix`-resolvable). ADR-012 R_eff = 0.90 после EVID linking. 14 audit findings closed (9 fixed across 2 fix rounds, 5 deferred с rationale per PR description). Phase 2 unblocked; ready for next sprint cycle.

**Status 2026-05-08 (Phase 2.3 audit)** — PR #268 dev-sync CI revealed что все 73 legacy artifacts (PRD-001..073, ADR-001..011, RFC-001..008, etc.) работают first-class через display id path без `slug` field. E2E suite `crates/forgeplan-cli/tests/legacy_compat_e2e.rs` (Fixer 2.3-A) фиксирует resolver fallback, MCP DTO `Option<String>` handling, `refs_form_from_body` graceful degradation. **Migration demoted с MUST до OPTIONAL CLEANUP** — см. §Phase 4 ниже и §Phase 4.5 «Legacy compatibility — no migration required». §Acceptance Criteria обновлены: legacy artifacts больше не должны иметь slug+assigned_number; вместо этого требуется E2E proof что они работают через display id path.
```

---

### Diff 2 — Demote §Phase 4.1 (Migration Script) от MUST до OPTIONAL CLEANUP

**Find** (line 197):

```markdown
- [ ] **4.1** Cutoff date announce в CHANGELOG. Open PRs grandfather rules: PRs открытые до cutoff merge'атся по старой схеме; new PRs после cutoff — по новой.
```

**Replace with**:

```markdown
- [ ] **4.1** *(OPTIONAL CLEANUP — Phase 2.3 audit demoted from MUST)* Cutoff date announce в CHANGELOG. Open PRs grandfather rules: PRs открытые до cutoff merge'атся по старой схеме; new PRs после cutoff — по новой. **Note**: cutoff не блокирует работу с legacy artifacts; они работают first-class via display id path (см. §Phase 4.5). Cutoff нужен только для new artifacts (требование slug в frontmatter).
```

---

### Diff 3 — Mark §Phase 4.2 (Backward compat for 73 legacy artifacts) as already-handled

**Find** (line 198):

```markdown
- [ ] **4.2** Migration script: legacy 298 artifacts get `slug` + `assigned_number` фrontmatter поля (additive only — никаких contents changes). Run on dev, validate, then push.
```

**Replace with**:

```markdown
- [ ] **4.2** *(OPTIONAL CLEANUP — Phase 2.3 audit demoted from MUST; backward compat already handled by Phase 2 resolver)* Migration script: legacy 73 artifacts get `slug` + `assigned_number` frontmatter поля (additive only — никаких contents changes). Run on dev, validate, then push. **Backward compat without migration**: Phase 2 resolver fallback paths уже обеспечивают first-class operation для legacy artifacts. Migration script — cosmetic housekeeping для consistency, не required для функциональности. Default recommendation: don't run unless team explicitly wants single schema across all artifacts.
```

---

### Diff 4 — Add new §Phase 4.5 «Legacy compatibility — no migration required»

**Find** (the line after `4.5 Activation gate`, around line 203):

```markdown
- [ ] **4.5** Activation gate: все EVID собраны (A, B, C, D, E), R_eff > 0.7. ADR-012 переключён в `active`. Feature flag `id_assignment` дефолт меняется на `new`.

**Exit criteria**: smoke test 5 параллельных AI-агентов создают по 3 артефакта, 0 collisions; visual regression Web pass; feature flag `legacy` остаётся доступным как rollback option до v0.34.
```

**Replace with**:

```markdown
- [ ] **4.5** Activation gate: все EVID собраны (A, B, C, D, E), R_eff > 0.7. ADR-012 переключён в `active`. Feature flag `id_assignment` дефолт меняется на `new`.

**Exit criteria**: smoke test 5 параллельных AI-агентов создают по 3 артефакта, 0 collisions; visual regression Web pass; feature flag `legacy` остаётся доступным как rollback option до v0.34.

### Phase 4.5: Legacy compatibility — no migration required (Phase 2.3 audit, 2026-05-08)

**TL;DR**: все 73 legacy artifacts (PRD-001..073, ADR-001..011, RFC-001..008,
EPIC-*, SPEC-001..004, NOTE-*, EVID-*, и т.д.) работают first-class через
display id path **без миграции**. Phase 2 resolver fallback paths,
MCP DTO `Option<String>` handling, и `refs_form_from_body` graceful
degradation обеспечивают backward compat by construction.

#### Mechanism (already shipped в Phase 1/Phase 2)

1. **Resolver** — `crates/forgeplan-core/src/artifact/store.rs`:
   - Если `slug` отсутствует в frontmatter (legacy state), lookup по
     `assigned_number` через display id (`PRD-074`).
   - Primary key: display id (always set для legacy). Slug — optional
     secondary key.
2. **MCP DTOs** — `crates/forgeplan-mcp/src/types.rs`:
   - `slug: Option<String>` с `#[serde(skip_serializing_if =
     "Option::is_none")]` — legacy artifacts возвращаются без поля `slug`
     в JSON.
   - Agent видит только `id` / `id_display` и работает с ними.
3. **`refs_form_from_body`** — fallback к display id:
   - Если parse slug fails (legacy `Refs: PRD-018`), возвращается
     canonical display id.
   - Resolver принимает оба формата.

#### E2E proof

`crates/forgeplan-cli/tests/legacy_compat_e2e.rs` (Fixer 2.3-A) фиксирует
все три fallback paths real CLI invocation. Tests should run as part of
the standard CI gate.

#### What this means для Phase 4

- Migration is cosmetic, not functional requirement.
- §4.1 cutoff date — нужен только для NEW artifacts (slug required).
- §4.2 migration script — optional housekeeping.
- §Activation gate (4.5) не зависит от migration completion.

**Exit criteria** для Phase 4.5: E2E suite `legacy_compat_e2e.rs` green
in CI; documentation (CLAUDE.md «Working with artifact IDs» legacy note,
`docs/methodology/ID-ASSIGNMENT.ru.md` §«Legacy artifacts compatibility»)
landed via `feat/prob-060-phase-2-3-fix-rfc-docs` branch (Fixer 2.3-C).
```

---

### Diff 5 — Update §Acceptance Criteria

**Find** (in §Goals or summary, line 58):

```markdown
3. Backward compat: все 298 существующих артефактов и `Refs:` к ним продолжают работать
```

**Replace with**:

```markdown
3. Backward compat: все 298 существующих артефактов и `Refs:` к ним продолжают работать **через display id path без миграции** (verified by `crates/forgeplan-cli/tests/legacy_compat_e2e.rs` E2E suite — Phase 2.3 audit, 2026-05-08)
```

**Find** (any acceptance criteria mentioning «73 legacy artifacts have slug+assigned_number»):

> Note: this exact text **may not exist** as a single bullet in current
> RFC-009 — search and replace contextually. The intent is: replace any
> language demanding "all 73 legacy artifacts must have slug + assigned_number
> as part of activation gate" with "all 73 legacy artifacts continue to work
> via display id path, verified by E2E suite".

If found, **replace**:

```markdown
- All 73 legacy artifacts have slug + assigned_number in frontmatter
```

**With**:

```markdown
- All 73 legacy artifacts continue to work via display id path (verified by `crates/forgeplan-cli/tests/legacy_compat_e2e.rs` E2E suite)
```

---

## Verification checklist (lead apply)

After applying via MCP `forgeplan_update`:

- [ ] `forgeplan get RFC-009` returns updated body (verify «Phase 2.3 audit» banner, «OPTIONAL CLEANUP» markers on §4.1/§4.2, new §Phase 4.5 section).
- [ ] `forgeplan validate RFC-009` returns 0 errors.
- [ ] LanceDB index synced (`forgeplan scan-import` if needed as fallback).
- [ ] Cross-refs intact (CLAUDE.md, ID-ASSIGNMENT.ru.md, ADR-012).
- [ ] `forgeplan list rfc | grep RFC-009` shows status unchanged (still Draft until Phase 2 GA).

---

## Why doc workaround instead of direct edit

RED-LINE #11 (CLAUDE.md): «STRICT: Forgeplan artifacts мутировать ТОЛЬКО через
MCP/CLI». Direct `Edit`/`Write`/`sed` на `.forgeplan/rfcs/RFC-009-*.md`
десинхронизирует:
- LanceDB index (`forgeplan_get` returns stale data)
- State machine (`.forgeplan/state/RFC-009.yaml`)
- Canonical body invariant

Therefore Fixer 2.3-C captures changes here as text для lead-applied MCP update,
keeping the audit trail and avoiding silent index corruption.

---

## Related artifacts (cross-refs)

| Artifact | Relation |
|----------|----------|
| RFC-009 | target of this update note |
| PROB-060 | underlying problem |
| PRD-076 | product requirements |
| ADR-012 | F-G-R decision |
| SPEC-005 | technical contract |
| `crates/forgeplan-cli/tests/legacy_compat_e2e.rs` | E2E proof (Fixer 2.3-A) |
| `CLAUDE.md` (Working with artifact IDs §) | aligned by Fixer 2.3-C |
| `docs/methodology/ID-ASSIGNMENT.ru.md` | aligned by Fixer 2.3-C |
| `docs/audit/PROB-060-legacy-compat-audit.md` | Fixer 2.3-A audit findings |
