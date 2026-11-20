## Round 1 verdict: CONCERNS

**Reviewer:** ProductScopeReview  
**Model:** openai-codex/gpt-6-astra, dispatched at high  
**Scope:** Independent documentary review of product scope, source preservation, ownership and architectural boundaries. This is one slice of Round 1, not another review round.

Two bounded corrections are required. The mature product target is substantially preserved; neither correction calls for reducing it or selecting an architecture prematurely.

### Reviewed freeze

Artifact: `prd-document-k6a638805c55b95a65262c2e83492a6fe` / PRD-093, draft/critical.

Root: `/Users/explosovebit/Work/ForgePlan-Analytical-Product/`

| File | Independently observed SHA-256 |
|---|---|
| `.forgeplan/prds/prd-document-k6a638805c55b95a65262c2e83492a6fe.md` | `984b5e67b966d7b3557d3597d12a5d58aecce7dbb94d7908acbb4df36557588f` |
| `product-prd-handoff.json` | `e5a607660a2b2d2b81a1d86521a2659c92bc92736ba46e6deadda2e296b3a956` |
| `product-prd-input-manifest.json` | `17a022a3b22304921aebc79c775010f3f9dd7334a2c47b57048410488feba91d` |
| `product-prd-frozen-selective-export.json` | `1f1bace49f583f53501a80d95fae9e7927eb6ab349e730a5646c474a00d80741` |
| `product-prd-frozen-crosswalk.json` | `842ec39a87a35ecd6fd1efdb967fe561d164e2e254b4a7266860715cb6a96c0b` |
| `product-prd-final-validate.json` | `07b8c9bffb28363c190afd5cfa5fc45d79ab0f81494204da362b64425d064234` |

All **19 input-manifest entries matched their recorded file hashes**, including ten discovery artifacts, EVID-224/225, four original research/rule files and three receipts.

Read the complete 404-line PRD, handoff, manifest, NOTE-067, PROB-107 and EVID-224/225; the complete 17-section dialogue appendix, all 34 source-register entries, relevant expanded roadmap/discussion sections and the factual audit. Read authoritative ADR-025/026/003, EPIC-009/PROB-082 and the relevant EPIC-010/Web boundary contracts. Individual HYP bodies were fingerprint-checked, not independently re-reviewed in this slice.

## Required corrections

PRD references below use the canonical file identified above.

### S-001 — Unfamiliar-stack review remains an underspecified “cognitive lens”

**Severity: Medium — scope/observable-outcome gap**

**Location:** PRD `:87`, `:200` (FR-26), `:202` (FR-28), `:313` (AC-6).

**Condition:** The PRD identifies a reviewer lacking expertise in the stack, names cognitive lenses and retains self-described expertise. However, its required card contents are change/affected/check revision/unknown/decision. It never requires an explanation of behavior in domain terms or evidence-backed input/output/error/effect examples that let that reviewer understand the decision without first interpreting unfamiliar source code.

The source intent is more specific:

- `dev/dialog.txt:1526`: the owner explicitly requests reduced cognitive load where expertise is missing, in CLI tools and Tauri UX.
- `dev/dialog.txt:1555,1559,1602`: behavior explanations and concrete examples.
- `dev/forgeplan-research-roadmap.md:1109-1111,1146`: behavior cards and unfamiliar-stack presentation.

The source’s rigid card counts, five-second promise and blame-based expertise inference need not survive. The underlying comprehension outcome does.

**Consequence:** A technically correct queue exposing raw diffs and provenance can satisfy the enumerated FR-26 fields and view-consistency checks while leaving this user unable to understand the behavior being accepted. F-26/T13 therefore has partial, not complete, substantive preservation.

**Required correction:** Extend FR-26/28 and the corresponding P7 acceptance to require:

- a plain-language account of the behavioral assumption/change and its consequences;
- source-bound examples from actual characterization/test evidence where available, including relevant errors/effects; absent examples remain explicitly unavailable, never fabricated;
- access to detailed code/evidence without requiring it for the initial explanation;
- the same substantive review information through CLI/MCP and Web/Tauri presentations;
- an unfamiliar-stack reviewer evaluation that checks understanding and correct decisions, separately from speed, accessibility and backlog visibility.

No particular widget, fixed example count or automatic approval mechanism is required.

### S-002 — Placement and architecture-alignment outcomes are retained mainly as command names

**Severity: Medium — scope/observable-outcome gap**

**Location:** PRD `:192` (FR-18), `:323` (P3).

**Condition:** FR-18 lists `place/align/compare/changes/read-first`, but its query acceptance requires only distinguishing accepted rules from guessed layers, base/head and why-read. FC-6 gives substantive impact/reading behavior; other requirements cover revision comparisons. Nothing comparably states what `place` must recommend or what `align` must evaluate and report.

Original outcomes are explicit:

- `dev/forgeplan-research-roadmap.md:215`: recommend where new code belongs using active RFC/ADR constraints.
- `dev/forgeplan-research-roadmap.md:216`: detect architectural-boundary violations against declared rules.
- The dialogue appendix distinguishes configured architectural rules from inferred layers and scoped conformance from universal correctness.

**Consequence:** A query that labels a layer “accepted” or “guessed” could satisfy the stated observable without recommending a placement or identifying a prohibited dependency. This leaves part of F-18/T03 represented by labels rather than a product outcome.

**Required correction:** Give these two query families explicit observables, within FR-18 or stable child criteria:

1. **Placement:** return an explained candidate location/boundary consistent with applicable accepted constraints, or explicit conflict/insufficient information.
2. **Alignment:** evaluate declared architectural constraints against scoped facts and report violations, supporting paths/spans, unchecked areas and uncertainty separately from inferred architecture.

