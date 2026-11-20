## Verdict: CONCERNS — documentary planning admission needs two corrections

**Signed:** RfcFitnessRoundOneSol, independent Profile B architecture-fitness equivalent, actual model `openai-codex/gpt-6.1-sol`. Not the author. Round 1 only; not whole-pipeline approval.

### Frozen scope verified

Product branch/base match the assignment. RFC canonical key `rfc-rust-analytics-captures-kbe91e3dbf1920005719327653381b2fe`, draft/Critical, 14 links.

- RFC SHA-256: `fdd53ac34a1268e10ed77c09f6e40e04793b32d60cc65325df61ef81904160de`
- Exact post-frontmatter body: `d31924ef5a7cbaf299173a7f190b880e71476dbced8a2f1dd4d72697199190af`
- Handoff: `44f456eaa29081906a65b06095fecd9a48b9a2791e099844e8a2abfccd8ec34b`
- All 25 manifest entries match both Product copies and original upstream files.
- Both FR-41 research receipt hashes match.
- Independently counted 94 unique SPEC scenarios, 41 RFC FR crosswalk rows and 26 distinct DAG slices. Explicit table dependencies resolve and are acyclic.

References below use Product-relative paths; **RFC**, **PRD**, **SPEC** mean the exact assigned canonical files.

### Findings

**F1 — MEDIUM: seven-family acceptance is scheduled inconsistently.**

RFC:367 requires the seven-family positive/negative matrix at `P3-QUERY`. However, policy/evidence integration is delivered by dependent `P4-EVID/P4-PROOF` (RFC:370–371), and calibrated estimation is delivered by `P6-EST`, itself dependent on P3 (RFC:377). RFC:488 retains the upstream P3→P5 label while adding P6-EST.

PRD:178 and SPEC:108–122 require seven **useful results through actual public queries/extension contracts**, including calibrated extension uncertainty, across P3→P5. A schema-only contract or an early “uncalibrated” negative does not discharge the positive case.

**Consequence:** a literal P3 acceptance imports unavailable future outputs; interpreting it as contracts alone leaves actual seven-family completion insufficiently allocated and moves part beyond its frozen deadline.

**Required disposition:** assign each family’s actual positive/negative closure to a concrete slice and deadline. Keep P3 contract preparation distinct from P4 evidence completion. Supply the existing calibrated extension path by the upstream deadline, or obtain an explicit upstream stage amendment. Do not add a P3↔P6 dependency cycle.

**Strongest counterargument:** “seven family contracts” intentionally means only early contracts, with later implementation covered by the crosswalk. That is plausible, but the acceptance column demands the seven-family matrix, and the document declares this table the sole execution-order source. The crosswalk does not settle which actual positive receipts permit closure.

---

**F2 — MEDIUM: AC-5’s controlled publication versus production admission boundary remains implicit.**

PRD:312 and SPEC:838 require AC-5 before “migration/publication” while also requiring actual build→publish→captured-read→snapshot/export proof. RFC:363–364 schedules implementation followed by recovery acceptance. RFC:450 clarifies draft-document publication versus runtime generation acceptance, but does not distinguish controlled runtime publication used to prove AC-5 from production admission. RFC:616 repeats the ambiguous deadline.

**Resolution:** controlled P1 publication is coherent with the frozen upstream contract: its explicitly required actual publication/fault/recovery proof cannot be obtained without exercising publication. This does **not** permit production cutover before AC-5.

**Required disposition:** state that distinction explicitly. Identify separately authorized controlled validation publications and their protected scope; production migration/publication admission remains blocked until P1-REC/AC-5 passes. Preserve real publication, held-reader, crash/durability, restore and per-field mismatch evidence. This is clarification, not a waiver of upstream acceptance or replacement with prototypes.

**Strongest counterargument:** AC-5’s publication exercise already makes this interpretation evident. Nevertheless, the RFC’s explicit explanatory clause currently distinguishes only documents from runtime, leaving the consequential runtime admission boundary unresolved.

### Coverage and positive findings

| Criterion | Assessment |
|---|---|
| Requirements/scenarios | All 34 F-items, 41 FRs, 17 themes, 12 NFRs, six FCs and six ACs have documentary mappings. All 82 B positive/negative groups and 12 X scenarios remain allocated; internal cells are mandatory, not sample choices. RFC:479–627. |
| Concrete architecture | Logical responsibilities, dependency direction, immutable binding types, boundary signatures, failure states and publication ordering are sufficiently concrete for planning. No needless crate-per-module requirement. RFC:159–241. |
| A/B/backend neutrality | Both full-target Rust alternatives survive; common assertions do not prohibit native partitions. Lance remains viable; SQLite, ranking, transport and portable identity remain separate decisions. RFC:243–245, 285–305. |
| Agent usability | Bug/API/onboarding routes, whole constraints, placement conflicts, alignment violations and independently solved-task oracles are explicit. RFC:247–255. |
| Mature surfaces | CLI/MCP, Web/Tauri, text/list/diff/2D/3D, editor, agent handoff, factual docs, standalone offline and optional team delivery remain in scope with separate permissions and actual consumer proofs. RFC:279–283, 324–330, 378–380. |
| Integration oracles | Independent expected missing facts/consumers, protected report-production authenticity, frozen held-out behavior/effect oracles and post-write recovery are preserved. Shared envelope/full-incremental agreement cannot substitute. RFC:324–330. |
| FR-41/authority | All 16 FPV dispositions and concrete map/TaskLink/local-policy deltas are retained without adopting the umbrella, overriding ADR-001 or absorbing protected work. RFC:415–477. |
| DAG usability | Explicit graph is usable and acyclic. P1 avoids future query/federation/UI dependencies; local P6 avoids P7. P7 evidence/legacy lens enablement requires relevant later proof. The semantic closure issue in F1 remains. RFC:334–392. |

### Open choices, not documentary defects

A/B selection, backend/ranking, domain-key encoding, portable identity, wire/transport formats, exact owner-approved producer boundaries, trust roots, retention and E0 parameters remain legitimately open. The RFC names responsible roles, blocking gates and affected enablement; it does not fabricate defaults or results. These choices must close before their affected execution/acceptance, not necessarily before admitting the corrected plan.

### Limits

This review establishes documentary fitness and the two stated gaps, not implementation feasibility or delivered conformance. Native validation/ADI outcomes are author-reported inputs, not independent runtime proof. Core ADR-031’s no-quota rule does not waive the second Critical round or other specialist/guardian gates.

**NOT_RUN:** native CLI/index/validation/reason reruns; builds, tests, linters, formatters, experiments, source/runtime audits, activation, tracker operations or writes of any kind. Only read-only document inspection, fingerprints, counts and explicit DAG consistency processing were performed.