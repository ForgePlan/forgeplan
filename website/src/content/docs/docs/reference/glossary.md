---
title: Glossary
description: Key terms used in Forgeplan methodology
---

| Term | Definition |
|------|-----------|
| **Artifact** | Structured markdown document (PRD, RFC, ADR, etc.) under `.forgeplan/`; files are authoritative and LanceDB is the derived index |
| **R_eff** | CLI reliability score: recursive min(own current evidence, eligible dependency scores after relation penalties). Own evidence uses verdict, CL, and type deductions; expired eligible packs instead score 0.1 before deductions. Terminal packs are excluded, drafts count; no eligible own evidence gives own score 0 |
| **Evidence** | Recorded test, measurement, benchmark, or audit that supports, weakens, or refutes a claim; declare `verdict`, `congruence_level`, and `evidence_type` in its body |
| **Congruence Level (CL)** | How closely the evidence context matches the decision context, from CL3 (same context, no penalty) to CL0 (weak transfer, 0.9 penalty); CL0 does not mean a refuting verdict |
| **Depth** | Rigor level: Tactical, Standard, Deep, Critical |
| **Pipeline** | Artifact creation sequence (e.g., PRD → RFC for Standard) |
| **ADI** | Abduction → Deduction → Induction reasoning cycle |
| **Blind Spot** | Active decision with zero linked evidence |
| **Orphan** | Artifact with no links to any other artifact |
| **Stale** | An expired `valid_until` is reported as stale; this does not automatically rewrite stored lifecycle status in 0.38. An expired eligible EvidencePack scores 0.1, not every expired decision |
| **Superseded** | Terminal state - replaced by newer artifact |
| **Deprecated** | Terminal state - no longer relevant |
| **Validation Gate** | Content/schema checks before normal activation; PRD/RFC/ADR/Epic/Spec also have length, filled-template, and linked-evidence gates |
| **Adversarial Review** | Independent review that actively looks for defects and reports evidence, scope, and limits; it must not invent findings to satisfy a quota |
| **PRD** | Product Requirements Document - what & why |
| **RFC** | Request for Comments - how to build |
| **ADR** | Architecture Decision Record - why this approach |
| **Epic** | Groups PRDs, RFCs, ADRs into initiative |
| **Spec** | Specification - API contracts, data models |
| **Problem** | Signal with context - bug, risk, observation |
| **Solution** | 2-3 variant comparison with weakest-link scoring |
| **Note** | Lightweight record of context or a small decision; `new` does not automatically set a 90-day expiry |
| **Refresh** | Re-evaluation of stale artifact |
| **Evidence Decay** | Expiry-based scoring of eligible evidence. Expired packs score 0.1; terminal packs are excluded rather than decayed |
| **F-G-R** | Formality, Granularity, Reliability. CLI artifact scores use 0–1 scales and a geometric mean; the FPF plugin's review rubric uses 0–3 |
| **FPF** | First Principles Framework - reasoning methodology |
| **MCP** | Model Context Protocol - AI agent integration |
| **Route** | Command that suggests depth + pipeline for a task |
| **Projection** | Markdown rendering synchronized with the derived index; markdown remains the source of truth, not disposable output generated only from LanceDB |
| **Workspace** | `.forgeplan/` directory with configuration, artifact markdown, and derived storage |
| **LanceDB** | Embedded derived index for artifacts, links, and vector embeddings; rebuild from artifact files with `reindex`, not `scan-import` |
| **DerivedStatus** | Computed progress label: STUB, SHAPED, VALIDATED, EVIDENCED, ACTIVATED; not a separately stored status or an independent quality guarantee |
| **BMAD** | Public `agents-bmad` greenfield pipeline: Analyst → PM → Architect → Scrum-Master → Dev → QA; needs `/bmad-init` for hook-based write enforcement |
| **SPARC** | Public `agents-sparc` single-feature pipeline in an existing active system: Specification → Pseudocode → Architecture → Refinement → Completion; phase ordering, no dedicated blocking hook |
| **TDD** | Public `agents-tdd` Build sub-cycle on an active PRD and scenario-bearing SPEC: plan → RED → independent validation/freeze → GREEN → Audit handoff |
| **Generator ≠ verifier** | The author does not certify their own output; pipeline handoffs require an independent context and recorded review verdict |
| **Frozen oracle** | The specification and tests against which code is judged; public TDD freezes the normalized SPEC hash and prevents test edits during GREEN |
| **Claim** | Time-limited coordination record for an agent working on an artifact; not a general file-write or methodology enforcement lock |
| **Git sync** | Incremental synchronization of artifact changes since a Git ref; does not generate embeddings or replace a full `reindex` recovery |
| **Restore** | Receipt-based recovery of accidental delete/deprecate/supersede, normally within 30 days; not a normal exit from terminal lifecycle states |
