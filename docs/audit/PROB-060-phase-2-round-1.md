# PROB-060 Phase 2 — Adversarial Audit Round 1

**Date**: 2026-05-07
**Audited branch**: `feat/prob-060-phase-2-wave3-integration` (HEAD `4cef534`)
**Auditors** (parallel, single message, mandatory ≥3 findings each):
- Security expert (`agents-pro:security-expert`) — CWE-22/78/88/94/117/200/345/367/426/829/942 + supply chain + secret leakage + race conditions
- Code reviewer (`agents-core:reviewer`) — idiom, error propagation, test coverage, API consistency, doc, module org, backwards compat, test quality

**Total findings**: 28 (4 CRITICAL, 9 HIGH, 11 MED, 4 LOW)
**Verdict**: BLOCK MERGE (both reviewers concur — CRITICAL fixes required before promotion to dev)

---

## Cross-cutting issues (overlap between auditors)

| Issue | Sec finding | Code finding |
|---|---|---|
| validate-frontmatter.sh write-once gate non-functional | CRIT-1 (HEAD=HEAD comparison) | HIGH-4 (wrong base ref `origin/main` vs `github.base_ref`) |
| Hint protocol regression (slug not propagated to all surfaces) | HIGH-5 (defense-in-depth: hint string interpolation) | CRIT-1 + CRIT-2 (11 CLI commands + 4 MCP tools emit display id) |
| reconcile-ids file ops missing safety | HIGH-3 (path traversal, missing `--`, no canonicalization) | HIGH-3 (non-atomic write_predicted_number) + MED-12 (helper drift) |

---

## CRITICAL findings (4 — must fix before merge)

### CRIT-1 [Sec-1 + Code-HIGH-4]: validate-forgeplan-frontmatter.sh write-once rule never fires

**Files**:
- `.github/scripts/validate-forgeplan-frontmatter.sh:49-78`
- `.github/workflows/ci.yml:39-49`

**CWE**: CWE-345 (insufficient verification), CWE-1287 (improper input validation)

**Threat**: Phase 2.1 validation gate's invariant I-2 (write-once on `assigned_number` per SPEC-005) is non-functional. Script reads `current` from working tree and `previous` from `git show HEAD:"$file"` — but `actions/checkout@v4` puts PR HEAD into the working tree. Working tree IS HEAD. Comparison is byte-identical. **Validator silently passes any tampering of existing `assigned_number`.**

Bonus issue: fallback diff base in `:150` uses `origin/main` instead of `${{ github.event.pull_request.base.ref }}`. PRs targeting `dev` (per `feat/* → dev` workflow) get diffed against wrong base.

**Reproduction**:
1. PR mutates existing `prd-074-foo.md` from `assigned_number: 73` to `assigned_number: 999999`
2. Validation gate passes
3. Merge into dev → counter poisoned

**Fix**:
- Plumb `BASE_REF` env var from `${{ github.event.pull_request.base.ref }}` into validator
- Read `previous` via `git show "origin/${BASE_REF}":"$file"`
- Drop `|| true` swallowing all diff failures (fail-closed)
- Add integration test fixture: write mutated artifact → validator must exit 1

---

### CRIT-2 [Sec-2]: Pre-set `assigned_number` on new artifact bypasses I-2 invariant + poisons sequence counter

