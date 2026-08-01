# FPV-13 — [ORCHESTRATOR ADAPTERS] Integrate Kandev, Vibe Kanban, Conductor and Paperclip

- **Repository:** `ForgePlan/marketplace`
- **Phase:** `5`
- **Dependencies:** `FPV-04, FPV-05, FPV-09, FPV-11`
- **Summary:** Connect external task/workspace/runtime systems without duplicating their state.

---

## Objective

Provide adapters and integration guides for Kandev, Vibe Kanban, Conductor and Paperclip.

## Ownership rules

- External systems own task/workspace/session/run/budget/heartbeat state.
- ForgePlan owns WorkContract, authority, Evidence requirements and VerificationVerdict.
- Correlation uses ExternalReference and ExecutionReceipt.

## Deliverables

- Kandev MCP profile and workflow template.
- Vibe Kanban task/workspace/session mapping.
- Versioned Conductor API adapter.
- Paperclip Plugin + Skill + MCP mapping goals/issues/agents/heartbeats to ForgePlan references.
- Generic orchestrator adapter guide/SDK.
- Per-integration conformance fixtures.

## Acceptance Criteria

- [ ] No adapter creates a duplicate canonical task status in ForgePlan.
- [ ] Retry and duplicate webhook/run events are idempotent.
- [ ] External completion does not equal ForgePlan acceptance.
- [ ] Every execution stores task/workspace/session/run references where available.
- [ ] Kandev, Vibe, Conductor and Paperclip each have responsibility-boundary docs.
- [ ] At least Kandev and one of Conductor/Paperclip pass Orchestrator Conformance v1.
- [ ] Paperclip heartbeat remains external runtime owner.
- [ ] Conductor/Vibe/Kandev retain worktree ownership.
