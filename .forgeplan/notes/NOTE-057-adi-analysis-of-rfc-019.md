---
author: null
depth: tactical
id: NOTE-057
kind: note
links:
- target: RFC-019
  relation: informs
parent_epic: null
status: draft
tags: []
title: ADI analysis of RFC-019
valid_until: null
---

# ADI Record: NOTE-057

**Artifact**: RFC-019 (rfc)
**Model**: openai/MiniMax-M3.1-Flash-Preview
**Hypotheses**: 4
**Confidence**: medium
**Recommendation**: Evidence ranking: H2 > H3 > H4 > H1. Proceed conditionally with Option C, using H2 for identity semantics, H3 for authoritative storage and linked-worktree numbering, and H4 for a staged, transactional rollout. The preferred identity model is to grandfather valid legacy unique slugs, random-mint all new and remapped keys, and amend the universal invariant to say that newly minted keys are random-derived while all canonical keys are opaque and immutable. Before implementation, RFC-019 must resolve four material ambiguities: whether preserving a document's bytes excludes frontmatter, whether all collision candidates are remapped or one retains the old key, how ambiguous legacy scores and edges are quarantined, and where schema v5 sits relative to LanceDB. It must also specify legacy slug immutability, deterministic old-import fallback, canonical-key collision checks, the filename rule after remapping, and fail-closed behavior for a damaged allocator ledger. Preserve the RFC's slice ownership and ordering, but activate v5 and the adapters atomically so no query mixes generations and no old/new CLI-MCP pair is used. If uniform random keys for the entire historical corpus are mandatory, choose H1 explicitly and revise the non-goals and migration budget rather than implementing both interpretations simultaneously.

## Structured Data

