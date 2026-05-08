# PROB-060 Phase 2.1 — Adversarial Audit Round 3

**Date**: 2026-05-08
**Audited branch**: `feat/prob-060-phase-2-1-integration` после Phase 2.1 hotfix sprint
**Auditors** (parallel, single message):
- Security expert (Round 3 adversarial)
- Code reviewer (Round 3 adversarial)

**ADI budget**: TERMINAL (3 rounds max per autorun policy). After this audit — STOP regardless of findings.

**Total NEW findings**: 13 (2 CRITICAL — but converge на one root cause + 1 HIGH + 5 MEDIUM + 5 LOW)

---

## Round 2 closure verification

| ID | Finding | Status |
|---|---|---|
| Sec FINDING-3 | Layer B substring matcher | ✅ VERIFIED FIXED via `slug_exists_in_filenames` |
| Sec FINDING-4 | assign-id self-deadlock (null→integer) | ⚠️ CLAIMED CLOSED but BROKEN by FINDING-1 below |
| Sec FINDING-5 | write_predicted_number SEC-6 | ✅ VERIFIED FIXED via mirror SEC-6 block |
| Sec FINDING-6 | sanitize shell metas | ✅ VERIFIED FIXED via reject set extension |
| Sec FINDING-7 | destructive MCP hints | ⚠️ PARTIAL — delete+activate fixed, list missed |
| Code FINDING-2 | cli_hint_slug_aware coverage | ✅ VERIFIED FIXED — 13 new tests |
| Code FINDING-3 | ci.yml dev base gate | ✅ VERIFIED FIXED |

---

## CRITICAL — Round 3 fix landed surgically

### CRIT-1 [Sec & Code FINDING-1 converged]: bash precedence bug в validate-frontmatter.sh

**File**: `.github/scripts/validate-forgeplan-frontmatter.sh:73-79` (PRE-FIX)
**CWE**: CWE-754 (improper exception check), CWE-840 (incomplete enforcement)

**Bug**: `git show A 2>/dev/null || git show B 2>/dev/null | sed -n ... | grep ...`. Bash `|` binds tighter than `||`, so this parses as `A || (B | sed | grep | ...)`. When A succeeds (typical CI case с `origin/dev` fetched), `previous` captures entire raw markdown body instead of parsed assigned_number value.

**Impact**:
- `[[ "$current" != "$previous" ]]` always true когда previous = full file body
- Round 2 FINDING-4 self-deadlock fix (null→integer legitimate) — **never fires** because previous is never literal `null`
- Every PR modifying existing artifact body would emit false write-once violation

**Why Round 2 smoke tests missed it**: tested tamper case (73 → 999999), which non-equal regardless of parse correctness. Did NOT test legitimate body edit или null→integer transition.

**Fix landed Round 3** (commit pending):
```bash
local raw
raw=$(git show "origin/${base_ref}:${file}" 2>/dev/null \
    || git show "${base_ref}:${file}" 2>/dev/null \
    || true)
local previous
previous=$(printf '%s\n' "$raw" \
    | sed -n '/^---$/,/^---$/p' \
    | grep "^assigned_number:" \
    | head -1 \
    | sed 's/^assigned_number:[[:space:]]*//' \
    | sed 's/^"\(.*\)"$/\1/')
```

**Smoke verified**:
- Test 1: edit body only (assigned stays 73 → 73) → exit 0 PASS ✅
- Test 2: tamper 73 → 999 → exit 1 FAIL ✅

---

## HIGH — Round 3 fix landed surgically

### HIGH-1 [Code FINDING-2]: forgeplan_list first_draft hint emits raw display id

**File**: `crates/forgeplan-mcp/src/server.rs:1340` (PRE-FIX)
**Category**: api_consistency / CD-5 violation

**Bug**: Round 2 Sec FINDING-7 listed 3 sites still emitting raw display id. Phase 2.1 closed delete + activate; list first_draft missed.

**Fix landed Round 3** (commit pending): change `&a.id` → `&a.id_canonical`. ArtifactSummaryDto::id_canonical всегда populated (slug if present, else lowercased display id fallback).

---

## DEFERRED — Round 3 NEW findings (not fixed, ADI budget exhausted)

### Sec FINDING-3 [MED]: ci.yml gate uncovered for hotfix→main и stacked PR flows

**File**: `.github/workflows/ci.yml:7,37`
**Issue**: HIGH-5 closure restricted gate к `dev` base. Coverage matrix:
- `feat/* → dev` ✅ runs
- `release/v* → main` ✅ skipped (intended)
- `hotfix/* → main` ❌ skipped (unintended bypass)
- `feat/* → feat/parent` (stacked) ❌ workflow doesn't fire
**Recommendation**: extend `if:` to include hotfix→main case.

### Sec FINDING-4 [MED]: apply_actions silent fail-open on canonicalize failure

