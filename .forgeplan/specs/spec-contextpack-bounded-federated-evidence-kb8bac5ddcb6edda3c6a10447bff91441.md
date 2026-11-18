---
assigned_number: 10
author: null
created: 2026-11-12
depth: standard
id: SPEC-010
kind: spec
links:
- target: epic-constellation-contextpack
  relation: based_on
parent_epic: null
prd: null
predicted_number: 10
slug: spec-contextpack-bounded-federated-evidence-kb8bac5ddcb6edda3c6a10447bff91441
status: draft
tags: []
title: ContextPack bounded federated evidence and continuation
type: API / Data Model / Protocol / UI Spec
updated: 2026-11-12
valid_until: null
---

---
assigned_number: 10
created: 2026-11-12
prd: null
predicted_number: 10
slug: spec-contextpack-bounded-federated-evidence-kb8bac5ddcb6edda3c6a10447bff91441
type: API / Data Model / Protocol / UI Spec
updated: 2026-11-12
---

## Summary

Complete the existing ContextPack surface with declared evidence closure, source-qualified provenance, whole units, bounded public delivery, and version-bound continuation. This is the S3 product contract under EPIC-010 / PRD-081. It does not claim S1-S4 acceptance or reduce F10/F11 obligations.

## Scope

In scope: the existing Core pack implementation and all CLI/read-MCP callers, relevant graph/source-view reuse, public schemas, human-readable output, behavioral tests and documentation. No new service, database, agent executor, implicit external source admission or generated knowledge graph. Existing source authorization, strict/partial behavior, canonical identity and snapshot semantics remain authoritative.

## Requirements

### Requirement: One explicit source and request basis

A pack MUST select only sources admitted by its scope or pinned snapshot. Identity MUST include source plus canonical key; aliases are display values. The response MUST name the source revisions actually read. A version vector is not an atomic distributed transaction. Public continuation MUST NOT grant access outside the admitted source set.

#### Scenario: Identical aliases in different projects
- GIVEN two admitted sources each contain ADR-001 and only one contains the selected canonical document
- WHEN the caller requests the qualified document
- THEN selection and relation resolution use that source only; an unqualified ambiguous explicit selector is refused.

#### Scenario: Unadmitted linked source
- GIVEN a selected document declares a qualified link to an unadmitted source
- WHEN its closure is built
- THEN no path outside the admitted set is read and an unresolved obligation is reported; the pack cannot claim full obligations coverage.

### Requirement: Relevant seeds without a second search system

Task text supplies ranked seed candidates using the existing lexical retrieval. Explicit qualified artifact selectors MUST also be supported and MUST NOT require matching words. The response MUST state the actual retrieval mode. This change does not add semantic pack retrieval; an unsupported requested mode must be rejected rather than silently substituted. Stable inputs and snapshots MUST produce stable selection and tie-breaking. Relevance MUST NOT establish normative authority.

#### Scenario: Explicit seed without matching terms
- GIVEN a valid qualified artifact selector whose body does not match the task words
- WHEN a pack is requested
- THEN that artifact is a seed and its declared obligations are considered.

### Requirement: Declared typed closure

The default closure policy MUST be named/versioned. For selected A: follow outgoing A based_on B and A refines B to their declared grounds; include incoming E informs A as informing material without inventing a supporting verdict; include incident contradicts links as declared conflicts; expose incident supersedes links and replacement status without substituting an exact historical identity. Incoming based_on/refines are dependents, not grounds, and MUST NOT be recursively pulled in as mandatory context by default. Closure MUST use typed declared edges independently of lexical score and preserve each edge's original direction/provenance. A kind, nearby code path, or semantic similarity alone MUST NOT make a document binding. No new inferred edges are written.

Admit mandatory grounds, declared conflicts and informing evidence before relevance-only material. If they cannot be delivered, report incomplete obligations or refuse according to the requested completeness policy.

#### Scenario: Ground without shared words
- GIVEN A matches the task and A based_on B, while B has no matching words
- WHEN the budget admits both units
- THEN B is included with a declared-ground selection reason and the A-to-B edge.

#### Scenario: Conflicting incoming evidence
- GIVEN E informs A and E records refutation, with no shared query words
- WHEN A is selected
- THEN E and its actual verdict are visible; informs is not mislabeled as support.

#### Scenario: Historical replacement
- GIVEN newer N supersedes O and evidence refers to O
- WHEN the caller selects exact historical O
- THEN O remains the selected identity, N is identified as replacement context, and evidence links present in the admitted basis remain attached to O. Missing historical links are not inferred or fetched outside that basis.

### Requirement: Bounded and explainable traversal

Depth, node, edge and elapsed-work limits MUST be explicit. Traversal MUST detect cycles and avoid repeated work for the same qualified identity. A stopped traversal MUST name its limiting condition and MUST NOT claim complete obligation coverage. Delivery continuation MUST NOT silently extend an incomplete traversal with different limits.

