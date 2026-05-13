---
depth: standard
id: PROB-070
kind: problem
last_modified_at: 2026-05-12T19:54:53.238072+00:00
last_modified_by: claude-code/2.1.139
links:
- target: PROB-051
  relation: based_on
status: active
title: v0.33.0 deferred audit findings — Wave 9 closure leftovers (Windows sanitizer, CI policy, supply-chain hygiene)
---

# PROB-070: v0.33.0 deferred audit findings — Wave 9 closure leftovers

## Signal

Adversarial 2-auditor review of Wave 9 integration (`feat/v032-w9-integration`) found 19 issues. 11 closed inline в audit-fix commit `b5a21bf`; 8 deferred here for v0.33.0 sprint.

Each item has explicit severity, file:line citation, reproduction steps, and recommended fix. Categorised by class.

## Deferred items

### SEC-003 (MED) — Windows / non-Unix path sanitization bypass

**File**: `crates/forgeplan-core/src/projection/error.rs::sanitize_text_with` (lines ~460-495)

Sanitizer hardcodes Unix prefixes (`/Users/`, `/home/`, `/tmp/`, `/var/folders/`) and reads `$HOME`. On Windows the equivalents are `%USERPROFILE%`, `%TEMP%/%TMP%`, `$Env:LOCALAPPDATA` — none are masked. Cargo.toml does NOT gate compilation by `cfg(unix)`, so a Windows port silently regresses path leakage.

**Fix**: Add `#[cfg(windows)]` branch that reads `USERPROFILE` + `TEMP` env vars and applies same anchored mask. Or `#[cfg(not(unix))] compile_error!(...)` if Windows is explicitly unsupported.

**Why deferred**: Forgeplan is currently Unix-targeted per `docs/ROADMAP.md`; no Windows users today. Forward-looking, no immediate impact.

---

### SEC-004 (MED) — `pull_request:` trigger is theatre with `continue-on-error: true`

**File**: `.github/workflows/security.yml:19-22, 35`

W4 re-enabled `pull_request:` trigger but job still has `continue-on-error: true` (line 35). Effect: PR check always shows green ✔ even when `cargo-deny` reports CRITICAL CVEs. False sense of security.

**Fix options**: (a) remove `continue-on-error: true` and make gate blocking; (b) revert PR trigger so workflow only fires on push + cron; (c) add separate comment-posting step that summarizes findings inline regardless of exit code.

**Why deferred**: Policy decision needed — blocking PR merges on transient Dependabot alerts is friction; advisory-only without inline comment is invisible. User input required before fix.

---

### SEC-005 (MED) — Third-party action pinned by mutable `@v2` tag

**File**: `.github/workflows/security.yml:38` (`EmbarkStudios/cargo-deny-action@v2`) + likely `actions/checkout@v4` org-wide

Supply-chain hygiene gap. Mutable tag can be reassigned by upstream maintainer (or attacker post-compromise) without notice. Same applies to `actions/checkout@v4` and others.

**Fix**: Pin all `uses:` to full 40-char commit SHA. Adopt Dependabot for action SHA rotation.

**Why deferred**: Workspace-wide audit needed (probably 10+ workflow files); separate PR scope.

---

### ARCH-003 (MED) — `partial_verdict` rustdoc promises external use but field not in JSON

**File**: `crates/forgeplan-core/src/health/mod.rs:331-345`

`HealthReport::partial_verdict` rustdoc says external library consumers MUST consume it for own verdict recomputation. But neither CLI JSON nor MCP JSON serialize this field — only Rust API callers see it.

**Fix options**: (a) add `"partial_verdict": report.partial_verdict.as_str()` to both CLI + MCP JSON payloads; (b) demote rustdoc claim and mark Rust-API-only.

**Why deferred**: Decision needed about JSON contract surface stability vs library-only API.

---

### TST-002 (LOW) — `health_help_test` coupling fragile

**File**: `crates/forgeplan-cli/tests/health_help_test.rs:39-67`

Test asserts `--help` contains "verdict". The word currently appears ONLY in `--strict` flag's long description. Future refactor renaming `--strict` desc breaks the test, but the contract (operator can discover verdict from --help) is independent of `--strict`.

**Fix**: Extend assertion to pin top-level subcommand description ALSO mentioning "verdict" so contract has two places.

**Why deferred**: Low severity; test is correct guard for current state.

---

### TST-003 (MED) — Single-point bench cannot catch O(N²) regression

**File**: `crates/forgeplan-core/tests/health_bench.rs:36-37`

Bench seeds 1000 artifacts. `find_duplicate_pairs` is documented as O(N²). A future change that adds O(log N) HashMap lookup per pair wouldn't show on the 1000-point bench. Need ≥2 data points to validate scaling.

**Fix**: Add `#[ignore]` bench `bench_health_report_with_phase_warm_latency_5000` (or parametrize inner loop).

**Why deferred**: Perf scope separate; bench infrastructure work.

---

### DOC-003 (LOW) — `strict_exit_code` rustdoc enumeration incomplete

**File**: `crates/forgeplan-cli/src/commands/health.rs:23-33`

Rustdoc enumerates "orphans / blind_spots / active_stubs / at_risk > 0" as direct critical signals. Omits `possible_duplicates` and `stale_count` which contribute via verdict promotion.

**Fix**: Either explicitly mention "duplicates and stale contribute via verdict promotion, not direct count check" OR extend direct count check to include them (defense-in-depth).

**Why deferred**: Cosmetic doc clarity; behavior is correct.

---

### LOG-003 (LOW) — Silent error swallow in parallel phase reader

**File**: `crates/forgeplan-core/src/health/mod.rs:507-510`

`buffer_unordered(16)` reader does `.ok().flatten()` on `read_phase` Err. No `tracing::warn!` for corrupted YAML, IO errors, schema-version-too-new. A workspace under attack (state files replaced with symlinks) would silently drop those artifacts from verdict — `phase_mismatches.len()` undercounts.

**Fix**: Replace `.ok().flatten()` with explicit match emitting `tracing::warn!(artifact = %id, err = %e, ...)`. Optionally thread error count into `HealthReport.phase_read_errors: usize`.

**Why deferred**: Logging improvement; threat model assumes filesystem integrity at workspace level. Forensic improvement, not exploit closure.

## Acceptance criteria for full closure

Each item closes with:
- [ ] Code change OR explicit accept-with-justification
- [ ] Test or regression guard (where applicable)
- [ ] CHANGELOG entry
- [ ] Linked EVID per closure batch

## Reversibility

Each item is reversible — additive code or isolated config change.

## Linked artifacts

- based_on PROB-051 (Wave-1 Round 5 audit family — same continuation)
- based_on EVID-122 (Wave 9 closure evidence — captures these as deferred)

## References

- Audit Wave 9 — security-expert + code-reviewer combined report
- Reviewers' recommendations: SEC-001/CR-001/CR-002 + 8 inline closures landed in commit `b5a21bf`



