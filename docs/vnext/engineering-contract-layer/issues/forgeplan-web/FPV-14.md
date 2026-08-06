# FPV-14 — [WEB V2] Visualize contracts, executions, evidence, authority and PR graph delta

- **Repository:** `ForgePlan/forgeplan-web`
- **Phase:** `5`
- **Dependencies:** `FPV-02, FPV-03, FPV-04, FPV-05, FPV-07`
- **Summary:** Evolve the read-only graph viewer into verification observability without becoming a task/runtime UI.

---

## Objective

Make ForgePlan Web answer: what was requested, who executed it, what changed, what evidence exists and why the result was accepted.

## Views

- WorkContract details and semantic diff.
- Acceptance criterion → Evidence matrix.
- ExecutionReceipt details and external links.
- Verification timeline.
- Authority map.
- PR graph delta before/after.
- Existing graph/health/time views preserved.

## Boundary

Web remains read-only. It must not add Kanban, agent launch, terminal, worktree management, scheduling or canonical mutation.

## Acceptance Criteria

- [ ] Web consumes Protocol v1 JSON without parsing Evidence body conventions.
- [ ] Contract source provenance is navigable.
- [ ] Criterion-level pass/fail/missing/stale is visible.
- [ ] External task/workspace/session/PR/CI links render safely.
- [ ] Actor roles and independent verifier are visible.
- [ ] Before/after graph delta is deterministic for a PR/base range.
- [ ] Large-workspace performance budget and fixtures exist.
- [ ] Read-only proxy allowlist is updated and security-tested.
- [ ] Marketplace Web documentation is synchronized with actual install/features.