```json
{
  "id": "NOTE-057",
  "artifact_id": "RFC-019",
  "created_at": "2026-10-28T20:28:37",
  "model": "openai/MiniMax-M3.1-Flash-Preview",
  "hypotheses": [
    {
      "id": "H1",
      "description": "Adopt a strict uniform canonical-key model: every artifact, including legacy artifacts, receives a key in the form `<kind>-<title-part>-k<32 lowercase hex>`, and legacy frontmatter is migrated to that form while display IDs and existing file paths remain unchanged.",
      "assumptions": [
        "The canonical key continues to be stored in the frontmatter slug, as described by RFC-019 and the existing Constellation behavior.",
        "Changing legacy slug fields can be performed without invalidating unclassified structural or prose references.",
        "The normative requirements in ADR-030, ADR-012, and SPEC-009 require uniform key minting rather than permitting grandfathered legacy values.",
        "The project is prepared to fund a corpus-wide migration despite the RFC's non-goals concerning bulk renaming and historical rewriting."
      ],
      "confidence": "medium",
      "verdict": null
    },
    {
      "id": "H2",
      "description": "Adopt a backward-compatible, opaque canonical-key model: grandfather valid legacy unique slugs as canonical keys, random-mint keys for all new and collision-repaired artifacts, and treat key syntax and provenance as an implementation concern rather than a universal invariant.",
      "assumptions": [
        "ADR-030 requires authoritative and immutable identity but does not require every historical key to have the new random format.",
        "Legacy unique slugs can be frozen so title, body, status, nested-frontmatter, and external-frontmatter updates cannot rewrite them.",
        "Legacy exports can deterministically recover a key from frontmatter without silently minting a replacement.",
        "The RFC's random-derivation invariant can be narrowed to newly minted or explicitly remapped keys."
      ],
      "confidence": "high",
      "verdict": null
    },
    {
      "id": "H3",
      "description": "Decouple artifact identity from display numbering: use the canonical key for records, relations, scores, and graph vertices, while repairing linked-worktree allocation with a per-kind high-watermark ledger in the Git common directory and a repeatable display-ID alias multimap.",
      "assumptions": [
        "PROB-067's measured cause is representative and all relevant creation paths use the shared allocator.",
        "The Git common directory and its existing per-kind lock are available and durable across linked worktrees.",
        "CLI and MCP callers can accept an explicit ambiguity error when a display ID maps to multiple candidates.",
        "The LanceDB synchronization path can propagate canonical-key identity without creating a second source of truth."
      ],
      "confidence": "high",
      "verdict": null
    },
    {
      "id": "H4",
      "description": "Implement the migration as a staged, transactionally gated rollout: build and verify a schema-v5 generation from authoritative Markdown, repair only known collision groups, atomically activate the new generation, and retain journaled rollback until real CLI and MCP verification succeeds.",
      "assumptions": [
        "The schema-v5 legacy index can be built from Markdown and compared with a deterministic digest, as required by SPEC-009 and RFC-019.",
        "A readiness marker can prevent readers from observing mixed v4 and v5 document sets.",
        "A repair journal with complete preimages can provide crash recovery and rollback.",
        "RFC-009's migration rollout and SPEC-005's frontmatter contract can be reconciled with the canonical-key changes."
      ],
      "confidence": "medium",
      "verdict": null
    }
  ],
  "deductions": [
    {
      "hypothesis_id": "H1",
      "consequence": "A uniform key format would make key validation, import behavior, and creation invariants straightforward. Because the canonical key is the frontmatter slug, however, applying the model to the historical corpus would rewrite document frontmatter and potentially invalidate references; it cannot simultaneously keep every document's bytes unchanged and preserve the existing non-goals.",
      "risks": [
        "Violates the stated non-goals of bulk historical renaming and rewriting historical prose.",
        "Creates a much larger repair surface than the 16 measured collision groups.",
        "May invalidate references that cannot be distinguished from ordinary prose or quotations.",
        "Conflicts with the explicit promise that existing unique slugs and paths stay unchanged.",
        "Requires careful coordination among frontmatter writers, importers, index rows, graph edges, scores, receipts, and undo data."
      ],
      "feasibility": "medium"
    },
    {
      "hypothesis_id": "H2",
      "consequence": "The workspace would contain a mixed population of legacy and random-format keys, but the resolver can correctly treat canonical keys as opaque and preserve the stated two-layer model. The RFC must change its universal key-derivation invariant to distinguish legacy grandfathering from new minting, and it must define how slug mutations by legacy or external frontmatter writers are detected and rejected.",
      "risks": [
        "The existing invariant that every canonical key is random-derived would no longer be true.",
        "A title-derived legacy slug may still be rewritten by an unaware frontmatter update path.",
        "Format-based detection of legacy keys may be unreliable without explicit migration provenance.",
        "Old imports without a slug need a deterministic key source; falling back to a display ID or minting silently would violate the stated import boundary.",
        "A 128-bit random collision is improbable but the absolute uniqueness invariant still requires a workspace-level check before commit."
      ],
      "feasibility": "high"
    },
    {
      "hypothesis_id": "H3",
      "consequence": "Duplicate display IDs would stop merging documents, scores, relations, or graph vertices because storage identity would be the canonical key. The common-directory ledger would coordinate linked worktrees, but independent clones would still be able to reuse display IDs, which is consistent with the RFC's non-goal; canonical-key uniqueness would remain the protection against conflating those artifacts.",
      "risks": [
        "A missing or damaged high-watermark ledger can cause number reuse unless the allocator fails closed or has a defined reconstruction procedure.",
        "An allocation path that bypasses the shared allocator can still issue duplicates.",
        "Worktree discovery can omit not-yet-initialized or concurrently added worktrees unless the persisted ledger is the primary watermark.",
        "Lock-order violations between workspace mutation and per-kind allocator locks can deadlock callers.",
        "Legacy rows, scores, and edges that were already collapsed cannot always be attributed to the correct physical document.",
        "MCP and CLI adapters may retain hidden `id`-based filters if the migration does not cover all consumers."
      ],
      "feasibility": "high"
    },
    {
      "hypothesis_id": "H4",
      "consequence": "A digest-verified, atomically activated v5 generation would satisfy the RFC's one-document-set invariant and permit rollback without mixed v4/v5 queries. Collision-only repair would minimize corpus churn, but a collided legacy row may have one score or set of edges that cannot be attributed among the preserved documents; those records need an explicit quarantine or non-attribution policy rather than a guessed mapping.",
      "risks": [
        "The RFC's statement that every physical document keeps its bytes conflicts with changing the frontmatter slug of collision candidates.",
        "The rules that only collision participants receive a key and that every candidate receives its own new key are not equivalent and need one precise interpretation.",
        "Repairing paths or slugs can invalidate filename conventions and unclassified inbound references.",
        "A crash between filesystem mutation, index publication, and LanceDB synchronization can leave divergent generations.",
        "An ambiguous display-ID reference or a collapsed historical score can be misattributed if the plan does not explicitly quarantine it.",
        "Old CLI and new MCP binaries must not be paired during activation or rollback."
      ],
      "feasibility": "medium"
    }
  ],
  "evidence_needed": [
    {
      "for_hypothesis": "H1",
      "test": "Review the exact normative requirements in ADR-030, ADR-012, and SPEC-009 and inventory every legacy artifact whose slug would need rewriting under the strict model.",
      "effort": "Medium"
    },
    {
      "for_hypothesis": "H2",
      "test": "Audit all core, CLI, MCP, import, export, receipt, nested-frontmatter, and external-frontmatter code paths that read or write `slug`; verify that legacy keys remain immutable and old payloads never mint keys implicitly.",
      "effort": "Medium"
    },
    {
      "for_hypothesis": "H3",
      "test": "Exercise concurrent creation in at least two linked worktrees of one clone, including process termination before file publication, ledger corruption, and worktree initialization races; confirm that no display number is reused.",
      "effort": "Medium"
    },
    {
      "for_hypothesis": "H3",
      "test": "Audit lock acquisition order, durable publication of the ledger and its parent directory, and all display-number allocation entry points in the CLI, MCP, import, repair, and CI paths.",
      "effort": "Medium"
    },
    {
      "for_hypothesis": "H4",
      "test": "Produce a full v4-to-v5 dry run from authoritative Markdown, compare generation digests, and validate atomic activation and rollback with the LanceDB synchronization step included.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H4",
      "test": "For all collision groups, classify every inbound structural reference, prose reference, legacy score, graph edge, and relation; identify records that cannot be attributed and specify their quarantine behavior.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H4",
      "test": "Build a shared CLI/MCP selector test matrix covering canonical-key lookup, unique and repeated display IDs, legacy slugs, paths, not-found, ambiguous, parse failure, I/O failure, and incomplete-capture outcomes.",
      "effort": "Medium"
    },
    {
      "for_hypothesis": "H4",
      "test": "Prototype interruption and restart during repair at each operation boundary, then verify complete preimage restoration, index-generation restoration, and no duplicated or lost documents.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H4",
      "test": "Reconcile the implementation scenarios with SPEC-009 and RFC-009, update SPEC-005's frontmatter semantics, and add the body-referenced ADR-029 to the traceable relation set.",
      "effort": "Low"
    }
  ],
  "recommendation": "Evidence ranking: H2 > H3 > H4 > H1. Proceed conditionally with Option C, using H2 for identity semantics, H3 for authoritative storage and linked-worktree numbering, and H4 for a staged, transactional rollout. The preferred identity model is to grandfather valid legacy unique slugs, random-mint all new and remapped keys, and amend the universal invariant to say that newly minted keys are random-derived while all canonical keys are opaque and immutable. Before implementation, RFC-019 must resolve four material ambiguities: whether preserving a document's bytes excludes frontmatter, whether all collision candidates are remapped or one retains the old key, how ambiguous legacy scores and edges are quarantined, and where schema v5 sits relative to LanceDB. It must also specify legacy slug immutability, deterministic old-import fallback, canonical-key collision checks, the filename rule after remapping, and fail-closed behavior for a damaged allocator ledger. Preserve the RFC's slice ownership and ordering, but activate v5 and the adapters atomically so no query mixes generations and no old/new CLI-MCP pair is used. If uniform random keys for the entire historical corpus are mandatory, choose H1 explicitly and revise the non-goals and migration budget rather than implementing both interpretations simultaneously.",
  "confidence": "medium"
}
```