#### Scenario: Cycle and capped chain
- GIVEN A based_on B, B based_on A and a longer outgoing chain
- WHEN the configured depth or node limit is reached
- THEN the cycle terminates without duplicate units; a bounded result identifies the unresolved boundary and reports incomplete traversal.

### Requirement: Exact whole evidence units

A unit MUST preserve exact source text for a whole requirement, scenario, paragraph, section or complete document; no arbitrary match-centered character windows. The conservative fallback is a larger whole block, not a generated summary pretending to be a quote. A unit MUST expose source identity, artifact key/alias, path, revision/content digest and an exact span with its coordinate basis explicitly named. Selection explanation and source quotes MUST be separate. Frontmatter-edge provenance MUST NOT be falsely attributed to body quote lines. Status and evidence freshness MUST remain visible where present; no invented verdicts or trust scores.

For a fixed basis, chosen unit boundaries and identities MUST be deterministic and non-overlapping. Every successful non-final page MUST advance; if the next whole unit cannot fit alongside required metadata, return an actionable refusal rather than repeating the cursor.

#### Scenario: Long requirement and Unicode
- GIVEN a requirement contains Unicode, quotes and an exception beyond the old excerpt window
- WHEN selected
- THEN the exception and complete chosen unit are preserved byte-for-byte and its span resolves to those exact source bytes.

### Requirement: One pack, useful views

The structured response MUST include selected units, source-qualified graph edges/nodes, selection reasons, source coverage, traversal coverage and delivery completeness. CLI and MCP MUST expose the same underlying pack semantics. JSON and human CLI text MUST deliver complete reconstructable units, not first-line summaries. Optional Mermaid navigation MUST use the same selected subgraph and be explicitly identified as navigation, not an alternative evidence payload. Escape Mermaid labels and Markdown source boundaries so content cannot create fictitious graph edges or break the presentation structure. Source content remains untrusted data. Exact source retrieval MUST reuse the existing source-qualified get contract rather than a new independent store. A separate trace/search mode is not required: provenance and inclusion reasons belong to the same result.

#### Scenario: Human and agent views agree
- GIVEN a stable pack containing a cross-source declared edge
- WHEN JSON, human text and optional graph navigation are requested for the same basis
- THEN selection identities, edge directions and source revisions agree. Byte caps may yield different page boundaries by format; concatenated evidence pages reconstruct the same selected units.

### Requirement: Honest full-response budgets

Keep the existing budget option as an explicitly estimated token allowance; charge it against the full owned serialized response rather than just excerpts. Label the estimator accurately (UTF-8 bytes divided by four, rounded up), not exact model tokens or characters. Add an explicit byte limit for callers needing an exact transport cap. The effective cap MUST respect both constraints with checked arithmetic. Every successful CLI representation includes its actual emitted formatting/newline; MCP measurement includes the actual tool result envelope, JSON escaping, text and structured fields if both are emitted. The outer client-controlled JSON-RPC request id/framing and chat history are outside the owned tool-result cap. Metadata, graph, receipt, omissions and continuation all count. Receipt accounting MUST converge on the actual emitted representation, not a pre-wrapper estimate.

#### Scenario: Exact emitted boundary
- GIVEN a pack representation requiring N bytes after escaping and wrappers
- WHEN the cap is N
- THEN the emitted successful representation is at most N bytes; with cap below the required atomic unit the response refuses or delivers fewer whole units with explicit incompleteness, never an oversized success.

#### Scenario: Metadata dominates payload
- GIVEN a long task, many source names/omissions and Unicode/quoted text
- WHEN a small cap is requested
- THEN the actual output still respects the successful-response cap; bounded diagnostics never hide an incomplete-source or obligation condition.

### Requirement: Partial delivery is not complete context

Use separate fields for source coverage, obligation/traversal completeness and delivered-page completeness. A missing required source follows existing strict-source behavior; optional failures remain named partial results. A missing ground makes obligations incomplete even when its source was optional. A zero-match result MUST be distinguishable from source failure, traversal stop and insufficient budget. The strict-source option MUST NOT silently acquire different semantics; use a distinct require_complete option for callers requiring a complete selected basis in one response. A whole unit too large to admit MUST produce a typed actionable refusal, not a mid-unit byte fragment labeled useful evidence. For invalid caps below even the minimal error envelope, a bounded typed protocol/CLI error may necessarily exceed that invalid requested cap; this exception MUST be explicit and never reported as a successful bounded pack.

#### Scenario: One mandatory unit cannot fit
- GIVEN a selected ground is larger than the permitted whole-unit delivery
- WHEN a pack is requested
- THEN it is not cut; the refusal reports the limiting condition and a way to request sufficient space. No complete=true success is returned.

