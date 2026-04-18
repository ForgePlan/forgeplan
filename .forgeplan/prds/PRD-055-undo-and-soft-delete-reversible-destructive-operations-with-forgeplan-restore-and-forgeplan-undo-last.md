---
depth: standard
id: PRD-055
kind: prd
status: draft
title: Undo and soft-delete — reversible destructive operations with forgeplan_restore and forgeplan_undo_last
---

# PRD-055: Undo and soft-delete — reversible destructive operations

## Executive Summary

### Vision

Every destructive operation in Forgeplan (`delete`, `supersede`, `deprecate`) becomes reversible within a configurable time window. Agents and operators recover from mistakes or malicious prompt-injection without git gymnastics. The feature is wired into the existing activity log (PRD-054) so undo-last is a one-tool call, not a manual git archaeology session.

### Problem

In v0.20.0 a destructive operation is terminal. Once `forgeplan_delete PRD-048` runs, the artifact is gone from LanceDB and its markdown file is removed. Recovery requires `git checkout HEAD~N -- .forgeplan/prds/PRD-048-*.md` plus `forgeplan scan-import` — not something an agent can reason about safely, and impossible if the deletion happened in a working tree that was never committed.

Concretely this creates three failure modes we have already seen in practice. An agent that hallucinates an artifact ID and calls `forgeplan_delete` with it can wipe unrelated work. A successful prompt-injection attack against a lifecycle tool turns into permanent data loss. A user who intends to `forgeplan_deprecate` and typos `forgeplan_delete` has no fast undo path. PRD-054 gave us visibility into what happened (the activity log records every call), but visibility without reversibility is only half the answer. Teams in regulated contexts additionally need a soft-delete pattern for compliance — some jurisdictions mandate that "deletion" means "marked inactive, recoverable for N days" rather than hard-erased.

**Impact**:
- Agent mistakes turn into permanent damage instead of recoverable slips.
- Prompt-injection attacks (a threat class the R2/R3 audits closed at the hint level) still have a data-loss tail if the attacker reaches a destructive tool.
- Manual recovery via git requires knowledge the agent doesn't have and the operator may not want to use.
- Compliance scenarios cannot be supported with the current hard-delete semantics.

### Target Users

| Persona | Описание | Ключевая боль |
|---------|----------|---------------|
| Solo developer | Runs forgeplan via Claude Code for personal projects | "I told it to deprecate, it deleted. Now what?" |
| AI agent | Claude Code / Cursor / Windsurf | No safe experimental mode — every destructive try is permanent |
| Compliance-bound team | Uses forgeplan under SOC2 / HIPAA / GDPR constraints | Hard-delete incompatible with mandated retention windows |

### Differentiators

- **Zero-config**: trash directory and TTL-based cleanup work on default settings; no setup required.
- **Uses existing activity log**: undo-last is cheap because PRD-054 already records every mutation.
- **Cryptographically linked records**: each soft-delete receipt references the activity log entry it was produced from, so partial recovery is auditable.
- **Two-level recovery**: `forgeplan_restore` for named targeted recovery, `forgeplan_undo_last` for "wait, don't, roll that back" speed.

---

## Success Criteria

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | Every destructive op is reversible within TTL | Coverage of reversible ops | 0/3 (delete, supersede, deprecate all terminal) | 3/3 via soft-delete + lifecycle-replay | v0.21.0 | Integration test: call each destructive tool on a fresh artifact, call restore, assert artifact is back with identical body + relations |
| SC-2 | Restore preserves full artifact state | Round-trip fidelity | n/a | Byte-identical body, identical metadata, equivalent relation graph | v0.21.0 | Hash artifact body before + after delete/restore cycle; assert relations recovered |
| SC-3 | Trash directory has a bounded size | TTL enforcement | n/a | Entries older than TTL are purged on next invocation | v0.21.0 | Backdate a trash entry, invoke any tool, assert purge runs |
| SC-4 | Undo-last finds the right operation | Correctness under concurrent load | n/a | Picks the last destructive op by activity log, not last mutation | v0.21.0 | Seed log with mixed destructive + non-destructive ops; undo_last reverses only the most recent destructive one |
| SC-5 | Restore response latency | User-visible delay | n/a | < 100 ms p95 for one artifact | v0.21.0 | Benchmark: 100 restore cycles on real workspace, measure p95 |

---

## Product Scope

### MVP (In-Scope)

