# Phase transitions: rules and gates

> What must be true before moving from one phase to the next. Each gate is a quality check — premature progression produces garbage downstream.

## Why gates matter

Each phase consumes the previous phase's output. If the previous output is incomplete or low-quality:
- C3 (intent) generates hypotheses for junk patterns.
- C6 (triangulation) triangulates unfounded hypotheses.
- C10 (canonical) renders docs based on unverified claims.

Gates catch these breaks early.

## Phase 1 → Phase 2 gate

**Invariant**: factum artifacts are sufficient to feed hypothesis generation.

Checks:
1. `glossary` count ≥ 20 terms for the domain.
2. Each `use-case` has a traced entry point (no dangling references).
3. Each `invariant` has at least one `code_guard` link.
4. Causal graph has ≥ 50% coverage of declared actions.

If any check fails, do not proceed. Report which check failed and what's missing.

Override: `--skip-gate=phase1` (dangerous, for pilots only).

## Phase 2 → Phase 3 gate

**Invariant**: generated hypotheses are well-formed and diverse.

Checks:
1. Each hypothesis has ≥ 3 candidates.
2. No two candidates are substring-similar (> 80% overlap = rejected).
3. Each hypothesis is tied to at least one `use-case` or `invariant`.
4. No hypothesis is tied to multiple orthogonal subjects (should be one question per hypothesis).

Override: `--skip-gate=phase2`.

## Phase 3 → Phase 4 gate

**Invariant**: hypothesis confidence is high enough for downstream use.

Checks:
1. `verified + strong-inferred + inferred` rate ≥ 70%.
2. No more than 30% of hypotheses are `parked`.
3. No contradictory hypotheses in `verified` state (detected by C9).
4. If `--interview-mode=manual`: packet delivered and acknowledged by DO.

If check 4 fails, halt. User manually runs `/ingest-interview` when ready.

If check 1 fails but `--interview-mode=autopark`, proceed with explicit warning.

## Phase 4 → Phase 5 gate

**Invariant**: canonical docs are complete and syntactically valid.

Checks:
1. Every verified use-case has ≥ 1 scenario.
2. Every verified invariant appears in at least one scenario.
3. Canonical DDL/SDL/Gherkin files are non-empty.
4. KG has no orphan artifacts (C9 orphan count = 0 for this domain).

## Phase 5 exit gate

**Invariant**: the domain passes reproducibility validation.

Checks:
1. `ddl_compile_pass` = true.
2. `sdl_parse_pass` = true.
3. `gherkin_parse_pass` = true.
4. `pseudo_code_coherence` ≥ 95%.
5. `deletion_simulation_reproduction_rate` ≥ 80% OR domain flagged `partial_reproducible` with reasons.

If any fail, route back to C10 for fix-up; retry validation once; then either pass or produce final `validation-report.md` with known gaps.

## Drift gates (refresh mode)

In `refresh` mode, a gate also guards what to re-run:

- `glossary` → C1.
- `use-case` → C2 → then re-trigger dependent hypothesis (C3) → C6.
- `invariant` → C4 → re-run scenarios verifying it (C8).
- `domain-model` DDL change → C10 → C11.
- `scenario` → re-run C11 validation.

## Interview ingestion gate

After `/ingest-interview`:

Checks:
1. Answered markdown parses cleanly.
2. Every answered question references a known hypothesis.
3. Hypothesis lifecycle transition is valid (`parked` → `verified` / `refuted` / still `parked` with follow-up note).
4. No ambiguous answers left unflagged.

## Gate enforcement

All gates are enforced by the orchestrator. Users can:
- View gate status: `/extract-business-logic:status <domain>`.
- See which check blocks progression.
- Override with `--skip-gate=<phaseN>` (logged, visible in session history).

## Partial success

If Phase 5 fails but Phase 4 succeeded, the workspace is still useful:
- Canonical docs exist (with gaps).
- Scenarios exist (testable).
- KG is populated.

The system should emit a `partial_success` artifact noting what's missing.

## Version history

- v1.0.0 — initial design.
