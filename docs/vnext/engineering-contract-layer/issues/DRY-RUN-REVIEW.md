# DRY-RUN — the 16 GitHub issues, exactly as `create_github_issues.py` would post them

**These issues were NOT created.** The adversarial audit returned 0 READY of 16.
Regenerated from `issues/bodies.json` + `issues/manifest.json`; `<url:FPV-NN>` marks where
real issue URLs would be substituted at creation time.

Blocking reason beyond content quality: `gh` 2.83.x has no `--parent` / `--add-blocked-by`.
The script calls both with `check=False`, so it would print WARN and proceed, leaving 16
public issues as a flat list with no epic parent and no dependency edges.

## Summary

| Key | Repo | Phase | Depends | Title |
|---|---|---|---|---|
| FPV-00 | ForgePlan/forgeplan | 0 | — | [EPIC] ForgePlan vNext — engineering contract and verification layer |
| FPV-01 | ForgePlan/forgeplan | 0 | — | [ARCH] Adopt canonical ForgePlan product boundary and target architecture |
| FPV-02 | ForgePlan/forgeplan | 1 | FPV-01 | [PROTOCOL] Publish ForgePlan Protocol v1 schemas and compatibility rules |
| FPV-10 | ForgePlan/forgeplan | 1 | FPV-01 | [DOCS] Rebuild product positioning, documentation IA and docs-as-code gates |
| FPV-03 | ForgePlan/forgeplan | 2 | FPV-02 | [CORE] Implement deterministic WorkContract compiler v1 |
| FPV-04 | ForgePlan/forgeplan | 2 | FPV-02 | [CORE] Add ExecutionReceipt and external reference model |
| FPV-05 | ForgePlan/forgeplan | 2 | FPV-02, FPV-03, FPV-04 | [CORE] Implement EvidenceBundle and ground-truth verification gates |
| FPV-06 | ForgePlan/forgeplan | 2 | FPV-02 | [CORE] Introduce claim-centric Evidence scoring and R_eff v2 |
| FPV-07 | ForgePlan/forgeplan | 2 | FPV-02, FPV-03 | [CORE] Implement authority and policy engine |
| FPV-08 | ForgePlan/forgeplan | 2 | FPV-02 | [AGENT-API] Unify CLI/MCP semantics and ship role-based Agent API v2 |
| FPV-09 | ForgePlan/forgeplan | 3 | FPV-02, FPV-03, FPV-04, FPV-05, FPV-07 | [CONFORMANCE] Add capability manifest and host/orchestrator conformance framework |
| FPV-11 | ForgePlan/marketplace | 3 | FPV-02, FPV-09 | [EXTENSIONS V2] Introduce extension taxonomy, manifests and generated catalog |
| FPV-12 | ForgePlan/marketplace | 4 | FPV-03, FPV-07, FPV-09, FPV-11 | [HOST ADAPTERS] Ship official Cursor, Codex and OpenCode integrations |
| FPV-13 | ForgePlan/marketplace | 5 | FPV-04, FPV-05, FPV-09, FPV-11 | [ORCHESTRATOR ADAPTERS] Integrate Kandev, Vibe Kanban, Conductor and Paperclip |
| FPV-14 | ForgePlan/forgeplan-web | 5 | FPV-02, FPV-03, FPV-04, FPV-05, FPV-07 | [WEB V2] Visualize contracts, executions, evidence, authority and PR graph delta |
| FPV-15 | ForgePlan/forgeplan | 6 | FPV-02, FPV-04, FPV-05, FPV-07, FPV-09 | [SERVER] Optional remote MCP and event-ingestion service for 24/7 systems |

---

## FPV-00 (MASTER) — final body as it would be posted

