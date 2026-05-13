---
depth: tactical
id: EVID-120
kind: evidence
links:
- target: PROB-066
  relation: informs
- target: PROB-051
  relation: informs
status: active
title: PROB-066 — centralized agent identity validation across CLI + core
---

# EVID-120: PROB-066 — centralized agent identity validation across CLI + core

## Summary

PROB-066 (CLI `forgeplan claim --agent <STR>` lacks identity-string validation that MCP path enforces) closed via two-tier helper в `forgeplan-core::claim`. Strict `validate_agent_id` gates CLI accept path с full character-class rejection. Relaxed `validate_agent_id_relaxed` gates core store, preserves canonical MCP `name/version` shape (`claude-code/1.0.50`).

Implemented в commit `7297583` on `fix/prob-066-claim-validate` (merged at `096f9f1`).

## Method

**Code changes** (6 files, +446/-18 LOC):
- `crates/forgeplan-core/src/claim/mod.rs` — new `validate_agent_id()` + `validate_agent_id_relaxed()`
- `crates/forgeplan-core/src/artifact/identity.rs` — `is_identity_char_forbidden` + `MAX_FIELD_LEN` bumped to `pub(crate)` для reuse
- `crates/forgeplan-cli/src/commands/claim.rs` — strict variant wired at CLI accept site с Fix: hint
- `crates/forgeplan-cli/tests/cli_claim_security.rs` (NEW) — 5 integration regression tests
- `crates/forgeplan-cli/tests/cli_hint_slug_aware.rs` — 6 agent strings migrated к hyphen form
- `scripts/smoke-test.sh` — `smoke-test/v1` → `smoke-test-v1`

**Tests added**:
- 9 unit tests в `claim/mod.rs` (positive control, slash, control chars, bidi/ZWJ/BOM/TAG-A, length cap, empty/whitespace, store-layer defense-in-depth newline reject, MCP slash-form preservation, release symmetry)
- 5 integration tests в `cli_claim_security.rs` — real binary exec across slash / newline / bidi / overlong + accept-hyphenated round-trip

## Findings

1. **Two-tier helper required**: simple "reject `/` everywhere" would break MCP path (passes `AgentIdentity::as_frontmatter_value()` returning `claude-code/1.0.50`). Resolution: strict at CLI accept, relaxed at store layer.
2. **Defense-in-depth**: validation enforced на BOTH `ClaimStore::claim()` и `ClaimStore::release()` — symmetric.
3. **Length cap added**: `MAX_AGENT_LEN = 64` (matches `MAX_FIELD_LEN` from identity.rs).

## Pipeline gate

| Gate | Result |
|---|---|
| `cargo fmt --check` | 0 diff |
| `cargo check --workspace` | 0 warnings |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings |
| `cargo test --lib -p forgeplan-core` | 1647 PASS |
| `cargo test -p forgeplan` | all integration PASS |
| `cargo test -p forgeplan-mcp` | 180 PASS |
| `bash scripts/smoke-test.sh` | 17 ops PASS |

## Note on evidence recreation

Original evidence (created by w5-prob-066-fix worker) был EVID-119, но collided с w5-prob-065-fix's EVID-119 due to forgeplan_new race condition (filed as PROB-067 — discovered during sprint). Original EVID-119 untracked, lost during cleanup. **This EVID-120 is the canonical evidence document** для PROB-066 fix; references commit SHA `7297583` and merge commit `096f9f1`.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Linked artifacts

- PROB-066 (fix verified)
- PROB-051 (parent family — w4-security-audit deferrals)
- PROB-064 (cross-surface contract guard family)
- PROB-067 (forgeplan_new race collision — root cause for EVID-119 originally being lost)




