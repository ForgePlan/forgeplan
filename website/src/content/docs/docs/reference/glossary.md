---
title: Glossary
description: Key terms used in Forgeplan methodology
---

| Term | Definition |
|------|-----------|
| **Artifact** | Structured canonical Markdown document with frontmatter (PRD, RFC, ADR, etc.); database rows are derived |
| **R_eff** | Weakest-link reliability. Explicit score reports include eligible evidence and dependency contributions; captured record fields use a flatter score. Terminal evidence is excluded; no eligible ordinary evidence means zero. |
| **Evidence** | Measurement, test, benchmark or audit with scoring verdict `supports`, `weakens` or `refutes`; explicit context/congruence fields matter |
| **Congruence Level (CL)** | How close evidence context is to decision context. CL3=same, CL0=opposed |
| **Depth** | Methodology rigor: Tactical, Standard, Deep, Critical; the current core parses Critical as a Deep alias |
| **Pipeline** | Artifact creation sequence (e.g., PRD → RFC for Standard) |
| **ADI** | Abduction → Deduction → Induction reasoning cycle |
| **Blind Spot** | Active decision with zero linked evidence |
| **Orphan** | Artifact with no links to any other artifact |
| **Stale** | Past `valid_until`; detection does not automatically rewrite the stored lifecycle status or force every artifact score to `0.1` |
| **Superseded** | Terminal state - replaced by newer artifact |
| **Deprecated** | Terminal state - no longer relevant |
| **Validation Gate** | Applicable kind/depth schema and content-heuristic checks; activation also has separate evidence, length and stub gates |
| **Adversarial Review** | Active search for evidenced defects. The documented review protocol allows zero confirmed defects with scope and unexamined risks recorded; native admission gates are separate and are not waived by this definition |
| **PRD** | Product Requirements Document - what & why |
| **RFC** | Request for Comments - how to build |
| **ADR** | Architecture Decision Record - why this approach |
| **Epic** | Groups PRDs, RFCs, ADRs into initiative |
| **Spec** | Specification - API contracts, data models |
| **Problem** | Signal with context - bug, risk, observation |
| **Solution** | 2-3 variant comparison with weakest-link scoring |
| **Note** | Micro-decision. A 90-day review interval is a methodology convention, not an expiry automatically assigned by the current `new` path |
| **Refresh** | Re-evaluation of stale artifact |
| **Evidence Decay** | Date-based scoring: an eligible expired item contributes `0.1` instead of its ordinary score; not gradual decay before expiry |
| **F-G-R** | Upstream FPF: Formality, ClaimScope, Reliability. Forgeplan separately implements Formality/Granularity/Reliability on 0–1 scales with geometric-mean overall scoring; not a probability or `min(F,G,R)` |
| **FPF** | First Principles Framework - reasoning methodology |
| **MCP** | Model Context Protocol - AI agent integration |
| **Route** | Command that suggests depth + pipeline for a task |
| **Projection** | Canonical Markdown file representing an artifact; not a disposable export from the index |
| **Workspace** | `.forgeplan/` directory with config + storage |
| **LanceDB** | Embedded database holding derived metadata and vector indexes, not canonical artifact content |
| **Constellation** | Versioned indexing/read protocol in local `0.39.0+constellation.s1.1`; legacy and Constellation layouts are not interchangeable |
| **Generation** | Published index state; a fresh canonical file read need not yet belong to the persisted generation |
| **Model cache** | Shared model/tokenizer files; distinct from semantic build support and workspace vectors |
| **Handle** | Process-local scope/snapshot reference in the separate `constellation-read` MCP profile |

See the [Constellation guide](/docs/guides/constellation/) and
[contract reference](/docs/reference/constellation/).
