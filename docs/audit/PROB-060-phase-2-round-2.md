# PROB-060 Phase 2 — Adversarial Audit Round 2

**Date**: 2026-05-08
**Audited branch**: `feat/prob-060-phase-2-wave3-integration` (HEAD `28ade1a` after Round 2 CRIT fixes)
**Auditors** (parallel, single message, mandatory ≥3 findings each):
- Security expert — CWE coverage post-fix
- Code reviewer — idiom + tests + API consistency

**Total NEW findings**: 22 (2 CRITICAL, 7 HIGH, 8 MED, 4 LOW)
**Verdict**: BLOCK MERGE (both reviewers concur)

---

## Round 1 closure verification (post Round 1 fixers)

| ID | Status | Evidence |
|---|---|---|
| **Round 1 CRIT-1+2** validation gate | NOT FIXED → fixed Round 2 | Round 1 fix had 3 bash bugs (file discovery empty, null treated as preset, ls-files vs base ref). All 3 fixed in commit `28ade1a`. |
| **Round 1 CRIT-3** CLI hint slug-aware | PARTIAL | 11 of 13 sites fixed; tests cover only 7 of 13 (FINDING-2 below). |
| **Round 1 CRIT-4** MCP hint slug-aware | PARTIAL | 4 named tools fixed (get/score/update/review); `forgeplan_delete`, `forgeplan_activate`, `forgeplan_list` still emit raw `p.id` (FINDING-1 below). |
| Round 1 HIGH-1 reconcile_ids hardening | VERIFIED FIXED | symlink + canonicalize + `--` separator |
| Round 1 HIGH-2 HEAD_REF | VERIFIED FIXED | env var + regex (regex permits `..` and leading `-` per FINDING-4 — defense-in-depth gap, not RCE) |
| Round 1 HIGH-3 slug validation chain | VERIFIED FIXED | identity_from_record drops invalid; refs_form fallback |
| Round 1 HIGH-4 atomic write | VERIFIED FIXED | tmp+rename pattern |
| Round 1 HIGH-5 mem prefix | VERIFIED FIXED | regex + scan path include mem |
| Round 1 HIGH-6 import_cmd kind | VERIFIED FIXED | bail before destructive delete |
| Round 1 HIGH-7 resolver positive tests | VERIFIED FIXED | Strategy A/B applied 13 commands |
| Round 1 HIGH-8 cargo build trust | UNADDRESSED | Phase 2.1 productionization backlog |
| Round 1 MED-1 u32 cap | VERIFIED FIXED | 1_000_000 cap (но FINDING-5 questions value choice) |
| Round 1 MED-7 DTO Option | VERIFIED FIXED | NewArtifactResponse aligned |
| Round 1 MED-10 fgr date | VERIFIED FIXED | shared helper, both branches |

---

## Round 2 CRITICAL — fixed в commit `28ade1a`

### CRIT-1 [Sec FINDING-1]: validate-frontmatter.sh discovers ZERO files in CI

`actions/checkout@v4` produces clean working tree. `git diff --name-only --cached` returns empty. Script's `||` fallback never fires. **Validator was non-functional in production.**

**Fix landed**: use `git diff --name-only "origin/${BASE_REF}...HEAD"`. Fail-closed if BASE_REF missing. Validate BASE_REF shape (CWE-78 defense for `git show` interpolation).

### CRIT-2 [Sec FINDING-2]: bash treats `assigned_number: null` as pre-set

`extract_field` returns literal `"null"` for YAML scalar null. `[[ -n "null" ]]` is true. **Rule 1 Layer A rejected EVERY legitimate Phase-2 artifact.**

**Fix landed**: treat `"null"`, `"~"`, `""` as YAML-null equivalent в Rule 1 + assigned_number_changed normalization on both sides of comparison.

### CRIT-3 [related, not in audit]: assigned_number_changed false-positive on new files

Used `git ls-files --error-unmatch` (HEAD-tracking) when needed "exists в base ref". New files в HEAD that aren't в base ref triggered false write-once violation.

**Fix landed**: use `git show "origin/${base_ref}:${file}"` existence check.