```markdown
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

## Child Issues

### Phase 0
- [ ] FPV-01 — <url:FPV-01> — Create authoritative ADR/docs defining what ForgePlan owns, delegates and never becomes.

### Phase 1
- [ ] FPV-02 — <url:FPV-02> — Create versioned neutral protocol types independent of CLI, MCP and individual hosts.
- [ ] FPV-10 — <url:FPV-10> — Align site, README and docs around contract/verification positioning and eliminate drift.

### Phase 2
- [ ] FPV-03 — <url:FPV-03> — Compile artifact graph, active decisions and policy into immutable execution contracts.
- [ ] FPV-04 — <url:FPV-04> — Correlate external executions without owning their runtime state.
- [ ] FPV-05 — <url:FPV-05> — Verify actual git/test/CI evidence instead of trusting agent completion claims.
- [ ] FPV-06 — <url:FPV-06> — Preserve weakest-link reasoning while scoring required claims rather than arbitrary attached items.
- [ ] FPV-07 — <url:FPV-07> — Enforce who may compile, execute, change scope, accept Evidence and activate decisions.
- [ ] FPV-08 — <url:FPV-08> — Reduce tool surface, add batch context and eliminate transport asymmetries/performance gaps.

### Phase 3
- [ ] FPV-09 — <url:FPV-09> — Make compatibility claims evidence-based and machine-generated.
- [ ] FPV-11 — <url:FPV-11> — Turn the Claude-first Marketplace into explicit cross-host ForgePlan Extensions.

### Phase 4
- [ ] FPV-12 — <url:FPV-12> — Provide native host packages over the same WorkContract and policy semantics.

### Phase 5
- [ ] FPV-13 — <url:FPV-13> — Connect external task/workspace/runtime systems without duplicating their state.
- [ ] FPV-14 — <url:FPV-14> — Evolve the read-only graph viewer into verification observability without becoming a task/runtime UI.

### Phase 6
- [ ] FPV-15 — <url:FPV-15> — Add optional durable integration infrastructure without becoming an agent scheduler.

## Execution Order
See `docs/vnext/engineering-contract-layer/governance/EXECUTION-ORDER.md` in the implementation pack.
```

---

## FPV-01 — ForgePlan/forgeplan

```markdown
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

## Program Links
- Master epic: <url:FPV-00>
- Dependencies: none

```

---

## FPV-02 — ForgePlan/forgeplan

```markdown
## Objective

Create the stable protocol layer shared by ForgePlan Core, CLI/MCP, Web, host adapters and orchestrator adapters.

## Required protocol types

- ArtifactReference
- ActorIdentity
- Claim/Lease reference
- WorkContract
- ExecutionReceipt
- EvidenceBundle
- VerificationVerdict
- AuthorityPolicy
- ExternalReference
- CapabilityManifest
- ErrorEnvelope

## Scope

- JSON Schema 2020-12 definitions.
- Rust domain/DTO mapping without transport-specific fields in core types.
- Canonical JSON serialization and digest algorithm.
- semantic versioning and compatibility policy.
- fixture corpus with valid/invalid examples.
- schema package/release strategy.
- generated protocol reference docs.

## Acceptance Criteria

- [ ] Every type has a versioned schema ID.
- [ ] Unknown major versions fail closed.
- [ ] Unknown optional fields survive supported round-trips where required.
- [ ] Canonical digest is deterministic across repeated serialization.
- [ ] Rust and JSON schema validation agree on fixture corpus.
- [ ] Protocol types contain no Cursor/Codex/Claude/Paperclip-specific fields outside namespaced extensions.
- [ ] CI validates all examples and generated docs.

## Non-goals

- Implementing every adapter.
- Starting agent processes.
- Adding UI.

## Program Links
- Master epic: <url:FPV-00>
- Dependencies:
  - FPV-01: <url:FPV-01>

```

---

## FPV-10 — ForgePlan/forgeplan

```markdown
## Objective

Make ForgePlan understandable in under one minute and keep all public documentation consistent with shipped capabilities.

## Canonical messaging

- Category: repository-native engineering contract and verification layer for AI coding agents.
- Hero: From engineering intent to verified change.
- Promise: Give every agent a contract. Require proof.
- Cycle: DEFINE → CONTRACT → EXECUTE ANYWHERE → VERIFY → ACCEPT.

## Scope

- README and website information architecture.
- What ForgePlan is / is not.
- Solo, Multi-agent and Autonomous use cases.
- Product family: Protocol, Core, Web, Extensions.
- Integration documentation template.
- Current/planned feature separation.
- generated CLI/MCP/schema/plugin references.
- cross-repo link/version/count/example tests.

## Acceptance Criteria

- [ ] README, website, Core docs, Marketplace and Web use one canonical product definition.
- [ ] Static MCP/CLI/plugin counts are removed or generated.
- [ ] Methodology acronyms are not required to understand the homepage.
- [ ] One end-to-end contract/execution/evidence example appears on the site.
- [ ] Documentation IA follows Start → Concepts → Protocol → Hosts → Orchestrators → Components → Methodologies → Reference → Operations.
- [ ] All code examples are smoke/schema tested in CI.
- [ ] EN/RU structural parity check exists.
- [ ] Old version references and deprecated install paths are detected.

## Important

Do not advertise unshipped Protocol v1 features as current until their release flags and compatibility pages say so.

## Program Links
- Master epic: <url:FPV-00>
- Dependencies:
  - FPV-01: <url:FPV-01>

```

