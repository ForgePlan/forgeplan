---
title: 10 Rules for Structured Decisions
description: Practical rules from the Forgeplan methodology
---

## 1. Route before you build
`forgeplan route "your task"` — determines depth and pipeline. Don't skip this.

## 2. Every requirement: "[Actor] can [capability]"
No tech names in PRDs. Describe WHAT, not HOW. "User can log in" not "React component renders login form".

## 3. Pipeline = guideline, not bureaucracy
Tactical = just code. Standard = PRD → RFC. Don't create all 10 artifact types for a bug fix.

## 4. Child references parent
PRD → Epic, RFC → PRD, ADR → RFC. Always traceable upward.

## 5. Supersede, don't delete
Old artifacts get `status: superseded`. History is preserved. Never `rm`.

## 6. R_eff = min(evidence)
Trust is the weakest link. Not average — minimum. One blind spot drags everything.

## 7. Evidence expires
`valid_until` TTL. Expired = 0.1 (stale, not absent). Re-test periodically.

## 8. Shape before you code
Create artifact → fill MUST sections → validate → THEN code. No stub PRDs.

## 9. Test every pub fn
Write test immediately after function. Don't move to next function without test.

## 10. Work isn't done until activated
PRD filled + validated + evidence created + R_eff > 0 + `forgeplan activate`.
