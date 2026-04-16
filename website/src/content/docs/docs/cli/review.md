---
title: forgeplan review
description: "Pre-activation gate — runs validation plus lifecycle checklist and reports READY / BLOCKED / NEEDS-ATTENTION."
---

`forgeplan review` is the human-facing gate you run **right before** `forgeplan activate`.
It bundles `validate` (MUST/SHOULD rules) with a lifecycle checklist (is there a parent?
is there evidence? is R_eff > 0? is the depth appropriate?) and returns one of three
verdicts:

- **READY** — validation passes and all lifecycle boxes are ticked. Safe to activate.
- **NEEDS-ATTENTION** — SHOULDs or non-blocking lifecycle items flagged, but you can proceed if you accept the risk.
- **BLOCKED** — MUST failures or missing critical links. Activate will refuse.

Where `validate` tells you *what rules fired*, `review` tells you *what to do next*.

## When to use

- Always run before `forgeplan activate` — it is the canonical pre-flight check.
- End of sprint when batching activations — loop over every candidate with `review` first.
- When returning to a draft after a break, to see what still needs attention.

## When NOT to use

- On Notes / Problems — they don't need a validation gate to activate.
- In CI pipelines — use `validate --ci` instead (machine-readable, exit codes).

## Usage

```text
forgeplan review <ID>
```

## Arguments

```text
  <ID>  Artifact ID
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Pre-activation check

```bash
forgeplan review PRD-001
```

Output when ready:

```text
PRD-001 — Auth System
  Validation:       PASS (0 MUST, 1 SHOULD)
  Parent link:      EPIC-002 ✓
  Evidence:         EVID-012, EVID-015 (2 linked)
  R_eff:            0.90
  FGR:              F=3 G=3 R=2 (overall 2.67)
  Depth:            Standard (matches route)
  Lifecycle:        draft → active

  Verdict: READY — run `forgeplan activate PRD-001`
```

Output when blocked:

```text
PRD-004 — Search Intelligence
  Validation:       FAIL (2 MUST errors)
    - Missing section: Non-Goals
    - FR list empty
  Parent link:      (none)
  Evidence:         (none — blind spot)
  R_eff:            0.00

  Verdict: BLOCKED — fix MUST errors, link parent, add evidence
```

### Reviewing before batch activate

```bash
for id in PRD-001 PRD-002 PRD-003; do
  forgeplan review "$id"
done
```

Quick triage of a sprint's worth of deliverables.

## Output interpretation

| Verdict          | Meaning                                           | Next action                    |
|------------------|---------------------------------------------------|--------------------------------|
| READY            | Validation clean, evidence present, R_eff > 0     | `forgeplan activate <ID>`      |
| NEEDS-ATTENTION  | SHOULDs or minor gaps — no MUST failures          | Fix what you can, then activate|
| BLOCKED          | MUST failures or R_eff = 0 on a decision          | Do not activate; fix first     |

The checklist also calls out common drift: missing parent link, stale evidence, depth
mismatch (route says Deep but artifact is Tactical).

## How it fits the workflow

```
code → new evidence → link → score → REVIEW → activate
                                        │
                                        ├── READY        → activate
                                        ├── ATTENTION    → fix SHOULDs (optional)
                                        └── BLOCKED      → fix MUSTs (required)
```

Think of `review` as the final readout the lifecycle engine shows you before flipping
the state. It is purely advisory when you have a clean setup, and a lifesaver when you
don't.

## See also

- [`forgeplan validate`](/docs/cli/validate/) — rule-level gate `review` wraps
- [`forgeplan activate`](/docs/cli/activate/) — state transition this gate feeds
- [`forgeplan score`](/docs/cli/score/) — R_eff input to the review checklist
- [`forgeplan fgr`](/docs/cli/fgr/) — F-G-R triple shown in the report
- [CLI overview](/docs/cli/)
