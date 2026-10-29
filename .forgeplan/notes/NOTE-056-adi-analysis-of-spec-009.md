---
author: null
depth: tactical
id: NOTE-056
kind: note
links:
- target: SPEC-009
  relation: informs
parent_epic: null
status: draft
tags: []
title: ADI analysis of SPEC-009
valid_until: null
---

# ADI Record: NOTE-056

**Artifact**: SPEC-009 (spec)
**Model**: openai/MiniMax-M3.1-Flash-Preview
**Hypotheses**: 4
**Confidence**: medium
**Recommendation**: Adopt H1 as the target architecture and use H2's immutable snapshot resolver as an internal component of H1, not as a separate release strategy. Sequence the work as: first amend the normative gaps in key grammar, not-found representation, exact-key versus numeric-alias precedence, source-qualified resolution, federation node scoping, plan digest canonicalization, preserve-all path semantics, and repair preimage storage; then implement the core identity and creation service and route every CLI, MCP, lifecycle, and projection call site through it; next stage and publish schema v5 as a derived index; then implement reconcile-ids transaction and recovery; finally validate semantic-backlog draining and execute the two pilot repairs. Do not use H3 as an identity authority, and do not rely on H4 alone because it does not prevent recurrence. Before freezing the contract, require the producer census, per-surface response matrix, sidecar inventory, concurrent-worktree allocation proof, and pilot reference census. This recommendation follows ADR-030's authoritative-key decision, PRD-090's collision-safety requirement, and SPEC-005's role as the frontmatter contract.

## Structured Data