#### Scenario: Optional source and mandatory ground
- GIVEN an unavailable optional source contains a declared ground of a selected artifact
- WHEN allow-partial delivery is used
- THEN the readable material may be returned, but both source and obligations coverage remain incomplete; require_complete refuses.

### Requirement: Bound public continuation

CLI and MCP MUST accept and return continuation for the existing pack surface. The token MUST bind source versions, normalized query/selectors, retrieval/closure profile, traversal limits, presentation and position; changed bindings or malformed tokens MUST be refused. It is not an authorization credential. On a stable basis, repeatedly consuming pages MUST reconstruct selected whole units without loss or duplicate identity. Per-page graph/provenance metadata may repeat when needed and MUST be distinguished from repeated units. A cursor MUST not embed the entire corpus or grow with every previous page. A stateless validated cursor plus deterministic recomputation is acceptable; no new persistent service is required. Pages must advance; an oversized next unit produces an actionable refusal, never an endless empty continuation.

#### Scenario: Consume all pages
- GIVEN a stable selected set larger than one response
- WHEN a real CLI or MCP consumer follows returned continuation requests
- THEN each selected unit is delivered exactly once in stable order and the final page has no continuation; all revisions match the first page.

#### Scenario: Source changes between pages
- GIVEN a live pack continuation
- WHEN a source changes before the next page
- THEN the next request fails with an explicit stale/expired-basis error instead of returning new bytes; a pinned snapshot continues using the original available Git objects.

#### Scenario: Tampered request binding
- GIVEN a valid continuation
- WHEN its task, source set, view, traversal profile or cursor bytes change
- THEN incompatible continuation is refused, including for unadmitted source targets.

### Requirement: Proportional work and usefulness

Reuse the request's source capture/graph rather than scanning the whole corpus for each visited node. No full reindex, whole-corpus embedding job or external LLM extraction is started by pack. Any cache MUST be version/query/profile keyed and optional for correctness. Performance claims require measured cold/warm results on named corpora.

#### Scenario: Repeated bounded closure
- GIVEN several selected nodes share a ground
- WHEN closure is assembled
- THEN that ground is resolved once within the request and delivered once; all selection paths remain explainable without duplicate source reads per node.

## Behavioral Contract

Inputs: admitted sources or immutable snapshot, task and optional qualified seeds, lexical retrieval, versioned closure profile, traversal limits, full-response estimated-token/byte caps, presentation, distinct require_complete and strict-source settings, optional continuation.

Output: versioned selected units and scoped subgraph; separate source/traversal/delivery completeness; bounded omissions/diagnostics and spend receipt; optional next request. No source/index writes on read.

Algorithm: capture admitted sources once -> find/resolve seeds -> traverse declared edges with qualified identity and cycle detection -> form whole exact units -> stable order and basis digest -> validate continuation -> pack whole units while measuring the actual public representation -> emit data and explicit incompleteness. Fail before emitting a false complete result when requirements cannot be satisfied.

Invariants: source scope is not widened by links; exact historical versions never become current aliases; evidence does not migrate to successor versions; similarity is not authority; successful output satisfies the actual owned byte cap; continuation cannot substitute a new basis.

## Contracts

Preserve the existing pack command/tool name and task/budget/strict source flags; add options for explicit qualified seeds, byte cap, presentation, traversal bounds, require_complete and continuation. Update all Core/CLI/MCP callers together; no alternate legacy pack engine or silent compatibility shim. Keep estimated budget visibly approximate. Public docs and schema descriptions MUST be English and reflect actual supported behavior.

## Validation Rules

Reject zero/overflowing or implementation-maximum-exceeding limits before large work; reject invalid qualified seeds and ambiguous identities; reject contradictory handle selectors; verify continuation binding and expiry before using its offset; escape rendered content. Source failures remain typed. Administrative diagnostics are bounded and avoid exposing unrelated paths or internal secrets.

## Verification

Run focused Core behavioral tests and real CLI/stdio MCP scenarios above; inspect exact final serialized byte counts and whole-source-span reconstruction. No proof from merely nonempty output, copied mock values or token receipts. Keep all existing ownership/readonly/snapshot regressions. F10 remains the fixed thirty tasks in three modes on the same snapshots: expected grounds/conflicts and relevance criteria must be frozen before comparison; measure missed obligations, invalid references/omissions, citation accuracy, extra reads, payload volume, first useful source and cold/warm p50/p95 latency. Require zero source-boundary violations, version substitutions and hidden incomplete-negative answers on that set. F11 remains separate program acceptance, not implied by S3 tests.

## Versioning

Initial S3 completion contract. Freeze behavior before implementation; independent review is required. A scope or policy change requires explicit contract revision, not weakening tests after implementation.

## Related

- EPIC-010: Constellation program and S3/F08-F10 obligations.
- PRD-081: federated project context.
- ADR-028 / ADR-029: existing read-plane and version/ownership boundaries.
- SPEC-009 / ADR-030: canonical identity and exact reference resolution.
