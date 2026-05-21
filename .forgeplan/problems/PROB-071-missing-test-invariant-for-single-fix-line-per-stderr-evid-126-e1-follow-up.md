---
depth: standard
id: PROB-071
kind: problem
last_modified_at: 2026-05-14T10:50:14.177931+00:00
last_modified_by: claude-code/2.1.141
links:
- target: EVID-126
  relation: based_on
status: active
title: 'Missing test invariant for single Fix: line per stderr (EVID-126 E1 follow-up)'
---

# PROB-071: No regression-fence for "single Fix: line per stderr" hint contract

## Signal

PRD-071 hint protocol contract (`CLAUDE.md` §"Hint protocol"): every CLI/MCP error output emits **one** `Fix:` line so an agent reading stderr per the agent-protocol can deterministically route to remediation.

Round-2 Wave 9 audit (commit `b5a21bf` + `cef1695`) caught a double-`Fix:` regression in `crates/forgeplan-cli/src/commands/reason.rs`: the missing-LLM error path emitted one `Fix:` line from inside the anyhow body (via `require_llm_config`) AND a second `Fix:` line from a sibling `eprintln!` in `reason::run`. Tests passed, clippy passed, code-review caught it.

EVID-126 finding E1 (FR-027 recursive audit) confirms the contract is enforced only by code review — no test, no clippy lint, no compile-time check.

## Constraints

- Contract MUST stay machine-readable: agents grep `^Fix:` and follow the first match
- MUST NOT introduce a heavyweight DSL just to wrap two `eprintln!` calls — solution should fit on one screen
- Fixing this in v0.32.0 expands scope post-audit; deferred to v0.33+

## Acceptance criteria

- [ ] Either: integration test asserts exactly one `^Fix:` line for each error path in `commands/reason.rs::run` adversarial-config-yaml fixtures
- [ ] OR: `forgeplan-core::hints::Hint` builder type that the call site composes; `Display` impl renders exactly one `Fix:` line; all current `Fix:`-emitting sites refactored to use it
- [ ] OR: clippy custom lint flagging `eprintln!("Fix: ...")` near another `Fix:`-emitting site in the same function

## Sub-items

47 current `Fix:`-emitting sites across `crates/forgeplan-cli/` and `crates/forgeplan-core/`:
```
grep -rEn '(eprintln|anyhow::bail).*Fix:' crates/
```

Pick the cheapest enforcement that ALL sites adopt. Test-based gate is cheapest first (≈30 LOC); builder type is more invasive but composable.

## Related artifacts

- EVID-126 (FR-027 recursive audit — E1 finding)
- PRD-077 (informs — sprint that exposed the gap)
- PRD-071 (hint protocol — the contract this fence protects)

## Reversibility

High — fence is additive, no behaviour change. Worst case: a test asserting count==1 misfires and gets `#[ignore]`'d temporarily.



