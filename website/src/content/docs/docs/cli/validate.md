---
title: forgeplan validate
description: "Check artifact completeness against depth-aware MUST/SHOULD rules before activation."
---

`forgeplan validate` runs the schema validator against one artifact (or all of them) and
reports every violation of the rules registered for that kind and depth. MUST rules are
blocking - they prevent `forgeplan activate` from flipping the lifecycle to `active`.
SHOULD rules are warnings that shape quality but do not block the gate. The validator is
depth-aware: a Tactical PRD has a thinner rule set than a Deep PRD, so you are never
penalised for choosing the right depth.

For recorded compatibility coverage and remaining limits, see
[Constellation](/docs/reference/constellation/). Do not recreate retired
artifact tables.

## When to use

- Immediately after creating an artifact from a template (catch empty MUST sections before you forget).
- Right before `forgeplan activate` - activation refuses to proceed on MUST failures anyway.
- In CI with `--ci` to validate active/stale artifacts; use an explicit ID for draft validation.
- During `/audit` to run `--adversarial` and surface devil's-advocate findings.

## When NOT to use

- Before an artifact exists - create and fill it first. Notes still have applicable rules.
- As a substitute for human review - the validator checks structure, not reasoning quality.

## Usage

```text
forgeplan validate [OPTIONS] [ID]
```

## Arguments

```text
  [ID]  Artifact ID (validates all if omitted)
```

## Options

```text
      --json         Output as JSON for machine consumption
      --adversarial  Run adversarial (devil's advocate) review
      --ci           CI mode: exit code 1 if any MUST rules fail
  -h, --help         Print help
  -V, --version      Print version
```

## Examples

### Validate a single PRD before activation

```bash
forgeplan validate PRD-001
```

Illustrative findings, not a captured runtime transcript:

```text
PRD-001 - Auth System
  MUST  Missing section: Problem
  MUST  FR list empty
  SHOULD density < 50 words in Goals
```

Fix the two MUSTs, rerun, then `forgeplan activate PRD-001`.

### Validate everything and fail CI on MUST errors

```bash
forgeplan validate --ci
```

Without an ID, CI mode checks active/stale artifacts and exits 1 on MUST errors.
An explicit ID selects that artifact, including a draft.

### Adversarial review for a Deep decision

```bash
forgeplan validate ADR-005 --adversarial
```

Adds local heuristic adversarial checks. It does not invoke an LLM or guarantee
a fixed number of real defects. Review findings without inventing issues.

### Machine-readable output

```bash
forgeplan validate PRD-001 --json | jq '.results[] | {artifact_id, errors, findings}'
```

## Output interpretation

- **MUST** - blocking. Activation will refuse. Fix before PR.
- **SHOULD** - warning. Track them, but they do not block the gate.
- **Rule aliases** apply (`## Motivation` == `## Problem`, `## Out of Scope` == `## Non-Goals`) - see the methodology guide for the full list.
- MUST errors produce exit code `1` in ordinary mode too; `--ci` also changes default selection to active/stale.

## CI mode (`--ci`)

Added in Sprint 11.3 as part of the methodology integrity work (PRD-034 / PRD-043),
`forgeplan validate --ci` turns the validator into a **hard pipeline gate**. In CI mode
the command exits 1 if **any MUST rule** fails on an `active` or `stale` artifact -
**drafts are intentionally excluded** so you are not blocked by work-in-progress stubs
that have not been activated yet.

### Behaviour

- `forgeplan validate --ci` → scans every artifact, filters to `status in {active, stale}`,
  runs the depth-aware MUST ruleset, and exits 1 if any artifact has at least one MUST
  violation.
- Without an ID, drafts are excluded entirely, not printed as warnings.
  Validate a draft explicitly with `forgeplan validate <ID>`.
- Output stays human-readable by default; pair with `--json` if you want to post-process
  in CI (e.g. annotate a PR with findings).

### GitHub Actions snippet

Drop this into `.github/workflows/ci.yml` next to your `forgeplan health --ci` step:

```yaml
- name: Forgeplan validate gate
  run: |
    forgeplan reindex
    forgeplan validate --ci
```

If this exits 1, the PR is blocked. Fix the MUST violations (or supersede / deprecate
the offending artifact if it is genuinely abandoned) before re-running.

Pair with [`health --ci`](/docs/cli/health/) for additional diagnostics.
These commands do not prove a PR is safe or universally prevent methodology debt;
their selection, thresholds and layout support matter.

## How it fits the workflow

```
route → new → validate → reason (ADI) → code → evidence → review → activate
                 ↑                                           ↑
                 └───────── re-run after each edit ──────────┘
```

`validate` is the "did I fill the form" gate. `review` is the "am I ready to ship" gate
on top of it. `activate` refuses to run unless `validate` passes.

## See also

- [`forgeplan review`](/docs/cli/review/) - adds lifecycle checklist on top of validation
- [`forgeplan activate`](/docs/cli/activate/) - the gate `validate` feeds
- [`forgeplan score`](/docs/cli/score/) - quality beyond structural rules
- [Quality Gates](/docs/methodology/evidence/) - MUST/SHOULD/adversarial philosophy
- [CLI overview](/docs/cli/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan validate --help`.

```text
Validate artifact completeness against schema rules

Usage: forgeplan validate [OPTIONS] [ID]

Arguments:
  [ID]  Artifact ID (validates all if omitted)

Options:
      --json         Output as JSON for machine consumption
      --adversarial  Run adversarial (devil's advocate) review
      --ci           CI mode: exit code 1 if any MUST rules fail
  -h, --help         Print help
  -V, --version      Print version
```
<!-- forgeplan-runtime:end -->
