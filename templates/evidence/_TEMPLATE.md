---
id: EVID-{NNN}
title: "{title}"
status: draft
kind: evidence
created: YYYY-MM-DD
updated: YYYY-MM-DD
---

# EVID-{NNN}: {Evidence Title}

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | YYYY-MM-DD |
| Valid Until | YYYY-MM-DD |
| Target | ADR-<id> (решение которое подтверждаем/опровергаем) |

<!-- REQUIRED for R_eff scoring. Legal values documented in templates/evidence/README.md. -->

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3

<!-- Git provenance — ONLY for code-claiming evidence (PRD-082 / #360).
     All three fields or none: a partial claim is rejected as `Incomplete`.
     `forgeplan activate` re-derives the claim against the real git delta
     instead of trusting the executor's self-report; green tests over an
     EMPTY delta are a null result, not a pass. Gate mode lives in
     .forgeplan/config.yaml → integrity.evidence_provenance_gate
     (block | warn | off, default warn). `--force` bypasses it.
base_sha: <sha before the change>
result_sha: <sha after the change>
changed_paths: crates/a/src/lib.rs, crates/a/tests/b.rs
-->

## Measurement

{Что измерено, как измерено, в каких условиях}

## Result

{Конкретный результат с числами}

## Interpretation

{Что результат означает для целевого решения}

## Congruence Level Justification

<!-- Legend: CL3 same-context (penalty 0.0); CL2 related (0.1); CL1 external (0.4); CL0 opposed (0.9). -->

{Обоснование выбранного Congruence Level}

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| ADR-<id> | informs |
