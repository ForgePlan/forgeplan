---
assigned_number: 90
author: null
created: 2026-10-28
depth: deep
domain: general / healthcare / fintech / govtech / edtech
epic: null
id: PRD-090
kind: prd
links:
- target: PROB-067
  relation: based_on
- target: SPEC-009
  relation: based_on
- target: RFC-019
  relation: based_on
- target: ADR-030
  relation: based_on
- target: ADR-012
  relation: informs
- target: RFC-009
  relation: informs
- target: SPEC-005
  relation: informs
parent_epic: null
predicted_number: 90
priority: P0 / P1 / P2 / P3
projectType: web_app / api_backend / mobile_app / cli_tool / library
slug: prd-collision-safe-artifact-identity-across-projects-and-git-workspaces
status: draft
stepsCompleted: []
tags: []
title: Collision-safe artifact identity across projects and Git workspaces
updated: 2026-10-28
valid_until: null
---

---
assigned_number: 90
created: 2026-10-28
domain: general / healthcare / fintech / govtech / edtech
epic: null
predicted_number: 90
priority: P0 / P1 / P2 / P3
projectType: web_app / api_backend / mobile_app / cli_tool / library
slug: prd-collision-safe-artifact-identity-across-projects-and-git-workspaces
stepsCompleted: []
updated: 2026-10-28
---

