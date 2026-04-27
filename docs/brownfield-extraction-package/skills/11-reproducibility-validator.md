# Skill: reproducibility-validator (C11)

> Verifies that canonical documentation is actually sufficient to reproduce the described system. The "code deletion test" in automated form.

## Why this skill exists

Documentation is only useful if someone can build the system from it. C10 produces standalone docs, but without validation we don't know if they're complete. C11 runs synthetic rebuild checks: given only the docs, can we reconstruct a working schema, a runnable pseudo-code, a valid GraphQL SDL?

Without C11, "standalone documentation" is an aspiration, not a guarantee.

## Input

- Canonical outputs from C10: `canonical/{domain}/{ddl.sql, api.graphql, pseudo-code/, scenarios.feature, invariants.md}`.
- Original code (only for comparison — validator never uses code to fill gaps in the docs).

## Output

- `validation-report.md` per domain with pass/fail per check.
- For failures: a pointer to the missing piece in the canonical docs (not the code).
- `problem` artifacts with `kind: reproducibility_gap` when validation fails.

## Modes

### Mode 1: `ddl-compile`
Take `canonical/{domain}/ddl.sql`, run `psql --check-syntax` (or equivalent). If it fails, the DDL is not standalone.

### Mode 2: `sdl-parse`
Take `canonical/{domain}/api.graphql`, parse with `graphql-js` / `graphql-ruby`. If it fails, the SDL is not standalone.

### Mode 3: `gherkin-parse`
Take `canonical/{domain}/scenarios.feature`, parse with `@cucumber/gherkin`. If syntax invalid, fail.

### Mode 4: `pseudo-code-coherence`
For each action's pseudo-code, check:
- All referenced entities appear in DDL.
- All referenced invariants appear in `invariants.md`.
- All side-effects align with declared domain events.
- All terms used match `glossary.md`.

### Mode 5: `deletion-simulation`
Hypothetical test — present docs to an independent agent with instructions: "Produce a schema and service stubs from these docs alone. Do not touch the original code." Then compare structure with the original. Delta = reproducibility gap.

### Mode 6: `rewrite-simulation`
Similar to deletion-simulation but target a different stack (e.g., "rewrite from Node.js docs into Go"). Tests that docs capture intent, not just syntax.

## Algorithm

### ddl-compile

```
for each domain:
  ddl = read(canonical/{domain}/ddl.sql)
  result = run('psql --file=- --set ON_ERROR_STOP=1', input=ddl)
  if result.exit_code != 0:
    report_failure(domain, 'ddl-compile', result.stderr)
```

### pseudo-code-coherence

```
for each action in canonical/{domain}/pseudo-code/:
  text = read(action)
  referenced_entities = extract_entities(text)
  referenced_invariants = extract_invariants(text)
  referenced_terms = extract_terms(text)

  for entity in referenced_entities:
    if entity not in ddl_tables:
      report_gap(action, f'references entity {entity} not in DDL')

  for inv in referenced_invariants:
    if inv not in invariants_md:
      report_gap(action, f'references invariant {inv} not declared')

  for term in referenced_terms:
    if term not in glossary_md:
      report_gap(action, f'uses term {term} not in glossary')
```

### deletion-simulation

Delegated to a sub-agent with minimal prompt:

```
You are rebuilding a system from scratch.
You have ONLY these documents: {canonical_docs}.
Do not read any original source code.
Produce:
- A directory of model files (schema).
- Pseudo-code for each action.
- GraphQL SDL if applicable.

Report any place where the documents are insufficient.
```

Diff the output with the real codebase structure to quantify gaps.

## Metric

- `ddl_compile_pass_rate`: target 100%.
- `sdl_parse_pass_rate`: target 100%.
- `gherkin_parse_pass_rate`: target 100%.
- `pseudo_code_coherence_rate`: target ≥ 95%.
- `deletion_simulation_reproduction_rate`: % of entities/actions reproducible from docs alone; target ≥ 80%.
- `rewrite_simulation_reproduction_rate`: same, across stacks; target ≥ 70%.

## Dependencies

- C10 (canonical docs).
- `psql` (or `sqlite3`, `mysql --syntax-check`) for DDL.
- `graphql-js` for SDL.
- `@cucumber/gherkin` for Gherkin.
- LLM for simulation modes.

## Integration with autoresearch

`/autoresearch:learn --mode validate-canonical`:
- Runs this skill as a post-C10 guard.
- Any failure → re-queue the failing artifact to C10 for fix-up.

This is the **validate step** in autoresearch's validate/fix loop applied to canonical docs.

## Prompt template (for simulation modes)

See `references/deletion-simulation-prompt.md`, `references/rewrite-simulation-prompt.md`.

## Failure modes

| Failure | Detection | Mitigation |
|---|---|---|
| DDL uses non-portable syntax | `psql --check` fails on another engine | Restrict to ANSI SQL in C10 |
| Pseudo-code references undocumented helper | Coherence check flags it | C10 must inline the helper or document it |
| Simulation agent "cheats" by reading code | Sandbox the simulation environment | Run in a temp directory with only docs |
| Diff too noisy to interpret | Structural diff, not line diff | Use semantic entity/action match |

## Example failure report

```markdown
# Reproducibility Report: Orders

## DDL compile
PASS

## SDL parse
PASS

## Gherkin parse
PASS (12 scenarios valid)

## Pseudo-code coherence
- FAIL: `confirm.md` references invariant INV-011, not declared in invariants.md.
- FAIL: `cancel.md` uses term "shoulder cascade", not in glossary.

## Deletion simulation
- Reproduced 18/22 tables (82%).
- Missing: `cargo_available_at` defaulting rule not extractable from docs.
- Missing: `decide.complete` fallback behavior opaque.

Recommendation: route failed items to C10 for re-extraction, and to C7 if Domain Owner needed.
```

## Testing

Fixture: known-good domain → expect 100% pass. Known-bad domain (with injected omission) → expect exactly the omission to be flagged.

## Version history

- v1.0.0 — initial design.
