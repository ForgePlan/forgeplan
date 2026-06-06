---
depth: tactical
id: EVID-145
kind: evidence
links:
- target: PROB-070
  relation: informs
status: draft
title: 'PROB-070 closure: SEC-001/004/005 + ARCH-003 verified done; SEC-003 Windows deferred per v0.33 plan scope-cut'
---

## Summary

Closure verification for **PROB-070** (v0.33 deferred audit findings — Wave 9 leftovers). All findings are resolved except SEC-003 (Windows), which is explicitly deferred by the v0.33 plan's Windows scope-cut.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: audit

CL3: direct verification of the shipped workflows + tests on `origin/dev`.

## Findings status

| Finding | What | Status |
|---|---|---|
| SEC-001 / CR-001 / CR-002 + 8 inline | Wave-9 reviewer closures | ✅ landed (`b5a21bf`, in dev + main) |
| **SEC-004** — CI gate "theatre" (`continue-on-error`) | `pull_request:` trigger dropped; only `continue-on-error` mention is a guard COMMENT ("do NOT add"). | ✅ done (`c525acd` FR-025, ADR-013 honest-signal) |
| **SEC-005** — mutable `@vN` action tags | ALL `uses:` across all 6 workflows pinned to 40-char SHA (checkout, upload/download-artifact, rust-cache, rust-toolchain, setup-protoc, sccache, cargo-deny, github-script, install-action). Zero `@vN`/`@main`/`@master`. | ✅ done |
| **ARCH-003** — `partial_verdict` contract test | `crates/forgeplan-core/tests/verdict_boundary_test.rs` — 23 tests passing. | ✅ done |
| **SEC-003** — Windows path sanitizer | Sanitizer hardcodes Unix prefixes; Windows `%USERPROFILE%`/`%TEMP%` unmasked. | ⏳ **DEFERRED** — v0.33 plan scope-cut ("Windows как первоклассная платформа — не делаем; Unix-first"). No Windows users today. |

## Verification (origin/dev)

- ARCH-003: `verdict_boundary_test` — **23 passed / 0 failed**.
- SEC-004: `grep continue-on-error:\s*true` across `.github/workflows/` → empty (only the guard comment).
- SEC-005: `grep uses:.*@(vN|main|master)` → empty (all SHA-pinned).
- SEC-001/CR: commit `b5a21bf` present in `origin/dev` and `origin/main`.

## Decision

PROB-070 closed as **resolved** (SEC-001/CR-*, SEC-004, SEC-005, ARCH-003) with **SEC-003 deferred** to a future Windows-support sprint (v0.34+), matching the v0.33 plan's explicit Unix-first scope-cut. SEC-003 carries no immediate impact (no Windows users); the existing `$HOME`/Unix sanitizer is correct for the supported platforms.

## Provenance

- Verified on `origin/dev` @ post-#370 merge.
- Workflows: `.github/workflows/security.yml` (SEC-004 comment + SHA pins), all 6 workflow files (SEC-005).
- Test: `crates/forgeplan-core/tests/verdict_boundary_test.rs` (ARCH-003).


