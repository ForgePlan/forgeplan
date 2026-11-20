# Round 1 acceptance review — CONCERNS

**Reviewer:** `ProductAcceptanceReview`  
**Model:** `openai-codex/gpt-6-astra`  
**Scope:** Independent documentary review of acceptance, evidence, traceability and delivery dependencies. This slice and the parallel scope review constitute **one round**.

One material finding: **A-001**, an inconsistent stage-exit dependency. The substantive acceptance model otherwise preserves the corrected discovery requirements. No implementation, runtime, market or activation approval follows.

## Reviewed freeze

Root: `/Users/explosovebit/Work/ForgePlan-Analytical-Product/`

| File | Frozen SHA-256 |
|---|---|
| `.forgeplan/prds/prd-document-k6a638805c55b95a65262c2e83492a6fe.md` | `984b5e67b966d7b3557d3597d12a5d58aecce7dbb94d7908acbb4df36557588f` |
| `product-prd-handoff.json` | `e5a607660a2b2d2b81a1d86521a2659c92bc92736ba46e6deadda2e296b3a956` |
| `product-prd-input-manifest.json` | `17a022a3b22304921aebc79c775010f3f9dd7334a2c47b57048410488feba91d` |
| Canonical EVID-225 | `8a660343e6f05dd4a3e3a14c461945878180c4a4a1ef20744266cba2c9c0986b` |

These are the freeze identifiers supplied by the assignment and read manifests; I did **not independently recompute hashes**.

Read the complete PRD and handoff, all eight HYP bodies, NOTE-067’s shared C1–C4 protocol, PROB-107, canonical EVID-225, the saved Round 2 discovery review and final PRD validation receipt. Also inspected the crosswalk inventory and relevant original dialogue sections on evidence, documentation and phased proof scenarios.

Discovery input fingerprints read from the manifest:

| Input | File SHA-256 |
|---|---|
| NOTE-067 | `1054167047e579b795bb9dea251d95b6c30a573bfe4df991977273876aac6eab` |
| PROB-107 | `294a846f8752167ea85214e5ea156dcb927dbeeaa322b880c6b40700d4b2d5d6` |
| HYP-001 | `b2941aa78f0e7a177ece3f886d28860af550a5dc58443a02a050f1165583a3bb` |
| HYP-002 | `32e8e8bf7c3ab8c8b745621cbfcbd5b0fedfd88dff40f2a4d3c539c03bfd85b5` |
| HYP-003 | `6365eb15fca5b90347fb666758f50633d23532ae03f30aad1d5ed67a117178ee` |
| HYP-004 | `837ca738720d6a7dca052c806eaa91b305ae5996dc28f6c56d88b1fbd7019b6e` |
| HYP-005 | `50423914396ec0818a2f79e1d4fec1dc2a18ca4276a3cae19ff53b39f5dbb9b2` |
| HYP-006 | `f9c1325540c8cc7ab41705740c2917bea8b5d01175c541adbb9d31e3a5849251` |
| HYP-007 | `936ef92abab97e75a707ac6376eb7785e8c582815b8eea374cc38ed061eb8a7d` |
| HYP-008 | `4069e9b5ba6575920effa675e7a7216461f506d7cea95ab55ca8a82c3af2df7b` |

## Finding

### A-001 — P1 exit requires integration evidence from dependent stages

**Severity:** Major  
**Disposition:** Required correction before acceptance of the phased PRD.

**Location:**  
`.forgeplan/prds/prd-document-k6a638805c55b95a65262c2e83492a6fe.md:312`, together with `:321–327` and `:171`.

**Condition:** AC-5 requires, **by P1 exit**, envelope-mismatch injections through:

> storage → federation → query → report/UI

Yet the stage table puts the relevant query/report functionality in P3, federation in P5, and mature UI surfaces in P7. Those stages depend on P1. The blanket stage rule says acceptance is required by the named exit; AC-5 does not distinguish foundation-contract verification from later integrated-product verification.

**Consequence:** A literal reading creates a dependency cycle: P1 cannot close without later surfaces, while those surfaces depend on P1. An alternative reading lets contract fixtures or early prototypes silently stand in for the promised production integration proof. Neither establishes an unambiguous, executable stage gate.

**Required correction:** Separate the proof obligations explicitly:

- Identify the foundation behavior and actual interfaces that must be exercised at P1 exit.
- Assign real end-to-end envelope preservation to the stages introducing each consuming surface, including federation and UI/offline export.
- State that earlier contract-conformance evidence does not discharge later integration acceptance.
- Carry these obligations into the stage/dependency matrix with responsible surfaces and evidence.
- If the intention is instead to build an early end-to-end vertical before P1 closes, name its required scope and dependencies explicitly.

Preserve FC-1’s complete mature-system invariant. Do not resolve the cycle by deleting downstream verification or presenting stand-ins as proof of the delivered surfaces.

## Coverage and substantive assessment

All **41 FRs, 12 NFRs, six foundation contracts, six top-level ACs and seven stages** were examined. The numbered FR register is not the entire contract: I also considered ownership, program admission, constraints, E0, success metrics, rollback and open-decision gates.

