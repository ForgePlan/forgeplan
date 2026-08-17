# FPV-15 — [SERVER] Optional remote MCP and event-ingestion service for 24/7 systems

- **Repository:** `ForgePlan/forgeplan`
- **Phase:** `6`
- **Dependencies:** `FPV-02, FPV-04, FPV-05, FPV-07, FPV-09`
- **Summary:** Add optional durable integration infrastructure without becoming an agent scheduler.

---

## Objective

Provide an optional ForgePlan Server for autonomous/multi-repository environments while preserving local git-native operation.

## Scope

- Streamable HTTP MCP/API.
- authentication and actor identity mapping.
- event ingestion with idempotency.
- audit log.
- multi-repository registry.
- subscriptions/webhooks.
- OpenTelemetry traces/metrics/logs.
- replay/recovery for integration events.
- remote Evidence artifact references.

## Explicit non-goals

- launching agent processes;
- task scheduling;
- owning heartbeats;
- worktree management;
- replacing Paperclip/Kandev/Conductor.

## Acceptance Criteria

- [ ] Local-only CLI/MCP remains fully functional without server.
- [ ] Same Protocol v1 schemas are used locally and remotely.
- [ ] Duplicate events are idempotent.
- [ ] Actor and authority decisions are audited.
- [ ] Server can correlate Paperclip/Kandev/Conductor events without storing duplicate task state.
- [ ] Kill/restart recovery tests pass.
- [ ] Security threat model and secret-handling docs exist.
- [ ] OpenTelemetry instrumentation covers contract, execution, Evidence and verdict flows.