Soft-delete infrastructure under `.forgeplan/trash/`. When a destructive tool runs, we do not remove LanceDB rows or markdown files. Instead we create a receipt file that captures what changed and move the markdown projection into trash. The receipt is a structured JSON document containing the operation kind, the original artifact state (frontmatter + body + relations), the activity-log entry hash, and a TTL deadline.

Two new MCP tools. `forgeplan_restore` takes an artifact ID and recovers the most recent soft-delete receipt for that ID: markdown back to its original path, LanceDB row re-created from receipt contents, relations re-linked. `forgeplan_undo_last` reads the activity log, finds the most recent successful destructive call across all tools, and applies the equivalent restore logic to that target.

Default TTL: 30 days. Configurable via `.forgeplan/config.yaml` under `undo.ttl_days`. Purge of expired receipts runs lazily — on any invocation of `forgeplan_restore` or the first destructive op of a session. No background daemon.

Activity log integration: each soft-delete writes one activity entry for the original tool call (unchanged) plus companion entry recording the receipt ID. Each restore writes a `forgeplan_restore` activity entry with the restored target.

CLI parity: `forgeplan restore <id>` and `forgeplan undo-last` mirror the MCP tools for scripted use.

Transactional semantics: if any step of restore fails partway through, the artifact is either fully back or remains in trash. Pattern: stage all changes in memory → validate → apply atomically.

### Out of Scope

Multi-artifact undo ("restore everything deleted in the last session") — deferred. Requires session boundary tracking we don't have. For v1 the user runs restore N times for N artifacts.

Undo for non-destructive operations. Creating an artifact and then wanting to undo that is just delete; no separate undo path needed. Body update rollback is ambiguous (revert to which prior state? activity log keeps args hash only, not content).

Cross-workspace undo. Receipts are scoped to the workspace where the destructive operation happened. No global trash.

Cryptographic signing of receipts. Append-only JSONL + filesystem permissions is the same trust boundary as the activity log.

### Growth Vision

A `forgeplan_diff <id> --from <receipt-id>` tool to inspect what a receipt contains before restoring. A batch mode that restores all receipts in a time window. Remote backup of trash to a git-lfs branch for multi-device recovery. Integration with Orchestra's task history.

---

## User Journeys

### Journey 1: Solo developer recovers from a typo

**Цель пользователя**: "Я хотел deprecate, а нажал delete. Верни."

- [ ] Agent runs `forgeplan_delete PRD-045` → receipt written, markdown moved to `.forgeplan/trash/`, LanceDB row removed
- [ ] User: "wait, I didn't mean that" → Agent calls `forgeplan_undo_last`
- [ ] Tool reads activity log, finds the delete op, calls restore internally
- [ ] PRD-045 is back: markdown in place, LanceDB row restored, relations restored
- [ ] Agent reports: "Restored PRD-045. Use `forgeplan_deprecate PRD-045 --reason ...` if that's what you meant"

**Результат**: One-call recovery. Latency < 1s.

### Journey 2: Compliance audit — destructive op window

**Цель пользователя**: "Show all destructive operations in the last 30 days and which ones were reverted."

- [ ] `forgeplan_activity --since 720h --tool forgeplan_delete,forgeplan_supersede,forgeplan_deprecate` → 47 destructive calls (uses PRD-054)
- [ ] Cross-reference with `forgeplan_activity --tool forgeplan_restore` → 6 restores logged
- [ ] Operator sees 41 destructive ops that were NOT reverted
- [ ] Reviews trash directory for those 41 receipts if needed; receipt files are inspectable
- [ ] Reverts any surprising ones via `forgeplan_restore <id>`

**Результат**: Full audit + selective recovery, no git required.

### Journey 3: Agent safely experiments

**Цель пользователя**: "Try deprecating PRD-051 and see what breaks. If nothing, keep it. If something, undo."

- [ ] Agent: `forgeplan_deprecate PRD-051 --reason "testing"` → receipt written, status → deprecated
- [ ] Agent runs dependent checks, finds broken link → sees 3 orphans surface in `forgeplan_blindspots`
- [ ] Agent: `forgeplan_undo_last` → PRD-051 back to active, orphans gone

**Результат**: Lower-risk agent autonomy. Opens the "try and observe" pattern.

---

## Functional Requirements

