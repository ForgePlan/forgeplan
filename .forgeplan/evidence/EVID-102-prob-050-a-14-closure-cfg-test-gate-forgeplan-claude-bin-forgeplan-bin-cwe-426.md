---
depth: tactical
id: EVID-102
kind: evidence
links:
- target: PROB-050
  relation: informs
status: active
title: PROB-050 A-14 closure — cfg(test) gate FORGEPLAN_CLAUDE_BIN/FORGEPLAN_BIN CWE-426
---

# EVID-102: PROB-050 A-14 closure — cfg(test) gate FORGEPLAN_CLAUDE_BIN/FORGEPLAN_BIN CWE-426

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-05-04 |
| Valid Until | 2027-05-04 |
| Target | PROB-050 A-14 (CWE-426 binary substitution closure verification) |

## Structured Fields

evidence_type: test
verdict: supports
congruence_level: 3

## Measurement

End-to-end closure verification of PROB-050 A-14 (cfg-gate
`FORGEPLAN_CLAUDE_BIN`) on branch `fix/prob-050-a14-cfg-gate-claude-bin`.

Three measurement axes:

### Axis 1 — code change correctness

`crates/forgeplan-core/src/playbook/dispatch/agent_dispatcher.rs`:
- `resolve_claude_binary` now wraps `if let Ok(_) = std::env::var("FORGEPLAN_CLAUDE_BIN")` in `#[cfg(test)]`
- Module-level invariants (lines 36-43), struct field doc (lines 90-98), method doc (lines 139-147) updated to document that the env override is honoured ONLY in test builds; release builds silently ignore it.
- Invariant comment (line 145) added: «removing or widening this `#[cfg(test)]` re-opens CWE-426 in release builds».
- New positive test `resolve_claude_binary_honours_env_override_in_test_builds` (line ~593) pins the cfg-gated branch reachability against silent regression.

`crates/forgeplan-core/src/playbook/dispatch/helpers.rs` (symmetric Round-1 audit fix, security HIGH-1):
- `resolve_forgeplan_binary` now wraps `if let Ok(_) = std::env::var("FORGEPLAN_BIN")` in `#[cfg(test)]`
- Symmetric invariant comment added (line 274).
- Currently latent (no production caller of `resolve_forgeplan_binary`), but symmetric pattern established for future Phase 7+ wiring.

### Axis 2 — quality gates (compile-time)

```
cargo fmt --check                                    → exit 0
cargo clippy --workspace --all-targets \
   --features test-helpers -- -D warnings            → exit 0, 0 warnings
cargo test --workspace --features test-helpers       → 1941 passed, 0 failed
                                                       (was 1940 baseline; +1 = new
                                                        positive test for cfg-gate)
cargo test -p forgeplan-core --lib \
   playbook::dispatch::agent_dispatcher              → 14 passed, 0 failed
                                                       (was 13; +1 = new positive test)
cargo test -p forgeplan-core --lib \
   playbook::dispatch::helpers                       → 10 passed, 0 failed (unchanged)
```

### Axis 3 — adversarial review

Round 1 — 2 parallel agents on full diff (security-expert + architect-review):
- security HIGH-1: `helpers::resolve_forgeplan_binary` ungated `$FORGEPLAN_BIN` — same CWE-426 shape, latent. **Closed in-flight** by symmetric `#[cfg(test)]` gate.
- architect H1-H4: CHANGELOG entry + PROB-050 A-14 checkbox + F-RUNTIME-7 closure note + EVID-097 forward-reference. **All closed in-flight** in same PR.
- security MED-1 / architect M4: cfg-gated branch had 0 test coverage. **Closed** by new positive test pinning the gate.
- Other MEDIUM/LOW (4-6 items): pre-existing asymmetries, not introduced by this PR; deferred to PROB-050 A-31/A-32 follow-ups.

Round 2 — 1 security-expert agent on the Round 1 fixes:
- 0 NEW security findings.
- Round 1 HIGH-1 + MED-1 confirmed CLOSED.
- 2 LOW observations: LOW-2 (1-line grep hint near cfg-gate) **applied inline**; LOW-1 (helpers test could take ENV_GUARD lock) **deferred** to PROB-050 (latent — no peer test today).

## Result

