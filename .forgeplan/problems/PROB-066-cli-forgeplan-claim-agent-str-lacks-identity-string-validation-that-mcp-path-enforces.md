---
depth: tactical
id: PROB-066
kind: problem
links:
- target: PROB-051
  relation: informs
- target: PROB-064
  relation: informs
status: draft
title: CLI forgeplan claim --agent <STR> lacks identity-string validation that MCP path enforces
---

# PROB-066 — CLI `forgeplan claim --agent <STR>` lacks the identity-string validation that MCP path enforces

## Signal

W4 adversarial security audit (Wave 4A, v0.31.0 sprint, 2026-05-11) found that the CLI `forgeplan claim --agent <STR>` command accepts arbitrary user-supplied strings as the agent identifier with **only an `is_empty()` check**. The matching MCP-side identity path (`crates/forgeplan-core/src/artifact/identity.rs::AgentIdentity::new`) rejects strings containing `/`, control characters, bidi-override codepoints, ZWJ, and other invisible / spoof-class characters — that defence was added to close R2 audit MED. The CLI path was never updated to match. The asymmetry is proven by the smoke-test invocation `forgeplan claim PRD-001 --agent "smoke-test/v1"`: the literal `/` would be rejected on the MCP side via `AgentIdentity::new`, but is silently accepted by the CLI and persisted to `.forgeplan/claims/<ID>.yaml::agent_id`.

The downstream display surfaces (`println!("Claimed {} for {}", claim.id, claim.agent_id)` at `claim.rs:103` and `eprintln!("Claim for {id} already held by {agent_id} (expires {expires_at})")` at `claim.rs:143`) emit the agent string verbatim. A malicious or careless agent value containing ANSI escape sequences, newlines, or bidi overrides reaches the operator's terminal with no neutralization.

## Context

- **Workspace state at detection**: `chore/v031-dependabot-bump` branch, dev-based sprint; smoke-test.sh T15 (claim/release cycle, added in commit 4b59341 of W3 coverage extension) uses the forbidden form `--agent "smoke-test/v1"` as its canonical example, enshrining the asymmetry in the test contract.
- **Reproducibility**: 100% deterministic on any workspace.
  ```
  forgeplan init -y
  forgeplan new prd t
  forgeplan claim PRD-001 --agent $'evil\nx: y' --ttl-minutes 1
  cat .forgeplan/claims/PRD-001.yaml  # observe multiline agent_id (YAML parse may break)
  forgeplan claims                     # observe agent string echoed unescaped
  ```
