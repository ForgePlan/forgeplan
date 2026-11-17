---
title: 10 Rules for Structured Decisions
description: Practical rules from the Forgeplan methodology
---

## 1. Route before you build
`forgeplan route "your task"` suggests depth and an artifact pipeline. Level 0 uses keywords; `--level 1` requires a configured LLM. This does not select or install marketplace agents.

## 2. Every requirement: "[Actor] can [capability]"
No tech names in PRDs. Describe WHAT, not HOW. "User can log in" not "React component renders login form".

## 3. Pipeline = guideline, not bureaucracy
Tactical can omit artifacts, not verification. Standard commonly uses PRD → RFC; add other kinds when the task needs them, not just to complete a list.

## 4. Child references parent
PRD → Epic, RFC → PRD, ADR → RFC. Always traceable upward.

## 5. Supersede, don't delete
Use `forgeplan supersede OLD-ID --by NEW-ID`, not a frontmatter edit or `update --status superseded`. Use `deprecate --reason` when there is no replacement. History and rationale stay explicit.

## 6. R_eff = min(evidence)
Trust is the weakest link, not an average. CLI `score` recursively takes the minimum of own current evidence and eligible dependency scores after relation penalties. Evidence scores include verdict, CL, and type deductions; expired eligible packs instead score 0.1 before deductions. Terminal `superseded`/`deprecated` packs are excluded, draft packs count, and no eligible own evidence gives an own score of 0.

## 7. Evidence expires
An expired eligible EvidencePack scores 0.1; this is not a fixed score for every expired decision. Inspect `forgeplan stale`, `forgeplan decay`, and `forgeplan score ID`, then recheck the claim before extending validity.

## 8. Shape before you code
Create the artifact, replace template content, validate, and independently review the handoff before coding. Use `forgeplan update ID --body @body.md` so markdown and the derived index stay aligned. No stub PRDs.

## 9. Test every pub fn
Tests and reviews must check the requirement, not merely agree with generated code. In the public BMAD/SPARC/TDD pipelines, the generator and verifier use separate contexts. TDD validates and freezes RED tests before GREEN; code authors cannot silently rewrite that oracle.

## 10. Work isn't done until activated
Fill and validate the artifact, record real evidence with `verdict`, `congruence_level`, and `evidence_type`, link it, inspect R_eff, review, then explicitly activate. Planning-review evidence can freeze a plan before code; implementation evidence must follow actual implementation. Activation is neither a test run nor proof that the feature shipped.
