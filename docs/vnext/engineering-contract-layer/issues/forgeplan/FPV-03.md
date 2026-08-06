# FPV-03 — [CORE] Implement deterministic WorkContract compiler v1

- **Repository:** `ForgePlan/forgeplan`
- **Phase:** `2`
- **Dependencies:** `FPV-02`
- **Summary:** Compile artifact graph, active decisions and policy into immutable execution contracts.

---

## Objective

Implement WorkContract as a compiled, immutable and versioned projection rather than a manually maintained eleventh artifact kind.

## Compiler inputs

- source PRD/Spec/RFC/ADR/Problem/Solution references;
- applicable active decisions and constraints;
- affected paths/domain;
- repository base ref/SHA;
- depth and project policy;
- acceptance criteria and required Evidence.

## Required commands/API

- `forgeplan contract compile <artifact>`
- `forgeplan contract get <id>@<version>`
- `forgeplan contract validate`
- `forgeplan contract diff`
- equivalent MCP/agent API operations.

## Acceptance Criteria

- [ ] Repeated compilation against identical graph/revision produces identical canonical digest.
- [ ] Every derived contract field exposes source provenance.
- [ ] Contradictory active constraints fail compilation with stable error codes.
- [ ] Contract records source artifact digests and base SHA.
- [ ] Contract contains included/excluded scope, allowed/forbidden paths, acceptance criteria, Evidence requirements and authority requirements.
- [ ] Started contract versions cannot be mutated; scope change creates a new version.
- [ ] Semantic diff distinguishes scope, criteria, policy and source changes.
- [ ] CLI and MCP outputs are schema-identical.
- [ ] Golden, negative and property tests cover compiler determinism.

## Non-goals

- Running the agent.
- Creating worktrees.
- Updating task trackers.
