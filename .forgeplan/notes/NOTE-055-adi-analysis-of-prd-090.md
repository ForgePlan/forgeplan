---
author: null
depth: tactical
id: NOTE-055
kind: note
links:
- target: PRD-090
  relation: informs
parent_epic: null
status: draft
tags: []
title: ADI analysis of PRD-090
valid_until: null
---

# ADI Record: NOTE-055

**Artifact**: PRD-090 (prd)
**Model**: openai/MiniMax-M3.1-Flash-Preview
**Hypotheses**: 4
**Confidence**: medium
**Recommendation**: Rank H1 > H2 > H3 > H4. Adopt H1 as the target architecture, implement it through H2's compatibility-first sequencing, and use H3's pilot repair as the production safety gate. Before coding, freeze the remaining contract details, especially: a legacy title-derived slug must be frozen when accepted as a canonical key; the random key encoding and any readable filename part must not introduce a second identity; canonical and display namespaces must not collide; no-key import and no-key legacy fallback must be deterministic; source_id behavior must cover moved or copied workspaces; and missing or damaged ledgers must fail closed. Then introduce the CanonicalKey type and central resolver in forgeplan-core, route all CLI and MCP reads and mutations through it, enable canonical minting plus the shared worktree ledger, migrate persisted state to canonical keys, and publish schema-5 LanceDB generations atomically from the authoritative Markdown. Perform the repair only on verified pilot copies, reconcile the reference census, and promote the same transaction and index procedure to the originals. Avoid a central identity service or permanent dual-write model because both conflict with the local, file-authoritative architecture. The four-day timeline should be re-baselined or expressed as gated implementation slices because it currently combines several independently high-risk migrations.

## Structured Data

