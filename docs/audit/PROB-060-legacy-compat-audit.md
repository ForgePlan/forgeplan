# PROB-060 Legacy Compatibility Audit (Phase 2.3 — T1)

**Scope.** Verify that artifacts created **before** the PROB-060 / SPEC-005 /
ADR-012 schema enforcement (Phase 1.5) work as **first-class citizens** through
every CLI / MCP / resolver / hint / validation / lifecycle code path.
Without migration. Forever.

**Trigger.** PR #268 (`feat → dev` sync) revealed 8 legacy PROB-060 artifacts
(`ADR-012`, `PRD-076`, `RFC-009`, `SPEC-005`, `EVID-114`, `EVID-115`,
`PROB-060`, `PROB-061`) had **double frontmatter** (template-generated outer
block + manually-edited inner block) and **no `slug:` field** in the outer
block. They technically work via the `is_new=false` resolver path, but no
audit had ever asserted that all surfaces (resolver, MCP DTO, hint emission,
validation, lifecycle, scoring) handle them gracefully.

**Methodology.** Read every relevant source path. For each, identify whether
it has a `Some/None` branch on the new identity fields (slug, predicted_number,
assigned_number) and whether the `None` branch produces a usable result for the
agent. When the branch was missing or the fallback was wrong → document and
(if surgical) fix. When the path is correct → record the evidence so the next
auditor doesn't re-derive it.

The companion E2E suite — `crates/forgeplan-cli/tests/legacy_compat_e2e.rs` —
exercises the same paths through the real `forgeplan` binary on synthetic
legacy fixtures.

---

## 1. Frontmatter accessors

`crates/forgeplan-core/src/artifact/frontmatter.rs`