| Area | Assessment |
|---|---|
| Analysis layers and capabilities | FC-2/4, FR-02/25/32/38 distinguish syntax, semantic resolution, bounded behavioral models, observed execution and intended requirements. Independent expected facts prevent shared full/incremental omissions from passing. Unsupported, ambiguous, broken and stale inputs have explicit meanings. |
| Identity and provenance | FC-1 binds qualified subject/source identity, revision and dirty digests, configuration, analyzer/producer and policy. FC-2 handles uncertain continuity and tombstones. The integrated invariant is strong; **its stage allocation is A-001**. |
| Tests and evidence | FC-5 and FR-19/21/37 distinguish declaration, discovery, authenticated execution, coverage, assertion strength and scenario/parity evidence. Trusted wrappers containing attacker-authored reports, contradictory counts/exits, replay, substitutions and absent required checks are explicitly non-passing. |
| Graphs and context | FC-6, FR-04/18/33 and AC-3 cover typed directions, cycles/SCCs, explained inclusion, truncation, continuation and mandatory constraints. Missing edges do not establish independence. |
| Cross-repository compatibility | FR-13/14/39 and AC-1 require an independent bounded consumer inventory, version/configuration and rollout matrices, old-message cases, unchanged-schema semantic breaks and harmless structural edits. Unknown or unavailable sources cannot become “no consumers.” |
| Legacy and recovery | FR-23/24/27/30/38 and AC-2 preserve independent frozen observations, errors, state, effects, ordering, idempotency, approved divergences and post-write recovery. Anti-stub signals are not equivalence proofs. External execution ownership remains explicit. |
| Storage authority | FC-3, FR-01/31 and NFR-03/10 distinguish canonical, local, external and derived records. Recovery covers history, links and exception expiry—not merely rebuilding search. Atomic visibility and crash durability are separate. |
| UI, triage and authorization | FR-08–11/16/22/26/28 and AC-6 preserve coherent captured results, accessible alternatives, authorization and stale-state disclosure. Sustained arrivals and a frozen visibility/escalation horizon address starvation. |
| Documentation and offline use | FR-35/36 explicitly preserve manual ownership and normative content, identify stale sections, and require authorized standalone viewing without a server. Offline viewing is correctly separate from evidence replay. |
| Ownership and admission | FR-41, ownership boundaries and the pre-code chain prevent accidental adoption of EPIC-009, absorption of protected work, or conversion of product-document review into architecture/execution authorization. |

NFR thresholds are not falsely claimed as measured. Cross-cutting applicability is supplied by `:250`, surface ownership and the named gates. Concrete future owners remain assignable, but unassigned experiments are explicitly blocked.

### Three proof scenarios and prerequisite assessment

- **AC-1, three repositories:** Testable and falsifiable once E0 freezes the corpus and decision rules. P4 trusted-check capability precedes P5.
- **AC-2, legacy replacement:** Testable within a declared boundary; P4 evidence/oracle execution precedes P6, and P5 is required when the seam crosses repositories. Recovery after actual new writes is mandatory.
- **AC-3, unfamiliar agent task:** Includes the discovery reading/documentation negatives and measures complete solutions rather than tokens alone. Independent external evaluation need not wait for ForgePlan’s own P4 adapters. I found no requirement that makes the product its own exclusive runtime oracle.
- Product review is not conditioned on completing these future experiments. The separate SOLUTION and architecture gates remain subsequent work.
- The explicit stage-exit inconsistency is **A-001**, not a reason to reduce the mature target.

## E0 assessment

**Acceptable at PRD stage.** The open numerical parameters are bounded decisions, not empty promises:

- Decision units, segments and denominators include missing facts, unknowns, failures and attrition.
- Three comparator arms have equivalent information, access, training, models and task limits.
- Useful-effect and full-cost thresholds require baseline and stakeholder rationale.
- Safety ceilings are consequence- and policy-derived and cannot be traded for token savings.
- Sample and horizon require precision, detectable-effect or bounded-coverage justification.
- Exploratory calibration is separate from confirmation.
- Corpus, oracle, versions, normalization, exclusions, stopping rules and independent approval are frozen before confirmation.
- Changes require a new protocol and decision; inconclusive results cannot become acceptance by removing failed cells.
- Costs include onboarding, integration, cold/index operation, provider/cache/retries, human review, migration, parallel operation, recovery and maintenance.
- Task benefit, UI benefit, adoption, authentic execution, parity and backend preference remain separate claims.

No invented sample sizes, performance targets or customer evidence are needed to correct this PRD.

## Validation and limits

The saved validator receipt reports zero errors, two SHOULDs and one COULD at **Deep**, while the artifact’s raw depth is **critical**. Inspection supports the handoff’s explanations:

- `todo!` is an anti-stub detection example, not unfinished work.
- The intended canonical links are present; ADR-003 is contextual ancestry through ADR-026.
- The FR warning is a checkbox-format mismatch, not absence of the 41 table requirements.

These observations do not provide Critical acceptance or override A-001.

**No writes, mutations, commands, revalidation, builds, tests, formatters, commits or reasoning reruns were performed.** No empirical claim was independently verified.

**Signed:** `ProductAcceptanceReview` · `openai-codex/gpt-6-astra` · **Round 1 CONCERNS — A-001 open**