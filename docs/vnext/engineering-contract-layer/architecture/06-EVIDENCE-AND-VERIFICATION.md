# EvidenceBundle and Verification

## Основной принцип

> Verify the artifact, not the claim.

Ответ агента «готово» не является доказательством.

## EvidenceBundle

Обязательные данные:

- contract ID/version/digest;
- execution ID;
- producer actor;
- verifier actor when applicable;
- base SHA and result SHA;
- changed paths and git delta digest;
- criterion-level results;
- commands with exit codes;
- test/build/lint/typecheck reports;
- benchmark/security/UI evidence as required;
- environment metadata;
- CI and PR references;
- unresolved limitations;
- Evidence artifact hashes;
- timestamps and validity.

## Verification Engine

Для каждого acceptance criterion:

1. найти required claim;
2. найти соответствующее Evidence;
3. проверить provenance;
4. проверить congruence и freshness;
5. учесть supports/weakens/refutes;
6. применить policy;
7. сформировать criterion verdict.

## Итоговые verdicts

```text
accepted
rejected
incomplete
blocked
requires_human_review
stale
```

## Ground-truth gates

- code-claiming Evidence с пустым relevant delta не проходит;
- changed paths вне allowed scope блокируют acceptance;
- изменённый base SHA требует revalidation;
- test claim без отчёта/CI receipt не проходит;
- refuting Evidence для required claim блокирует acceptance;
- Critical result не принимается исполнителем;
- Evidence с истёкшим validity не удовлетворяет active gate.

## Claim-centric R_eff

```text
claim_score = evaluate(supports, weakens, refutes, congruence, freshness, provenance)
R_eff = min(required_claim_scores)
```

Informational claims не обрушивают весь decision score. Dismissal Evidence является отдельным audited action.

## Связанные существующие проблемы

Реализация обязана учесть, а не дублировать:

- ForgePlan/forgeplan#360 — git-delta provenance;
- #325 — leaf Evidence scoring;
- #328 — core-side decay triggers;
- #329 — per-source Trust Calculus breakdown.
