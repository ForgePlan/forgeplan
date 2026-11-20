---
assigned_number: 19
author: null
created: 2026-10-28
depth: deep
id: RFC-019
kind: rfc
links:
- target: PRD-090
  relation: based_on
- target: SPEC-009
  relation: based_on
- target: ADR-030
  relation: based_on
- target: RFC-009
  relation: informs
- target: ADR-012
  relation: informs
- target: PROB-067
  relation: based_on
- target: SPEC-005
  relation: informs
parent_epic: null
prd: null
predicted_number: 19
slug: rfc-implementation-of-canonical-key-artifact-identity
status: draft
tags: []
title: Implementation of canonical-key artifact identity
updated: 2026-10-28
valid_until: null
---

---
assigned_number: 19
created: 2026-10-28
prd: null
predicted_number: 19
slug: rfc-implementation-of-canonical-key-artifact-identity
updated: 2026-10-28
---

# RFC-019: Implementation of canonical-key artifact identity

## Progress

```
Phase 1  ░░░░░░░░░░░░░░░░░░░░░░░░  0/0  (  0%)
Phase 2  ░░░░░░░░░░░░░░░░░░░░░░░░  0/0  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/0  (  0%)
```

---

## Summary

Introduce a permanent, randomly minted canonical key as the authoritative identity of every ForgePlan artifact, make the legacy index store and mutate by that key, and add an explicit repair path for the collisions already present in the corpus.

## Motivation

