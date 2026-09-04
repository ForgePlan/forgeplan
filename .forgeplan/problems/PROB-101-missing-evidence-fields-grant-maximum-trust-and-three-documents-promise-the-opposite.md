---
depth: tactical
id: PROB-101
kind: problem
links:
- target: PRD-086
  relation: informs
status: draft
title: Missing evidence fields grant maximum trust, and three documents promise the opposite
---

---
assigned_number: 101
predicted_number: 101
slug: prob-missing-evidence-fields-grant-maximum-trust-and-three-documents-promise
---

# PROB-101: the evidence gate fails open, and everything says it fails closed

(touched to force a re-embed)

## Signal

An EvidencePack containing no structured fields at all — pure prose, no
`verdict`, no `congruence_level`, no `evidence_type` — gives the artifact it
informs a perfect score.

Measured on 0.36.0, fresh workspace:

```
Evidence breakdown:
  EVID-001 [Supports] CL3 = 1.0
R_eff:        1.00 -- Adequate
```

Three documents promise the opposite, in the same words:

- `CLAUDE.md` RED LINE #7 — "без этих structured fields parser тихо ставит CL0
  (silent failure → R_eff = 0.1)"
- the `/forge` skill, shipped to every user by `setup-skill` — "Without them,
  the R_eff parser silently defaults to CL0 (penalty 0.9), making R_eff = 0.1"
- `docs/methodology/EVIDENCE-PROTOCOL.md`, same claim

The code does the reverse, deliberately:
`crates/forgeplan-core/src/scoring/evidence.rs` — `(None, None) => 3`, with the
comment *"Default CL=3 (same context) — evidence created locally is
same-context by default."*

## Why this is the dangerous direction

Absent metadata resolves to **maximum** trust. Every other unknown in this
codebase fails closed: unparseable `congruence_level` → CL0 with a warning; an
unclosed HTML comment that swallows the fields → CL0 with a warning. Both were
fixed as trust-inflation bugs (PROB-034 and its follow-ups). The plain-absence
case was left as the one path where saying nothing is rewarded.

It also inverts the incentive. An agent that writes the fields honestly:

```
verdict: supports
congruence_level: 2     → score 0.9
```

An agent that writes nothing:

```
(no fields)             → score 1.0
```

Skipping the discipline scores **better** than following it. The red line that
exists to prevent silent failure is enforced by nothing, and the documentation
that would warn an author is wrong in the direction that hides the problem.

## Scale

This repository: **3 of 166** packs lack `congruence_level`. The discipline is
followed here, which is exactly why the missing guard went unnoticed — the
convention holds, so nothing ever tested what happens when it does not.

That number says nothing about other workspaces. A workspace where an agent
skipped the fields has inflated scores and no signal that it did.

## Not the same as #325

#325 is a pack scoring 0 for itself. This is a pack scoring 1.0 for someone
else on no information. Opposite direction, same root: the scorer has no
concept of "this pack did not tell me anything."

## The decision this needs

Changing `(None, None)` from CL3 to CL0 aligns behaviour with all three
documents and makes RED LINE #7 real. It is a **breaking scoring change**:
every pack without the fields drops from 1.0 to 0.1, and every artifact whose
weakest link was such a pack drops with it. Three artifacts here; unknown
elsewhere.

The alternative — keep CL3 and correct the three documents — is cheaper and
defensible on the code's own rationale (locally-authored evidence really is
same-context). But it means the product's headline number treats "I measured
this and it strongly supports the claim" and "I wrote some prose" as identical,
and no gate anywhere distinguishes them.

Recommendation: fail closed, with the absence reported as a factor rather than
a silent default, and a `forgeplan score --all` note in the release. The whole
point of R_eff is that a number can be traced to something. A default of
maximum trust on absent input is the one case where it cannot.

## Related

| Artifact | Relation |
|---|---|
| PRD-086 | informs |