```json
{
  "id": "NOTE-056",
  "artifact_id": "SPEC-009",
  "created_at": "2026-10-28T20:26:46",
  "model": "openai/MiniMax-M3.1-Flash-Preview",
  "hypotheses": [
    {
      "id": "H1",
      "description": "Implement a file-authoritative identity kernel in forgeplan-core: mint immutable canonical keys, resolve selectors from a complete parsed-document snapshot, route every producer through one creation service, key LanceDB and sidecars by canonical key, and add transactional repair support to reconcile-ids.",
      "assumptions": [
        "forgeplan-core is the correct shared boundary for the Rust CLI, MCP server, lifecycle writers, and other producers.",
        "Markdown frontmatter can be extended consistently with SPEC-005 to carry an authoritative canonical_key without changing existing display-id behavior.",
        "The current allocator lock can be extended to support a shared per-kind ledger across Git worktrees.",
        "LanceDB can stage the three v5 datasets and publish them through the readiness marker without becoming the source of truth.",
        "All creation and mutation call sites can be routed through a single core service rather than maintaining compatibility writers.",
        "ADR-030's canonical-key authority and PRD-090's collision-safety goals can be treated as architectural constraints rather than optional behavior."
      ],
      "confidence": "high",
      "verdict": null
    },
    {
      "id": "H2",
      "description": "Deliver resolution first through an immutable, fully parsed DocumentSnapshot and compatibility API adapters: exact canonical-key resolution, then display-alias resolution, with explicit error types and canonical_key added to responses while producer and index migration work proceeds separately.",
      "assumptions": [
        "The existing parser can expose the complete workspace document set, including malformed and partially captured files.",
        "Resolution consumers can accept a snapshot or resolver service without changing their public arguments.",
        "Existing CLI and MCP error envelopes can carry the required machine-readable taxonomy.",
        "A snapshot can be rebuilt or invalidated correctly when files, frontmatter, or linked worktrees change."
      ],
      "confidence": "medium",
      "verdict": null
    },
    {
      "id": "H3",
      "description": "Migrate LanceDB to schema v5 first, backfill canonical identities and relation endpoints, publish the staged tables through a readiness marker, and then make the new index the operational identity plane used by CLI and MCP.",
      "assumptions": [
        "LanceDB supports the required staged dataset creation, atomic publication behavior, key-based upserts, and uniqueness checks.",
        "A schema-4 corpus can be mapped completely from Markdown without canonical_key = id substitutions.",
        "The index can be made authoritative enough to support resolution while still honoring the file-authoritative architecture."
      ],
      "confidence": "low",
      "verdict": null
    },
    {
      "id": "H4",
      "description": "Canonicalize the historical corpus first through an extended reconcile-ids plan/apply workflow, resolve all current collisions, and only then introduce canonical-key minting and storage for new artifacts.",
      "assumptions": [
        "The existing reconcile-ids command has a safe filename and predicted-number foundation that can be extended rather than replaced.",
        "Every structural inbound reference can either be attributed exactly or be resolved by owner review before apply.",
        "All relevant legacy sidecars and state locations can be inventoried.",
        "Repair transactions can provide reversible, conflict-aware publication without relying on Git staging."
      ],
      "confidence": "medium",
      "verdict": null
    }
  ],
  "deductions": [
    {
      "hypothesis_id": "H1",
      "consequence": "forgeplan-core would become the single authority for canonical-key minting, selector resolution, creation ordering, and key-preserving updates. A practical creation sequence is: acquire the workspace mutation lock, mint the key, reserve and fsync the display number, verify no key or path collision, publish the Markdown file atomically, and then update key-addressed projections and sidecars. CLI and MCP would remain adapters that translate the same core result and error taxonomy into their existing envelopes.",
      "risks": [
        "The required workspace-lock then allocator-lock order must be enforced across every code path; a reverse order elsewhere could deadlock linked-worktree creation.",
        "Atomicity cannot span the Markdown file, LanceDB, relations, claims, sidecars, and semantic backlog. Failure states need an explicit committed-versus-pending model.",
        "A complete parsed snapshot may be expensive and may become inconsistent if taken without a defined concurrency boundary.",
        "The title-part algorithm is underspecified for punctuation-only titles, non-ASCII transliteration, reserved words, and exact 80-character truncation.",
        "Federated copies of the same canonical key conflict with the instruction that the graph use the canonical key as its node id; the federation graph likely needs an internal source-scoped identity while preserving the canonical key as the artifact attribute.",
        "Legacy display numbers, sidecars, receipts, and event consumers create a large migration surface that the response-shape requirement does not fully enumerate.",
        "Missing evidence: the actual creation call-site census, allocator implementation, sidecar inventory, and LanceDB layout implementation are required before finalizing the design."
      ],
      "feasibility": "medium"
    },
    {
      "hypothesis_id": "H2",
      "consequence": "Resolution can be made deterministic independently of LanceDB by producing an immutable snapshot containing canonical key, display id, title, and relative path for every parsed document. Exact-key conflicts can then be detected before alias matching, and ambiguous aliases can be returned as sorted candidate lists without exposing absolute paths.",
      "risks": [
        "Parsing the whole workspace on every operation may be too slow for large repositories unless a safely invalidated snapshot or manifest is introduced.",
        "Concurrent edits require a defined snapshot boundary; otherwise two calls can observe different document sets.",
        "Parse and I/O failures must remain distinct from not-found and must produce incomplete_capture where capture is incomplete.",
        "The current ResolveError model omits an explicit not-found outcome even though the requirements distinguish it from errors.",
        "The rule that an exact canonical key wins conflicts with the statement that a numeric-shaped canonical key must not outrank a competing display alias unless the selector grammar and precedence are amended.",
        "Source-qualified resolution needs separate qualified and unqualified modes; the current wording that rejects another-source qualifiers is ambiguous when read with the federation scenario.",
        "Missing evidence: parser behavior, cache facilities, API response schemas, and the existing MCP error envelope have not been inspected."
      ],
      "feasibility": "medium"
    },
    {
      "hypothesis_id": "H3",
      "consequence": "Schema v5 could be built from Markdown into artifacts_identity_v5, relations_identity_v5, and evidence_identity_v5, verified, and selected through IDENTITY_V5.json. Existing v4 tables could remain readable until publication, reducing rollback risk during the index transition.",
      "risks": [
        "Backfill from SQL rather than Markdown would violate the migration rule and could permanently assign display ids as canonical keys.",
        "LanceDB uniqueness and upsert behavior must be verified at the application level; a column declared canonical does not establish workspace-wide uniqueness.",
        "A marker can atomically select a set of datasets but cannot make the three staged datasets and all external sidecars one atomic mutation.",
        "A stale index could resolve to content that differs from the authoritative Markdown, violating the fully parsed resolution requirement.",
        "The constellation-layout rule requires explicit handling so that constellation workspaces neither open nor accidentally create legacy datasets.",
        "Missing evidence: the available LanceDB dataset publication APIs, schema constraints, transaction behavior, and legacy-versus-constellation layout code are unknown."
      ],
      "feasibility": "low"
    },
    {
      "hypothesis_id": "H4",
      "consequence": "A preserve-all repair can assign each colliding document its own reserved key while leaving historical display numbers intact. The plan would identify every structural reference occurrence, block application on ambiguous attribution, and use a filesystem journal to restore preimages after a failed or crashed apply.",
      "risks": [
        "The requirement that repair perform no deletion conflicts with the proposed-path and key-repair behavior unless the intended meaning is no loss of document content rather than no path removal.",
        "The plan digest is said to cover every field even though the digest is itself a field; a canonical serialization and exclusion of the digest field must be defined.",
        "The journal model records hashes but rollback requires the actual preimage bytes, so the on-disk manifest must include protected content or a content-addressed equivalent.",
        "Concurrent unrelated edits require current-file hash checks before rollback; otherwise recovery could overwrite user work.",
        "Historical prose, quotations, sidecars, and relation-specific context create a reference-classification problem that cannot be solved by regex or language-model inference.",
        "Repair-first leaves all new producers capable of recreating display-id collisions unless minting and uniqueness enforcement are implemented concurrently.",
        "Missing evidence: the existing reconcile-ids behavior, pilot-corpus reference census, sidecar formats, and current crash-recovery primitives are not available."
      ],
      "feasibility": "medium"
    }
  ],
  "evidence_needed": [
    {
      "for_hypothesis": "H1",
      "test": "Inventory every NewArtifact builder, create_artifact_with_projection, import_artifact_with_projection, allocate_and_create_artifact* call site, CLI/MCP adapter, lifecycle writer, and direct side-effect writer.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H1",
      "test": "Inspect the current per-kind allocator lock, Git common-directory behavior, worktree discovery, ledger durability, and lock acquisition order in forgeplan-core.",
      "effort": "Medium"
    },
    {
      "for_hypothesis": "H1",
      "test": "Prove creation ordering with fault injection at entropy failure, number reservation, file publication, relation publication, and index publication.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H1",
      "test": "Define and test canonical-key grammar cases for ASCII, Cyrillic, punctuation-only, empty, reserved-word, and maximum-length titles.",
      "effort": "Low"
    },
    {
      "for_hypothesis": "H2",
      "test": "Build a resolution conformance fixture covering exact-key conflicts, alias ambiguity, not found, invalid selectors, numeric-shaped aliases, incomplete parsing, I/O failures, and source-qualified selectors.",
      "effort": "Medium"
    },
    {
      "for_hypothesis": "H2",
      "test": "Measure complete-workspace parse cost and determine whether an invalidated immutable snapshot can preserve error and concurrency semantics.",
      "effort": "Medium"
    },
    {
      "for_hypothesis": "H2",
      "test": "Compare current CLI and MCP response and error schemas against the required canonical_key, code, input, candidates, and source-disclosure constraints.",
      "effort": "Medium"
    },
    {
      "for_hypothesis": "H3",
      "test": "Verify LanceDB dataset creation, rename or publication, uniqueness, upsert, vector replacement, and recovery semantics using the installed version.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H3",
      "test": "Exercise the staged migration with interruption before and after marker publication and with missing, damaged, stale, and constellation-layout workspaces.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H4",
      "test": "Run a read-only plan against copies of ForgePlanWeb and ForgePlanMarketplace and classify all inbound references, ambiguous sidecars, destination changes, and path-preservation cases.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H4",
      "test": "Fault-inject the repair journal before and after every publication, then verify rollback preserves concurrent unrelated edits and reports conflicts without overwriting them.",
      "effort": "High"
    },
    {
      "for_hypothesis": "H4",
      "test": "Specify canonical plan serialization, digest-domain separation, reserved-key handling, preimage storage, and the exact boundary between canonical commit and index-pending recovery.",
      "effort": "Medium"
    },
    {
      "for_hypothesis": "H1",
      "test": "Verify how federation vertices are scoped and ensure equal canonical keys in different sources remain distinct without violating local canonical-key uniqueness.",
      "effort": "Medium"
    }
  ],
  "recommendation": "Adopt H1 as the target architecture and use H2's immutable snapshot resolver as an internal component of H1, not as a separate release strategy. Sequence the work as: first amend the normative gaps in key grammar, not-found representation, exact-key versus numeric-alias precedence, source-qualified resolution, federation node scoping, plan digest canonicalization, preserve-all path semantics, and repair preimage storage; then implement the core identity and creation service and route every CLI, MCP, lifecycle, and projection call site through it; next stage and publish schema v5 as a derived index; then implement reconcile-ids transaction and recovery; finally validate semantic-backlog draining and execute the two pilot repairs. Do not use H3 as an identity authority, and do not rely on H4 alone because it does not prevent recurrence. Before freezing the contract, require the producer census, per-surface response matrix, sidecar inventory, concurrent-worktree allocation proof, and pilot reference census. This recommendation follows ADR-030's authoritative-key decision, PRD-090's collision-safety requirement, and SPEC-005's role as the frontmatter contract.",
  "confidence": "medium"
}
```
