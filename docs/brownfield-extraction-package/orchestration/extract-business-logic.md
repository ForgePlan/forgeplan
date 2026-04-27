# Orchestration: `/extract-business-logic <domain>`

> The meta-command that drives the full 12-skill pipeline end-to-end for a given domain.

## Purpose

Running individual skills is possible, but the power of the methodology comes from running them in the right order with the right inputs and stopping at the right gates. This orchestration turns a pile of skills into a workflow.

## Entry point

```
/extract-business-logic <domain> [--mode=full|factum-only|intent-only|refresh]
                                  [--limit-use-cases=N]
                                  [--interview-mode=autopark|manual]
                                  [--output-rag=bool]
```

## Phases

```
Phase 1 — Factum extraction (no LLM reasoning)
  C1  ubiquitous-language     → glossary[]
  C2  use-case-miner          → use-case[]
  C4  invariant-detector      → invariant[]
  C5  causal-linker           → causal graph in KG

Phase 2 — Intent generation (LLM reasoning)
  C3  intent-inferrer         → hypothesis[]

Phase 3 — Triangulation + interview setup
  C6  hypothesis-triangulator → hypothesis[] with confidence
  C7  interview-packager      → interview-packet[] for parked ones

              GATE: Domain Owner answers if --interview-mode=manual
                      (autopark: proceed without and mark parked hypotheses)

Phase 4 — Synthesis
  C8  scenario-writer         → scenario[]
  C9  kg-curator              → graph updates + contradiction reports
  C10 canonical-reproducer    → canonical/{domain}/*.{sql,md,graphql,feature}

Phase 5 — Validation + output
  C11 reproducibility-validator → pass/fail per canonical artifact
  C12 rag-packager (optional)   → rag-package/
```

## Dependencies between phases

- Phase 2 depends on Phase 1 completing.
- Phase 3 depends on Phase 2 (needs hypotheses).
- Phase 4 depends on Phase 3 (needs triangulated hypotheses and may wait on DO).
- Phase 5 depends on Phase 4.

## Orchestration algorithm

```
function extract_business_logic(domain, options):
  # Phase 1
  emit_progress("Phase 1/5: factum extraction")
  run_skill(C1_ubiquitous_language, domain=domain)
  run_skill(C2_use_case_miner, domain=domain, limit=options.limit_use_cases)
  run_skill(C4_invariant_detector, domain=domain)
  run_skill(C5_causal_linker, domain=domain)

  if options.mode == "factum-only":
    emit_progress("Stopping at factum-only mode.")
    return

  # Phase 2
  emit_progress("Phase 2/5: intent generation")
  new_code_patterns = collect_uncommented_patterns_in(domain)
  for pattern in new_code_patterns:
    run_skill(C3_intent_inferrer, pattern=pattern)

  # Phase 3
  emit_progress("Phase 3/5: triangulation")
  drafted_hypotheses = forgeplan.query(kind=hypothesis, state=drafted, domain=domain)
  for h in drafted_hypotheses:
    run_skill(C6_hypothesis_triangulator, hypothesis=h)

  parked = forgeplan.query(kind=hypothesis, state=parked, domain=domain)
  if parked:
    packet = run_skill(C7_interview_packager, hypotheses=parked)
    if options.interview_mode == "manual":
      emit_progress(f"GATE: deliver {packet.id} to Domain Owner and run /ingest-interview when answered.")
      return
    # autopark mode: proceed; parked hypotheses remain with inferred confidence

  # Phase 4
  emit_progress("Phase 4/5: synthesis")
  use_cases = forgeplan.query(kind=use-case, confidence >= inferred, domain=domain)
  for uc in use_cases:
    run_skill(C8_scenario_writer, use_case=uc)
  run_skill(C9_kg_curator, mode="incremental")
  run_skill(C10_canonical_reproducer, mode="render-domain", domain=domain)

  # Phase 5
  emit_progress("Phase 5/5: validation")
  validation = run_skill(C11_reproducibility_validator, domain=domain)
  if validation.fail_count > 0:
    emit_progress(f"Validation failures: {validation.fail_count}. Re-routing to C10 fix-up.")
    for failure in validation.failures:
      run_skill(C10_canonical_reproducer, mode="fix", target=failure)
    validation = run_skill(C11_reproducibility_validator, domain=domain)

  if options.output_rag:
    run_skill(C12_rag_packager, domain=domain)

  emit_progress("Done.")
  return report(domain)
```

## Modes

### `--mode=full`
All phases 1-5.

### `--mode=factum-only`
Only phase 1 — useful for a first pass without LLM cost.

### `--mode=intent-only`
Phases 2-3. Requires phase 1 output to exist.

### `--mode=refresh`
Re-runs only artifacts flagged `needs_update` by forgeplan drift. Cheaper than full.

### `--interview-mode=autopark` vs `manual`
- `autopark`: parked hypotheses stay parked with current confidence; pipeline completes with known gaps.
- `manual`: pipeline stops after C7; waits for human ingestion.

## Progress reporting

Each phase emits structured log events:

```json
{ "timestamp": "...", "phase": "phase-2", "skill": "C3-intent-inferrer", "progress": "34/120 code patterns processed", "artifacts_created": 18 }
```

## Error handling

- Skill failure → retry up to 3x with exponential backoff.
- Validation failure in phase 5 → route failures back to C10; re-validate once. If still failing, produce a `validation-report.md` and continue.
- LLM rate limit → pause and resume.

## Checkpointing

After each phase, write `extraction-checkpoint.json` so re-runs can resume:

```json
{
  "domain": "orders",
  "phase_completed": 3,
  "artifacts_created": ["TERM-012", "UC-003", "INV-003", "HYP-042"],
  "parked_hypotheses": ["HYP-042", "HYP-051"],
  "next_step": "phase-4-synthesis",
  "timestamp": "..."
}
```

## Ingestion sub-command: `/ingest-interview <packet-id>`

When a Domain Owner returns an answered packet:

1. Parse answered markdown.
2. For each answered question, locate the referenced hypothesis.
3. Update hypothesis → `verified` or `refuted`.
4. Re-queue dependent use-cases / scenarios for phase 4 re-run.
5. Update the KG (C9) with the new verified facts.

## Integration with forgeplan session

When `/extract-business-logic` runs inside a forgeplan-enabled session:

- All emitted artifacts carry the session's `session_id`.
- The session is tagged with `domain` and `phase` so its activity feeds `forgeplan_activity_stats`.
- `forgeplan_health` gains visibility into parked hypotheses counts.

## Testing

Fixture: small domain (e.g. 3 use-cases, 5 invariants, 1 aggregate). Expected:
- Phase 1 completes in < 5 minutes.
- Phase 2 generates ≥ 3 hypotheses.
- Phase 3 triangulates at least 2 to inferred, parks 1.
- Phase 4 produces scenarios + canonical docs.
- Phase 5 validates DDL/SDL/Gherkin.

## Version history

- v1.0.0 — initial design.
