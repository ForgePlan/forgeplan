# Execution Order

## Wave 0 — Alignment

- FPV-01 Product boundary.
- FPV-10 Documentation/product architecture may begin after FPV-01 decision, but may only publish shipped capabilities.

## Wave 1 — Protocol

- FPV-02 Protocol v1.

No Core vNext implementation should merge before schemas and compatibility rules are accepted.

## Wave 2 — Core foundations

Parallel after FPV-02:

- FPV-03 WorkContract.
- FPV-04 ExecutionReceipt.
- FPV-06 R_eff v2 design/implementation.
- FPV-07 Authority Policy.
- FPV-08 Agent API parity work that does not depend on WorkContract.

Then:

- FPV-05 EvidenceBundle and Verification depends on WorkContract + Receipt.

## Wave 3 — Conformance and Extensions foundation

- FPV-09 Conformance.
- FPV-11 Marketplace v2.

## Wave 4 — Host adapters

- FPV-12 Cursor, Codex, OpenCode.

Implement one reference adapter first, preferably OpenCode for permission granularity, then validate semantic portability in Cursor and Codex.

## Wave 5 — Orchestrators and Web

Parallel:

- FPV-13 Kandev/Vibe/Conductor/Paperclip.
- FPV-14 ForgePlan Web v2.

## Wave 6 — Optional server

- FPV-15 only after stable Protocol, receipts, Evidence, authority and conformance.

## Agent allocation

- Protocol agent: schemas, compatibility, fixtures.
- Core contract agent.
- Evidence/verifier agent.
- Policy/security agent.
- CLI/MCP parity/performance agent.
- Docs/product agent.
- Marketplace/extensions agent.
- One agent per host adapter.
- One agent per orchestrator adapter.
- Web agent.
- Independent verifier agents for every PR.

## Merge discipline

- Protocol PRs before dependent core PRs.
- Core interfaces before adapters.
- No cross-repo breaking change without compatibility fixture and coordinated release note.
- Use feature flags for partially shipped vNext surfaces.
