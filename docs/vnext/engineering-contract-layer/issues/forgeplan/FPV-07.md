# FPV-07 — [CORE] Implement authority and policy engine

- **Repository:** `ForgePlan/forgeplan`
- **Phase:** `2`
- **Dependencies:** `FPV-02, FPV-03`
- **Summary:** Enforce who may compile, execute, change scope, accept Evidence and activate decisions.

---

## Objective

Move binding authority rules from prompts and host-specific hooks into a core-evaluated, auditable policy model.

## Scope

- actor roles and stable identities;
- action/resource policy evaluation;
- Tactical/Standard/Deep/Critical default profiles;
- builder ≠ verifier rule;
- human approval requirements;
- policy versioning;
- append-only authority audit events;
- adapter enforcement capability reporting.

## Acceptance Criteria

- [ ] Contract scope expansion requires an allowed actor and new version.
- [ ] Deep/Critical policy can require a different verifier actor instance.
- [ ] Critical activation can require human principal approval.
- [ ] Agents cannot change the policy governing their active execution.
- [ ] Denials return stable machine-readable reasons.
- [ ] CLI, MCP and CI use the same evaluator.
- [ ] Audit log records actor/action/resource/policy/decision/reason/trace.
- [ ] Unsupported host enforcement is reported as partial/advisory, never silently described as full.

## Non-goals

- User directory or enterprise IAM implementation; actor identities may be supplied by adapters/providers.
