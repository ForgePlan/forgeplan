# FPV-08 — [AGENT-API] Unify CLI/MCP semantics and ship role-based Agent API v2

- **Repository:** `ForgePlan/forgeplan`
- **Phase:** `2`
- **Dependencies:** `FPV-02`
- **Summary:** Reduce tool surface, add batch context and eliminate transport asymmetries/performance gaps.

---

## Objective

Expose a compact role-based agent interface over one application layer shared by CLI and MCP.

## Existing issues to incorporate

- #304 — MCP latency.
- #353 — CLI/MCP identity asymmetry.
- #374 — missing JSON output.
- #397 — CLI JSON projection gaps.

## Required profiles

`minimal`, `planner`, `builder`, `reviewer`, `operator`, `full`.

## High-level operations

`next`, `context`, `contract`, `claim`, `execution`, `evidence`, `verify`, `status`, `search`.

## Acceptance Criteria

- [ ] CLI and MCP call the same application services and validators.
- [ ] No `@file` or identity parsing asymmetry remains.
- [ ] Every agent-facing read operation has versioned JSON Schema.
- [ ] Role profiles hide unavailable mutation/approval tools.
- [ ] Context bundle replaces common N+1 query path.
- [ ] MCP cold/warm latency benchmark and budget are added.
- [ ] Stable error codes and retryability exist.
- [ ] Existing #304, #353, #374 and #397 are closed or explicitly superseded.
- [ ] Full low-level API remains available for advanced clients without being the default prompt surface.