# PRD-090: Collision-safe artifact identity across projects and Git workspaces

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/0  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/0  (  0%)
```

---

## Executive Summary

### Vision

Every ForgePlan artifact carries a permanent, collision-free canonical key, so that a document is never selected, mutated, or linked by accident when two projects, branches, worktrees, or clones happen to assign the same display number.

### Problem

ForgePlan today stores a document's identity in its display id (`PRD-074`) and treats the ADR-012 slug as a title-derived alias. Both are derived from mutable, non-unique data:

- The slug is derived from the title, so two documents with the same title collide by construction.
- The display number is allocated from a counter that only reads the local tree, so two linked worktrees of one clone issue the same number for different documents (issue #552, PROB-067).
- The legacy index stores, filters, mutates, and deletes by `id` (`db/store.rs`, 14 filter/mutation sites), so a duplicate number silently merges two documents into one row.
- `WorkspaceRead` converts a canonical slug back into a display id and keeps a `by_slug` map that overwrites on collision; `get_record` then returns the last of several equal ids (last-write-wins).

Impact: a user who addresses a document by number may write to, link, score, or delete a different document. In the pilot corpus this is already realized in 16 groups across two projects (5 in ForgePlanWeb, 11 in ForgePlanMarketplace), each with two or more physically distinct documents claiming one number.

### Target Users

| Persona | Description | Key pain |
|---------|-------------|-----------|
| Solo maintainer working across worktrees | Uses linked worktrees to run several branches of one repository at once | Two `new` calls in two worktrees return the same number; the second document is unaddressable |
| Maintainer of several projects | Keeps independent ForgePlan workspaces per project | `PRD-001` exists in both; addressing by number is ambiguous across projects |
| Maintainer merging independent clones | Merges branches produced in separate clones | After merge, two documents share one number; a link or update lands on the wrong one |
| Reviewer / auditor | Reads and repairs a workspace | Cannot tell which document an inbound reference meant when numbers repeat |

### Differentiators

- The permanent key is random, not derived from title, number, path, PID, time, or content hash — so it does not collide when documents share a title.
- The key survives title, body, and status edits and is not rewritable through ordinary updates.
- Ambiguity is an explicit, machine-readable diagnostic rather than a silent first-match winner.
- The same resolution algorithm governs CLI, MCP, index reads, and the graph, so there is one read source per query.

### Contract gaps closed

The ADI analysis of this PRD (recorded as a Note linked to this artifact) raised four gaps in the contract. Each is closed here; SPEC-009 carries the mechanics.

1. **Backfill of the existing corpus.** No bulk backfill and no bulk renaming. Every existing artifact keeps the canonical key it already has: a unique slug when it has one, otherwise its stored display id. A new document gets a minted key; an imported or restored document keeps the key it arrives with. Only a document that genuinely shares its key with another document receives a new key, one per physical document, through the explicit repair plan. The consequence is deliberate: the corpus is a mix of legacy and new keys, and the resolver handles both by looking up the key first and the display id second.
2. **Exact-key duplicates.** Two files claiming one canonical key is a distinct error — `ConflictingCanonicalKey` — naming every claimant. It is not resolved by choosing one, and it is not a forward reference. This keeps the "one document set, one read source" rule: a duplicate key means the workspace is inconsistent and the operation is refused before anything is written.
3. **Scope of source-qualified addressing (FR-019, Journey 2).** The qualified address form `(source_id, canonical_key)` and the `KEY@qualifier` syntax already exist in the Constellation layer and stay. The new work in this PRD is the canonical-key half: the qualified address must resolve through the one shared resolver, and the qualified form must never return a document from the wrong source. Unqualified ambiguity across sources is reported, not resolved. Federation of separate clones into one logical project stays out of scope.
4. **Identity through export and restore.** An export carries the canonical key as a first-class field. A payload without that field — an older export, or an older receipt — is read by a compatible decoder that takes the key from the stored frontmatter and falls back to the legacy key; the absence of the field never mints a new key. An import preserves the key from the document. A restore preserves the key from the receipt and refuses to overwrite a different canonical document.

---

## Success Criteria

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | Two documents with identical titles receive different canonical keys and different filenames | Distinct key ratio for 1000 same-title pairs | 0% | 100% | This release | V1 scenario, asserted per pair |
| SC-2 | Two linked worktrees of one clone never issue the same display number for different documents | Duplicate numbers across worktrees | 1 observed (issue #552) | 0 | This release | V2 scenario: sequential and concurrent `new` |
| SC-3 | Addressing by a display number that matches two documents fails with an ambiguity diagnostic instead of mutating one | Documents mutated per ambiguous address | 1 (silently) | 0 | This release | V5 scenario: mutation count unchanged after the error |
| SC-4 | The legacy index stores identity by canonical key, not by display id | Index sites filtering by `id` | 14 | 0 | This release | Source audit of `db/` plus V6 read-after-write |
| SC-5 | Every collision group in the pilot corpus keeps all its documents | Documents lost in repair | unknown | 0 | This release | V10 census: source count before equals after |
| SC-6 | Independent clones may reuse a number without losing documents on merge | Documents lost after a real Git merge | 1 observed | 0 | This release | V3 scenario: real merge, both documents readable |
| SC-7 | Existing CLI and MCP inputs keep working on unambiguous input | Breaking changes to accepted arguments | 0 | 0 | This release | V6 matrix over 18 command surfaces |
| SC-8 | Semantic search returns the edited document for its new text, under the correct canonical key | Documents found by new unique text | n/a | 100% | This release | V11 with a real embedder, no manual embed |

---

## Product Scope

### MVP (In-Scope)

- A random canonical key minted once per new artifact, carried in frontmatter, and used as the storage key in the legacy index.
- One resolution algorithm over the fully parsed document set: exact canonical key, then normalized display id collecting all matches, then an explicit ambiguity error.
- Immutable identity: ordinary updates cannot change the canonical key, including through nested or external frontmatter.
- A shared per-kind allocation ledger in the Git common directory, so linked worktrees agree on the next number.
- Legacy index schema version 5 with canonical-key storage and a staged, atomic readiness marker.
- A repair path for the 16 accumulated collision groups that preserves every document and every historical display number.
- File naming by canonical key for newly created documents, with existing paths left in place.

### Out of Scope

- A central number server, a third identity field, or a global sequence of short numbers across independent clones.
- Renaming the historical corpus in bulk. Existing unique slugs are preserved; only a real collision gets a new key.
- Merging two independent clones into one logical project. `source_id` remains path-derived, as today.
- Rewriting historical prose, quotations, or commit messages that mention a document by number.
- Editing the retired lessons, Starlight accessibility attributes, or the full semantic revision of historical documentation.

### Growth Vision

- Federated lookup across many projects with a source-qualified address.
- A repair UI that previews a full inbound-reference census before applying any key change.

---

## User Journeys

### Journey 1: Solo maintainer across linked worktrees

**Goal**: Create a document in each of two worktrees without either document becoming unaddressable.

| Step | User action | System response | Notes |
|------|-------------|-----------------|-------|
| 1 | Runs `forgeplan new note "Weekly review"` in worktree A | Creates `NOTE-<n>` with a random canonical key; reserves the number in the shared ledger | Number recorded once, shared across worktrees |
| 2 | Runs the same command in worktree B | Reserves the next number, not the same one | The ledger is the shared accounting |
| 3 | Reads either document by its canonical key | Exactly one document is returned | Key is the storage identity |
| 4 | Reads either document by its display number | Exactly one document is returned | Numbers differ |

**Outcome**: Two documents, two numbers, two keys — no silent overwrite.

### Journey 2: Maintainer with two projects

**Goal**: Address documents that share a display number across projects.

| Step | User action | System response | Notes |
|------|-------------|-----------------|-------|
| 1 | Works in project A, where `PRD-001` exists | — | — |
| 2 | Works in project B, where `PRD-001` also exists | — | Same number, different project |
| 3 | Uses a source-qualified address | The requested source and exact body are returned | Qualification is part of the address |
| 4 | Uses an unqualified address in a federation scope | Both candidates are returned as an explicit ambiguity | No silent winner |

**Outcome**: Equal numbers in different projects are addressable, and unqualified ambiguity is visible.

### Journey 3: Reviewer repairing a historical collision

**Goal**: Give two physically distinct documents their own permanent keys without losing either.

| Step | User action | System response | Notes |
|------|-------------|-----------------|-------|
| 1 | Runs `forgeplan reconcile-ids --plan-out plan.json` | Writes a read-only plan with the full inventory digest and every inbound reference occurrence | Workspace untouched |
| 2 | Reviews the plan | Every proposed key change and reference occurrence is listed per source path and field | No title, number, or mtime is used as evidence |
| 3 | Runs `forgeplan reconcile-ids --apply-plan plan.json` | Re-checks the digest, then publishes each file atomically with a recovery journal | A stale plan is refused |
| 4 | Runs `forgeplan reconcile-ids --recover <txn> --rollback` if something is wrong | Restores the preimages | Refuses to overwrite unrelated edits |

**Outcome**: All documents survive with distinct keys and their historical display numbers intact.

---

## Functional Requirements

- [ ] FR-001: Artifact author can create a document that receives a permanent canonical key generated from 16 random bytes
- [ ] FR-002: System can keep the canonical key stable across title, body, and status edits
- [ ] FR-003: System can refuse an ordinary update that would change the canonical key
- [ ] FR-004: Reader can address a document by its exact canonical key
- [ ] FR-005: Reader can address a document by a normalized display id when exactly one document matches
- [ ] FR-006: Reader can receive an explicit ambiguity diagnostic listing every candidate when a display id matches more than one document
- [ ] FR-007: System can distinguish "no document matched" from "the input was ambiguous" from "the read failed"
- [ ] FR-008: System can allocate display numbers through a shared per-kind ledger so that linked worktrees of one clone agree
- [ ] FR-009: System can persist a number reservation before writing the file so a crash leaves a gap rather than a duplicate
- [ ] FR-010: Derived index can store, filter, upsert, and delete artifacts by canonical key
- [ ] FR-011: Derived index can be migrated to canonical-key storage atomically, publishing either a complete new index or the previous one
- [ ] FR-012: System can keep every relation, evidence link, phase state, claim, and receipt keyed by canonical key
- [ ] FR-013: Command line client can print the ambiguity candidates with canonical key, title, and relative path, and exit non-zero
- [ ] FR-014: Programmatic interface can return an ambiguity as a tool error with a machine-readable code, the input, and the candidates
- [ ] FR-015: Maintainer can produce a read-only repair plan listing every inbound reference occurrence per source path and field
- [ ] FR-016: Maintainer can apply a repair plan only when the workspace inventory digest, revision, and source hashes still match
- [ ] FR-017: Maintainer can roll back a repair transaction without overwriting unrelated edits made since
- [ ] FR-018: Repair can preserve every historical display number while assigning a distinct key to each physical document
- [ ] FR-019: System can resolve a source-qualified address within a federation scope to exactly one document
- [ ] FR-020: Successful responses can return the canonical key alongside the existing display id

---

## Requirement-to-journey traceability

| Journey | Requirements |
|---------|--------------|
| Journey 1 — Solo maintainer across linked worktrees | FR-001, FR-002, FR-003, FR-004, FR-005, FR-008, FR-009, FR-010, FR-011, FR-020 |
| Journey 2 — Maintainer with two projects | FR-005, FR-006, FR-007, FR-013, FR-014, FR-019 |
| Journey 3 — Reviewer repairing a historical collision | FR-012, FR-015, FR-016, FR-017, FR-018 |

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Performance | Resolution over a parsed document set shall complete without a full index scan per lookup | ≤ 1 pass over the parsed set | Every resolve call | Unit benchmark on a 500-document workspace |
| NFR-002 | Durability | A reserved number shall be persisted and fsynced before the document file is published | 0 reuse after a crash between reserve and write | Simulated crash injection | V2 scenario |
| NFR-003 | Correctness | Canonical key generation shall draw entropy from the OS source | 16 bytes per key, 0 keys derived from time, number, PID, path, or content hash | Every minted key | Source audit plus V1 forced-collision test |
| NFR-004 | Compatibility | Existing CLI and MCP arguments shall keep working on unambiguous input | 0 changed accepted inputs | Every command surface | V6 matrix |
| NFR-005 | Safety | A repair transaction shall refuse to apply against a changed workspace | 0 stale applications | Concurrent edit during apply | V9 scenario |
| NFR-006 | Observability | Every ambiguity diagnostic shall name the candidates and the reason | 100% of ambiguity errors carry candidates | Every ambiguous address | V5, V6 assertions |
| NFR-007 | Isolation | A single build lease shall hold for all Rust compilation in this repository | ≤ 1 concurrent Cargo process | Whole task | Process check before each build |

---

## Acceptance Criteria

### AC-1: Same-title documents get distinct keys

```gherkin
Given a workspace with no documents
When  two documents are created with the identical title, including a Cyrillic title
Then  the two documents have different canonical keys and different filenames
And   both titles and bodies are preserved byte for byte
And   with injected identical entropy, no existing document byte is overwritten
```

### AC-2: Linked worktrees never reuse a number

```gherkin
Given two linked worktrees of one clone at the same commit
When  a document is created sequentially in each, and then concurrently in each
Then  the reserved display numbers differ and the canonical keys differ
And   a crash between reservation and file write leaves the number unused
```

### AC-3: Independent clones merge without document loss

```gherkin
Given two independent clones with no shared ledger that create a document with the same title and number
When  the branches are merged with Git
Then  both documents survive, are independently readable and updatable by their canonical keys
And   a display-only selector returns both candidates and writes nothing
```

### AC-4: Source-qualified addressing is exact

```gherkin
Given two projects that both hold PRD-001 and a federation scope holding a copy of one canonical key
When  a qualified get is issued
Then  the requested source and the exact body are returned
And   an unqualified get over the same scope reports ambiguity explicitly
And   the source graph keeps both vertices
```

### AC-5: Legacy selector routes never pick a winner silently

```gherkin
Given a workspace with exact-key, lowercase-display, missing, duplicate-display, duplicate-key, and numeric-shadow cases
When  each is resolved through every legacy route
Then  no first-match, last-match, or freshest-file winner is chosen
And   absence is distinguishable from ambiguity and from a read error
And   no file, index row, or claim is mutated before the error
```

### AC-6: Existing command surfaces keep working

```gherkin
Given the CLI and MCP surfaces for new, get, update, link, unlink, list, search, score, validate, activate, delete, restore, export, and import
When  each runs on an unambiguous input
Then  the same arguments are accepted as before
And   two documents sharing one display number never mix body, relations, state, score, or embedding
```

### AC-7: Identity is immutable and restorable

```gherkin
Given a document with a canonical key
When  its title, body, and nested metadata change, and it is exported without the key field, and it is deleted and restored
Then  the canonical key is preserved
And   an attempt to substitute the key through an update is rejected
And   a restore does not overwrite a different canonical document
```

### AC-8: Legacy index migration is atomic

```gherkin
Given a schema-4 workspace with nested slugs, two display aliases, and old evidence
When  the v5 migration is interrupted at each stage and re-run
Then  either a complete v5 index or the preserved v4 index exists, never a partial set
And   the readiness marker never promises a partial set
And   document counts, edges, and embeddings are reconciled
And   a constellation layout creates no legacy index tables
```

### AC-9: Repair is safe and reversible

```gherkin
Given a repair plan over a workspace with collision groups
When  the plan is produced repeatedly, then a source, HEAD, reference, or destination changes, then a symlink or traversal target is introduced, then a crash, rollback, and restart occur
Then  producing the plan does not change the workspace
And   a stale or unsafe plan is refused
And   the original bytes and paths are restored
And   a concurrent unrelated edit is not overwritten
```

### AC-10: The pilot corpus keeps every document

```gherkin
Given copies of the ForgePlanWeb and ForgePlanMarketplace workspaces with their collision groups
When  the repair is applied to the copies
Then  every source document is preserved
And   canonical collisions are zero
And   the reference census reconciles exactly
And   unresolved structural references on remapped keys are zero before apply
And   both projects answer reads and writes
```

### AC-11: Semantic search tracks the edited document

```gherkin
Given a configured semantic index
When  a unique text is edited through the CLI and then through MCP, with no manual embed
Then  the new text is found by meaning under the correct canonical key
And   the old text is not returned as its current version
And   the embedding backlog is actually drained
```

### AC-12: Damaged state fails loudly

```gherkin
Given an empty, missing, or damaged ledger, a u32 maximum, a missing worktree config, an unfinished repair, and a long-lived MCP handle after a migration or reindex
When  each condition is exercised
Then  the result is an explicit diagnosable error or a documented recovery
And   no silent number reuse, no partial success, and no write into the wrong tree occurs
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| ADR-012 (slug-canonical identity) | Decision | Active — partially superseded by ADR-030 | ForgePlan |
| SPEC-005 (frontmatter contract) | Contract | Draft — extended by SPEC-009 | ForgePlan |
| RFC-009 (lazy ID assignment rollout) | Plan | Active — completed by its own phases | ForgePlan |
| PROB-067 (ID counter race) | Problem | Active — closed by this work | ForgePlan |
| ADR-029 (Constellation source model) | Decision | Active — unchanged by this work | ForgePlan |
| Issue #552 (linked-worktree duplicate numbers) | Defect | Open — closed by this work | ForgePlan |

