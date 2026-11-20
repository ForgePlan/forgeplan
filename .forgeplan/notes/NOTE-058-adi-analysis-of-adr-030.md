---
author: null
depth: tactical
id: NOTE-058
kind: note
links:
- target: ADR-030
  relation: informs
parent_epic: null
status: draft
tags: []
title: ADI analysis of ADR-030
valid_until: null
---

# ADI Record: NOTE-058

**Artifact**: ADR-030 (adr)
**Model**: openai/MiniMax-M3.1-Flash-Preview
**Hypotheses**: 4
**Confidence**: medium
**Recommendation**: Advance H1 decisively as the target architecture, while keeping ADR-030 in draft and blocking activation. H1 is the only option that fixes both the measured identity collision and the worktree allocation race without adding a third identity, a central service, or silent resolution. Before implementation, freeze the remaining contract details and reconcile PRD-090, SPEC-009, SPEC-005, RFC-019, and the PROB-067 regression boundary. Then implement in the stated order: shared minting and preserve-identity paths; canonical-key core, WorkspaceRead, and graph; schema-v5 projection rebuild with digest and readiness marker; transactional repair tooling; unified CLI/MCP ambiguity errors; and the Git-common-directory number ledger. Preserve ADR-012's two-layer model, retain ADR-029's `(source_id, canonical_key)` address, keep `artifact/identity.rs` exclusively for caller identity, and use the new `artifact/canonical_key.rs` module for document identity. Do not bulk-rename legacy documents; repair only explicitly identified physical collisions, and refuse an apply while any structural reference remains unresolved. H2 is only a fallback if ADR-029 is formally reopened, H4 may serve as a temporary safety guard but not the target state, and H3 should be rejected. Activation should require V1–V12 on both storage layouts, the read-only V10 pilot repair result, a successful real-embedder V11 run, independent review, and the guardian gate.

## Structured Data

