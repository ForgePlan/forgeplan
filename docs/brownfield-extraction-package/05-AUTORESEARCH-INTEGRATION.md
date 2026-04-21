# Autoresearch Integration

> How the new skills wire into the existing `/autoresearch:*` command family without duplication.

## Current autoresearch capabilities (baseline)

From `/Users/explosovebit/Work/TripSales - Ex Multiroad/sources/autoresearch/`:

| Command | What it does |
|---|---|
| `/autoresearch` | Generic autonomous loop: modify → verify → keep/discard |
| `/autoresearch:plan` | Interactive goal/metric/scope wizard |
| `/autoresearch:learn` | Codebase doc generation with validate-fix loop (INIT/UPDATE/CHECK/SUMMARIZE modes) |
| `/autoresearch:predict` | Multi-persona pre-analysis with adversarial debate |
| `/autoresearch:reason` | Adversarial refinement via isolated generate→critique→synthesize→judge |
| `/autoresearch:debug` | Autonomous bug-hunting with scientific method |
| `/autoresearch:fix` | Iterative repair loop |
| `/autoresearch:security` | STRIDE + OWASP + red-team audit |
| `/autoresearch:ship` | Universal shipping workflow |
| `/autoresearch:scenario` | Use-case and edge-case generator |

## Mapping: new skills ↔ existing autoresearch

> Summary. Full I/O and prompt details in `integration/autoresearch-hooks.md`. When this table and that file disagree, **`integration/autoresearch-hooks.md` is the source of truth**.

| Skill (this package) | Best-fit existing command | Why |
|---|---|---|
| C1 Ubiquitous Language | `/autoresearch:learn --mode=glossary` | Learn already scouts — glossary mode reuses coverage + validate-fix |
| C2 Use-Case Mining | `/autoresearch:learn --mode=use-case` | Learn coverage + anti-herd for entry-point journey diversity |
| C3 Intent Inference | `/autoresearch:reason --mode=intent` | Perfect fit — generate/critique/synthesize = ADI |
| C4 Invariant Detection | `/autoresearch:learn --mode=invariant` | Pattern-based extraction, coverage metric |
| C5 Causal Linking | **New** command `/extract-business-logic:causal` | Multi-file trace + cycle detection; reuses KG pattern only |
| C6 Hypothesis Triangulation | `/autoresearch:predict --mode=triangulate` | Domain-specific personas: git-historian, naming-linguist, contrarian |
| C7 Interview Packaging | **New** command `/interview:draft` | Domain Owner workflow has no autoresearch equivalent |
| C8 Scenario Writing | `/autoresearch:scenario --template=gherkin` | Direct extension of existing command |
| C9 KG Curation | **New** continuous process | Graph reasoning, no autoresearch equivalent |
| C10 Canonical Reproducer | `/autoresearch:learn --mode=canonical` | Doc generation, reuses learn validate/fix loop (C11 is validator) |
| C11 Reproducibility Validator | **New** validator plug-in for C10 | Plugs into learn's validate/fix loop |
| C12 RAG Packager | `/autoresearch:ship --target=rag` | Shipping / export step |

## Strategy: extend, don't replace

Keep autoresearch's proven loop pattern, extend its modes.

### Pattern: New subcommand under existing command

Example — `C1 Ubiquitous Language`:

```
/autoresearch:learn --mode glossary
```

The `glossary` mode:
- Uses existing scout, but tailored to extract noun phrases, domain-specific terms, and code symbols.
- Uses existing validate-fix loop, but the validation is glossary-specific (definition length, uniqueness, aliases).
- Writes to `glossary/` subdirectory (or directly to forgeplan workspace).

### Pattern: New top-level command wrapping several

Example — `/extract-business-logic`:

This meta-command orchestrates many skills. It invokes other autoresearch commands as steps:
1. `/autoresearch:learn --mode glossary` → C1
2. `/autoresearch:learn --mode use-case` → C2
3. `/autoresearch:learn --mode invariant` → C4
4. `/autoresearch:predict --persona causality-analyst` → C5
5. `/autoresearch:reason --mode intent` → C3
6. (new) `hypothesis-triangulator` → C6
7. (new) `interview-packager` → C7
8. `/autoresearch:scenario --template gherkin` → C8
9. (new) `kg-curator` → C9
10. `/autoresearch:learn --mode canonical` → C10
11. (new) `reproducibility-validator` → C11
12. (new) `rag-packager` → C12

## New modes for `/autoresearch:learn`

Add four new modes to the existing four (init/update/check/summarize):