**Files**:
- `.github/scripts/validate-forgeplan-frontmatter.sh:101-119` (Rule 1 doesn't reject pre-set `assigned_number` on NEW files)
- `crates/forgeplan-cli/src/commands/ci_assign_id.rs:484-490, 504-513` (`compute_assignment_plan` Pass-1 absorbs PR-controlled value into `seq_per_kind`)

**CWE**: CWE-345, CWE-639 (auth bypass via user-controlled key), CWE-770 (counter exhaustion DoS)

**Threat**: PR adds new `prd-evil.md` with frontmatter `assigned_number: 999999`. Validator Rule 1 only checks slug+predicted (line 101-119). Bot's `discover_candidates` reads `current_assigned: Some(999999)`. `compute_assignment_plan` Pass-1 (line 488: `*entry = (*entry).max(existing)`) poisons sequence counter to ≥999999. Pass-2 takes `if let Some(existing) = c.current_assigned` branch (line 504), emitting `already_assigned: true`. `apply_plan` skips `set_assigned_number` (line 712 no-op path), so frontmatter.rs:245-252 enforcement never runs. Silent contract violation + counter exhaustion.

**Fix** (defense in depth):
- Layer A: validator Rule 1 must reject non-null `assigned_number` on new artifacts
- Layer B: `discover_candidates` re-validates non-null `current_assigned` against `--base` via `git show <base>:<path>`. New file in PR + `current_assigned.is_some()` → exit `EXIT_INVARIANT_VIOLATION` (= 4, currently `#[allow(dead_code)]` const)
- Layer C: `compute_assignment_plan` Pass-1 only absorbs `current_assigned` for candidates genuinely in `--base`

---

### CRIT-3 [Code-1]: 11 of 13 W3-wired CLI commands emit display id, defeating CD-5 slug-aware contract

**Files**: Per command, hint emission lines:
- `crates/forgeplan-cli/src/commands/update.rs:177` — `forgeplan validate {id}`
- `crates/forgeplan-cli/src/commands/delete.rs:96-99` — `forgeplan restore {id}`
- `crates/forgeplan-cli/src/commands/supersede.rs:86` — `forgeplan get {by}`
- `crates/forgeplan-cli/src/commands/renew.rs:65` — `forgeplan score {id}`
- `crates/forgeplan-cli/src/commands/reopen.rs:114` — `forgeplan validate {result.new_id}`
- `crates/forgeplan-cli/src/commands/estimate.rs:192-193`
- `crates/forgeplan-cli/src/commands/calibrate_estimate.rs:193, 198, 203`
- `crates/forgeplan-cli/src/commands/fgr.rs:99-100, 172-173`
- `crates/forgeplan-cli/src/commands/claim.rs:75-76, 91, 96-100, 113`
- `crates/forgeplan-cli/src/commands/release.rs:64, 88`
- `crates/forgeplan-cli/src/commands/import_cmd.rs:259` (no id, OK; flagged for completeness)

Only `decompose.rs:62` and `reason.rs:146` consume `refs_form_from_body` correctly.

**Category**: api_consistency / backwards_compat (CD-5 violation)

**Issue**: For pre-merge artifact (assigned_number: null) reached via slug, all 11 surfaces emit `forgeplan score PRD-074` — but `PRD-074` is unstable, may flip on merge. Exactly the problem CD-5 was meant to prevent. Test `cli_hint_slug_aware.rs` only covers `get/list` — gap.

**Fix**: Each command, after fetching `record`:
```rust
let ref_form = forgeplan_core::artifact::frontmatter::refs_form_from_body(&record.body, &record.id);
// then use ref_form (not id/record.id) in hint actions
```
Extend `cli_hint_slug_aware.rs` with one slug-pre-merge case per command.

---

### CRIT-4 [Code-2]: MCP `forgeplan_get/score/update/review` emit display id in `_next_action`, server-side regression

**Files**:
- `crates/forgeplan-mcp/src/server.rs:1957-1978` (`forgeplan_get`)
- `crates/forgeplan-mcp/src/server.rs:1732-1770` (`forgeplan_score`)
- `crates/forgeplan-mcp/src/server.rs:2136-2141` (`forgeplan_update`)
- `crates/forgeplan-mcp/src/server.rs:2340-2349` (`forgeplan_review`)

**Category**: api_consistency

**Issue**: All four MCP tools use resolved canonical `r.id` in `_next_action` instead of `refs_form_from_body(&r.body, &r.id)`. CLI `get.rs:52-55` uses `refs_form` correctly — cross-surface inconsistency for the same artifact.

**Fix**: Compute `let ref_form = refs_form_from_body(&r.body, &r.id);` once after `get_record`, use `sanitize_for_hint(&ref_form)` everywhere в `_next_action`. Add MCP-side hint tests mirroring CLI pattern.

---

## HIGH findings (9 — strongly recommend fix in same sprint)

### HIGH-1 [Sec-3]: reconcile-ids rename has no symlink check, no canonicalization, missing `--` separator

**File**: `crates/forgeplan-cli/src/commands/reconcile_ids.rs:291-307, 333-345, 549-570, 754`

**CWE**: CWE-22 (path traversal), CWE-88 (argv injection), CWE-367 (TOCTOU)

**Issue**:
1. `read_record` line 295 doesn't call `validate_slug` (companion `ci_assign_id.rs:417` does)
2. `apply_actions` line 754 calls `rename_with_git_fallback` directly — no SEC-6 hardening (symlink_metadata + reject; canonicalize + workspace boundary check)
3. Line 558: `Command::new("git").arg("mv").arg(from).arg(to)` missing `["mv", "--"]` separator (companion ci_assign_id.rs:606 has it)

**Fix**: Mirror `ci_assign_id.rs:752-782` SEC-6 block. Add `validate_slug`. Add `--` separator on git mv. Add path canonicalization assertion before rename.

---

### HIGH-2 [Sec-4]: `assign-id.yml` `git push` interpolates `${{ github.head_ref }}` directly

**File**: `.github/workflows/assign-id.yml:120`

**CWE**: CWE-94 (code injection via Actions context)

**Issue**: Same vector closed via `commit_msg` env var pattern (lines 99-105) but left open for `head_ref`. Branch names allow `$`, backticks, `(`, `)`, `&`, `|`. Fork PR with `evil$(cmd)` triggers RCE с runner's `GITHUB_TOKEN` (write to contents + PRs).

**Fix**: Pass `head_ref` via env var (mirror pattern на line 109) + sanitize: `[[ "$HEAD_REF" =~ ^[A-Za-z0-9._/-]+$ ]] || exit 1`.

---

### HIGH-3 [Sec-5]: MCP DTO slug fields propagate без validation; downstream hints interpolate без sanitize_for_hint

**Files**:
- `crates/forgeplan-mcp/src/convert.rs:40-69` (`identity_from_record` no validation)
- `crates/forgeplan-cli/src/commands/decompose.rs:61-67` (interpolates `refs_form_from_body` без sanitize)
- `crates/forgeplan-core/src/artifact/frontmatter.rs:139-161` (`refs_form` returns slug verbatim)

**CWE**: CWE-117 (output neutralization), CWE-79/74 (improper neutralization downstream), prompt injection

**Issue**: `sanitize_for_hint` (server.rs:305-371) defends against zero-width / bidi / format-character prompt injection в agent-visible strings. New Phase 2 identity triple ships verbatim. Tampered slug `"; rm -rf $HOME #"` flows into CLI hint suggestions.

**Fix**: 
- `convert.rs::identity_from_record` — `validate_slug(&s).is_ok()` check, replace with None on failure
- `frontmatter.rs::refs_form` — return `fallback_id` when `validate_slug(slug).is_err()`
- Apply `sanitize_for_hint` to `ref_form` в decompose/reason hint sites

---

### HIGH-4 [Code-3]: reconcile_ids::write_predicted_number non-atomic — crash mid-write loses data

**File**: `crates/forgeplan-cli/src/commands/reconcile_ids.rs:574-584`

**Issue**: Direct `fs::write(path, rendered)` truncates and writes in place. Crash mid-write leaves empty/partial file. Companion `ci_assign_id.rs:799-812` uses tmp+rename pattern.

**Fix**: Mirror tmp+rename — write to `*.md.tmp` then `fs::rename`.

---

### HIGH-5 [Code-5]: bash validator slug regex omits `mem` prefix, scans memory dir

**File**: `.github/scripts/validate-forgeplan-frontmatter.sh:18, 21, 147`

**Issue**: 
- Line 18 `SLUG_REGEX` omits `mem`
- Line 147 includes `memory/` in scan path
- Rust core (`types.rs:136`) treats memory as first-class kind

False positive: PR adds `mem-architecture-context.md` → bash rejects (regex no match) while Rust accepts.

**Fix**: Add `mem` to SLUG_REGEX. Drop unused `ARTIFACT_KINDS` array. Pull kind list from single source (e.g. `forgeplan list-kinds --json`).

---

### HIGH-6 [Code-6]: import_cmd resolver path silently rewrites kind

**File**: `crates/forgeplan-cli/src/commands/import_cmd.rs:97-104, 125-148, 180-198`

**Issue**: With `--force`, JSON `{ "id": "prd-foo", "kind": "rfc" }` resolves `prd-foo` → `PRD-001` (PRD), then deletes existing PRD-001 + projection, then creates row с `id="PRD-001"` but `kind="rfc"`. Kind/id incoherent. Silent data corruption.

**Fix**: After `resolve_id`, fetch existing record. Bail if `record.kind != art["kind"]`.

---

### HIGH-7 [Code-7]: cli_resolver_wiring tests false confidence

**File**: `crates/forgeplan-cli/tests/cli_resolver_wiring.rs:74-88, etc.`

**Issue**: `assert_no_not_found` greps for literal `"Artifact 'X' not found"`. If resolver is removed, downstream commands fail with different message (e.g. "lifecycle gate failed"). Test still passes — silent regression.

**Fix**: Add positive assertion per test (e.g. `forgeplan get <slug> --json` returns expected `title`). Or assert exit code + structured error code.

---

### HIGH-8 [Sec-9]: `cargo build --release` on PR HEAD remains unmitigated

**File**: `.github/workflows/assign-id.yml:65-66`

**CWE**: CWE-94 (code injection), CWE-829 (untrusted source)

**Issue**: Phase 2.1 productionization (rebuild from `origin/dev`) listed as backlog в SECURITY-PROB-060.md but ships unchanged. RCE via `build.rs` / proc-macro / `Cargo.lock` patch on every `ready-to-merge` label. Runner has `contents: write` + `pull-requests: write`.

**Fix**: 
- Add step `git checkout origin/dev -- crates/ Cargo.toml Cargo.lock .cargo/` before `cargo build --release`
- Hard-enforce SECURITY checklist as CI gate (fail если PR diff touches Cargo.toml/Cargo.lock/build.rs/.cargo/)

---

## MED findings (11)

### MED-1 [Sec-6]: `assigned_number_from_frontmatter` accepts arbitrary `u32` без upper bound
**File**: `crates/forgeplan-core/src/artifact/frontmatter.rs:98-102`
**Fix**: Cap at 1_000_000. Return None on out-of-range.

### MED-2 [Sec-7]: `redact_path` defense-in-depth bypass — basenames leak
**File**: `crates/forgeplan-cli/src/commands/ci_assign_id.rs:64-73, reconcile_ids.rs:126-134`
**Fix**: Canonicalize workspace once. Emit `<redacted>` for outside-workspace paths instead of basename.

### MED-3 [Sec-8]: TOCTOU between `is_inside_git_repo` probe and rename
**File**: `crates/forgeplan-cli/src/commands/ci_assign_id.rs:578-585, 603-628`
**Fix**: Pass `canonical_workspace` into `rename_artifact_file`. Assert `from.canonicalize().starts_with(canonical_workspace)` immediately before spawn.

### MED-4 [Code-8]: reconcile_ids JSON `workspace` field leaks absolute path
**File**: `crates/forgeplan-cli/src/commands/reconcile_ids.rs:851`
**Fix**: Use relative `.forgeplan` или omit.

### MED-5 [Code-9]: dead `dash_pos` variable + redundant length check в body_artifact_refs
**File**: `crates/forgeplan-cli/src/commands/reconcile_ids.rs:385, 401-405, 413-417`
**Fix**: Delete dead code, fix lying comment.

### MED-6 [Code-10]: fgr text vs JSON date parsing inconsistency
**File**: `crates/forgeplan-cli/src/commands/fgr.rs:60-68 vs 135-138`
**Fix**: Extract date-parse helper, use в both branches.

### MED-7 [Code-11]: NewArtifactResponse field shape diverges from sibling DTOs
**File**: `crates/forgeplan-mcp/src/types.rs:117-160 vs 6-39, 41-69, 386-431`
**Issue**: NewArtifactResponse uses `slug: String` + `predicted_number: u32`; siblings use `Option<>`.
**Fix**: Align на `Option<>` с `#[serde(default, skip_serializing_if = "Option::is_none")]`. Document in CHANGELOG.

### MED-8 [Code-12]: `rename_with_git_fallback` inconsistency between commands
**File**: `crates/forgeplan-cli/src/commands/reconcile_ids.rs:551-570 vs ci_assign_id.rs::rename_artifact_file`
**Fix**: Extract shared `_id_helpers.rs` или move to `forgeplan-core::artifact::ids`.

### MED-9 [Code-13]: reconcile_ids missing edge case tests
**File**: `crates/forgeplan-cli/src/commands/reconcile_ids.rs:983-1277`
**Missing tests**:
- Malformed frontmatter on one of N files (continue?)
- Apply mode where rename succeeds but write_predicted fails (partial state)
- Two FilenameMismatch с colliding paths
- render_json with scan_errors populated
- per_kind_count omission verification
- detect_body_links_drift с self-references

### MED-10 [Code-14]: `assert_no_not_found` brittle string matching
**File**: `crates/forgeplan-cli/tests/cli_resolver_wiring.rs:83`
**Fix**: Replace string-grep с structured exit code или tracing subscriber.

### MED-11 [Sec-10]: bash YAML parser fragile against multi-line strings, indented sub-fields
**File**: `.github/scripts/validate-forgeplan-frontmatter.sh:25-36`
**Fix**: Use `yq -r` или `python3 -c yaml.safe_load` for parser parity с Rust runtime.

---

## LOW findings (4)

### LOW-1 [Sec-11]: discover_artifacts swallows IO errors via `.flatten()`
**File**: `crates/forgeplan-cli/src/commands/reconcile_ids.rs:271-287`

### LOW-2 [Code-15]: Dead variable + lying comment в body_artifact_refs
**File**: `crates/forgeplan-cli/src/commands/reconcile_ids.rs:413-417`

### LOW-3 [Code-16]: Redundant branch в validate-frontmatter.sh
**File**: `.github/scripts/validate-forgeplan-frontmatter.sh:71-78`

### LOW-4 [Code-17]: import_cmd jargon-laden comment
**File**: `crates/forgeplan-cli/src/commands/import_cmd.rs:213-215`

---

## Closure status

| Category | Count | Status (after autorun fixer round) |
|---|---|---|
| CRITICAL | 4 | TARGETED FOR FIX in autorun |
| HIGH | 9 | DEFERRED TO NEXT SPRINT (documented здесь, not autorun-fixed) |
| MED | 11 | DEFERRED — log в CHANGELOG / PROB- as appropriate |
| LOW | 4 | DEFERRED — minor hygiene, batch fix later |

**Phase 2 ship gate** (post fix): CRITICAL fixed + pipeline gate green.

**Pre-merge checklist for user**:
- [ ] CRITICAL fixes verified (auditor re-runs OR positive test for each fix)
- [ ] HIGH-1..HIGH-9 triaged: file as separate PROBs, или batch fix sprint
- [ ] MED + LOW logged for batch hygiene cleanup
- [ ] User explicit approval before `git push`