```json
{
  "id": "NOTE-058",
  "artifact_id": "ADR-030",
  "created_at": "2026-10-28T20:29:46",
  "model": "openai/MiniMax-M3.1-Flash-Preview",
  "hypotheses": [
    {
      "id": "H1",
      "description": "Adopt Option C: make a permanent canonical key the authoritative document identity and treat the display id as a repeatable human-facing alias.",
      "assumptions": [
        "All creation paths can distinguish minting a new key from preserving the key during import, restore, branch copy, or repair.",
        "Existing unique title-derived slugs can be retained as legacy canonical keys, while a deterministic representation can be defined for legacy documents without a slug.",
        "The Markdown corpus remains authoritative and the LanceDB projection can be rebuilt from it, verified by digest, and published atomically behind a readiness marker.",
        "Storage selectors, relations, graph vertices, sidecars, scores, embeddings, CLI commands, and MCP tools can all be migrated to canonical-key addressing.",
        "Repair tooling can exhaustively identify structural references and refuse to remap a key when historical intent cannot be determined.",
        "The Constellation address remains `(source_id, canonical_key)`, with `source_id` remaining path-derived as established by ADR-029."
      ],
      "confidence": "high",
      "verdict": null
    },
    {
      "id": "H2",
      "description": "Adopt Option A: add a UUID field beside the existing id and slug and use it as an additional document identifier.",
      "assumptions": [
        "ADR-029 would be formally reopened because it explicitly declines a canonical artifact UUID.",
        "A UUID could survive import, branch copy, export, restore, and deletion without creating new identity ambiguity.",
        "The storage schema and all reference-bearing sidecars could be extended and migrated at the same cost as canonical-key routing."
      ],
      "confidence": "low",
      "verdict": null
    },
    {
      "id": "H3",
      "description": "Adopt Option B: make the display number globally unique and use it as the durable cross-project identifier.",
      "assumptions": [
        "Independent clones could coordinate through a global allocator or another universally reachable authority.",
        "The u32 number space would remain adequate for the required scale.",
        "A globally unique number would remain readable enough to serve as the primary storage and user-facing identifier."
      ],
      "confidence": "low",
      "verdict": null
    },
    {
      "id": "H4",
      "description": "Adopt Option D: retain the title-derived slug as the nominal key and add collision detection without changing the authoritative storage identity.",
      "assumptions": [
        "Every selector could detect multiple display-id or title-derived-slug candidates before mutation.",
        "Title edits could be prevented from changing an existing slug, despite the current `slug_from_kind_title` behavior.",
        "The legacy index could be changed to retain multiple physical documents without adopting a permanent canonical key."
      ],
      "confidence": "low",
      "verdict": null
    }
  ],
  "deductions": [
    {
      "hypothesis_id": "H1",
      "consequence": "The storage schema must move from display-id identity to canonical-key identity. Display ids then become a many-to-one alias dimension, while rows, graph vertices, relations, scores, and embeddings are partitioned by canonical key. New documents receive `<kind>-<title-part>-k<32 lowercase hex>` and random filenames, whereas unique legacy slugs remain valid until an explicit collision repair changes the affected physical document. The existing `(source_id, canonical_key)` Constellation model remains intact, and only linked worktrees share number accounting.",
      "risks": [
        "This is the largest identity-model change since ADR-012 and touches every creation route, selector route, relation format, graph edge, sidecar, index filter, CLI command, and MCP operation.",
        "A missed creation route could mint a key incorrectly or omit the key from an artifact, while a missed selector could continue resolving by display id.",
        "A partially rebuilt LanceDB projection could disagree with the authoritative Markdown corpus and silently mix or omit rows, relations, scores, or vectors.",
        "Repairing an ambiguous inbound reference is the weakest semantic step; guessing could silently repoint a relation to the wrong physical document.",
        "Legacy title-derived slugs are not naturally immutable, so ordinary title updates could violate the permanent-key invariant unless the parser and writer explicitly enforce it.",
        "The representation of the canonical key for legacy documents without a slug is underspecified and must be frozen before migration.",
        "The new-key grammar still needs rules for title-part normalization, maximum length, reserved separators, and retry behavior after a mocked 128-bit collision.",
        "Duplicate-key imports and restored receipts need explicit semantics because preserving a key can expose an existing duplicate rather than minting a replacement.",
        "The shared per-kind ledger needs transactional crash recovery in the Git common directory; a damaged or stale ledger must never cause number reuse or document loss.",
        "Conflating the proposed `artifact/canonical_key.rs` with the existing `artifact/identity.rs`, which tracks caller identity for PROB-066, would regress a separate identity concept."
      ],
      "feasibility": "medium"
    },
    {
      "hypothesis_id": "H2",
      "consequence": "Adding a UUID without changing the authoritative lookup path would preserve the current failure mode: two documents with the same display id could still occupy one legacy row, one score, and one graph vertex. Making the UUID authoritative would effectively create another version of H1, requiring the same selector, migration, graph, and repair work, while leaving slug and display id as additional potentially divergent fields.",
      "risks": [
        "The artifact would violate ADR-029 and the ADR-030 admissibility rule barring a UUID field.",
        "Slug, id, and UUID could drift unless every writer and update path enforces a single precedence and immutability policy.",
        "The schema, relation, export, restore, and sidecar migration would be at least as broad as H1.",
        "Copy semantics would require deciding whether preserving a UUID means preserving the same logical document or minting identity for a new document.",
        "The additional field would not by itself reduce migration cost or repair ambiguity."
      ],
      "feasibility": "medium"
    },
    {
      "hypothesis_id": "H3",
      "consequence": "A globally unique display number would require clones that are deliberately independent to coordinate allocation, consume a shared number space, and remain correct during partitions, merges, and exhaustion. It would also make a human-oriented alias authoritative and leave the title-derived slug problem unresolved unless slugs were removed or treated as aliases as well.",
      "risks": [
        "A central allocator would contradict the no-central-server and independent-clone architecture.",
        "Network unavailability or clone partitioning would block or desynchronize artifact creation.",
        "The u32 number space could be exhausted, especially in large multi-project deployments.",
        "Globally unique short numbers would be less locally meaningful and less readable than the current project-scoped display ids.",
        "Merging independently allocated histories would still require an explicit collision or ordering policy."
      ],
      "feasibility": "low"
    },
    {
      "hypothesis_id": "H4",
      "consequence": "A detector could turn current silent first-match or last-match behavior into an explicit ambiguity error and prevent some destructive updates. It would not, however, provide a stable identity across title changes, prevent two same-title files from sharing one display-id-keyed row, or establish distinct graph and embedding identities for those files.",
      "risks": [
        "Detection without canonical storage keys would leave relations, state, scores, and embeddings vulnerable to cross-document mixing.",
        "Title edits would either rewrite the nominal identity or require an exception to normal slug derivation.",
        "The implementation could expand into most of H1 while retaining the inconsistent legacy key format.",
        "Operators could interpret ambiguity diagnostics as resolution without understanding that no safe mutation occurred."
      ],
      "feasibility": "medium"
    }
  ],
  "evidence_needed": [
    {
      "for_hypothesis": "H1",
      "test": "Freeze SPEC-009 V1–V12 with observable results, reconcile it with RFC-019 and SPEC-005, and explicitly specify legacy-slug immutability, slugless-document representation, title-part normalization, duplicate-key imports, and entropy-collision retry behavior.",
      "effort": "Medium"
    },
    {
      "for_hypothesis": "H1",
      "test": "Complete the census of `NewArtifact` builders, `create_artifact_with_projection`, `import_artifact_with_projection`, and every `allocate_and_create_artifact*` route; classify each as key-minting, key-preserving, or repair-only.",
      "effort": "Medium"
    },
    {
      "for_hypothesis": "H1",
      "test": "Fault-inject the schema-4 to schema-5 migration at every stage and verify that each interruption leaves either a complete verified v5 projection or the preserved v4 state, with full-digest comparison and no mixed read source.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H1",
      "test": "Exercise the shared per-kind ledger with sequential and concurrent linked worktrees, crash between reservation and publication, damaged-ledger recovery, missing worktree configuration, and u32 exhaustion.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H1",
      "test": "Run a read-only repair preview over the 16 pilot collision groups, produce an exhaustive structural-reference census, confirm that every physical document is preserved, and verify that ambiguous historical intent causes refusal rather than a guessed remap.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H1",
      "test": "Execute V1–V12 against both storage layouts using the real CLI, MCP server, and local BGE-M3 embedder, including exact-key, lowercase-display, missing, duplicate-display, duplicate-key, numeric-shadow, qualified-get, update, delete, restore, and export paths.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H2",
      "test": "Prototype UUID-authoritative routing over the pilot collision corpus and demonstrate that every row, relation, score, embedding, branch-copy, import, and restore path uses the UUID correctly; separately test whether ADR-029 can be reopened without violating source-qualified Constellation semantics.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H3",
      "test": "Demonstrate a globally coordinated allocator that remains correct across disconnected independent clones, concurrent allocation, clone divergence, merge, service outage, and u32 exhaustion without a central service.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H4",
      "test": "Prototype ambiguity detection over all 16 pilot groups and across title edits, then verify whether it actually prevents cross-document row, relation, score, graph, and embedding mixing without adopting canonical-key routing.",
      "effort": "Medium"
    }
  ],
  "recommendation": "Advance H1 decisively as the target architecture, while keeping ADR-030 in draft and blocking activation. H1 is the only option that fixes both the measured identity collision and the worktree allocation race without adding a third identity, a central service, or silent resolution. Before implementation, freeze the remaining contract details and reconcile PRD-090, SPEC-009, SPEC-005, RFC-019, and the PROB-067 regression boundary. Then implement in the stated order: shared minting and preserve-identity paths; canonical-key core, WorkspaceRead, and graph; schema-v5 projection rebuild with digest and readiness marker; transactional repair tooling; unified CLI/MCP ambiguity errors; and the Git-common-directory number ledger. Preserve ADR-012's two-layer model, retain ADR-029's `(source_id, canonical_key)` address, keep `artifact/identity.rs` exclusively for caller identity, and use the new `artifact/canonical_key.rs` module for document identity. Do not bulk-rename legacy documents; repair only explicitly identified physical collisions, and refuse an apply while any structural reference remains unresolved. H2 is only a fallback if ADR-029 is formally reopened, H4 may serve as a temporary safety guard but not the target state, and H3 should be rejected. Activation should require V1–V12 on both storage layouts, the read-only V10 pilot repair result, a successful real-embedder V11 run, independent review, and the guardian gate.",
  "confidence": "medium"
}
```