```json
{
  "id": "NOTE-055",
  "artifact_id": "PRD-090",
  "created_at": "2026-10-28T20:25:17",
  "model": "openai/MiniMax-M3.1-Flash-Preview",
  "hypotheses": [
    {
      "id": "H1",
      "description": "Adopt an end-to-end canonical-key architecture: mint an immutable random key per new artifact, make it the sole storage and relation key, and route every CLI, MCP, index, graph, export, and restore operation through one core resolver.",
      "assumptions": [
        "SPEC-009 can define a canonical-key serialization, normalization, legacy fallback, duplicate-key, and source-qualification contract without conflicting with SPEC-005.",
        "RFC-019 and ADR-030 establish the canonical key as immutable and the display ID as a non-authoritative alias.",
        "The Markdown files under .forgeplan remain authoritative, so LanceDB can be rebuilt and atomically republished as a derived generation.",
        "Existing relations, claims, receipts, scores, embeddings, and phase state can be migrated or associated with canonical keys.",
        "Linked Git worktrees can safely coordinate allocation through a locked, fsynced ledger in the Git common directory."
      ],
      "confidence": "high",
      "verdict": null
    },
    {
      "id": "H2",
      "description": "Deliver H1 as a compatibility-first vertical rollout: introduce the key type and shared resolver first, preserve legacy display-ID inputs where unambiguous, migrate index generations next, and perform corpus repair only after the new runtime is proven.",
      "assumptions": [
        "A feature flag or index-generation marker can isolate the old and new resolution paths.",
        "All legacy CLI and MCP routes can be inventoried and forced through the shared resolver.",
        "Unambiguous legacy inputs can retain their existing output while exposing the canonical key additively.",
        "Long-lived MCP handles can detect an index-generation change instead of retaining a stale view."
      ],
      "confidence": "medium",
      "verdict": null
    },
    {
      "id": "H3",
      "description": "Use a repair-first rollout: inventory and remediate known canonical-key collision groups before enabling strict canonical resolution and index migration, using the pilot corpora as the safety gate.",
      "assumptions": [
        "The 16 pilot collision groups are representative enough to expose repair-journal and reference-census failure modes.",
        "Structural references can be distinguished reliably from historical prose mentions.",
        "Each physically colliding document can be assigned a distinct key without changing historical display numbers.",
        "New-document minting is introduced before or alongside repair so that the repair window does not create additional unsafe documents."
      ],
      "confidence": "medium",
      "verdict": null
    },
    {
      "id": "H4",
      "description": "Implement only the shared Git allocation ledger and ambiguity detection, retaining display IDs as the primary identity and deferring canonical-key migration.",
      "assumptions": [
        "Correct number allocation and explicit ambiguity errors are sufficient to prevent the most damaging mutations.",
        "The existing LanceDB display-ID keying and export/restore behavior can remain unchanged."
      ],
      "confidence": "low",
      "verdict": null
    }
  ],
  "deductions": [
    {
      "hypothesis_id": "H1",
      "consequence": "The canonical key becomes the join key for the asserted 14 legacy-index sites as well as relations, evidence, phase state, claims, receipts, scores, and embeddings. A shared resolver can classify NotFound, Ambiguous, ConflictingCanonicalKey, and ReadFailed before any mutation, eliminating last-write-wins behavior. The corpus becomes intentionally mixed: newly minted random keys coexist with legacy slug or display-ID keys, so every route must preserve and resolve those aliases rather than silently normalizing them. Missing evidence: the contents of SPEC-009, RFC-019, and ADR-030; a verified call graph for all 18 command surfaces; LanceDB generation-swap behavior; and measured repair-census coverage.",
      "risks": [
        "The PRD says a unique legacy slug may become the canonical key but also says slugs are title-derived. If a title edit rewrites that slug, FR-002's immutability guarantee is impossible; accepted legacy slugs must instead be frozen as identifiers.",
        "R-3's readable title part is not reconciled with the statement that the key is random and not derived from the title. The exact filename and key grammar need one unambiguous definition.",
        "Changing all persisted state from display IDs to canonical keys is broader than the 14 indexed filter sites and may orphan legacy edges, claims, receipts, scores, or embeddings.",
        "An atomic LanceDB schema cutover may require new generation tables plus a metadata pointer rather than an in-place transaction; long-lived MCP handles must reject or refresh stale generations.",
        "The shared ledger needs a defined fail-closed policy for a missing or damaged ledger, a non-Git workspace, lock contention, and u32 exhaustion. Failing open could recreate issue #552.",
        "Source-qualified lookup depends on path-derived source_id semantics; source movement, copied workspaces, and federation-scope identity are not covered by the supplied source model.",
        "The proposed four-day timeline spans creation, allocation, storage migration, repair, 18 command surfaces, semantic indexing, and crash safety, so it may require explicit release slices or re-baselining."
      ],
      "feasibility": "medium"
    },
    {
      "hypothesis_id": "H2",
      "consequence": "A compatibility adapter can accept existing display-ID arguments, resolve them through the canonical resolver, and return the canonical key in addition to the current display ID. New documents can immediately receive canonical keys while schema-4 and schema-5 generations coexist behind an explicit readiness marker. Once all routes are proven to use the resolver, the legacy generation can be retired. Missing evidence: an exact inventory of the 18 surfaces, the current feature-flag facilities, and a demonstrated stale-handle invalidation path for MCP.",
      "risks": [
        "Dual-read or dual-write paths can diverge and recreate the problem if the legacy route remains callable.",
        "Compatibility behavior may mask unresolved ambiguity during rollout, especially in nested or programmatic APIs not covered by CLI tests.",
        "Schema generations left active indefinitely increase cleanup, rollback, and observability complexity.",
        "Migrating in vertical slices can accidentally leave a partial canonicalization state that SC-4's source audit would still detect."
      ],
      "feasibility": "medium"
    },
    {
      "hypothesis_id": "H3",
      "consequence": "The pilot workspaces become the executable safety specification for repair: they expose duplicate-key handling, reference-census precision, plan staleness, journal recovery, and rollback behavior before originals are touched. If new-document minting is enabled first, the same repair can handle both legacy collisions and any collisions introduced during rollout. Missing evidence: copies of the ForgePlanWeb and ForgePlanMarketplace corpora, a classified inventory of inbound references, and a prototype journal demonstrating concurrent-edit-safe rollback.",
      "risks": [
        "A full inbound-reference census may include ambiguous prose mentions that cannot be safely attributed to one physical document.",
        "Repair-first sequencing can leave the live system exposed to new title, number, or worktree collisions unless key minting and the shared ledger are deployed concurrently.",
        "The 16 known groups may not cover duplicate canonical keys, missing keys, malformed frontmatter, symlinks, or interrupted-transaction states.",
        "Rollback cannot restore original bytes safely unless every preimage, path, and concurrent-edit comparison is journaled and fsynced before mutation."
      ],
      "feasibility": "medium"
    },
    {
      "hypothesis_id": "H4",
      "consequence": "A shared ledger would prevent linked worktrees of one clone from allocating the same display number, and ambiguity checks could stop unsafe mutations. However, independent clones would still reuse display IDs, same-title artifacts would still lack permanent distinct identity, LanceDB rows would still merge by display ID, and exports or restores could still lose or substitute identity. Missing evidence: a route-level residual-risk inventory and demonstration that no mutation path bypasses ambiguity checks.",
      "risks": [
        "The approach creates a false sense of safety because display-number allocation improves while storage-level collisions remain.",
        "It cannot satisfy FR-001 through FR-020 or SC-1 through SC-8 as a complete solution.",
        "It would defer the largest migration and repair costs rather than remove them, potentially increasing future compatibility work."
      ],
      "feasibility": "low"
    }
  ],
  "evidence_needed": [
    {
      "for_hypothesis": "H1",
      "test": "Review SPEC-009, RFC-019, and ADR-030 against SPEC-005 and ADR-012 to freeze key encoding, normalization, legacy fallback, duplicate-key precedence, source qualification, and the rule that an accepted legacy slug becomes immutable.",
      "effort": "Medium"
    },
    {
      "for_hypothesis": "H1",
      "test": "Audit all core, CLI, MCP, Constellation, projection, and db resolution and mutation routes, including the asserted 14 db/store.rs sites and relation, claim, receipt, score, phase-state, and embedding paths.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H1",
      "test": "Run a 500-document benchmark proving exact-key, display-alias, missing, ambiguous, and duplicate-key resolution within one parsed-set pass, including cache invalidation after edits.",
      "effort": "Medium"
    },
    {
      "for_hypothesis": "H1",
      "test": "Exercise sequential and concurrent creation in linked worktrees with crash injection between reservation, fsync, and file publication; also test missing, damaged, locked, exhausted, and non-Git ledgers.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H1",
      "test": "Prototype versioned LanceDB generation publication with an atomic readiness pointer, interrupted migration at every stage, stale MCP-handle behavior, and rebuild reconciliation of edges and embeddings.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H1",
      "test": "On pilot copies, classify every inbound reference as structural, prose, ambiguous, or external; verify exact counts before and after repair and ensure unresolved remapped references block apply.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H1",
      "test": "Edit a document through both CLI and MCP with the local BGE-M3 embedder and verify the embedding backlog drains and semantic search returns the edited document under its canonical key.",
      "effort": "Medium"
    },
    {
      "for_hypothesis": "H2",
      "test": "Produce the exact compatibility matrix for all 18 command surfaces, including the operations enumerated in AC-6, and verify accepted inputs, output shape, exit codes, and tool-error codes on unambiguous data.",
      "effort": "Medium"
    },
    {
      "for_hypothesis": "H2",
      "test": "Add an architecture test that fails whenever a legacy display-ID lookup or mutation bypasses the shared resolver during the migration window.",
      "effort": "Medium"
    },
    {
      "for_hypothesis": "H3",
      "test": "Execute repeated plan generation and apply attempts against pilot copies while changing HEAD, source hashes, references, destinations, symlinks, and traversal targets; verify stale and unsafe plans are refused.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H3",
      "test": "Crash the repair after each publication step, restart recovery, and roll back while making an unrelated concurrent edit; verify byte-, path-, and edit-level preservation.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H4",
      "test": "Construct the same-title, independent-clone, duplicate-index-row, export/restore, and cross-project cases and quantify which failures remain after only the ledger and ambiguity changes.",
      "effort": "Medium"
    }
  ],
  "recommendation": "Rank H1 > H2 > H3 > H4. Adopt H1 as the target architecture, implement it through H2's compatibility-first sequencing, and use H3's pilot repair as the production safety gate. Before coding, freeze the remaining contract details, especially: a legacy title-derived slug must be frozen when accepted as a canonical key; the random key encoding and any readable filename part must not introduce a second identity; canonical and display namespaces must not collide; no-key import and no-key legacy fallback must be deterministic; source_id behavior must cover moved or copied workspaces; and missing or damaged ledgers must fail closed. Then introduce the CanonicalKey type and central resolver in forgeplan-core, route all CLI and MCP reads and mutations through it, enable canonical minting plus the shared worktree ledger, migrate persisted state to canonical keys, and publish schema-5 LanceDB generations atomically from the authoritative Markdown. Perform the repair only on verified pilot copies, reconcile the reference census, and promote the same transaction and index procedure to the originals. Avoid a central identity service or permanent dual-write model because both conflict with the local, file-authoritative architecture. The four-day timeline should be re-baselined or expressed as gated implementation slices because it currently combines several independently high-risk migrations.",
  "confidence": "medium"
}
```