**File**: `crates/forgeplan-cli/src/commands/reconcile_ids.rs:892`
**Issue**: `let canonical_workspace = fs::canonicalize(workspace).ok();` — if canonicalize fails, SEC-6 boundary check silently skipped.
**Recommendation**: bail (return Err) rather than fail-open.

### Code FINDING-3 [MED]: silently bundled FINDING-4 closure not in Phase 2.1 brief

**File**: `.github/scripts/validate-forgeplan-frontmatter.sh:85-93`
**Issue**: Round 2 commit 28ade1a embedded FINDING-4 (null→integer legitimate) closure but doc lists FINDING-4 as "DEFERRED". Process / changelog hygiene.
**Recommendation**: Update Round 2 audit doc OR break out as separate fix.

### Code FINDING-4 [MED]: Layer B integration uncovered, only predicate tested

**File**: `crates/forgeplan-cli/src/commands/ci_assign_id.rs:495-552`
**Issue**: 2 new predicate tests verify `slug_exists_in_filenames` correctness. NOT tested: `compute_assignment_plan` integration (consumer site). Future refactor could regress consumer без catching predicate.
**Recommendation**: extract base_files_per_kind construction для injectable factory; unit test full plan.

### Sec FINDING-5 [MED]: render_human still emits absolute workspace path

**File**: `crates/forgeplan-cli/src/commands/reconcile_ids.rs:1040`
**Issue**: Round 2 MED-2 fixed JSON output to `.forgeplan` relative; human path still absolute. Asymmetry.
**Recommendation**: emit `Workspace: .forgeplan` parity с JSON.

### Code FINDING-2 [MED]: import_post_run_hint test is weak proxy

**File**: `crates/forgeplan-cli/tests/cli_hint_slug_aware.rs:909-928`
**Issue**: Test claims to verify slug-aware import hints но import emits `forgeplan health` (no per-id). Tests negative (no leak) without positive coverage.
**Recommendation**: rename test или replace с slug-positive assertion.

### Code FINDING-5 [LOW]: reopen test positive arm over-permissive

**File**: `crates/forgeplan-cli/tests/cli_hint_slug_aware.rs:609-612`
**Issue**: `next.contains("PRD-002") || next.contains("prd-")` — second clause matches any string c "prd-". Specific regression к pre-merge slug emission could pass undetected.

### Sec FINDING-6 [LOW]: Layer B predicate tests don't exercise integration site

(Same as Code FINDING-4 above — overlapping.)

### Sec FINDING-12 [LOW]: deterministic tmp filename collision (Round 2 carryover)

**File**: `crates/forgeplan-cli/src/commands/reconcile_ids.rs:680`
**Recommendation**: use `tempfile::NamedTempFile::new_in(parent)`.

### Sec FINDING-13 [LOW]: bash kind regex hand-maintained (Round 2 carryover)

**File**: `.github/scripts/validate-forgeplan-frontmatter.sh:20`
**Recommendation**: pull kind list from `forgeplan list-kinds --json`.

### Code FINDING-6 [LOW]: sanitize `!` reject changes hint readability

**File**: `crates/forgeplan-core/src/artifact/sanitize.rs:118`
**Issue**: `!` rejected for bash history expansion (interactive only). Titles с `!` lose punctuation.
**Recommendation**: re-evaluate threat model; consider dropping `!` from reject set.

### Code FINDING-7 [LOW]: docstring "Layer C" naming collision

**File**: `crates/forgeplan-cli/src/commands/ci_assign_id.rs:2700-2710`
**Issue**: Test docstring uses "Layer C" for ci.yml error reporting; Round 2 audit also called CRIT-2 unimplemented Layer C "per-file base partition". Reader confusion.

---

## Closure status

| Severity | Round 3 NEW count | Phase 2.1 status |
|---|---|---|
| CRITICAL | 1 (CRIT-1) | ✅ FIXED surgically (commit pending) |
| HIGH | 1 (Code-2) | ✅ FIXED surgically (commit pending) |
| MED | 5 | ⏳ DEFERRED |
| LOW | 5 | ⏳ DEFERRED |

**ADI budget**: Round 3 of fixes для Phase 2.1 blocker (validation gate correctness). 3-rounds-max per blocker reached. **NO Round 4**. User triages remaining 10 deferred findings (5 MED + 5 LOW).

**Phase 2.1 fixer scope verdict**:
- HIGH-1..HIGH-5 originals ALL closed (verified)
- Round 3 found 2 BLOCKING issues (CRIT-1 + HIGH-1) — both surgically fixed in this round
- Phase 2.1 sprint shippable post-fix; Round 3 deferred items go в backlog

**Pre-merge checklist for user**:
- [ ] Verify CRIT-1 fix smoke tests (body edit PASS, tamper FAIL)
- [ ] Verify HIGH-1 fix (forgeplan_list first_draft uses id_canonical)
- [ ] Triage 10 deferred findings — file as PROB-XXX or accept в next sprint
- [ ] Approve push (red-line #2)
