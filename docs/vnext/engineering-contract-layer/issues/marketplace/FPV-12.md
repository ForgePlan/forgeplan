# FPV-12 — [HOST ADAPTERS] Ship official Cursor, Codex and OpenCode integrations

- **Repository:** `ForgePlan/marketplace`
- **Phase:** `4`
- **Dependencies:** `FPV-03, FPV-07, FPV-09, FPV-11`
- **Summary:** Provide native host packages over the same WorkContract and policy semantics.

---

## Objective

Ship official host adapters for Cursor, Codex and OpenCode, plus normalize the existing Claude Code integration under the same extension contract.

## Cursor deliverables

Plugin, MCP, rules, skills, subagents, hooks, local/cloud capability matrix.

## Codex deliverables

AGENTS.md generator, `.agents/skills`, MCP, planner/verifier skills, SDK adapter, thread correlation and resume.

## OpenCode deliverables

TypeScript plugin, MCP, agents/skills, granular permission compiler and event bridge.

## Acceptance Criteria

- [ ] Each adapter has a validated extension manifest.
- [ ] Same fixture WorkContract is executed in all three hosts.
- [ ] Contract digest and criterion semantics remain identical.
- [ ] Builder/verifier separation is demonstrated.
- [ ] Scope and forbidden-path behavior is tested.
- [ ] ExecutionReceipt and EvidenceBundle are submitted in canonical schemas.
- [ ] Unsupported capabilities are reported honestly.
- [ ] Installation, doctor, upgrade and uninstall are tested.
- [ ] Host Conformance v1 passes at the claimed level.

## Non-goals

Owning host worktrees, model selection or session scheduling.