---

## FPV-03 — ForgePlan/forgeplan

```markdown
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

## Program Links
- Master epic: <url:FPV-00>
- Dependencies:
  - FPV-02: <url:FPV-02>

```

---

## FPV-04 — ForgePlan/forgeplan

```markdown
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

## Program Links
- Master epic: <url:FPV-00>
- Dependencies:
  - FPV-02: <url:FPV-02>

```

---

## FPV-05 — ForgePlan/forgeplan

```markdown
## Objective

Implement criterion-level EvidenceBundle ingestion and VerificationVerdict with core-side provenance checks.

## Existing work to incorporate

This issue coordinates rather than duplicates:

- #360 — git-delta provenance and activate-time ground-truth gate.
- #328 — core-side Evidence decay triggers.

## Scope

- EvidenceBundle persistence/schema.
- base/result SHA and git delta digest verification.
- changed-path scope validation.
- criterion-level supports/weakens/refutes mapping.
- command/report/CI references and hashes.
- VerificationVerdict and stable reasons.
- independent verifier policy integration.
- activation gate integration.

## Acceptance Criteria

- [ ] Empty relevant git delta cannot satisfy a code-changing criterion.
- [ ] Changed paths outside WorkContract scope block acceptance.
- [ ] Missing required Evidence produces `incomplete`, not success.
- [ ] Refuting Evidence fails its required claim.
- [ ] Stale Evidence cannot satisfy active gates.
- [ ] Base SHA drift is detected and requires revalidation/new contract according to policy.
- [ ] Builder self-report alone never creates accepted verdict.
- [ ] Verification can run read-only and produce deterministic results from the same inputs.
- [ ] Existing #360 and #328 are closed or explicitly superseded by this implementation.

## Evidence

Integration fixture with successful change, empty-delta false success, out-of-scope change, stale Evidence and refuting Evidence.

## Program Links
- Master epic: <url:FPV-00>
- Dependencies:
  - FPV-02: <url:FPV-02>
  - FPV-03: <url:FPV-03>
  - FPV-04: <url:FPV-04>

```

---

## FPV-06 — ForgePlan/forgeplan

```markdown
## Objective

Replace coarse `min(all linked evidence)` behavior with criterion/claim-centric scoring while retaining conservative weakest-link semantics.

## Existing work to incorporate

- #325 — leaf Evidence scoring bug.
- #329 — per-source F/G/R breakdown.
- #328 — decay trigger enforcement.

## Model

- required and informational claims;
- Evidence relations: supports, weakens, refutes;
- claim_score with congruence, reliability, freshness and provenance;
- `R_eff = min(required_claim_scores)`;
- audited Evidence dismissal rather than silent deletion.

## Acceptance Criteria

- [ ] Leaf Evidence with valid structured fields can receive a non-zero self score.
- [ ] Per-source F/G/R and contribution are available in JSON.
- [ ] Informational low-quality Evidence does not automatically destroy the entire artifact score.
- [ ] Missing Evidence for a required claim creates a blind spot.
- [ ] Refuting Evidence for a required claim blocks acceptance.
- [ ] Evidence dismissal records actor, reason, timestamp and policy permission.
- [ ] Migration preserves historical scores and exposes old/new comparison.
- [ ] Existing #325, #329 and relevant #328 scope are closed or superseded.

## Compatibility

Document whether R_eff v2 is opt-in, schema-version gated or introduced by major version.

## Program Links
- Master epic: <url:FPV-00>
- Dependencies:
  - FPV-02: <url:FPV-02>

```

---

## FPV-07 — ForgePlan/forgeplan

```markdown
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

## Program Links
- Master epic: <url:FPV-00>
- Dependencies:
  - FPV-02: <url:FPV-02>
  - FPV-03: <url:FPV-03>

```

---

