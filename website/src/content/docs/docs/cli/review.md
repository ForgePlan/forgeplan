---
title: forgeplan review
description: "Read-only validation and methodology-gate review: PASSED, FAILED or BLOCKED."
---

`forgeplan review` runs validation and shared methodology gates without changing
the artifact. The CLI renderer reports:

- **Review PASSED** — no MUST failures and the body-length, evidence-link and stub gates pass.
- **Review FAILED** — MUST findings exist.
- **Review BLOCKED** — methodology gates fail without MUST findings.

SHOULD findings and dependency-on-draft warnings are printed separately. This
is not a calculated R_eff/F-G-R/route dashboard. A passed review does not replace
checking the current status transition or the CLI activation provenance gate.

## When to use

- Always run before `forgeplan activate` - it is the canonical pre-flight check.
- End of sprint when batching activations - loop over every candidate with `review` first.
- When returning to a draft after a break, to see what still needs attention.

## When NOT to use

- On Notes / Problems - they don't need a validation gate to activate.
- For CI exit-code gating — `review` reports findings but does not return failure solely for a failed review; inspect the appropriate validation/health gate separately.

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

Illustrative clean output excerpt (not a captured run):

```text
Review PASSED
Ready to activate
```

With MUST failures the heading is `Review FAILED`, followed by findings. When
only methodology gates fail, it is `Review BLOCKED`, with
`Methodology gates not met (see warnings below)`.

The command also emits contextual hints. It does not print the fabricated
parent/evidence/R_eff/FGR/route checklist that a separate context report may summarize.

### Reviewing before batch activate

```bash
for id in PRD-001 PRD-002 PRD-003; do
  forgeplan review "$id"
done
```

Quick triage of a sprint's worth of deliverables.

## Output interpretation

| CLI heading | Meaning | Action |
|---|---|---|
| Review PASSED | MUST and methodology gates pass | Check transition and remaining activation requirements |
| Review FAILED | MUST findings | Fix the reported findings |
| Review BLOCKED | Body length, evidence-link or stub gate not met | Address the printed warnings |

The evidence-link gate checks linkage, not a positive computed R_eff.

## How it fits the workflow

```
code → new evidence → link → score → REVIEW
                                        │
                                        ├── PASSED → check remaining activation requirements
                                        ├── FAILED → fix MUSTs
                                        └── BLOCKED → fix methodology gates
```

Review is a read-only pre-activation report, not an automatic state change.
Lightweight kinds skip methodology gates, while structured decisions are checked
for sufficient body content, evidence linkage and remaining template stubs.

## See also

- [`forgeplan validate`](/docs/cli/validate/) - rule-level gate `review` wraps
- [`forgeplan activate`](/docs/cli/activate/) - state transition this gate feeds
- [`forgeplan score`](/docs/cli/score/) - R_eff input to the review checklist
- [`forgeplan fgr`](/docs/cli/fgr/) - F-G-R triple shown in the report
- [CLI overview](/docs/cli/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan review --help`.

```text
Review an artifact — run validation and show lifecycle checklist

Usage: forgeplan review <ID>

Arguments:
  <ID>  Artifact ID

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