- **Code paths**:
  - **CLI accept site (vulnerable)**: `crates/forgeplan-cli/src/commands/claim.rs:60-63` — `let agent_str = match agent.map(str::trim).filter(|a| !a.is_empty()) { ... }` — only an emptiness filter.
  - **Store accept site (vulnerable)**: `crates/forgeplan-core/src/claim/mod.rs:244` — `if agent.is_empty() { return Err(ClaimError::EmptyAgent); }` — same single check.
  - **Reference: hardened MCP path**: `crates/forgeplan-core/src/artifact/identity.rs:26-49::is_identity_char_forbidden` — full character-class rejection (controls, bidi, ZWJ, format chars, path separators including `/` and `\`, NUL).
  - **Display sites (leak surface)**: `crates/forgeplan-cli/src/commands/claim.rs:103,128,143` (println/eprintln/Hint::warning body); `crates/forgeplan-cli/src/commands/claims.rs` (listing).
  - **Test enshrining bug**: `scripts/smoke-test.sh:317,320,332` (`--agent "smoke-test/v1"`).

## Root cause

The Claim subsystem (PRD-057 multi-agent dispatcher) and the artifact identity stamping subsystem (PRD-057 FR-009, R2 audit MED closure) were authored as adjacent but distinct surfaces. `AgentIdentity::new` was hardened for the MCP write-stamping path. `ClaimStore::claim` was hardened for filesystem safety on the `id` parameter (`validate_id` at `claim/mod.rs:112` correctly rejects path traversal there), but the `agent` parameter received only the trivial empty-string guard and no character-class filter. Both inputs land on disk — `id` via filename construction, `agent` via the YAML body — so the threat models for the two parameters are nearly identical, but only one was closed.

The smoke-test author chose `"smoke-test/v1"` because it visually resembles the MCP `clientInfo.name/version` shape (`"claude-code/1.0.50"`) — a reasonable mental model from the operator side, but exactly the form `AgentIdentity::new` rejects on the MCP side. Acceptance into the CLI path leaks the asymmetry into a documented smoke contract.

## Why now

Discovered during W4 adversarial security audit of v0.31.0 sprint accumulated changes (`dev..chore/v031-dependabot-bump`). The W3 coverage worker added the smoke test using the forbidden agent form; W4 cross-referenced it against `AgentIdentity::new` rules and surfaced the mismatch as finding F-7 / MED-3. No prior PROB / ADR captured the identity-validation parity between MCP and CLI claim surfaces — the audit chain assumed `AgentIdentity::new` was the single guard, missing that `forgeplan claim` is a parallel write path that bypasses it.

## Decision — proposed fix (Option A is the recommended path)

### Option A (recommended) — centralise agent-string validation in `forgeplan-core::claim`

1. Extract a `validate_agent_id(agent: &str) -> Result<(), ClaimError>` helper in `crates/forgeplan-core/src/claim/mod.rs`. Reject:
   - Empty / whitespace-only (already covered, keep)
   - Length > 64 bytes (mirroring `MAX_FIELD_LEN` in `identity.rs:18`)
   - Any character matching `is_identity_char_forbidden` (control chars, bidi overrides, ZWJ, BOM, variation selectors, tag chars)
   - Newlines (`\n`, `\r`) — explicitly named in the reject list because of YAML-injection risk
2. Call `validate_agent_id` from both `ClaimStore::claim` (line ~244) and `ClaimStore::release` (the symmetric write path that currently has the same emptiness-only check at line 293).
3. Decide explicitly on `/`: the smoke-test uses `smoke-test/v1` as a familiar shape; if `/` is desired for path-like agent ids, allow it but document explicitly in the docstring. If parity with `AgentIdentity::new` is preferred, reject `/` and rename the smoke-test agent to `smoke-test-v1`. Recommendation: **reject `/`** — parity with MCP identity rules is the simpler invariant; smoke-test rename is trivial.
4. Add `ClaimError::InvalidAgent(String)` variant mirroring `InvalidId(String)`.
5. Update `crates/forgeplan-cli/src/commands/claim.rs:60-63` to surface `ClaimError::InvalidAgent` as a typed CLI error with a `Fix:` hint (`Fix: rename agent to alphanumeric + dash, e.g. smoke-test-v1`).
6. Update `scripts/smoke-test.sh:317,320,332` to use `smoke-test-v1` (no slash).

### Option B (lighter — accept but neutralise on display)

Keep the CLI accept-all behaviour. Sanitize on **display** only: pipe `claim.agent_id` through `forgeplan_core::artifact::sanitize::sanitize_for_hint` (or a relaxed variant that keeps `/` but strips controls/bidi/ANSI) before every `println!`/`eprintln!`/hint emission. Does not close the YAML-injection vector (agent_id with newline still corrupts the on-disk file); only protects terminal output. Insufficient on its own.

Option A is strictly better — it closes accept-time, write-time, and display-time vectors with a single helper. Option B is the fallback if Option A breaks an unknown agent-naming convention discovered during fix work.

## Acceptance criteria

1. **A-1** — `ClaimStore::claim` and `ClaimStore::release` both reject agent strings containing controls, bidi overrides, ZWJ, format chars, BOM, variation selectors, newlines, NUL — return typed `ClaimError::InvalidAgent`.
2. **A-2** — Length cap of 64 bytes enforced (mirroring `MAX_FIELD_LEN` in `identity.rs`).
3. **A-3** — Regression test `claim_rejects_agent_with_control_chars` covers `\n`, `\r`, `\u{0007}`, `\u{202E}` (RLO), `\u{200B}` (ZWSP). Test must hit both `claim()` and `release()`.
4. **A-4** — `scripts/smoke-test.sh` agent string renamed from `smoke-test/v1` → `smoke-test-v1` and the test passes against the hardened validator.
5. **A-5** — CLI error path emits `Fix:` hint with a concrete remediation example (alphanumeric + dash form).
6. **A-6** — Existing `claim_rejects_empty_ids_and_agents` test (`claim/mod.rs:551`) extended to cover the new validator (or kept and a new sibling test added).

## Linked artifacts

- **Informs**: PROB-051 (CLI/MCP parity class), PROB-064 (dual-key emission asymmetry — same cross-surface class), PRD-057 (multi-agent dispatcher), R2 audit MED on identity propagation (already-closed via `AgentIdentity::new`)
- **Related**: ADR / RFC for PRD-057 claim subsystem; `crates/forgeplan-core/src/artifact/sanitize.rs` (sibling defence helper for hint strings)
- **Discovered by**: Wave 4A security audit, v0.31.0 sprint (2026-05-11), finding F-7 / MED-3

## References

- `crates/forgeplan-cli/src/commands/claim.rs:60-63` (CLI agent accept site)
- `crates/forgeplan-core/src/claim/mod.rs:244` (claim store accept site)
- `crates/forgeplan-core/src/claim/mod.rs:293` (release store accept site — symmetric)
- `crates/forgeplan-core/src/artifact/identity.rs:18-49` (canonical identity char-class rules — the reference implementation)
- `crates/forgeplan-cli/src/commands/claim.rs:103,128,143` (display sites that emit agent_id verbatim)
- `scripts/smoke-test.sh:317,320,332` (`--agent "smoke-test/v1"` enshrined form, to rename)
- W4 audit report: MED-3 (this PROB), paired with HIGH-1 (tag injection) and MED-1 (MCP dual-key); see sprint-v031-cleanup teammate log



