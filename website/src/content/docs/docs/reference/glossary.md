---
title: Glossary
description: Key terms used in Forgeplan methodology
---

| Term | Definition |
|------|-----------|
| **Artifact** | Structured document (PRD, RFC, ADR, etc.) stored in LanceDB |
| **R_eff** | Reliability score = min(evidence_scores). Weakest link, never average |
| **Evidence** | Test result, benchmark, or audit finding that supports/weakens a decision |
| **Congruence Level (CL)** | How close evidence context is to decision context. CL3=same, CL0=opposed |
| **Depth** | Rigor level: Tactical, Standard, Deep, Critical |
| **Pipeline** | Artifact creation sequence (e.g., PRD → RFC for Standard) |
| **ADI** | Abduction → Deduction → Induction reasoning cycle |
| **Blind Spot** | Active decision with zero linked evidence |
| **Orphan** | Artifact with no links to any other artifact |
| **Stale** | Artifact past `valid_until` date. Score drops to 0.1 |
| **Superseded** | Terminal state — replaced by newer artifact |
| **Deprecated** | Terminal state — no longer relevant |
| **Validation Gate** | Quality check before artifact activation (30+ rules) |
| **Adversarial Review** | Reviewer MUST find problems. 0 findings = re-review |
| **PRD** | Product Requirements Document — what & why |
| **RFC** | Request for Comments — how to build |
| **ADR** | Architecture Decision Record — why this approach |
| **Epic** | Groups PRDs, RFCs, ADRs into initiative |
| **Spec** | Specification — API contracts, data models |
| **Problem** | Signal with context — bug, risk, observation |
| **Solution** | 2-3 variant comparison with weakest-link scoring |
| **Note** | Micro-decision, auto-expires in 90 days |
| **Refresh** | Re-evaluation of stale artifact |
| **Evidence Decay** | TTL-based score degradation. Expired = 0.1 |
| **F-G-R** | Formality, Granularity, Reliability — quality dimensions |
| **FPF** | First Principles Framework — reasoning methodology |
| **MCP** | Model Context Protocol — AI agent integration |
| **Route** | Command that suggests depth + pipeline for a task |
| **Projection** | Markdown file generated from LanceDB record |
| **Workspace** | `.forgeplan/` directory with config + storage |
| **LanceDB** | Embedded database for artifacts + vector embeddings |