The measured defect (issue #552) is that two linked worktrees of one clone issue the same display number for two different documents. The cause is not the lock: the per-kind allocator lock in the Git common directory is shared, but the accounting it protects (`artifact/id_alloc.rs:214-266`) reads only the local tree and the local index. A shared lock without shared state is not shared allocation.

The deeper cause is that the display number is treated as identity everywhere. The slug is title-derived, the filename is `<id>-<title-slug>.md`, the legacy index filters on `id = '...'` in fourteen places, and `WorkspaceRead` converts a canonical slug back into a display id and keeps a `by_slug` map that overwrites silently on collision. Two documents with one number therefore become one row, one graph vertex, and one subject of a write.

What happens if we do nothing: the pilot corpus keeps 16 collision groups (5 in ForgePlanWeb, 11 in ForgePlanMarketplace), any future merge of independent clones adds more, and a user addressing a document by number can write to, link, score, or delete the wrong document with no error shown.

## Goals

- One permanent key per document, minted once, stable across title, body, and status edits.
- One resolution algorithm shared by the CLI, the MCP layer, the index, and the graph, with ambiguity as an explicit outcome.
- The legacy index stores, filters, upserts, and deletes by canonical key.
- Linked worktrees of one clone agree on the next display number.
- The 16 accumulated collision groups can be repaired with every document preserved.
- Existing CLI and MCP inputs keep working on unambiguous input.

## Non-Goals

- A central number server, a third identity field, or a global sequence across independent clones.
- Bulk renaming of the historical corpus. Existing unique slugs are preserved.
- Merging independent clones into one logical project; `source_id` stays path-derived.
- Rewriting historical prose, quotations, or commit messages.
- Changing the Constellation source/checkout model of ADR-029.

## Options Considered

### Option A: Add a UUID field alongside id and slug

**Description**: Add a `uuid` column and a `uuid:` frontmatter field; keep `id` as the storage key; use the UUID only to detect collisions.

**Pros**: No migration of the storage layer; the existing index keeps working.

**Cons**: Two documents that share an id still share one row, one score, and one vertex — the collision is detected but not fixed. Every read path still has to decide between two identities. A third identity contradicts ADR-029, which explicitly declines to add a canonical artifact UUID.

### Option B: Make the display number globally unique by construction

**Description**: Widen the number space with a per-project prefix or a random suffix on the display id itself.

**Pros**: No new concept; the display id stays the only identity.

**Cons**: The display id is human-facing; polluting it with entropy destroys readability. It also fails across independent clones, where no shared allocator exists at all — the very case that must be safe.

### Option C: Canonical key as authoritative identity, display id as alias

**Description**: Keep the two-layer model of ADR-012. The slug becomes the permanent key, minted with 16 random bytes instead of being title-derived. The legacy index stores and mutates by that key. The display id becomes an alias that may repeat and is resolved to a candidate set.

**Pros**: Completes ADR-012 instead of contradicting it; the Constellation layer already keys by `canonical_key` (`capture.rs:971-975` derives it from the frontmatter slug), so the change aligns the legacy layer with the layer that already has it right. No third identity. Display id stays human-readable.

**Cons**: Requires a staged index migration and a repair path for the existing corpus.

### Option D: Keep the title-derived slug, add collision detection only

**Description**: Leave the slug as-is; report collisions when detected.

**Pros**: Smallest change.

**Cons**: The slug remains title-derived, so two documents with the same title collide by construction — the most common case, not the rare one. Detection without a resolution path still leaves the write ambiguous.

**Decision: Option C.** It is the only option that makes the address unambiguous by construction while keeping the existing two-layer model and the human-facing number.

## Design

### Identity types

A new core module `crates/forgeplan-core/src/artifact/identity.rs` carries the contract types:

```rust
pub struct ArtifactIdentity {
    pub canonical_key: String,
    pub display_id: String,
}

pub struct ResolvedArtifact {
    pub identity: ArtifactIdentity,
    pub relative_path: PathBuf,
}

pub struct ArtifactCandidate {
    pub canonical_key: String,
    pub display_id: String,
    pub title: String,
    pub relative_path: PathBuf,
}

pub enum ResolveError {
    Ambiguous { input: String, candidates: Vec<ArtifactCandidate> },
    ConflictingCanonicalKey { key: String, candidates: Vec<ArtifactCandidate> },
    InvalidSelector { input: String },
    IncompleteCapture { message: String },
}

pub fn mint_artifact_key(kind: &ArtifactKind, title: &str) -> anyhow::Result<String>;
```

`WorkspaceRead::resolve_artifact` and the file-level `artifact::store::resolve_artifact` share one algorithm over the fully parsed document set. I/O, parse, and limit errors surface as errors, never as "not found". A partial capture stays a diagnostic result; writing against it is forbidden.

### Key minting

The key is `<kind>-<title-part>-k<32 lowercase hex>`. The entropy comes from `rand` 0.9 `OsRng.try_fill_bytes` — 16 bytes, rendered as 32 hex characters. A failure of the entropy source aborts the creation before any write. The key is minted once, before any attempt to reserve a display number, so an internal retry cannot change identity.

The title part reuses the existing `slugify` and `validate_slug`. The total key length is at most 80 characters; only the title part is truncated, at a hyphen boundary. An empty ASCII result — which a Cyrillic title produces — becomes the word `document`. The `k` prefix guarantees the key is never numeric-only.

Existing unique slugs are preserved. A legacy document without a slug keeps its previous canonical key. Only a real collision receives a new key, one per physical document, decided by an explicit repair plan.

### Storage

`NewArtifact` and `ArtifactRecord` gain a required internal `canonical_key: String`. A single creation function prepares the key before constructing `NewArtifact`; the lower-level create and import paths require a prepared non-empty key and never re-derive one from the display id. Old export and receipt payloads are read by a compatible decoder that prefers the slug from the stored frontmatter and falls back to the legacy key; a missing field in an old payload never causes a new key to be minted.

`WorkspaceRead` stores records, outgoing and incoming relations, and scores by canonical key. Display aliases become a separate multimap. `get_record_by_key` and `get_relations_by_key` replace the ambiguous internal `resolve_id → display_id` conversion, and every caller migrates in one delivery. The graph uses the key as the node id and labels the node with the number and the title; equal display ids do not merge vertices.

The legacy index moves to schema version 5: the `artifacts` table gains `canonical_key`; relations keep their `source_id` and `target_id` fields but hold keys; evidence links are rebuilt with the same resolver. The public `id` stays the display id.

### Number allocation

`artifact/id_alloc.rs` keeps the existing per-kind lock in the Git common directory and adds `forgeplan/id-<KIND>.json` beside it, holding a schema version and a high watermark. Under the lock the next number is the maximum of the ledger, the current tree's canonical frontmatter, the frontmatter of every initialized linked worktree of the same common directory — discovered with `git worktree list --porcelain -z` — and any admissible legacy index. The reservation is persisted and fsynced before the file is created, so a failure leaves a gap rather than a duplicate. A `u32` overflow is an error. Lock order is workspace mutation lock, then per-kind allocator lock.

### Repair

`reconcile-ids` gains `--plan-out`, `--apply-plan`, and `--recover <id> --rollback`; the existing safe operations and `--check-only` remain. The plan is version 1 and carries the workspace identity, the Git HEAD and common dir, a full inventory digest, per-source path and file/body/frontmatter hashes, old and new keys, the proposed path, the exact reference occurrences, and the mapping. Before the apply, the workspace identity, the inventory, the HEAD, the source and reference hashes, the group membership, and the absence of the target paths are re-checked; any change makes the plan stale. The transaction keeps a journal under `.forgeplan/identity-repair/<transaction-id>/` with the manifest, the preimages, the planned post-image hashes, and per-operation state.

The first application is preserve-all: every physical document keeps its bytes and its historical display number, and each candidate for one key gets its own new key. Conflicting inbound references are never guessed.

## Implementation Phases

### Phase 1: Core foundation

| Task | Owner | Files |
|------|-------|-------|
| 1.1 Identity types, minting, resolver | Core owner | `artifact/identity.rs`, `artifact/mod.rs` |
| 1.2 `NewArtifact`/`ArtifactRecord` gain `canonical_key` | Core owner | `db/store.rs` |
| 1.3 Canonical `WorkspaceRead` and graph | Core owner | `constellation/workspace_read.rs`, `constellation/graph.rs` |
| 1.4 Behavioural tests for every new public function | Core owner | `artifact/identity.rs` tests, `constellation/workspace_read.rs` tests |

### Phase 2: Independent slices

Each slice has exactly one owner and its own worktree from one frozen input.

| Slice | Ownership | Result |
|-------|-----------|--------|
| A. Creation and writes | `artifact/id_alloc.rs`, `artifact/frontmatter.rs`, `projection/`, core creation and lifecycle producers | Shared allocator, immutable identity, safe filenames, preserve-identity import boundary |
| B. Legacy storage | `db/`, `scan/`, legacy index and bootstrap integration | Schema v5, staged readiness, canonical filters and relations, old commands without alias collision |
| C. Recovery and CI numbering | new `artifact/identity_repair.rs`, `commands/reconcile_ids.rs`, `commands/ci_assign_id.rs` | Preview, apply, recovery journal, frontmatter-aware numbering, no suffix or no-op workaround |
| D. Public adapters | `commands/common.rs` and the remaining CLI consumers, `forgeplan-mcp/src/server.rs` | Unified selector and error model, source-qualified read, canonical writes, compatible old inputs |

Shared files have a single owner: `undo/restore.rs` belongs to A; `commands/migrate.rs`, `reindex`, and `scan_import` belong to B; the remaining CLI creators belong to D against the core API from A; `artifact/mod.rs` and the shared exports belong to the integrator. D may prepare adapters and fixtures against the frozen signatures but does not deliver before A and B land. C uses the A and B APIs and does not copy their implementation.

### Phase 3: Integration, review, verification

An independent reviewer reads the full diff, the SPEC scenarios, and the migration and recovery logic, and reports findings with file:line and a verdict. It does not fix code. One verification owner runs the project gates sequentially on the frozen source and exercises the real CLI and MCP processes. A documentation owner updates the methodology page, adds a reproducible guide, and writes the handoff. A guardian checks the whole chain and renders the binary verdict.

## Resolved ambiguities

The ADI analysis of this RFC (recorded as a Note linked to this artifact) raised four material ambiguities. Each is closed here; the mechanics live in SPEC-009.

1. **Does "preserve the bytes" exclude the frontmatter?** No. The frontmatter is part of the document's bytes and is rewritten in place: the canonical key changes inside it, everything else in it stays. The preimage kept by the recovery journal is the complete file, frontmatter included, so a rollback restores the exact bytes. What the repair never rewrites is the document's *body*, its title, its status, and its historical display number.

2. **Do all collision candidates get remapped, or does one keep the old key?** All of them. When two physical documents claim one canonical key, every claimant receives its own new key. Keeping the old key on one of them would leave that key ambiguous for as long as the other document exists, and would make the choice depend on an arbitrary rule — file order, modification time, or a score. The repair is preserve-all: no document is deleted, no content is merged, and no candidate is preferred. Historical display numbers are unaffected: they are aliases, and several documents may carry the same one. This is a supersession of the ADR-030 line that said "each candidate for one key gets its own new key" only for real collisions — the rule is the same, and it applies to every member of a colliding group, including the one that could plausibly be called the original.

3. **How are ambiguous legacy scores and edges quarantined?** A relation whose endpoint display id matches more than one document is not written, not resolved, and not treated as a forward reference. It is recorded as an unresolved structural reference and it contributes nothing to any R_eff score. A score that depended on such an edge is recomputed without it, and the affected document's score is reported as provisional until the reference is resolved. Historical journal and activity events keep their original wording; a link to the canonical key may be added, but the events are not rewritten.

4. **Where does schema v5 sit relative to LanceDB?** Schema v5 is a version of the **legacy** index, not of the constellation index. It lives in the existing legacy Lance root under the three staged table names, and the legacy index stays the authoritative derived plane for a legacy-layout workspace until that workspace is converted to the constellation layout. On a constellation layout there is no legacy index at all: the existing owner/CURRENT protocol has priority, and no v5 tables are opened or created. The two layouts never coexist in one workspace, and no query mixes them.

## Invariants

These hold regardless of how the implementation is written:

- A document's canonical key never changes through an ordinary update, including through nested or external frontmatter.
- The canonical key is derived only from the operating-system random source plus the kind and title parts.
- A display id may repeat; a canonical key may not, within a workspace.
- Resolution never returns a first match, a last match, or a freshest file. Ambiguity is an error.
- "Not found", "ambiguous", and "read error" stay three distinct outcomes.
- The derived index filters, upserts, updates, and deletes by canonical key only.
- One query reads one document set: no query mixes the legacy index and the capture.
- A repair never deletes a document, never merges content, and never rewrites a historical display number.

## Risks

| ID | Risk | Mitigation |
|----|------|------------|
| R-1 | Renaming files breaks external references | New files only; existing paths stay; the repair plan lists every inbound occurrence |
| R-2 | The migration leaves the index inconsistent with the Markdown | Build from Markdown, verify a full digest, publish atomically behind a readiness marker |
| R-3 | Random keys are unreadable | Keep the display id as the human-facing handle; the title part stays readable |
| R-4 | Repair misattributes an ambiguous reference | Refuse the apply while any unresolved structural reference targets a remapped key |
| R-5 | Parallel worktrees corrupt the ledger | One per-kind lock in the common dir, fsync before publish, documented recovery for a damaged ledger |
| R-6 | The change exceeds the disk and build budget | One build lease, sequential verification, measured cleanup of reproducible outputs |

## Rollback Plan

**Triggers**: the v5 index cannot be verified against the Markdown; the readiness marker is damaged; a repair transaction cannot be completed or rolled back; a regression in an existing CLI or MCP surface appears.

**Steps**: stop the writers; restore the canonical preimages through the repair journal; restore the corresponding index generation; return to the previous CLI and MCP pair. A version rollback is a stop-writers operation, not a mixed-mode operation: the previous server and the new CLI are never run side by side.

**Blast radius**: the legacy index and the document paths touched by a repair. Documents are never lost by a rollback, because the preimages are complete and fsynced before the first mutation.

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| PRD-090 | PRD | based_on |
| SPEC-009 | SPEC | based_on |
| ADR-030 | ADR | based_on |
| PROB-067 | Problem | based_on |
| SPEC-005 | SPEC | refines |
| RFC-009 | RFC | informs |
| ADR-012 | ADR | supersedes_in_part |
