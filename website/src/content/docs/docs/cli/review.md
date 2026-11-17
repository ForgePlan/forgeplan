---
title: forgeplan review
description: "Report validation and activation-methodology findings as PASSED, FAILED, or BLOCKED. Diagnostic only; does not activate the artifact."
---

`forgeplan review` reports validation and activation-methodology findings for
the stored artifact. It does not activate anything or run an LLM.

- **Review PASSED** — no MUST findings and all methodology gates pass.
- **Review FAILED** — one or more MUST findings.
- **Review BLOCKED** — no MUST findings, but a methodology gate fails.

SHOULD findings and build-on-draft warnings may coexist with PASSED.
A passed review is not a guarantee that `activate` will accept the current
status or a kind-specific transition. Run `activate` separately.

## When to use

- Always run before `forgeplan activate` - it is the canonical pre-flight check.
- End of sprint when batching activations - loop over every candidate with `review` first.
- When returning to a draft after a break, to see what still needs attention.

## When NOT to use

- On Notes / Problems - they don't need a validation gate to activate.
- For CI enforcement — `review` returns exit `0` even when FAILED or BLOCKED; use `validate --ci` for validation exit semantics.

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

Illustrative success excerpt (additional SHOULD findings or hints may follow):

```text
  Review PASSED
  Ready to activate
```

Illustrative methodology-blocked excerpt:

```text
  Review BLOCKED
  Methodology gates not met (see warnings below)

  ! No evidence linked — create evidence and link it before activating
```

MUST failures instead print `Review FAILED`, `Fix MUST validation errors first`,
and `x [MUST] <rule-id>: <message>` lines. No parent/F-G-R/route comparison
table is emitted by this CLI command.

### Reviewing before batch activate

```bash
for id in PRD-001 PRD-002 PRD-003; do
  forgeplan review "$id"
done
```

Quick triage of a sprint's worth of deliverables.

## Output interpretation

| Result | Meaning | Next action |
|---|---|---|
| PASSED | No MUST findings and gates pass | Inspect any warnings, then activate |
| FAILED | MUST findings present | Fix reported rules |
| BLOCKED | Body/evidence/stub gate failed | Fix the warning's underlying cause |

For PRD/Epic/Spec/RFC/ADR, shared gates require a trimmed body of at least
100 bytes, a qualifying evidence relation, and no detected template stub.
Other kinds skip these methodology gates, though review still runs validation.
An outgoing relation to a draft adds a build-on-draft warning. Review does
not require a positive R_eff score, recalibrate depth, or display F-G-R.
It reads the store; after direct file edits, synchronize before reviewing.

## How it fits the workflow

```text
code → evidence → link → synchronize stored content → review → activate
```

Missing artifacts and I/O/config failures are errors. A reported FAILED or
BLOCKED result is still a successful diagnostic command, not a non-zero CI gate.

## See also

- [`forgeplan validate`](/docs/cli/validate/) - rule-level gate `review` wraps
- [`forgeplan activate`](/docs/cli/activate/) - state transition this gate feeds
- [`forgeplan score`](/docs/cli/score/) - inspect R_eff separately from the review findings
- [`forgeplan fgr`](/docs/cli/fgr/) - inspect F-G-R separately; review does not display this triple
- [CLI overview](/docs/cli/)
