---
title: 10 Rules for Structured Decisions
description: Practical rules from the Forgeplan methodology
---

## 1. Route before you build
`forgeplan route "your task"` - determines depth and pipeline. Don't skip this.

## 2. Every requirement: "[Actor] can [capability]"
No tech names in PRDs. Describe WHAT, not HOW. "User can log in" not "React component renders login form".

## 3. Pipeline = guideline, not bureaucracy
Tactical = just code. Standard = PRD → RFC. Don't create all 10 artifact types for a bug fix.

## 4. Child references parent
PRD → Epic, RFC → PRD, ADR → RFC. Always traceable upward.

## 5. Supersede, don't delete
Use `forgeplan supersede OLD --by NEW` for a replacement, not a direct status edit. Preserve history; never `rm` the old record.

## 6. R_eff = min(evidence)
Trust uses a weakest-link minimum over current eligible evidence and dependency contributions, not an average or `min(F,G,R)`. See [the scoring rules](/docs/methodology/evidence/).

## 7. Evidence expires
Optional `valid_until` is an expiry date. An expired evidence item scores 0.1; detection does not rewrite its persisted status. Re-test before renewing.

## 8. Shape before you code
Create artifact → fill MUST sections → validate → THEN code. No stub PRDs.

## 9. Test every pub fn
Write test immediately after function. Don't move to next function without test.

## 10. Work isn't done until activated
PRD filled + validated + evidence created + R_eff > 0 + `forgeplan activate`.
