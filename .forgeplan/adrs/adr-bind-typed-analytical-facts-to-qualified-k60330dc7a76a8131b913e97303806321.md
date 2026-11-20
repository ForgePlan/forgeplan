---
assigned_number: 34
author: null
created: 2026-11-20
depth: critical
id: ADR-034
kind: adr
links:
- target: prd-document-k6a638805c55b95a65262c2e83492a6fe
  relation: based_on
- target: solution-rust-k510d4bfe9ce8b00c10cca1094f17db7d
  relation: based_on
- target: evidence-solution-proposed-input-independent-kcbb00a779e0884c3458714ae3e2df403
  relation: based_on
- target: adr-own-cross-domain-rust-analytics-keep-kf1722a1f4ac7e2bfa083f25edf92c62e
  relation: based_on
parent_epic: null
predicted_number: 34
problem_ref: null
slug: adr-bind-typed-analytical-facts-to-qualified-k60330dc7a76a8131b913e97303806321
status: draft
tags: []
title: Bind typed analytical facts to qualified subjects and captured configurations
updated: 2026-11-20
valid_until: null
---

---
assigned_number: 34
created: 2026-11-20
predicted_number: 34
problem_ref: null
slug: adr-bind-typed-analytical-facts-to-qualified-k60330dc7a76a8131b913e97303806321
updated: 2026-11-20
---

# Bind typed analytical facts to qualified subjects and captured configurations

## Status

D-GRAPH — Canonical ADR draft / Critical, 2026-11-20. PROPOSED and UNACCEPTED: publication records proposed contracts and open gates, not architecture activation, an A/B or backend choice, implementation authorization or delivered capability. The D-* name is a local traceability convention; the artifact's allocated canonical key/frontmatter is authoritative. No prior ADR is superseded. Active ADR-001/003/025/026 and current independently owned contracts remain binding; new analytical producer exceptions require explicit owner clarification or an approved amendment, not an implied bypass.

This publication succeeds the immutable disk-hold preparation. Corrected canonical EVID-229 (key evidence-solution-proposed-input-independent-kcbb00a779e0884c3458714ae3e2df403, SHA-256 865fba7aa30832200f2b7b90b7e75f3044c498319f22307f055c6b0c16167ae3) records one proposed-input round with two independent Sol judgments; it is not final Critical architecture acceptance. Actual native command/ADI/validation exits, provider, parsing and input-generation limitations are recorded under .local/design-adr-publication/, not presumed successful in this body. No F/G/R score or C4 acceptance is fabricated. Runtime/product experiments, builds, activation and downstream RFC remain NOT_RUN.

## Context

Display IDs repeat across repositories and sometimes within one repository. A path, name, source span or content hash alone is not durable semantic identity: rename/move, overloads, trait resolution, generated code and different configurations make those shortcuts unsafe. Conversely, inserting a new global identifier everywhere would silently replace existing Constellation qualification and pending identity work.

The PRD requires one binding through storage, federation, queries, evidence and every consuming surface. Separately valid layers referring to different subjects must not compose into PASS. Existing `(source_id, canonical_key)` and `KEY@qualifier` routes are reused. Core draft ADR-030 describes permanent keys/display aliases and path-derived sources; it is relevant context, not newly activated by this body. Source portability needs an explicit owner-reviewed delta rather than a claim that a path-derived ID already survives relocation.

## Decision drivers

- FC-1 requires exact source, subject, revision/dirty content, configuration, producer, policy and evidence binding.
- FC-2 requires typed entities/edges, spans, methods, uncertainty and continuity evidence.
- FC-6 requires deterministic bounded queries and semantic parity across human, machine, agent, UI and offline consumers.
- FR-03/07/12/14/15/22/31/32 require preserved identity/history, one authoritative link resolver, portability and workspace isolation.
- Existing qualified source identities and protected current main contracts must not be replaced using stale worktree precedent.

## Considered options

### A — One untyped graph keyed by names or file locations

**Pros:** straightforward ingestion; useful exploratory visualization with minimal mapping.

**Cons:** conflates imports, calls, executions and intent; rename/configuration changes merge or detach facts silently; ambiguous IDs cannot safely target writes or evidence. A visualization convenience becomes a false analytical authority.

**Assessment:** unsuitable as normative contract; may only be a derived display with preserved typed provenance.