## FPV-08 — ForgePlan/forgeplan

```markdown
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

## Program Links
- Master epic: <url:FPV-00>
- Dependencies:
  - FPV-02: <url:FPV-02>

```

---

## FPV-09 — ForgePlan/forgeplan

```markdown
## Objective

Create CapabilityManifest, host conformance and orchestrator conformance suites so ForgePlan only claims support that is automatically verified.

## Host checks

- install/discovery;
- MCP and skills;
- contract retrieval;
- path/command policy;
- actor propagation;
- execution registration;
- Evidence submission;
- independent verification;
- errors and retry/resume;
- uninstall.

## Orchestrator checks

- state ownership;
- external references;
- idempotency;
- duplicate prevention;
- result/Evidence collection;
- failure recovery;
- completion ≠ acceptance.

## Acceptance Criteria

- [ ] Manifest schema is published under Protocol v1.
- [ ] Fixture repository and canonical WorkContract exist.
- [ ] Conformance output is versioned JSON with tested versions, commit SHA and date.
- [ ] Capability levels are full/partial/advisory/unsupported/experimental.
- [ ] Website/Marketplace compatibility matrix can be generated from results.
- [ ] Failed required test prevents stable badge/publication.
- [ ] At least one reference adapter runs end-to-end in CI.

## Program Links
- Master epic: <url:FPV-00>
- Dependencies:
  - FPV-02: <url:FPV-02>
  - FPV-03: <url:FPV-03>
  - FPV-04: <url:FPV-04>
  - FPV-05: <url:FPV-05>
  - FPV-07: <url:FPV-07>

```

---

## FPV-11 — ForgePlan/marketplace

```markdown
## Objective

Restructure Marketplace as ForgePlan Extensions with honest maturity and capability declarations.

## Categories

- Host Integrations
- Orchestrator Adapters
- Methodology Packs
- Domain Packs
- Evidence Providers
- Migration Packs
- Visualization Extensions

## Scope

- extension manifest implementation;
- compatibility and permissions disclosure;
- owned/non-owned states;
- maturity labels;
- conformance references;
- generated root catalog/docs;
- migration of current plugins;
- explicit Claude-native vs portable surfaces;
- Smith positioned as optional router.

## Acceptance Criteria

- [ ] Every published extension has a validated manifest.
- [ ] Catalog and counts are generated from manifests.
- [ ] Each extension declares permissions and state ownership.
- [ ] Full/partial support is based on conformance results.
- [ ] Claude-specific agents/commands/hooks are clearly marked.
- [ ] `.agents/skills` remains portable baseline.
- [ ] Deprecated plugins have machine-readable replacement and sunset metadata.
- [ ] Core operation does not require Marketplace or Smith.

## Program Links
- Master epic: <url:FPV-00>
- Dependencies:
  - FPV-02: <url:FPV-02>
  - FPV-09: <url:FPV-09>

```

---

## FPV-12 — ForgePlan/marketplace

```markdown
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

## Program Links
- Master epic: <url:FPV-00>
- Dependencies:
  - FPV-03: <url:FPV-03>
  - FPV-07: <url:FPV-07>
  - FPV-09: <url:FPV-09>
  - FPV-11: <url:FPV-11>

```

---

## FPV-13 — ForgePlan/marketplace

```markdown
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

## Program Links
- Master epic: <url:FPV-00>
- Dependencies:
  - FPV-04: <url:FPV-04>
  - FPV-05: <url:FPV-05>
  - FPV-09: <url:FPV-09>
  - FPV-11: <url:FPV-11>

```

---

## FPV-14 — ForgePlan/forgeplan-web

```markdown
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

## Program Links
- Master epic: <url:FPV-00>
- Dependencies:
  - FPV-02: <url:FPV-02>
  - FPV-03: <url:FPV-03>
  - FPV-04: <url:FPV-04>
  - FPV-05: <url:FPV-05>
  - FPV-07: <url:FPV-07>

```

---

## FPV-15 — ForgePlan/forgeplan

```markdown
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

## Program Links
- Master epic: <url:FPV-00>
- Dependencies:
  - FPV-02: <url:FPV-02>
  - FPV-04: <url:FPV-04>
  - FPV-05: <url:FPV-05>
  - FPV-07: <url:FPV-07>
  - FPV-09: <url:FPV-09>

```