Include positive/negative corpus cases for a valid placement, conflicting constraints, a prohibited dependency and an unresolved boundary. Leave algorithms, schema and CLI names to SOLUTION/SPEC/RFC.

## Coverage assessment

The mapping was checked against normative requirements and supporting contracts, not merely the F-item crosswalk.

| Source items | Substantive preservation |
|---|---|
| F-01 | Storage alternatives, explicit candidate mechanisms, RU/EN/identifier comparison, durable-class recovery and full migration cost. |
| F-02 | Own Rust graph with reusable syntax/semantic producers and independent completeness oracle; not adapter-only. |
| F-03 | Durable artifact/scenario/code links, revision binding, rename/delete/ambiguity and history. |
| F-04 | Seven command-family outcomes, conflict/unknown handling and separate Core/extension/executor ownership. |
| F-05–07 | Real dependency separation, ranking/vector provenance, authoritative code-reference resolution and proposed—not automatically accepted—links. |
| F-08–11 | Supported layered 3D, transport failure semantics, common Web/Tauri data contracts, watchers/editor/multiworkspace/inspector. |
| F-12–15 | Qualified identity, bounded federation, actual portable contents and per-call workspace isolation. |
| F-16–17 | Optional Hub/source adapters and workflow-sized MCP/UI bridge with permissions and measurements. |
| **F-18** | Reading, comparisons and language-extension delivery retained; placement/alignment gap **S-002**. |
| F-19–22 | Four CI stages, protected policy, operation journal/dry-run, attestation trust and authorized navigation. |
| F-23–25 | Legacy seams/progress, dossier/behavior ledger, contextual anti-stub signals, framework/DI/re-export limitations. |
| **F-26** | Queue, visibility/escalation, unknown independence and re-evaluation retained; unfamiliar-stack comprehension gap **S-001**. |
| F-27–30 | External isolated migration workflow, role/pinning controls, extension-owned calibrated effort and independent behavioral parity. |
| F-31–34 | Safe rebuild/publication, explicit stale partial parsing, deterministic context/caching hypothesis and reproducible full-cost evaluation. |

**Result:** all 34 are represented; 32 have adequate substantive preservation in this slice, while F-18 and F-26 need the corrections above.

All **17 themes** are represented:

- T01–02: full target, native core and ownership.
- **T03: partial through S-002**; interaction, offline-report and licensing requirements otherwise retained.
- T04–07: competitor evidence limits, storage/identity, five analysis levels and six test-evidence levels.
- T08–12: task-specific reading, legacy, factual documentation, federation and protected CI.
- **T13: partial through S-001**; 2D/3D/Web/Tauri/offline/triage integrity otherwise retained.
- T14–17: reuse boundaries, phased dependencies, three proof scenarios and provenance/NOT_RUN limits.

## Boundaries and claims that pass this review

- **Mature target:** Goals, FR-02, FC-4, FR-38 and P2/P6 preserve substantive native analysis beyond imports. Rust/TS/JS/Python are depth anchors, not a permanent language ceiling. On-demand extension delivery survives.
- **Storage and format:** Canonical, local, externally owned and derived classes remain distinct. Identity, generation publication and recovery are requirements; SQLite/FTS/vector/ATTACH and specific containers remain alternatives. No custom-format mandate, destructive reset or universal rebuild-time claim.
- **Tests and parity:** Declaration, discovery, execution, coverage, assertion strength and scenario/parity claims stay separate. Trusted-wrapper/untrusted-report attacks, missing required checks and independent post-write recovery are retained.
- **Documentation and agents:** Shared query semantics, provenance, unknowns, SCC/budgets, preserved mandatory constraints, manual explanations and normative-document protection are explicit.
- **Ownership:** ADR-025’s read-only planning versus external execution distinction is preserved. Estimate/calibrate and Day/Week/Month remain extension-owned with migration RFC/deprecation requirements. Tracker WHO/WHEN, Web execution boundaries and desktop permissions are not silently absorbed into Core.
- **Program placement:** EPIC-009 remains unaccepted. FR-41 requires per-surface/FPV disposition rather than automatic adoption. EPIC-010 remains independent; RFC-022 and current main contracts are explicitly protected. Open placement/maintenance questions have named gates rather than invented precedence.
- **Evidence and market claims:** Reference A source, action and map versions are distinguished. Prior browser interaction proves interaction only. Vendor/Bun figures are attributed, not replicated. Adoption, superiority and performance remain hypotheses; licensing and redistribution conditions remain explicit.
- **Architecture sequencing:** The standalone Critical SOLUTION and later SPEC/RFC/ADR gates precede implementation. The PRD does not select SQLite, a new compiler or a fixed storage layout prematurely.

## Validation-warning disposition

I inspected the existing receipt and actual artifact/export; I did **not** rerun validation.

- **`todo` SHOULD:** the match is the required `todo!` anti-stub example in FR-24, not unfinished requirement text.
- **Body-links SHOULD:** the selective export contains all eight intended relationships. Six reported direct targets have canonical-key links. ADR-003 is contextual ancestry through ADR-026, whose active frontmatter explicitly `refines ADR-003`; the PRD does not claim a missing direct edge. This specific warning does not establish a relationship defect.
- **FR-format COULD:** 41 substantive table rows exist; the checkbox-oriented warning does not establish missing requirements.
- **Depth mismatch:** raw/export `critical` versus validator `Deep` remains a real tooling limitation. The exit-0 receipt is not an automatic Critical review PASS.

### Disposition

Correct S-001 and S-002 through the authorized author, freeze the revised artifact and review the affected requirements/crosswalk/acceptance delta. No source implementation or scope reduction is needed.

This report supports neither product readiness nor activation. No files or state were changed; no builds, tests, benchmarks, formatter/linter runs, author-reasoning reruns, tracker operations or commits occurred.