### B — Qualified typed subjects and assertions with explicit revision/configuration context

**Pros:** preserves existing source qualification; supports historical lineage, mismatches and ambiguous resolution; lets A and B kernels share result semantics without forcing one physical graph.

**Cons:** every producer and consumer must carry richer context; historical identity repair requires owner judgment; continuity cannot always be resolved automatically.

**Assessment:** proposed logical contract because it preserves FC-1/2 without selecting storage. Runtime completeness is unproved.

### C — Replace source and subject identity with global content-addressed objects

**Pros:** exact bytes and transport integrity can be addressed independently of local paths; attractive for immutable bundles.

**Cons:** byte identity does not establish conceptual continuity or author identity; identical bytes may have different owners/configurations; every edit changes the digest; wholesale cutover would conflict with existing canonical keys and references.

**Assessment:** retain digests for revisions/content and transport, not a replacement for subject identity. A future portable source scheme must be independently decided.

## Decision outcome

Propose B. Distinguish four identities rather than overloading one field:

1. **Subject address:** existing source identity plus canonical key for artifacts; versioned producer-qualified symbol/contract/test identities for their domains. Domain key encoding remains a reviewed RFC/profile decision, not a guessed universal symbol hash.
2. **Occurrence:** a subject's exact revision/content digest, file/span and configuration. Configuration includes relevant features, target, dependencies, generated inputs and framework/producer settings. Dirty content participates explicitly; HEAD alone is insufficient.
3. **Assertion:** typed relationship or fact, with origin/method, producer/tool version, spans/call sites, evidence references, evaluation policy and uncertainty. Conflicting assertions coexist with diagnostics rather than last-writer-wins truth.
4. **Capture/result:** exact source-set and configuration bindings, generation/capture identity, evaluated time, policy and omissions, allowing coherent replay without claiming an atomic snapshot of arbitrary external editor writes.

A result carries these bindings directly or via immutable digest-linked manifests resolvable within its declared scope. Referential compression may reduce payload size but must not make verification depend on an unavailable mutable service. Wrong source/subject/revision/dirty/configuration/producer/policy/evidence binding is rejected or explicitly incomplete, never silently relabeled current.

### Logical entity and relationship contract

Keep source, revision/configuration, file, symbol, endpoint/schema/event/package/FFI, artifact/scenario, test/run, evidence, policy and finding distinct. Preserve `imports`, `references`, `type-reference`, `calls`, `implements`, `reads`, `writes`, `publishes`, `consumes`, `declares` and verification relationships with explicit domain/range and direction. An invocation has its call site; a type-only dependency is not a runtime call. A static test-use assertion is not an executed-by assertion, and neither alone proves scenario verification. Cross-service edges require contract/producer/consumer evidence, not class-name or HTTP-name similarity.

Analytical edge kinds are NOT additions to ForgePlan's five canonical artifact-link relations (`informs`, `based_on`, `supersedes`, `contradicts`, `refines`). Preserve this vocabulary boundary in import/export and UI filters; a graph traversal cannot turn an analytical call into a lifecycle dependency by accident.

`resolved`, `heuristic`, `ambiguous`, `unknown` and `stale` are distinct result dimensions, not confidence percentages. Unknown includes unsupported language/configuration, parse/read error, missing/denied source and dynamic behavior with reasons. Freshness and permission are not erased by a resolved identity. No numeric total ordering or probability is inferred from these labels.

### Continuity and linking

Rename/move/deletion/overload resolution must cite explicit continuity evidence. Same name, line or hash is insufficient to merge. Unresolved candidates remain separate or ambiguous; tombstones/history retain deleted subjects and broken inbound links for drift. Resolve authored frontmatter links, doc-tags and proposed diff links through one authoritative resolver; conflicting declarations are visible. Inferred auto-links remain proposals until accepted through authorized canonical operations. A title or alias edit must not rewrite permanent artifact identity.

Use the existing per-call workspace resolver, explicitly separate the maintenance target, and bind caches/views to the resolved workspace and capture. Do not invent a registry to fix already-closed #303/#305 without a reproduced current defect. Qualified chip/editor navigation validates target existence, version and permission before opening; ambiguous/deleted/stale/forged destinations cannot pick another object or execute arbitrary commands.

## Consequences

### Positive