- [ ] FR-001: System can move an artifact's markdown projection into trash instead of hard-deleting on filesystem when delete is invoked (Journey 1/3, Must)
- [ ] FR-002: System can write a soft-delete receipt containing original body, frontmatter, and relation graph (All, Must)
- [ ] FR-003: Agent can recover a soft-deleted artifact by ID via restore (Journey 1/2/3, Must)
- [ ] FR-004: Agent can reverse the most recent destructive operation via undo-last (Journey 1/3, Must)
- [ ] FR-005: System can purge trash receipts older than the configured TTL on demand, without a background daemon (SC-3, Must)
- [ ] FR-006: Soft-delete semantics apply equally to supersede and deprecate in addition to delete (SC-1, Must)
- [ ] FR-007: Operator can configure TTL via `undo.ttl_days`; default 30 (Must)
- [ ] FR-008: Restore preserves the artifact's relation graph — both outgoing and incoming links — identically to pre-delete state (SC-2, Must)
- [ ] FR-009: CLI user can invoke restore and undo-last outside of MCP (Should)
- [ ] FR-010: Trash receipts cryptographically reference the originating activity log entry via content hash (Should)
- [ ] FR-011: Restore is transactional — either fully succeeds or leaves trash untouched on failure (Must)
- [ ] FR-012: Both tools return a workflow hint pointing at plausible follow-up (Journey 1, Could)

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Performance | Soft-delete overhead vs hard-delete | < 10 ms p95 extra latency | One artifact with evidence links | Benchmark delete with and without soft-delete flag |
| NFR-002 | Performance | Restore single artifact | < 100 ms p95 | Real workspace with 212 artifacts and 500 relations | Micro-benchmark restore cycle |
| NFR-003 | Durability | Receipt write survives process kill | fsync before ack | SIGKILL immediately after destructive tool returns Ok | Chaos test: kill mid-op, verify receipt recoverable |
| NFR-004 | Correctness | No lost data on crash mid-op | Invariant: artifact is always either in store OR in trash, never neither | Randomized crash tests | Inject panic at each await point; assert invariant |
| NFR-005 | Storage | Trash disk usage bounded by TTL | Purge on demand | 30-day default, 100 deletes per day | Synthetic load test, check directory size |
| NFR-006 | Portability | Works on macOS, Linux, Windows | Same file-rename semantics | CI matrix | GH Actions matrix passes integration test |
| NFR-007 | Security | Trash directory inherits workspace permissions | No widening of access | Default umask respected | Inspect stat output on trash dir |

---

## Acceptance Criteria

### AC-1: Soft-delete preserves artifact body

```gherkin
Given PRD-100 exists with body "Hello world" and one evidence link
When the agent calls forgeplan_delete PRD-100
Then PRD-100 is not visible via forgeplan_list
And trash contains a receipt file for PRD-100
And the receipt contains "Hello world" as the body
And the receipt captures the evidence link
```

### AC-2: Restore brings artifact back with identical body

```gherkin
Given PRD-100 was soft-deleted in the last 30 days
And trash contains its receipt
When the agent calls forgeplan_restore PRD-100
Then PRD-100 is visible via forgeplan_list with status draft or its prior status
And forgeplan_get PRD-100 returns a body byte-identical to the pre-delete body
And all original relations are restored
And the receipt is marked as consumed
```

### AC-3: Undo-last reverses only the most recent destructive op

```gherkin
Given an activity log with (in order) forgeplan_delete on PRD-A, forgeplan_score on PRD-B, forgeplan_deprecate on PRD-C
When the agent calls forgeplan_undo_last
Then PRD-C is restored to its prior active status
And PRD-A remains soft-deleted
And the operation is recorded in the activity log as forgeplan_restore
```

### AC-4: TTL purge removes only expired receipts

```gherkin
Given two receipts in trash: one dated today, one dated 35 days ago
And undo.ttl_days is 30
When any destructive tool is invoked
Then the 35-day-old receipt is removed from disk
And today's receipt remains
```

### AC-5: Transactional restore — mid-op failure leaves trash untouched

```gherkin
Given PRD-200 has a soft-delete receipt
And the workspace's store is deliberately made read-only to simulate failure
When the agent calls forgeplan_restore PRD-200
Then the tool returns an error result with recovery guidance
And the receipt for PRD-200 is still present in trash
And no partial row for PRD-200 exists in store
```

### AC-6: Supersede goes through soft-delete too

