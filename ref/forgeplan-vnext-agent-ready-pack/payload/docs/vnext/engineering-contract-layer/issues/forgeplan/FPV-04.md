# FPV-04 — [CORE] Add ExecutionReceipt and external reference model

- **Repository:** `ForgePlan/forgeplan`
- **Phase:** `2`
- **Dependencies:** `FPV-02`
- **Summary:** Correlate external executions without owning their runtime state.

---

## Objective

Represent executions performed by Cursor, Codex, OpenCode, Claude Code and external orchestrators without making ForgePlan the runtime or scheduler.

## Scope

- ExecutionReceipt persistence and retrieval.
- Stable normalized statuses.
- ActorIdentity and provider metadata.
- External task/workspace/session/run/PR/CI references.
- base/result SHA and reported changed paths.
- idempotency key and trace correlation.
- namespaced provider extension payloads.

## Acceptance Criteria

- [ ] Repeated registration with the same provider/idempotency key is idempotent.
- [ ] External IDs are opaque and do not become ForgePlan task state.
- [ ] `completed` execution does not activate artifacts or imply acceptance.
- [ ] Provider-specific fields remain isolated under extensions.
- [ ] Receipt can be linked to WorkContract and EvidenceBundle.
- [ ] CLI/MCP/SDK parity is tested.
- [ ] Invalid state transitions fail with stable errors.

## Non-goals

- Heartbeats, process spawning, retry scheduling or worktree ownership.