- Shared binding supports compositional queries and rejection of mixed-subject evidence.
- Typed relation direction makes impact explanations and test/contract distinctions inspectable.
- Canonical subject history survives storage/backend or renderer changes.

### Negative and risks

- Rich identity/provenance increases schema and migration obligations; all callers must be inventoried.
- Historical ambiguity can block repair or lineage claims and may require human decisions.
- A stable address does not establish semantic equivalence, compatibility or authorized distribution.

### Neutral

Logical graph semantics are independent of relational tables, in-memory graphs, native language partitions, .sexp/JSON export, map interchange and 2D/3D layouts. None is selected as universal storage or transport here.

## Compliance and evidence gates

Identity owner and Constellation/current-main owners must reconcile source portability and current capture/read-maintenance contracts before affected P1 execution. Required proof covers duplicate display IDs/canonical collisions, relocation and independent clones, title/body preservation, rename/move/trait ambiguity, deletion/tombstones, generated/configured subjects and rejected cross-field substitution. Use current source fingerprints, not historical issue assertions.

EP-1 covers delivered foundation write/reference→publish→captured read→snapshot/export. EP-2 adds real producers; EP-3 adds query/CLI/MCP/docs; EP-4 actual reports; EP-5 actual delivered repositories/CI; EP-6 actual oracle/dossier; EP-7 actual UI/offline/team consumers. Shared envelope fixtures are reused obligations, not seven independent proofs. E0 owners and experiment parameters remain blocking prerequisites where measurement is needed.

## Invariants

No first/last/freshest alias winner; no name-based subject merge; no silent relation coercion; no missing source as zero consumers; no aggregate coverage as per-test attribution; no schema version as publication protocol; no renderer-created approval. Required constraints remain whole even if a query budget is insufficient.

## Rollback Plan

Inventory existing source-qualified keys, reference forms, resolver callers and schemas before a delta. Preserve existing keys and file paths; any real collision repair needs an explicit attribution plan, preimages and unresolved-reference refusal. Version fact/result formats and keep unknown fields or reject unsupported versions rather than reinterpret them. Rebuild only derived projections, verify identity/link/history census and old consumers, then publish coherently. Failed migrations leave the previous usable generation and canonical bytes intact. Reversing a source-ID algorithm requires explicit mapping and transport replay; do not simply rewrite aliases across histories.

## Revisit Trigger

Reopen on a silent identity merge, redirected qualified link, incorrect continuity assertion, cross-config fact leak, failed relocation/replay, new domain whose relationships cannot be expressed without loss, or a material current-owner compatibility conflict. Preserve a minimal source-qualified counterexample and suspend only the affected claim/profile; no silent schema weakening. Any one event is sufficient.

## Open questions

Portable source identity algorithm, domain key encodings, relation schema evolution and exact compatibility negotiation remain OPEN for RFC decision/evidence. The logical distinction between subject, occurrence and assertion is proposed now; it does not activate draft ADR-030 or supersede current protected ADR-032/PRD-092/RFC-021/SPEC-013. Display labels across projects are not globally unique.

## References and proposed canonical relationships

- Corrected PRD `prd-document-k6a638805c55b95a65262c2e83492a6fe` — based_on; FC-1/2/6; FR-03/07/12/14/15/18/22/31/32/33.
- Frozen SOLUTION `solution-rust-k510d4bfe9ce8b00c10cca1094f17db7d` — based_on; A/B-independent envelope.
- Core `src:c3baf53b9bec0501`: `adr-canonical-key-as-authoritative-artifact-identity-display-id-as-alias` (draft), `adr-constellation-manifest` (draft) — context/informs only, no new acceptance.
- Core ADR-003, `adr-storage-classes-for-machine-written-records`, `adr-orchestration-sits-above-forgeplan-the-core-is-the-contract-layer` — binding authority/ownership context.
- Existing Web `src:61600c6e7e40d4b0` SPEC-006 map interchange and Marketplace `src:003b4e75d638bcca` tracker/EvidencePack designs — qualified context requiring current-key/status resolution before canonical linking; no guessed keys.


## Publication and upstream admission boundaries