| Aspect | Before | After |
|---|---|---|
| `AgentDispatcher::resolve_claude_binary` reads `$FORGEPLAN_CLAUDE_BIN` in **release** | YES — CWE-426 vector | NO — gated `#[cfg(test)]` |
| `AgentDispatcher::resolve_claude_binary` reads `$FORGEPLAN_CLAUDE_BIN` in **test** | YES | YES (preserved) |
| `helpers::resolve_forgeplan_binary` reads `$FORGEPLAN_BIN` in **release** | YES — latent CWE-426 vector | NO — gated `#[cfg(test)]` |
| `PluginDispatcher` reads either env var | NO (always) | NO (unchanged) |
| **Release-build parity Agent ↔ Plugin** | broken (Agent honoured, Plugin not) | **restored** — both ignore env vars |
| Test for cfg-gated branch reachability | 0 | 1 (`resolve_claude_binary_honours_env_override_in_test_builds`) |
| Lib + integration tests | 1940 pass | 1941 pass (+1 new) |
| `cargo clippy -D warnings` | 0 warnings | 0 warnings |
| Adversarial findings on this diff | n/a | 2 rounds, all HIGH/CRITICAL closed inline |

CWE-426 (uncontrolled search path / binary substitution) is closed in
release builds for both env vars in this module; the symmetric test-only
hook remains for fixture wiring without exposing a production attack
surface.

## Interpretation

PROB-050 A-14 acceptance criterion («gate `FORGEPLAN_CLAUDE_BIN` env
override behind `#[cfg(test)]`, restore parity with PluginDispatcher in
release builds, **REQUIRED** per audit S-2 escalation») is fully
satisfied:

1. Cfg-gate applied with the prescribed `#[cfg(test)]` shape.
2. Release-build parity with PluginDispatcher achieved (both ignore env
   var unconditionally; Plugin-side behaviour unchanged).
3. Symmetric vulnerability in `helpers::resolve_forgeplan_binary`
   (`FORGEPLAN_BIN`) closed pre-emptively under the same pattern,
   eliminating a latent reincarnation if Phase 7+ promotes the
   ForgeplanCoreDispatcher to subprocess invocation.
4. Positive test pins the test-build half of the contract; release-build
   half is enforced compile-time by `#[cfg(test)]` itself (unfalsifiable
   from `cargo test` — invariant grep-discoverable via the inline
   PROB-050 A-14 hint comment).
5. CHANGELOG `[Unreleased]` documents the fix under `### Security` per
   `docs/methodology/release-workflow.md` §2.
6. PROB-050 markdown checkbox flipped `[x]`; original wording preserved
   for traceability.
7. F-RUNTIME-7 row in `docs/operations/phase-b-real-e2e-2026-05-03.md`
   marked CLOSED with cross-reference.
8. EVID-097 historical claim left intact; forward-reference note added so
   the next reader is not misled by the v0.27.0/v0.28.0-state evidence
   when running against post-v0.29.0 binaries.

Decision (PROB-050 A-14 closure): activate this evidence, tick `[x]` on
A-14 in PROB-050 (already done in same PR), open PR-B against `dev`.

## Congruence Level Justification

CL3 (same-context, penalty 0.0).

The measurement is taken from the **identical** codebase artifact that
PROB-050 A-14 names (`agent_dispatcher.rs::resolve_claude_binary` plus
the helpers sibling). Quality gates run on the exact branch shipping the
fix (`fix/prob-050-a14-cfg-gate-claude-bin`). Adversarial findings
verified on the same `git diff` content audited.

There is no inference, abstraction, or cross-context generalisation —
the AC asked for a `#[cfg(test)]` gate and the measurement records
literally that gate landed (line numbers, doc block citations, test
counts). The release-build behaviour assertion is the strongest
available form (compile-time exclusion via Cargo's standard cfg
mechanism) and is verified by the security-expert audit's
case-by-case verification matrix (cargo build, --release, --tests,
doctests, integration tests, downstream consumers).

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-050 | informs (closure evidence for A-14 acceptance criterion) |
| ADR-011 | informs (Phase B Wave 1 dispatcher decision — A-14 is a follow-up to its R1 audit) |
| EVID-096 | informs (Phase B Wave 1 baseline measurement; this evidence closes the audit S-2 escalation referenced there) |
| EVID-097 | informs (real-E2E F-RUNTIME-7 — empirical confirmation that motivated A-14 audit S-2 escalation) |



