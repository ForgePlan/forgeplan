# FPV-00 — [EPIC] ForgePlan vNext — engineering contract and verification layer

- **Repository:** `ForgePlan/forgeplan`
- **Phase:** `0`
- **Dependencies:** `none`
- **Summary:** Umbrella program coordinating product boundary, Protocol v1, Core, integrations, Web and documentation.

---

## Objective

Evolve ForgePlan into the repository-native engineering contract and verification layer for AI coding agents without turning it into a task tracker, agent runtime, worktree manager or scheduler.

## Canonical product definition

> ForgePlan keeps engineering intent, execution and evidence connected across any agent or orchestrator. It compiles versioned work contracts and accepts results only when required evidence is verified.

## Program outcomes

- Product boundary and architecture are canonical.
- ForgePlan Protocol v1 is versioned and published.
- WorkContract, ExecutionReceipt, EvidenceBundle, VerificationVerdict and AuthorityPolicy exist in core.
- CLI and MCP share one semantic application layer.
- Cursor, Codex and OpenCode pass the same host conformance suite.
- Kandev, Vibe Kanban, Conductor and Paperclip integrate through adapters and external references.
- ForgePlan Web presents contract → execution → evidence → verdict while remaining read-only.
- Site, README, Core docs, Marketplace and Web use one product definition.
- Documentation and capability matrices are tested/generated in CI.

## Non-goals

- Building a Kanban/task tracker.
- Owning agent processes, terminals or worktrees.
- Replacing Kandev, Vibe Kanban, Conductor or Paperclip.
- Shipping a general workflow scheduler in core.
- Making Marketplace or Smith mandatory for core operation.

## Delivery phases

1. Product truth and boundary.
2. Protocol and schemas.
3. Core contract/verification/policy.
4. Agent API and conformance.
5. Extensions and adapters.
6. Web and product surface.
7. Optional remote server.

## Definition of Done

All child issues complete; cross-host semantic portability test passes; docs-as-code gates are green; public site contains working integration paths for Solo, Multi-agent and Autonomous usage.
