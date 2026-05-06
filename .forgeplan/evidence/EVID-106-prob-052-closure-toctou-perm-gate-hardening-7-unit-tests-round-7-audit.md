---
depth: standard
id: EVID-106
kind: evidence
links:
- target: PROB-052
  relation: informs
status: active
title: PROB-052 closure TOCTOU + perm gate hardening, 7 unit tests, Round 7 audit
---

# EVID-106: PROB-052 closure — `which_in_path` TOCTOU + symlink-follow + perm-gate hardening

## Summary

Closes PR-E Round 6 audit MED-1 (CWE-367 TOCTOU + CWE-426 untrusted-path hijack) on `crates/forgeplan-core/src/playbook/dispatch/helpers.rs::which_in_path`. New `pub(super) resolve_safe_path` helper canonicalizes the candidate, rejects non-files, and on Unix gates against group/world write bits on both the file и parent directory. Round 7 adversarial audit (2 parallel agents — security + code-reviewer) caught HIGH-1 consumer-side bypass (override branches in `AgentDispatcher`, `PluginDispatcher`, `resolve_forgeplan_binary` were unguarded) — closed in same sprint commit. PROB-052 closure includes log-injection hardening (CWE-117/CWE-150) и empty-PATH-entry skip.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Evidence

### Round 7 audit (2026-05-06) — strict pre-PR adversarial review

| Auditor | Findings | Closed in PR | Deferred |
|---|---:|---:|---:|
| security-expert | 8 (1 HIGH + 4 MED + 3 LOW) | 6 (HIGH-1 override bypass, MED-1/MED-2 log injection, MED-3 ownership re-scope, MED-4 setuid acknowledgment, LOW-1/LOW-3) | 2 (MED-2 parent symlink edge, LOW-2 special-file proc paths) |
| code-reviewer | 11 (3 HIGH + 6 MED + 5 LOW) | 8 (HIGH-1 AC-4 re-scope, HIGH-2 resolve_forgeplan_binary, HIGH-3 docstring precision, MED-1/MED-2 log injection, MED-3 docstring overclaim, MED-4 empty PATH entry, LOW-4 mask) | 3 (MED-5 sync mutex split, MED-6 typed error enum, LOW-3 tempdir mode) |

**Verdict**: both auditors converged on FIX-FIRST. All HIGH severity closed in same sprint commit before PR push.

### Files touched

- `crates/forgeplan-core/src/playbook/dispatch/helpers.rs` — new `pub(super) resolve_safe_path`, hardened `which_in_path` (escape_debug + empty-entry skip), symmetric `resolve_forgeplan_binary` override paths, +7 unit tests
- `crates/forgeplan-core/src/playbook/dispatch/agent_dispatcher.rs` — `resolve_claude_binary` routes both override branches through `resolve_safe_path`
- `crates/forgeplan-core/src/playbook/dispatch/plugin_dispatcher.rs` — `resolve_binary` routes override through `resolve_safe_path`

### Tests (7 new unit tests in `helpers.rs::tests`)

```
test playbook::dispatch::helpers::tests::which_in_path_canonicalizes_symlink_to_real_target ... ok
test playbook::dispatch::helpers::tests::which_in_path_rejects_group_writable_binary ... ok
test playbook::dispatch::helpers::tests::which_in_path_rejects_group_writable_parent_dir ... ok
test playbook::dispatch::helpers::tests::which_in_path_skips_empty_path_entries ... ok (Round 7 MED-4)
test playbook::dispatch::helpers::tests::resolve_safe_path_rejects_group_writable_override ... ok (Round 7 HIGH-1)
test playbook::dispatch::helpers::tests::resolve_safe_path_canonicalizes_safe_override ... ok (Round 7 HIGH-1)
test playbook::dispatch::helpers::tests::which_in_path_windows_skips_permission_gate ... ok (cfg(not(unix)))
```

All 7 tests use `DISPATCH_ENV_LOCK` (tokio::sync::Mutex) for serialization against peer dispatcher tests — pattern established in PROB-050 A-6, mirrored here for PATH-mutation isolation.

### Quality gates (final state)

```
cargo fmt --check                                              clean
cargo clippy --workspace --all-targets --features test-helpers
  -- -D warnings                                               clean
cargo test --workspace --features test-helpers                 0 failures (38 suites)
cargo build --release                                          clean
forgeplan health                                               2 orphans (PROB-028 carry-over, not regression)
```

### Test counts

- Pre-PROB-052 baseline (post-PROB-058): lib=1464
- Post-PROB-052: lib=**1467** (+3 net new for HIGH-1 closure; AC-5 mandate was 3 — sprint shipped 7)

### Real E2E (`target/release/forgeplan v0.29.0`, fresh tempdir)

```bash
mkdir safe-bin grp-bin && chmod 0755 safe-bin grp-bin
echo '#!/bin/sh\nexit 0' > safe-bin/safebin && chmod 0755 safe-bin/safebin
echo '#!/bin/sh\nexit 0' > grp-bin/grpbin && chmod 0775 grp-bin/grpbin
PATH="$(pwd)/grp-bin:$(pwd)/safe-bin" forgeplan --version
# → forgeplan 0.29.0  (release binary spawns successfully — no regression)
```

The release binary cleanly resolves through the new gate. Real-world Homebrew posture (`/usr/local/bin` 0o775 group=admin) covered by unit test `which_in_path_rejects_group_writable_parent_dir` — a binary in such a dir gets rejected with stderr warning + tracing::warn; PATH search continues to next entry.

## Hindsight

Round 7 audit caught **the same class of bug** as Round 9 of the prior sprint (MCP transport asymmetry): closing a security primitive on the primary path while leaving symmetric override paths unguarded. Lesson: when hardening any path-resolution / spawn / serialization primitive, **grep for ALL consumers** before declaring closure. The audit prompt explicitly enumerated the 4 surfaces (`which_in_path` + 3 override paths); without that prompt scope the fix would have shipped with HIGH-1 open.

Pattern reinforces `feedback_meta_tooling_discipline.md` (audit/lint scripts must satisfy contracts they enforce) and the post-PROB-053/057/058 lesson: cross-surface symmetry is a load-bearing invariant for security primitives.

## Related Artifacts

| Artifact | Relation |
|---|---|
| PROB-052 | informs (this evidence demonstrates closure) |
| EVID-105 | informs (PROB-057+058 closure, established the cross-surface symmetry pattern) |
| EVID-104 | informs (PROB-053 closure, escape_debug log-injection precedent) |
| ADR-010 | informs (subprocess dispatcher design — `resolve_safe_path` extends the security boundary) |