The six allocated ADR drafts retain the complete F-01–34, FR-01–41, T01–17, NFR-01–12, FC-1–6 and AC-1–6 target and the preparation inventory's traceability, without automatic adoption of EPIC-009/FPV-00…15. Corrected EVID-229 is documentary proposed-input admission only. Native reasoning is author analysis, not independent evidence, empirical verification or final Critical approval. Raw/export Critical and any validator Deep classification, provider/exit/parse warnings and last-published-generation caveats belong to actual receipts and remain disclosed.

The preparation publication-manifest.tsv SHA-256 867fdca01fcef9e62418351216163b253925c1bb889f0128287058df902b23c2 and every preparation file remain immutable. Current publication framing replaces the historical noncanonical/disk-hold status; local labels retain traceability only. SPEC-015 is separately published in Product, absent from this Decisions input set, and is not linked as though locally present; successors meet once in the authorized downstream RFC. The future RFC must reconcile exact keys, contracts and dependencies rather than produce a competing SPEC. No absent cross-project record is a native link target.

Core FR-41 research local://analytical-fpv-disposition-research-full.md (SHA-256 589e66bffd7e9e942aedde942eb7c26d6eea5f93dc02f770ba57c9fc100c359d) and Web/Marketplace boundary research local://analytical-consumer-boundary-research.md (SHA-256 1d54e0adeb4e857ef7bc86c0ab4320b0ebb1d3b7c4b86c289354435bc20341b1) are proposed documentary dispositions, not policy overrides or implementation proof. EPIC-009 is unaccepted and PROB-082/FR-41 remains OPEN. Current FPV dependency records agree on 02+03→07; the contrary preserved import is historical. Freeze a reviewed dependency source and resolve compiler/policy bootstrap and later evidence-model/consumer edges before execution; this does not authorize sixteen inherited tasks. Current maintenance/capture/allowlist discrepancy requires current owners' reconciliation, not a precedence guess or demonstrated runtime bug.

## Owner-reviewed map and tracker identity deltas

Web source src:61600c6e7e40d4b0 SPEC-006 and Marketplace source src:003b4e75d638bcca SPEC-003/EPIC-004 are active contracts, not automatically extensible containers for new analytical meaning. Preserve pure geometry/no supplied node coordinates, stable presentation identity and legacy base-edge `{from,to,relation}` compatibility. Do not promote 12-hex path hashes to canonical semantic subjects, bare artifact_id to federated identity, or grep/confirmed markers to resolved semantics or execution proof.

The inspected Web schema permits unknown relations while Marketplace G3 restricts relations to its existing eleven and INV-1 permits only four named extra edge keys. Joint owner-approved delta RFC must resolve this precise conflict through typed-edge/provenance versioning, old/new consumer inventory, explicit version migration for breaking meaning, rollout and retained-map rollback. Ignoring unknown optional keys does not prove mandatory semantics were understood. Required real emitter→guardian→GET→validator→renderer negatives include duplicate qualified aliases, stale/grep provenance, unknown relations, missing endpoints, partial writes, unauthorized metadata and old consumers ignoring mandatory meaning.

Marketplace TaskLink migration must extend current bare artifactId addressing with source-qualified canonical identity/provenance while preserving native external task identity. Exact serialization is OPEN. Require ambiguous/missing legacy-link disposition, owner-confirmed backfill and rollback without task/history deletion; preserve non-null existing links and runtime-discovered field/option UIDs. Web's reference-not-body rule versus Marketplace's derived checklist text needs an explicit permitted-derived-checklist boundary, not silently duplicated artifact authority. No external keys are guessed from filenames/slugs; only certified present keys may become native links.

## Preconditions

Before implementation: current identity/Constellation owners reconcile portable addressing and protected capture/maintenance contracts; canonical binding/relation schemas and supported consumers are frozen, independently reviewed and separately authorized. Ambiguous lineage or legacy references require owner attribution, not a guessed repair.

## Postconditions

Every introduced producer/storage/query/report consumer preserves exact qualified subject/configuration binding, typed relations, explicit uncertainty and retained lineage/history. Required mismatches reject or remain incomplete; old-consumer and rollback/replay evidence precedes cutover.

## Affected Files

This publication owns only the newly allocated ADR file. Future scope is identity/resolution/capture, analytical relation schema, authored-link ingestion, query/export/navigation and consuming DTO callers. The downstream RFC must inventory actual source files and owners; no existing source, map schema or tracker record is modified here.
