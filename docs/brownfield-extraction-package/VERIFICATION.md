# Verification Criteria

> How to know the extension is done and working.

## Top-level success

The extension is complete when, on a fresh brownfield project:

1. `/extract-business-logic` runs end-to-end without manual intervention (except Domain Owner interviews).
2. Output contains all 10 goal artifacts from `01-PROBLEM-STATEMENT.md`.
3. `extract_score ≥ 0.85` on a reference brownfield (TripSales).
4. "Code deletion test" passes — docs remain readable without original code.
5. RAG bundle imports cleanly into a target vector store.
6. Output can serve as a rewrite spec in a different language/stack.

## Per-wave verification

### Wave 1 ✔

| Check | How to verify |
|---|---|
| 6 new kinds registered | `forgeplan new glossary test` succeeds; same for all 6 kinds |
| Validation rules enforced | Creating a `glossary` without `definition` field → validation fails |
| Templates in place | `ls <forgeplan-templates>/` shows 6 new `.template.md` files |
| New relations work | `forgeplan link A B --relation causes` returns success |
| Confidence scoring parses | `<!-- confidence: inferred -->` wrapping renders confidence badge in projection |
| MCP tools callable | All 10 new `mcp__forgeplan__forgeplan_*` tools from `integration/forgeplan-mcp-additions.md` available in agent tools |
| CLI commands work | `forgeplan hypothesis promote HYP-001 --state parked` updates the artifact |
| C1 skill produces output | Run on 5-service fixture → glossary artifacts created |
| C4 skill produces output | Run on a file with `if/throw` → invariants detected |

### Wave 2 ✔

| Check | How to verify |
|---|---|
| C2 finds entry points | GraphQL mutations + REST routes + queue producers all catalogued |
| C2 maps user journeys | Each entry point has a `use-case` artifact linked |
| C5 detects causality | Graph contains `causes`, `emits`, `listens_to` edges |
| C5 detects cycles | Running on a loop-containing codebase finds and flags loops |

### Wave 3 ✔

| Check | How to verify |
|---|---|
| C3 generates hypotheses | Each code pattern has ≥ 3 alternatives |
| C3 alternatives are diverse | Automated similarity check — semantic distance between alternatives ≥ 0.3 |
| C6 triangulates sources | Per hypothesis, at least 2 signal types queried (git + one other) |
| C6 state transitions | `drafted → inferred` happens; `inferred → verified/refuted/parked` happens |
| Confidence distribution healthy | Not > 30% verified without DO input; not > 50% speculation |

### Wave 4 ✔

| Check | How to verify |
|---|---|
| C7 clusters questions | Interview packet groups questions by domain + entity |
| C7 provides context | Each question has 2-4 sentences of supporting context |
| C7 answer ingestion works | `forgeplan interview ingest <packet>.md` updates hypotheses |
| C8 valid Gherkin | Output parses with a standard Gherkin parser (e.g., `@cucumber/gherkin`) |
| C8 coverage | ≥ 80% of verified use-cases have scenarios |
| C9 contradiction detection | Fixture with known contradictions → all flagged |
| C9 graph navigable | Tier-0 overview → click into tier-1 domains → tier-2 entities |

### Wave 5 ✔

| Check | How to verify |
|---|---|
| C10 standalone output | Grep `file:` or `:line` in final docs → zero matches in "Canonical" sections |
| C10 complete DDL | Produced DDL executes in a fresh PostgreSQL — no errors |
| C10 complete pseudo-code | Pseudo-code preserves invariants; mechanical check possible |
| C11 finds real discrepancies | Inject a known drift (model change) → validator flags it |
| C11 routes to fix | Discrepancies create problem artifacts or re-trigger C3 |
| C12 RAG bundle valid | Chunks all have IDs, metadata, content; JSON schema validates |
| C12 importable | At least one target vector store can ingest the bundle |
| Orchestrator runs end-to-end | `/extract-business-logic` on TripSales → all phases complete |
| `extract_score ≥ 0.85` | Scored according to formula in `05-AUTORESEARCH-INTEGRATION.md` |

## Code deletion test (the acid test)

Procedure:
1. Run full extraction on a brownfield project.
2. **Delete the codebase** (simulate: `mv src backup-src`).
3. A fresh agent, given only the extraction output, must answer:
   - What are the main business entities?
   - What user journeys exist for each?
   - What are the key invariants?
   - How does feature X (a named use-case) work, step-by-step?
   - What are the domain events, and what triggers them?
   - What's in the glossary for term Y?
4. Restore codebase. Compare agent's answers to reality.

**Pass criteria**: 80%+ of answers are substantially correct. Missing information is acknowledged as "not in extracted docs" rather than hallucinated.

## Rewrite test

Procedure:
1. Pick a specific user journey (e.g., "Order confirmation").
2. Give only the extracted docs to an engineer unfamiliar with the codebase.
3. Ask them to write a rewrite in a target language (e.g., Go).
4. Compare business behavior of rewrite vs original via test scenarios.

**Pass criteria**: rewrite passes ≥ 80% of the original scenario catalog (from C8).

## RAG query test

Procedure:
1. Import RAG bundle into a vector store.
2. Ask 20 business questions (varying specificity).
3. Evaluate retrieval quality (precision@5) and answer correctness.

**Pass criteria**: precision@5 ≥ 0.7, answer correctness ≥ 0.8.

## Regression tests

Each skill has a fixture:
- Wave 1: `fixtures/wave1/` — 5-service fixture with known glossary/invariants.
- Wave 2: `fixtures/wave2/` — entry-point + cycle fixture.
- Wave 3: `fixtures/wave3/` — ambiguous code patterns for hypothesis generation.
- Wave 4: `fixtures/wave4/` — contradiction fixture.
- Wave 5: `fixtures/wave5/` — full small brownfield.

CI runs all fixtures on every PR.

## Monitoring (post-deployment)

When the extension is in use on real brownfields:
- Track `extract_score` distribution.
- Track hypothesis verification rate (how many Domain Owner interviews conducted).
- Track canonical reproducibility pass rate.
- Track RAG query success rate (if telemetry available).

## Definition of "done"

The extension is considered **done** when:

1. All Wave 1-5 tasks in `TASKS.md` are checked off.
2. All fixture tests pass.
3. Code deletion test passes on TripSales.
4. Documentation is updated.
5. A real external user (not the forgeplan team) successfully runs `/extract-business-logic` on their own brownfield and produces useful output.

## Escalation criteria

If any of the following happen, escalate to the user/maintainer:

- A wave's exit criteria cannot be met within 2 sessions.
- LLM quality is insufficient for C3 / C6 on real codebases.
- Forgeplan's architecture requires breaking changes.
- A critical dependency (autoresearch API) becomes unstable.

## Rollback plan

If the extension causes regressions in forgeplan:
1. Feature-flag all new kinds (`forgeplan --enable-brownfield-extension`).
2. Ensure existing workspaces are unaffected when flag is off.
3. Document how to disable and revert.

## Next document

→ `GLOSSARY.md` (meta-terms used in this package)