**Smoke test verified**:
- New artifact с `assigned_number: null` → PASS exit 0
- Tamper existing artifact 73 → 999999 → FAIL exit 1

---

## Round 2 HIGH findings — DEFERRED to user triage / next sprint

### Sec FINDING-3 — CRIT-2 Layer B substring matcher broken bidirectionally
**File**: `crates/forgeplan-cli/src/commands/ci_assign_id.rs:512-515`
**Issue**: Layer B uses ad-hoc `f.contains(&format!("{}-", c.slug))` — false positive (allows tampered when overlap с existing slug substring) AND false negative (rejects legitimate re-runs because case-sensitivity vs uppercase post-merge filename `PRD-074-foo.md`).
**Fix**: replace с `forgeplan_core::git::slug_exists_in_filenames` (already exists в codebase).

### Sec FINDING-4 — assign-id.yml self-deadlock once validator works
**Files**: `.github/workflows/{ci,assign-id}.yml`
**Issue**: bot push triggers `synchronize` event → ci.yml re-runs validate-frontmatter → detects `null → 74` change → exits 1. Bot's success commit fails CI.
**Fix**: skip validation on commits authored by `forgeplan-bot`, OR treat `null → integer` transition as legitimate.

### Sec FINDING-5 — write_predicted_number lacks SEC-6 hardening
**File**: `crates/forgeplan-cli/src/commands/reconcile_ids.rs:669-695`
**Issue**: Round 1 HIGH-1 fix added SEC-6 to rename path but не к predicted-number write path. Defense-in-depth gap.
**Fix**: mirror `ci_assign_id.rs:752-782` SEC-6 block (symlink check + canonicalize + workspace boundary).

### Sec FINDING-6 — sanitize_for_hint allowlist misses shell metacharacters
**File**: `crates/forgeplan-core/src/artifact/sanitize.rs:83-86`
**Issue**: filters только `` ` { } " ' \ ``. Misses `; $ | & ( ) < > ! # *`. `"; rm -rf $HOME #"` survives sanitize.
**Fix**: extend rejection list to include POSIX shell metas + redirection + comment + glob. Add lint asserting every hint format! goes через sanitizer.

### Sec FINDING-7 / Code FINDING-1 — destructive MCP tools still emit raw p.id
**Files**: `crates/forgeplan-mcp/src/server.rs:2206 (forgeplan_delete), :2431 (forgeplan_activate), :1340 (forgeplan_list first_draft)`
**Issue**: Round 1 CRIT-4 fix only covered get/score/update/review. Destructive surfaces (delete restore hint, activate message, list first_draft hint) still use raw `p.id` или `a.id` (display-id form).
**Fix**: derive `ref_form` from record body, apply sanitize_for_hint. Add MCP-side hint regression tests.

### Code FINDING-2 — cli_hint_slug_aware tests cover only 7 of 13 W3 commands
**File**: `crates/forgeplan-cli/tests/cli_hint_slug_aware.rs`
**Issue**: 6 commands missing regression test: supersede, reopen, claim, release, calibrate-estimate, import.
**Fix**: add 6 tests using existing `make_pre_merge` + `slug_for` helpers.

### Code FINDING-3 — validate-frontmatter fires on release PRs (false positive)
**Files**: `.github/workflows/ci.yml`
**Issue**: gate runs on PRs targeting `main` (release/v*) but assign-id bot only runs on `dev`. main lags dev → release PRs see `assigned_number: null` в base, `73` in HEAD → false write-once violation.
**Fix**: gate с `if: github.base_ref == 'dev'` OR special-case forward-promotion from dev.

---

## Round 2 MED findings — DEFERRED

### Sec FINDING-8 — HEAD_REF whitelist permits `..` + leading `-`
**File**: `.github/workflows/assign-id.yml:124`
**Fix**: tighten regex или `git check-ref-format --branch`.

### Sec FINDING-9 — BASE_REF interpolated unsanitised in `git show "origin/${base_ref}:..."`
**File**: `.github/scripts/validate-forgeplan-frontmatter.sh:69`
**Status**: ADDRESSED by Round 2 fix (commit 28ade1a добавляет BASE_REF shape validation). VERIFY in Round 3.

