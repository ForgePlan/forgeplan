# Trust Section v2 — Design Concept

> Full-width layout with rings at center, 5 story cards around them.
> To be implemented in next sprint.

## Philosophy

**"Trust is not a feeling. It's a measurement."**

This section bridges "we structure decisions" (hero) to "here's how quality is enforced" (pipeline). It answers: **"Why should I trust the structure?"**

## Narrative position

```
Hero:     "Your decisions are chaos" → crystallization → "F●rge your plan"
Trust:    "OK structure exists. But HOW do you know it's reliable?"
Pipeline: "Here's the workflow: Shape → Validate → Reason → Build → Prove"
Install:  "Get started"
```

## 5 Rings = 5 Levels of "How Do You Know"

| Ring | Question | Answer | Real example |
|------|----------|--------|-------------|
| **Outer (dashed)** | "Is this still valid?" | Expired evidence → 0.1 | "2024 benchmark for a 2026 decision" |
| **Ring 1** | "Where did you get this?" | SO answer, blog post | CL0-CL1: far from context |
| **Ring 2** | "Did anyone verify?" | External docs, someone's PoC | CL1-CL2: not your project |
| **Ring 3** | "Is this from our project?" | Related module, similar use case | CL2-CL3: close |
| **Ring 4 (ember)** | "Did YOU test it?" | Benchmark in this project | CL3: maximum trust |
| **Center dot** | **R_eff = ?** | min(all scores) | One weak link = whole score |

## Card Content (storytelling, not definitions)

### Card 1 — "Still valid?" (decay ring)
> That benchmark from last quarter — it scored 0.9 when you ran it. But valid_until expired 2 months ago. Now it's 0.1. Not deleted — just stale.

### Card 2 — "Source matters" (ring 1)
> A Stack Overflow answer is not the same as your own load test. Same conclusion, different trust. CL0 vs CL3 = 0.9 penalty difference.

### Card 3 — "Tested where?" (ring 2)
> Your colleague's PoC in a different service ≠ your benchmark in your service. Related context helps, but doesn't prove.

### Card 4 — "Prove it here" (ring 3, ember)
> Evidence from the same project, same context, same conditions. No penalties. Full trust. This is CL3.

### Card 5 — "Weakest link wins" (center)
> 3 strong evidences + 1 weak = weak. R_eff = min(), never average. One blind spot drags everything down.

## Layout — Full Width

```
     ┌─────────────┐
     │ Still valid? │╌╌╌╌○ (decay ring)
     └─────────────┘
                          ╌╌╌╌○ (ring 1)  ┌───────────────┐
                                            │ Source matters │
                                            └───────────────┘
     ┌───────────────┐
     │ Tested where? │╌╌╌╌╌○ (ring 2)
     └───────────────┘
                          ╌╌╌╌○ (ring 3)   ┌───────────────┐
                                            │ Prove it here │
                                            └───────────────┘
     ┌─────────────────┐
     │ Weakest link     │╌╌╌╌╌● (center)
     │ wins             │
     └─────────────────┘

     Title: "Trust Is Measured, Not Assumed"
     Subtitle: "Every decision has a score. The score is your weakest evidence."

     + LIVE SCORING dashboard:
       PRD-018  ████████░░  0.82  3 evidences
       RFC-003  ████░░░░░░  0.41  1 evidence
       ADR-001  ░░░░░░░░░░  0.00  NO EVIDENCE  [BLIND SPOT]
```

## Technical approach

- Full-width section (no left/right split)
- Rings SVG centered
- Cards as HTML divs positioned absolutely
- Dashed SVG lines from card edges to ring vertex points
- Cards use forge design system: border, bg/90, ember accent
- Title + dashboard as overlay at bottom
- CSS sticky (250-300vh section height)
- Scroll-driven progress for card appear/disappear

## Methodology source

R_eff formula and CL penalties from:
- `docs/guides/QUALITY-GATES.md` — section 4
- `crates/forgeplan-core/src/scoring/reff.rs`

```
R_eff = min(evidence_scores)
evidence_score = max(0, verdict_score - CL_penalty)

Verdict: supports=1.0, weakens=0.5, refutes=0.0
CL3=0.0, CL2=0.1, CL1=0.4, CL0=0.9
Expired: score → 0.1 (stale, not absent)
```

---

*Created: 2026-04-05*
*Status: concept — to be implemented next sprint*