| New mode | Purpose |
|---|---|
| `--mode glossary` | Extract terms, definitions, aliases. Writes glossary artifacts. |
| `--mode use-case` | Map entry points to user journeys. Writes use-case artifacts. |
| `--mode invariant` | Extract business rules from guards. Writes invariant artifacts. |
| `--mode canonical` | Render standalone docs per domain. Produces DDL/SDL/pseudo-code. |

Each mode has its own validation rules and metric:
- `glossary`: `covered_terms / total_candidate_terms` (candidate = noun phrase frequency ≥ N).
- `use-case`: `mapped_entry_points / total_entry_points`.
- `invariant`: `detected_guards / total_guards_in_code`.
- `canonical`: `verified_domains / total_domains` (verified = reproducibility validator passes).

## New mode for `/autoresearch:reason`

| New mode | Purpose |
|---|---|
| `--mode intent` | For a code pattern, generate 3+ hypotheses via ADI cycle. Output: `hypothesis` artifacts. |

Uses existing adversarial refinement loop, but judges are domain-expertise personas:
- **Business Analyst** — "does this make sense as a business rule?"
- **Technical Historian** — "could this be legacy / migration artifact?"
- **Domain Lawyer** — "could this be regulatory compliance?"
- **Anti-Pattern Detective** — "could this just be a bug / accidental design?"
- **Devil's Advocate** — mandatory dissent.

## New mode for `/autoresearch:predict`

| New mode | Purpose |
|---|---|
| `--persona causality-analyst` | Multi-persona analysis of action → consequence chains. Output: causality edges. |
| `--persona reproducibility-judge` | Can the docs rebuild the system? Adversarial check. Output: discrepancy report. |

## New mode for `/autoresearch:scenario`

| New mode | Purpose |
|---|---|
| `--template gherkin` | Produces Given/When/Then feature files following project's scenario conventions. |

Already mostly exists — formalize output template.

## New top-level command: `/extract-business-logic`

Design details in `orchestration/extract-business-logic.md`. Brief summary:

```
/extract-business-logic
Scope: services/**/*.js
Domain: orders
Depth: deep | standard | quick
Iterations: N (per sub-step) or unlimited
Interview: yes | no | packaged-only
```

- Runs C1-C12 in the correct order.
- Handles failures with skill-specific retries.
- Generates final report with coverage, confidence distribution, and interview packets.

## Meta-command: `/interview <domain>`

Shortcut:
```
/interview orders
```

Runs C7 only — picks up all parked hypotheses for `orders` domain, clusters, generates markdown packet. User sends to Domain Owner, answers come back in file, uses `/interview apply <file>` to ingest.

## State machine integration

autoresearch tracks state per iteration via TSV files. The extraction workflow extends this:

- `extract/{YYMMDD}-{HHMM}-{slug}/`
  - `extract-results.tsv` — overall progress.
  - `glossary-results.tsv` — C1 iterations.
  - `use-case-results.tsv` — C2.
  - ... etc.
  - `hypothesis-register.json` — all hypotheses with states.
  - `interview-packets/` — generated packets.
  - `canonical/` — standalone docs per domain.
  - `rag-export/` — RAG bundle.
  - `validation-report.md` — C11 output.

## Composite metrics

```
extract_score =
    (glossary_coverage * 0.10) +
    (use_case_coverage * 0.15) +
    (invariant_coverage * 0.10) +
    (hypothesis_verified_pct * 0.20) +
    (scenarios_written * 0.15) +
    (kg_consistency * 0.10) +
    (canonical_reproducibility * 0.15) +
    (rag_export_quality * 0.05)
```

Each term ∈ [0, 1]. Target: `extract_score ≥ 0.85` for a brownfield to be considered "fully extracted".

## Handoff points

autoresearch's existing `--chain` flag (predict → debug → fix) inspires extraction chaining:

```
/extract-business-logic --chain security,scenario,ship
# Extract business logic, then audit security, then generate test scenarios, then ship docs
```

## Integration with `/autoresearch:ship`

The extraction output can be "shipped" to:
- A documentation site (markdown).
- A RAG vector store.
- A Confluence / Notion instance (via connector).
- A PR to the target rewrite repo (as initial docs).

This is handled by `/autoresearch:ship --type docs` with the extraction output as input.

## Changes to autoresearch repository

Ideally zero — new skills live in their own dir and reference autoresearch via slash commands. But if deep integration is desired:

- Add `--mode glossary/use-case/invariant/canonical/intent` flag parsing in respective `workflow.md` reference files.
- Add detection of "this is an extraction run" for metric reporting.

## Non-goals

- We do NOT rewrite autoresearch's core loop.
- We do NOT replace its validation-fix pattern.
- We do NOT change its existing commands' semantics.

## Next document

→ `06-SKILLS-INVENTORY.md`