---

## Rollback Plan

**Triggers**: the migrated index cannot be verified against the document set; the readiness marker is damaged; a repair transaction cannot be completed or rolled back; a regression appears on an existing command surface.

**Steps**: stop the writers; restore the document preimages through the repair journal; restore the corresponding index generation; return to the previous command-line and programmatic-interface pair together.

**Blast Radius**: the derived index and the document paths a repair touched. No document is lost by a rollback, because the preimages are complete and fsynced before the first mutation.

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | Renaming historical filenames breaks external references | Medium | High | New files only get canonical names; existing paths stay; the repair plan lists every inbound occurrence before apply | ForgePlan |
| R-2 | Key migration leaves the index inconsistent with the Markdown | Medium | Critical | Build the v5 tables from Markdown, verify a full digest, and publish atomically via a readiness marker | ForgePlan |
| R-3 | Random keys collide or look unrecognizable to users | Low | Medium | 128 bits of entropy plus a readable title part; the display id stays the human-facing handle | ForgePlan |
| R-4 | Repair picks the wrong document for an ambiguous inbound reference | Medium | Critical | Refuse to apply while any unresolved structural reference targets a remapped key; the owner resolves it | ForgePlan |
| R-5 | Parallel worktrees corrupt the shared ledger | Low | High | Single per-kind lock in the Git common directory, fsync before publish, and a documented recovery on a damaged ledger | ForgePlan |
| R-6 | The change exceeds the disk and single-build budget | Medium | Medium | One build lease, sequential verification, and a measured cleanup of reproducible build outputs | ForgePlan |

