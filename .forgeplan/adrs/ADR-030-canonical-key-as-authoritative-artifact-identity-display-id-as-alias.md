---
assigned_number: 30
author: null
created: 2026-10-28
depth: deep
id: ADR-030
kind: adr
links:
- target: PRD-090
  relation: based_on
- target: SPEC-009
  relation: based_on
- target: ADR-012
  relation: supersedes
- target: ADR-029
  relation: informs
- target: PROB-067
  relation: based_on
- target: RFC-019
  relation: informs
- target: SPEC-005
  relation: refines
parent_epic: null
predicted_number: 30
problem_ref: null
slug: adr-canonical-key-as-authoritative-artifact-identity-display-id-as-alias
status: draft
tags: []
title: Canonical key as authoritative artifact identity, display id as alias
updated: 2026-10-28
valid_until: null
---

---
assigned_number: 30
created: 2026-10-28
predicted_number: 30
problem_ref: null
slug: adr-canonical-key-as-authoritative-artifact-identity-display-id-as-alias
updated: 2026-10-28
---

# ADR-030: Canonical key as authoritative artifact identity, display id as alias

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/0  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/0  (  0%)
```

---

## Context

ADR-012 established a two-layer identity: a slug that is canonical and a display number assigned at merge. That decision is sound in principle and still holds. What has drifted is the implementation of two parts of it.

First, the slug is not actually permanent. It is derived from the title (`artifact/types.rs:390-450`, `slug_from_kind_title`), so two documents with the same title necessarily collide, and a title edit rewrites the slug. ADR-012's premise — that the slug is the canonical key — is therefore not upheld by the code.

Second, the display id is used as the storage key rather than as an alias. The legacy index filters, upserts, updates, and deletes by `id = '...'` in fourteen places (`db/store.rs`), and the slug is stored only inside the artifact body. `WorkspaceRead` (`constellation/workspace_read.rs`) converts a canonical slug back into a display id, keeps a `by_slug` map that overwrites on collision, and resolves a repeated display id by taking the last record. So the canonical key exists in the constellation layer (`capture.rs:971-975` derives it from the frontmatter slug) and is thrown away one layer up.

The consequence is measured, not theoretical. Issue #552: two linked worktrees of one clone, sharing the Git common directory and therefore the per-kind allocator lock, produced two different documents both numbered NOTE-002 — because the lock protects an accounting that reads only the local tree (`artifact/id_alloc.rs:214-266`). And the pilot corpus already holds 16 collision groups: five in ForgePlanWeb (EVID-020, EVID-021, PRD-001, PRD-016, RFC-015) and eleven in ForgePlanMarketplace (EVID-119, EVID-133, EVID-141, EVID-143, EVID-146, EVID-150, EVID-153, evid-adr-027-2, NOTE-008, PRD-072, SPEC-001). In each group two or more physically distinct documents claim one number.

ADR-029 declines to add a canonical artifact UUID, and that decision stands. The Constellation source and checkout model of ADR-029 is unaffected: the address stays `(source_id, canonical_key)`, and `source_id` remains path-derived.

## Decision

Complete the two-layer model of ADR-012 rather than introduce a third identity.

- The **canonical key** is the permanent identity of a document. In code and in the index it is called `canonical_key`; in the artifact frontmatter it is the `slug` field, as ADR-012 already defines.
- The **display id** (`id`, `assigned_number`) is a human-facing alias that may repeat within a workspace and across projects.
- No UUID field, no central number server, no branch identifier inside the key is added.

The canonical key of a new document is `<kind>-<title-part>-k<32 lowercase hex>`. The suffix is 16 random bytes from `OsRng.try_fill_bytes`; a failure of the entropy source aborts the creation before any write. The key never derives from time, number, process id, path, or a content hash. Uniqueness is probabilistic at 128 bits, not a mathematical guarantee, and that is accepted.

Existing unique slugs are preserved; the historical corpus is not renamed in bulk. A legacy document without a slug keeps its previous canonical key. Only a real collision receives a new key, one per physical document, through an explicit repair plan.

New files are named `<canonical_key>.md`. The title and the number do not determine the path. Existing files keep their paths; a change to title, status, or body renames nothing and changes no key.

Creation and restoration are different operations. Creating a new artifact — `new`, generate, capture, ingest, promote, `reason --save`, `remember`, the core lifecycle and playbook writers, and the MCP creators — mints a key. Import and restore preserve the key from the full document or the receipt. An ordinary update cannot change the key, including through nested or external frontmatter; only a verified identity-repair operation may.

A copy of one document in two branches keeps its key, and conflicting edits produce an ordinary Git conflict. Two independently created documents with the same title and number have different keys and different filenames.

In Constellation the address stays `(source_id, canonical_key)` and the existing `KEY@qualifier` form is kept. The same number in different projects is admissible; a result shows its source and key. Independent clones are not merged into one logical project.

Globally consecutive short numbers are not promised to independent clones. Between linked worktrees of one clone the numbers agree through a shared ledger. After independent clones are merged, a repeated number is admissible and must not lose documents; a command addressed by such a number requires the permanent key.

**Why Selected**: it completes a decision already taken instead of replacing it; it aligns the legacy layer with the constellation layer, which already keys correctly; and it keeps the human-facing number readable while making the permanent key unambiguous by construction.

## Alternatives Considered

| Option | Verdict | Why |
|--------|---------|-----|
| A. Add a UUID field beside id and slug | Rejected | Contradicts ADR-029; detects collisions without fixing them — two documents still share one row, one score, one vertex |
| B. Make the display number globally unique | Rejected | Destroys readability of the human-facing id, and still fails across independent clones where no shared allocator exists |
| C. Canonical key authoritative, display id as alias | **Chosen** | Completes ADR-012; one identity, no third field, ambiguity becomes explicit |
| D. Keep the title-derived slug, detect collisions only | Rejected | The slug would remain title-derived, so the most common collision — same title — stays unresolved |

## Consequences

### Positive

- Addressing a document by its permanent key is exact, in any workspace, in any branch, in any clone.
- Two documents sharing a number can no longer be merged into one row, one score, or one graph vertex.
- Ambiguity becomes a machine-readable diagnostic instead of a silent first-match win.
- The linked-worktree race is closed by shared accounting, not by a shared lock over local accounting.
- The legacy index and the constellation index finally agree on what identity means.

### Negative (trade-offs)

- A new identity field touches the storage schema, the index filters, the relations, the graph, the sidecars, and every creation site — the largest single change to the identity model since ADR-012.
- Historical documents keep title-derived slugs, so the corpus is a mix of legacy and new keys until a repair touches a group. The resolver handles both.
- The display id alone is no longer sufficient to address a document in a workspace that holds a collision. The key must be exposed to users and agents.
- A random key is less memorable than a title-derived slug; the title part mitigates this.

### Risks

- A migration that publishes a partially rebuilt index would make the index disagree with the Markdown. Mitigation: build from Markdown, verify a full digest, publish atomically behind a readiness marker.
- A repair that guesses the target of a conflicting inbound reference would silently re-point a relation. Mitigation: refuse the apply while any unresolved structural reference targets a remapped key.
- Parallel worktrees could corrupt the shared ledger. Mitigation: one per-kind lock in the Git common directory, fsync before publish, an explicit error on a damaged ledger.

## Invariants

- A document's canonical key never changes through an ordinary update, including through nested or external frontmatter.
- Every canonical key is opaque and immutable. A key **newly minted** by this design is derived from the operating-system random source plus the kind and title parts; a **grandfathered** legacy key keeps the value it already had. Both are treated identically by every read and write path — no code branches on how a key was produced.
- A display id may repeat; a canonical key may not, within a workspace.
- Resolution never returns a "first match", a "last match", or a "freshest file". Ambiguity is an error.
- "Not found", "ambiguous", and "read error" are three distinct outcomes and never collapse into one.
- The legacy index filters, upserts, updates, and deletes by canonical key only.
- One document set, one read source: no query mixes the legacy index and the capture.
- A repair never deletes a document, never merges content, and never rewrites a historical display number.

## Evidence Requirements

- V1 — two same-title documents get distinct keys and filenames; a forced entropy collision overwrites no byte.
- V2 — two linked worktrees, sequential and concurrent, reserve different numbers and keys; a crash between reserve and write leaves the number unused.
- V3 — two independent clones, same title and number, a real Git merge, both documents survive and are addressable by key.
- V4 — a qualified get across two projects returns the requested source and body; the unqualified form reports ambiguity; the source graph keeps both vertices.
- V5 — every legacy selector route: exact key, lowercase display, missing, duplicate display, duplicate key, numeric shadow — no silent winner, and no mutation before an error.
- V6 — the full CLI and MCP surface works on unambiguous input, and two documents sharing a number never mix body, relations, state, score, or embedding.
- V7 — the key survives title, body, nested metadata, export without the key, and delete/restore; an update that tries to substitute the key is rejected.
- V8 — the schema-4 to schema-5 migration interrupted at each stage leaves either a complete v5 or the preserved v4, never a partial set.
- V9 — the repair preview is read-only; a stale or unsafe plan is refused; a crash and rollback restore the bytes; a concurrent edit is not overwritten.
- V10 — the 16 pilot collision groups: every document preserved, zero canonical collisions, exact reference census, zero unresolved structural references before apply.
- V11 — the semantic index finds the edited text under the right key, with no manual embed, and the backlog is drained.
- V12 — a damaged ledger, a u32 maximum, a missing worktree config, an unfinished repair, and a long-lived handle after a migration each yield an explicit error or a documented recovery.

## Valid Until

**Date**: `2027-04-28`

**Rationale**: the decision changes the storage schema and the identity model of every artifact, so it must be revisited when the corpus has fully migrated, when a federation spanning many projects is exercised, or when the key format itself shows a defect.

**Refresh Triggers**:
- a real collision occurs between two documents that both carry minted keys — the format's uniqueness assumption failed;
- the legacy schema-4 index is retired and the staged migration has served every workspace;
- a federation or a multi-project deployment reaches a scale where `source_id` path-derivation becomes a measured defect;
- the number space is exhausted for a kind in a large workspace.

## Pre-conditions (checklist BEFORE implementation)

- [ ] The SPEC scenarios V1–V12 are frozen and every scenario names an observable result.
- [ ] The owner of each shared file is recorded: `db/store.rs`, `projection/mod.rs`, `mcp/server.rs`, `artifact/mod.rs`.
- [ ] A census of `NewArtifact` builders, `create_artifact_with_projection`, `import_artifact_with_projection`, and the `allocate_and_create_artifact*` functions is taken.
- [ ] The source-ownership map for the repair is written, and the reference census method is agreed.
- [ ] One build lease and a disk budget of at least 10 GB are confirmed.
- [ ] The native CLI used to author these artifacts is the verified binary, and its SHA-256 is recorded.

## Post-conditions (Definition of Done)

- [ ] V1–V9 and V12 pass on both storage layouts.
- [ ] The legacy index no longer filters by `id` anywhere.
- [ ] Every creation site uses the shared mechanism or the preserve-identity path.
- [ ] V10 passes on the pilot copies; the originals are untouched until a separate reviewed apply.
- [ ] V11 passes with a real embedder; the binary and source pins match.
- [ ] An independent review closes every finding; the gate is not declared passed on a test count alone.
- [ ] The methodology page describes the contract in force, and the handoff records the final state, the remaining blockers, and the path to the evidence.

## Admissibility

- NOT: a UUID field, a central number server, or a branch identifier inside the key.
- NOT: resolving an ambiguous reference by picking the first, the last, the freshest, or the one whose title looks closest.
- NOT: writing to an index or a file while the document set is only partially parsed.
- NOT: deleting or merging a document during a repair.
- NOT: rewriting a historical display number, a quotation, or a commit message by mass substitution.
- NOT: treating an ambiguous reference as a forward reference, or as evidence for an R_eff score.
- NOT: running two Cargo processes at once, or editing source while a build runs.
- NOT: activating the artifacts of this task before the guardian gate passes.

## Rollback Plan

**Triggers**: the v5 index cannot be verified against the Markdown; the readiness marker is damaged; a repair transaction cannot be completed or rolled back; a regression appears in an existing CLI or MCP surface.

**Steps**:
1. Stop the writers.
2. Restore the canonical preimages through the repair journal.
3. Restore the corresponding index generation.
4. Return to the previous CLI and MCP pair together.

**Blast Radius**: the legacy index and the document paths a repair touched. No document is lost by a rollback, because the preimages are complete and fsynced before the first mutation.

## Weakest Link

The weakest link is the repair step, not the key format. Minting 128 random bits and storing by key are both straightforward; attributing a conflicting inbound reference to the right physical document is a judgement call about historical intent, and the corpus proves that titles, numbers, and timestamps do not settle it. The design therefore refuses to guess and escalates to the owner. If the reference census is not measured exhaustively before the apply, this decision's evidence chain is weakest at exactly that point.

## Affected Files

| File | Baseline Hash |
|------|---------------|
| `crates/forgeplan-core/src/artifact/canonical_key.rs` | new — see note below |
| `crates/forgeplan-core/src/artifact/id_alloc.rs` | pending |
| `crates/forgeplan-core/src/artifact/store.rs` | pending |
| `crates/forgeplan-core/src/artifact/frontmatter.rs` | pending |
| `crates/forgeplan-core/src/constellation/workspace_read.rs` | pending |
| `crates/forgeplan-core/src/db/store.rs` | pending |
| `crates/forgeplan-core/src/db/schema.rs` | pending |
| `crates/forgeplan-core/src/db/convert.rs` | pending |
| `crates/forgeplan-core/src/projection/mod.rs` | pending |
| `crates/forgeplan-cli/src/commands/reconcile_ids.rs` | pending |
| `crates/forgeplan-cli/src/commands/ci_assign_id.rs` | pending |
| `crates/forgeplan-mcp/src/server.rs` | pending |

**Module path correction.** The implementation plan named the new module
`crates/forgeplan-core/src/artifact/identity.rs`. That path is already taken:
it holds `AgentIdentity` — the identity of the *caller* that last mutated an
artifact, tracked since commit `72975834` for PROB-066. Document identity and
caller identity are different concepts, and merging them would conflate "who
changed this" with "what this document is". The new module is therefore
`crates/forgeplan-core/src/artifact/canonical_key.rs`, registered as
`pub mod canonical_key;`. The existing `identity.rs` is untouched, and every
public type and signature named by this ADR keeps its exact name — only the
file path differs.

## AI Guidance

- Treat the canonical key as the identity in every new code path; the display id is an alias for humans.
- Never resolve a reference by first match, last match, or freshest file. Collect all candidates and report ambiguity.
- Do not add a UUID, a central allocator, or a branch-scoped key without a new ADR.
- Do not rename existing files or bulk-rewrite historical references.
- When a task would change a canonical key, route it through the repair plan rather than editing files directly.
- When the shared allocator lock is held, do not take another tree's workspace lock.
- If a scenario in SPEC-009 is unclear, raise it explicitly rather than choosing a winner.

## Implementation Plan

### Phase 0: Contract
- [ ] **0.1** PRD-090, SPEC-009, RFC-019, ADR-030 authored, validated, and reasoned.
- [ ] **0.2** Interface freeze and the file-ownership map recorded.

### Phase 1: Core foundation
- [ ] **1.1** Identity types, minting, and the resolver.
- [ ] **1.2** `NewArtifact` and `ArtifactRecord` carry `canonical_key`.
- [ ] **1.3** Canonical `WorkspaceRead` and graph by key.
- [ ] **1.4** Behavioural tests for every new public function.

### Phase 2: Slices
- [ ] **2.A** Creation, allocation ledger, immutable identity, preserve-identity import.
- [ ] **2.B** Legacy storage schema v5, staged readiness, canonical filters.
- [ ] **2.C** Repair plan, apply, recovery journal, CI numbering.
- [ ] **2.D** Unified CLI and MCP selector and error model.

### Phase 3: Verification
- [ ] **3.1** Independent review of the frozen diff.
- [ ] **3.2** Sequential project gates plus the real CLI and MCP processes for V1–V12.
- [ ] **3.3** Documentation, guide, and handoff.
- [ ] **3.4** Guardian gate; activation as a separate action.

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| PRD-090 | PRD | based_on |
| SPEC-009 | SPEC | based_on |
| RFC-019 | RFC | based_on |
| PROB-067 | Problem | based_on |
| SPEC-005 | SPEC | refines |
| ADR-012 | ADR | supersedes_in_part |
| ADR-029 | ADR | informs |