```gherkin
Given PRD-300 is active
When the agent calls forgeplan_supersede PRD-300 --by PRD-301
Then PRD-300 gets a soft-delete receipt preserving its full state
And forgeplan_undo_last can restore PRD-300 to active
And the supersede link to PRD-301 is undone on restore
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| PRD-054 activity log | Internal | Shipped | — |
| tokio fs rename operations | Runtime | Ready | stdlib |
| serde_json for receipt serialization | Runtime | Ready | workspace dep |
| No upstream lifecycle logic changes — soft-delete wrapper sits above it | Internal | — | — |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | Race: two concurrent deletes of same ID produce duplicate receipts | Low | Medium | Receipt path includes timestamp + random suffix; restore picks the newest non-consumed | Core |
| R-2 | TTL purge deletes receipt the user wanted | High | High | Default TTL 30 days is generous; purge writes activity log entry so audit trail remains | Core |
| R-3 | Restore overwrites an artifact re-created with the same ID after delete | Medium | Medium | Restore refuses if store already has an artifact with that ID; prompts operator to resolve | Core |
| R-4 | Trash fills disk on long-lived project | Medium | Medium | NFR-005 plus TTL; document escape hatch | Core |
| R-5 | Relation restore fails silently if target artifact is also deleted | Medium | High | Restore validates each link target exists; orphan links become warnings in response, not silent drops | Core |
| R-6 | Crash between "remove from store" and "write receipt" loses data | Low | Critical | Write receipt FIRST, then remove from store. Replay on startup: trash entries without matching removal are no-ops | Core |

---

## Timeline

| Milestone | Target Date | Description |
|-----------|-------------|-------------|
| PRD Approved | 2026-04-18 | This doc validated |
| Architecture inline | 2026-04-18 | Key decisions in Architecture section below |
| MVP | 2026-04-20 | FR-001..006, 011 shipped |
| v0.21.0 Release | 2026-04-22 | Tagged, brew updated |

---

## Stakeholders

| Role | Name | Sign-off |
|------|------|----------|
| Product Owner | user (project owner) | [ ] |
| Engineering Lead | gogocat | [ ] |
| Design | n/a | [x] |
| QA | n/a (integrated with Rust test suite) | [x] |

---

## Architecture (inline decisions)

**Decision 1: Soft-delete via move-to-trash plus receipt, not store tombstone.**
We considered adding a `deleted_at` column and filtering at query time. Rejected: pollutes every query path with filter logic, doesn't help if someone nukes the store file, and makes trash inspection require a separate tool. Move-to-trash plus receipt is filesystem-native, greppable, and survives store corruption.

**Decision 2: Receipt format is JSON, not binary.**
Same rationale as activity log: operators should be able to cat and jq a receipt without special tooling. JSON for receipts adds some size overhead but trash is low-traffic.

**Decision 3: One receipt per operation, not one per artifact.**
If the same artifact is deleted, restored, deleted again, each call produces its own receipt. Restore picks the newest non-consumed. Makes the history inspectable — you can see how many times someone tried to delete a given artifact.

**Decision 4: Write receipt BEFORE lifecycle mutation, not after.**
Crash invariant: trash is the source of truth during the critical section. A crash between write_receipt and remove_from_store leaves orphan trash — harmless, purged by TTL. Reverse order would be fatal data loss.

**Decision 5: TTL purge runs lazily on invocation, not via background task.**
A background cron in an MCP server is added complexity and another failure mode. On-demand purge when restore or the first destructive op of a session runs is enough.

**Decision 6: Relations are captured in the receipt, not re-derived on restore.**
Re-deriving would require diffing the store against a snapshot. Capturing in receipt is O(1) and works even if the related artifacts were themselves deleted afterwards.

---

## Affected Files

- crates/forgeplan-core/src/undo/ (new module — receipt format, trash I/O, TTL purge)
- crates/forgeplan-core/src/lifecycle/ (wrap destructive ops with soft-delete)
- crates/forgeplan-mcp/src/server.rs (add restore and undo-last handlers; wire soft-delete into destructive handlers)
- crates/forgeplan-cli/src/commands/restore.rs (new)
- crates/forgeplan-cli/src/commands/undo.rs (new)
- crates/forgeplan-core/src/config/ (add undo.ttl_days field)
- crates/forgeplan-mcp/tests/undo_integration.rs (new)

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| PRD-054 | Depends on — soft-delete wires into activity log | Shipped |
| PROB-039 | Motivates — prompt-injection tail | Closed |

---

> **Next step**: approved PRD → immediate code phase.