---

## Timeline

| Milestone | Target Date | Description |
|-----------|-------------|-------------|
| Contract frozen | 2026-10-29 | PRD, SPEC, RFC, ADR written, validated, and reasoned |
| Core foundation | 2026-10-30 | Identity types, minting, resolver, canonical read model |
| Slices integrated | 2026-10-31 | Creation, legacy storage, recovery, and public adapters integrated |
| Pilot repair | 2026-11-01 | 16 collision groups repaired on the pilot copies |
| Originals connected | 2026-11-02 | Web and Marketplace connected after a reviewed repair |

---

## Stakeholders

| Role | Name | Sign-off |
|------|------|----------|
| Product Owner | Eli Rum | [ ] |
| Engineering Lead | | [ ] |
| QA | | [ ] |

---

## Affected Files

- `crates/forgeplan-core/src/artifact/**`
- `crates/forgeplan-core/src/constellation/**`
- `crates/forgeplan-core/src/db/**`
- `crates/forgeplan-core/src/projection/**`
- `crates/forgeplan-cli/src/commands/**`
- `crates/forgeplan-mcp/src/**`

---

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| SPEC-009 | API contracts | Draft |
| RFC-019 | Architecture proposal | Draft |
| ADR-030 | Decision record | Draft |
| PROB-067 | Problem | Active |
| SPEC-005 | Superseded in part | Draft |
| RFC-009 | Historical rollout | Active |
| ADR-012 | Partially superseded | Active |