### Sec FINDING-10 — render_human still emits absolute workspace path
**File**: `crates/forgeplan-cli/src/commands/reconcile_ids.rs:995`
**Fix**: emit `Workspace: .forgeplan` (parity с JSON).

### Sec FINDING-11 — CRIT-2 Layer C (only absorb base candidates) never implemented
**File**: `crates/forgeplan-cli/src/commands/ci_assign_id.rs:539-545`
**Fix**: partition candidates into `base_candidates` (verified в base via per-file git show) и `pr_candidates`. Pass-1 only absorbs from base_candidates.

### Code FINDING-4 — HEAD_REF regex too permissive
**File**: `.github/workflows/assign-id.yml:124`
**Fix**: tighten к Forgeplan branch convention (feat/, fix/, etc.) или add explicit `..` reject.

### Code FINDING-5 — MAX_ARTIFACT_NUMBER = 1_000_000 questionable
**File**: `crates/forgeplan-core/src/artifact/frontmatter.rs:80-92`
**Fix**: drop cap к 100_000 OR add tier (warning at jump > 1000 from previous).

### Code FINDING-6 — release Strategy A test fragility
**File**: `crates/forgeplan-cli/tests/cli_resolver_wiring.rs:651-703`
**Fix**: add positive stdout assertion: `assert!(stdout.contains("Released claim on PRD-001"))`.

### Code FINDING-7 — sanitize_for_hint scope misclassification
**File**: `crates/forgeplan-core/src/artifact/sanitize.rs:23-24`
**Fix**: add `sanitize_for_hint_strict` allowlist mode `[A-Za-z0-9_\-./]` for path-like ids.

---

## Round 2 LOW findings — DEFERRED

### Sec FINDING-12 — deterministic tmp filename collision
**File**: `crates/forgeplan-cli/src/commands/reconcile_ids.rs:680`
**Fix**: use `tempfile::NamedTempFile::new_in(parent)`.

### Sec FINDING-13 — bash kind regex hand-maintained (drift risk)
**File**: `.github/scripts/validate-forgeplan-frontmatter.sh:20`
**Fix**: pull kind list from `forgeplan list-kinds --json`. Add CI drift check.

### Code FINDING-8 — cross_pr marker test gap
**File**: `crates/forgeplan-cli/src/commands/reconcile_ids.rs:202-210`
**Fix**: add `reconcile_ids_cross_pr_marker_does_not_affect_unresolved` test.

### Code FINDING-9 — RFC 3339 `Z` suffix not parsed by is_valid_until_expired
**File**: `crates/forgeplan-cli/src/commands/fgr.rs:22-30`
**Fix**: add third parse_from_str attempt for `%Y-%m-%dT%H:%M:%SZ`.

---

## Closure status

| Category | Count | Status |
|---|---|---|
| Round 2 CRITICAL | 3 (incl. self-discovered #3) | ✅ CLOSED commit `28ade1a` |
| Round 2 HIGH | 7 | ⏳ DEFERRED (user triage) |
| Round 2 MED | 8 | ⏳ DEFERRED (user triage) |
| Round 2 LOW | 4 | ⏳ DEFERRED |

**Honest assessment of Round 1 fixers**: introduced bugs by relying on bash sed/grep YAML parsing instead of `yq`/python (Round 1 finding MED-11 — recommended in audit, ignored in implementation). Net result: Round 1 CRITs were "claimed fixed" but not end-to-end verified. Round 2 caught this. **Lesson**: bash-only YAML validation is fragile; production-grade validator needs `yq` или Rust binary integration.

**ADI budget**: 2 rounds expended. Per autorun policy (3 rounds max), Round 3 audit может run after deferred items addressed but не recommended without explicit user direction.

**Recommendation для user**:
1. Either: accept Phase 2 as-is (CRITs closed, HIGHs deferred) → push с full audit doc для transparency
2. Or: schedule Phase 2.1 hotfix sprint to address Round 2 HIGHs before merge to dev

---

## Files referenced (absolute)

Sec findings: lots — see findings inline.
Code findings: lots — see findings inline.

Round 1 audit doc: `docs/audit/PROB-060-phase-2-round-1.md`.