| Function | Legacy behavior | Verdict |
|---|---|---|
| `parse_frontmatter` | Reads the **first** `---…\n---` block. A double-frontmatter file (legacy PROB-060 case) has its second block read as part of body — semantically correct since the FIRST block is the canonical one. Tested at line 31. | OK |
| `slug_from_frontmatter` | `fm.get("slug").and_then(...as_str)` → `None` for legacy files. Documented contract (line 75–78) explicitly mandates fall-through to filename-derived id. | OK |
| `predicted_number_from_frontmatter` | Returns `None` when field absent. Bounded by `MAX_ARTIFACT_NUMBER` (1M). | OK |
| `assigned_number_from_frontmatter` | Returns `None` when field absent **or** when explicit `null`. Treats both equivalently (legacy = pre-merge). | OK |
| `is_pre_merge` | Documented to treat **legacy artifacts (no `assigned_number` field at all) as pre-merge** — same as explicit `null` (line 137–142). Hint-emission can rely on this. | OK |
| `refs_form` | When pre-merge but **no slug**, falls back to `fallback_id` (the resolver's display id) — line 174–181. Slug shape validation gates HIGH-3 (CWE-117 / prompt-injection) on the slug content; legacy `None` slugs short-circuit cleanly. | OK |
| `refs_form_from_body` | `parse_frontmatter` failure → returns `fallback_id` verbatim (line 196–197). Non-fatal on malformed input. | OK |

**Test fixture in module:** `legacy_frontmatter_returns_none_for_all_new_fields`
(line 476–483) already asserts the legacy contract for the three accessors.

**Gap?** None. Helpers were designed with explicit legacy fall-through and
contracts are documented.

---

## 2. Resolver — `LanceStore::resolve_id`

`crates/forgeplan-core/src/db/store.rs:849-918`

Two paths:

1. **Display-id form** (`KIND-NNN`) — `prefix.split_once('-')` →
   `from_slug_prefix(prefix)` → uppercase + zero-pad → `get_record`.
   This path **does not depend on slug at all**, so legacy artifacts with
   only `id: PRD-018` in the bare frontmatter resolve via direct DB lookup.
2. **Slug form** (`prefix-suffix-...`) — filters records by kind, parses each
   record's body frontmatter, calls `slug_from_frontmatter`. Records without
   slug field are silently skipped (line 906–914). For legacy artifacts this
   path returns `None` — caller must use display-id form.

**Test in module:**
`resolve_id_legacy_artifact_without_slug_field_still_resolves_by_display_id`
(line 2041–2066) already asserts:
- `resolve_id("PRD-018")` → `Some("PRD-018")` for an artifact with bare
  `id: PRD-018\n---\n` frontmatter.
- `resolve_id("prd-legacy-artifact")` → `None` (no slug to match).

**Gap?** None. Display-id form is the documented fallback for legacy.

---

## 3. Display-id rendering & `IdentityFields`

`crates/forgeplan-mcp/src/convert.rs:49-96` (`identity_from_record`)

For a legacy record (no slug, no predicted_number, no assigned_number in body):

- `slug` → `None` (gated through `validate_slug` HIGH-3 defence)
- `predicted` → `None`
- `assigned` → `None`
- `id_canonical` → `id.to_ascii_lowercase()` (line 79) — round-trippable via
  resolver display-id path
- `id_display` → falls back to verbatim `id` (line 84–87) when either kind
  parse fails OR `predicted` is `None`

**Verdict.** Legacy records produce a fully-populated `IdentityFields` with
the canonical lowercased display id. Downstream MCP DTOs (`ArtifactRecordDto`,
`ArtifactSummaryDto`) accept `Option<String>` / `Option<u32>` for
slug/predicted/assigned, so `None` propagates without panic.

**Gap?** None. `id_canonical` always populated; `id_display` falls back to
verbatim id when no predicted number is available — round-trips.

---

## 4. CLI command resolver wiring (Phase 2.6 — 21 commands)

`grep -rn "resolve_id" crates/forgeplan-cli/src/commands/` lists 21 call sites
(activate, calibrate_estimate, claim, deprecate, decompose, estimate, delete,
fgr, get, import_cmd, link×2, reason, release, reopen, validate, score,
renew, supersede×2, update). Each delegates to `LanceStore::resolve_id`
which has the path-1 (display-id) fallback documented in §2.

Spot-checked patterns:
- `forgeplan get` (line 14): `match store.resolve_id(id).await? { Some(c) => ..., None => bail }` — legacy display-id input resolves on path 1, no error path triggered.
- `forgeplan supersede` (line 14–20): source must resolve (legacy display-id ok); target may not exist (pre-merge cross-PR), falls back to raw input.
- `forgeplan link` (line 15–22): symmetric — both source + target use `resolve_id` with bail-on-None for source, fallback for target.

**Gap?** None. Wiring is uniform across 21 surfaces.

---

## 5. Hint emission — `refs_form_from_body`

CLI sites (`get.rs:54`, `validate.rs:118`) and MCP sites (`server.rs:1934`
forgeplan_get; multiple W3 hint paths) call `refs_form_from_body(&body, &id)`.

Behavior on legacy body (no frontmatter or no slug):
- `parse_frontmatter` succeeds (frontmatter is present, just minimal):
  → `is_pre_merge(fm)` returns `true` (no `assigned_number`)
  → `slug_from_frontmatter` returns `None`
  → `refs_form` returns `fallback_id` (record.id, e.g. `PRD-018`)
- `parse_frontmatter` fails (truly malformed body):
  → `refs_form_from_body` returns `fallback_id` verbatim

**Result.** Hint lines emit the **display id** for legacy artifacts (e.g.
`Next: forgeplan validate PRD-018`). No `?` marker (since predicted is None
and the path doesn't hit `render_display_id`). The agent receives a runnable
command. Commit `Refs:` lines made from legacy artifacts use display-id form,
which post-Phase-1.5 is canonical for any artifact with `assigned_number`
already set or never tracked.

**Gap?** None. Confirmed correct by reading `refs_form` documentation
(line 154–161) and contracts in `forgeplan get` (line 47–55).

---

## 6. Validation — `forgeplan validate`

`crates/forgeplan-cli/src/commands/validate.rs:58-72`

Validation runs against `record.frontmatter_map()` — which is built from
**DB columns** (id, kind, status, title, depth, ...), NOT from the body's
actual frontmatter (`crates/forgeplan-core/src/db/store.rs:151-180`).

**Implication.** Validation never inspects whether the on-disk body has slug
or predicted_number — those fields aren't part of the validation rule set.
Legacy artifacts validate on equal footing with new ones.

**Gap?** None. Validation is identity-field-agnostic.

---

## 7. Scoring (`forgeplan score`) and R_eff

`crates/forgeplan-cli/src/commands/score.rs:113`

Score resolves the target via `resolve_id`, then runs the cached R_eff
pipeline against evidence relations. No code path queries slug or
predicted/assigned fields.

**Gap?** None.

---

## 8. Lifecycle — activate / supersede / deprecate / renew / reopen

All lifecycle commands resolve the source via `resolve_id` (Phase 2.6 wiring),
then delegate to `forgeplan_core::lifecycle::*`. The lifecycle module
(`crates/forgeplan-core/src/lifecycle/mod.rs`) operates on the canonical DB id
returned by the resolver — slug/predicted/assigned are not consulted.

**Gap?** None.

---

## 9. Linking (typed cross-artifact relations)

`crates/forgeplan-cli/src/commands/link.rs:15-22`

Source resolves (bail-on-None); target falls back to raw input on resolve
miss (forward-reference cross-PR semantics). Storage layer
(`store.add_relation(src, tgt, rel)`) takes only the canonical ids.

A legacy artifact can be linked TO a modern artifact and vice-versa: both
ids round-trip through `resolve_id` independently.

**Gap?** None.

---

## 10. Search (BGE-M3 semantic + text fallback)

`crates/forgeplan-mcp/src/server.rs:3397-...` (`forgeplan_search`).

Semantic search operates on body text + title via embedding similarity. No
identity field is consulted. Text fallback uses `tantivy` indices over the
same body content. Legacy artifacts with no slug are searchable by title and
body content like any other artifact.

**Gap?** None.

---

## 11. JSON DTOs (MCP)

`crates/forgeplan-mcp/src/types.rs` → `ArtifactSummaryDto`, `ArtifactRecordDto`

Both DTOs declare:
- `slug: Option<String>` (line ~)
- `predicted_number: Option<u32>`
- `assigned_number: Option<u32>`
- `id_canonical: String` (always populated — slug or lowercased display id)
- `id_display: String` (always populated — pretty render or verbatim id)

For legacy records, the three optional fields serialize as JSON `null`. The
two always-populated fields hold the legacy display id (round-trippable).

**Gap?** None — schema by design.

---

## 12. Double-frontmatter parsing (the actual bug-trigger from PR #268)

The legacy PROB-060 artifacts have a body shape like:

```
---
depth: standard
id: PRD-076
kind: prd
status: draft
title: Lazy artifact ID assignment with slug-canonical and number-display
---

---
id: PRD-076
title: "Lazy artifact ID assignment with..."
status: Draft
priority: P0
...
---

# PRD-076: ...
```

`parse_frontmatter` (line 8–31) finds the **first** closing `\n---`, so:
- The OUTER block (template-derived) is parsed as frontmatter.
- The SECOND block becomes part of `body`.
- All subsequent operations (slug extraction, predicted_number extraction)
  read from the OUTER block. Since the outer block lacks a `slug:` field,
  `slug_from_frontmatter` returns `None` — exactly the legacy contract.

**Gap?** None. The parser semantically ignores second blocks (treats them
as body), which matches the migration-friendly contract: "the FIRST block
is canonical; manual edits in subsequent blocks are documentation, not
schema."

---

## Summary — gaps found and fixed

| Gap | Severity | Location | Fix |
|---|---|---|---|
| (none found) | — | — | — |

**Conclusion.** All 12 surfaces handle legacy artifacts (no slug, no
predicted_number, no assigned_number, optional double frontmatter) as
first-class citizens **without migration**. The schema enforcement was
intentionally non-breaking from day 1 — every accessor returns `Option<T>`
with a documented fallback, every resolver path has a display-id branch,
every DTO field is `Option<String> | always-populated-fallback`.

**Migration becomes truly cosmetic / optional.** A user who never migrates
their legacy artifacts can:

- `forgeplan get PRD-018` → works (display-id resolver path)
- `forgeplan list` → returns the legacy artifact with `slug: null`
- `forgeplan validate PRD-018` → works (validation reads DB columns)
- `forgeplan score PRD-018` → works (R_eff is identity-field-agnostic)
- `forgeplan supersede PRD-018 --by PRD-074` → works (lifecycle ops use canonical id)
- `forgeplan link PROB-060 PRD-076 --relation based_on` → works (link layer takes canonical ids)
- `forgeplan search "auth system"` → finds the legacy artifact (semantic search is body-based)
- Hint emission: legacy artifacts emit display-id refs, which are valid
  post-merge canonical refs.

The accompanying E2E suite (`legacy_compat_e2e.rs`) materializes 12+
synthetic legacy fixtures (one per kind + edge cases) and exercises the full
lifecycle to keep this guarantee load-bearing as the codebase evolves.

---

**Audit date.** 2026-05-08
**Auditor.** Phase 2.3 fixer (FIXER 2.3-A — autopilot)
**Branch.** `feat/prob-060-phase-2-3-fix-legacy-e2e`
**Companion evidence.** `crates/forgeplan-cli/tests/legacy_compat_e2e.rs`
