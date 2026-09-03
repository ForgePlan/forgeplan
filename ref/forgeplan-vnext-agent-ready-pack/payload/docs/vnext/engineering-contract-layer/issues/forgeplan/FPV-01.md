# FPV-01 — [ARCH] Adopt canonical ForgePlan product boundary and target architecture

- **Repository:** `ForgePlan/forgeplan`
- **Phase:** `0`
- **Dependencies:** `none`
- **Summary:** Create authoritative ADR/docs defining what ForgePlan owns, delegates and never becomes.

---

## Problem

ForgePlan is currently described as an engineering decision framework, methodology, project-management layer and agent harness. Marketplace documentation also places implementation and orchestration responsibilities inside ForgePlan, creating an unstable boundary.

## Scope

- Add a canonical ADR for the ForgePlan product boundary.
- Add a target architecture document covering Protocol, Core, CLI/MCP, Extensions, Web and optional Server.
- Define state ownership across trackers, orchestrators, agent hosts, ForgePlan and CI/CD.
- Define SOLID and FORGE architectural principles.
- Mark existing documentation that conflicts with the new boundary and update it.
- Decide whether current dispatch scheduling remains core, becomes an optional planner adapter, or is retained only as advisory graph analysis.

## Required decisions

- Core responsibility: intent → contract → evidence → verdict → lifecycle.
- External task/workspace/session state is referenced, not duplicated.
- Agent execution and scheduling remain outside core.
- Methodology packs do not define core identity.
- ForgePlan Web remains read-only.

## Acceptance Criteria

- [ ] One canonical ADR is active and linked to the vNext Epic.
- [ ] `docs/architecture/product-boundary.md` exists and includes ownership matrix.
- [ ] README, methodology overview and Marketplace architecture no longer call ForgePlan a task/project manager.
- [ ] Current `forgeplan_dispatch` ownership is explicitly decided with migration consequences.
- [ ] Architecture dependency rules are enforceable by crate/module tests or lint where practical.
- [ ] EN/RU canonical documents are structurally aligned.

## Evidence

- Documentation link check.
- Search proving conflicting product labels were removed or intentionally qualified.
- Architecture tests/lints where added.
