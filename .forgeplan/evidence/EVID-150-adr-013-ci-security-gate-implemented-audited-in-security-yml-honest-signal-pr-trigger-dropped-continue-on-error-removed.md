---
depth: tactical
id: EVID-150
kind: evidence
links:
- target: ADR-013
  relation: informs
status: draft
title: 'ADR-013 CI security gate implemented + audited in security.yml (honest-signal: PR trigger dropped, continue-on-error removed)'
---

## Summary

ADR-013's CI security-gate policy is implemented and independently audited in `.github/workflows/security.yml`: the `pull_request:` trigger is dropped (the scan runs on `push:[dev,main]` + a weekly cron + `workflow_dispatch`), and `continue-on-error: true` is removed so a real advisory turns the badge red. Honest CI signal over instant per-PR feedback — exactly what the ADR decides.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: audit

CL3: same context — the audited artifact IS the ADR's own `security.yml`.

## Evidence

- **`c525acd2`** — `ci(security): FR-025` drops the `pull_request:` trigger from `security.yml`, keeps push+cron (this commit also creates ADR-013). Refs PRD-077 FR-025.
- **`0214993f`** — `fix(ci)` removes `continue-on-error: true` and adds `workflow_dispatch:`; closes the v0.32 Wave-4 adversarial-audit findings S1 (CRITICAL) + F1 (HIGH), amending the ADR-013 FAQ.
- **Live `.github/workflows/security.yml`** — `on:` = `{push:[dev,main], schedule '23 7 * * 1', workflow_dispatch}`, NO `pull_request`, NO `continue-on-error`; a guard comment forbids re-adding `continue-on-error`, and ADR-013 is cited in the workflow header.
- **Corroborating packs** — EVID-127 (v0.32 Wave-4 audit panel: documents the S1 cosmetic-gate finding + fix) and EVID-145 (PROB-070 closure: records SEC-004 "CI gate theatre" closed via `c525acd` FR-025 / ADR-013 honest-signal).

## Method

No unit-test surface exists (workflow YAML), so this is `audit`-type evidence: a diff-read of the live `security.yml` `on:` block + absence of `continue-on-error`, cross-checked against the two implementing commits (`git show`), plus two independent adversarial-audit EvidencePacks. Records what was observed, not a fabricated test. Confirmed no artifact supersedes ADR-013.


