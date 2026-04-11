---
title: forgeplan validate
description: "Check artifact completeness against depth-aware MUST/SHOULD rules before activation."
---

`forgeplan validate` runs the schema validator against one artifact (or all of them) and
reports every violation of the rules registered for that kind and depth. MUST rules are
blocking — they prevent `forgeplan activate` from flipping the lifecycle to `active`.
SHOULD rules are warnings that shape quality but do not block the gate. The validator is
depth-aware: a Tactical PRD has a thinner rule set than a Deep PRD, so you are never
penalised for choosing the right depth.

## When to use

- Immediately after creating an artifact from a template (catch empty MUST sections before you forget).
- Right before `forgeplan activate` — activation refuses to proceed on MUST failures anyway.
- In CI with `--ci` on the docs / artifacts branch to block merges that introduce stub PRDs.
- During `/audit` to run `--adversarial` and surface devil's-advocate findings.

## When NOT to use

- On a Note or freshly routed Tactical task — there is nothing to validate yet.
- As a substitute for human review — the validator checks structure, not reasoning quality.

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

Prints a table of MUST and SHOULD findings. Typical output:

```text
PRD-001 — Auth System
  MUST  Missing section: Problem
  MUST  FR list empty
  SHOULD density < 50 words in Goals
```

Fix the two MUSTs, rerun, then `forgeplan activate PRD-001`.

### Validate everything and fail CI on MUST errors

```bash
forgeplan validate --ci
```

Exit code 1 if any artifact has at least one MUST violation. Use this in a pre-merge
GitHub Action on branches that touch `.forgeplan/`.

### Adversarial review for a Deep decision

```bash
forgeplan validate ADR-005 --adversarial
```

Runs the devil's-advocate pass (BMAD-inspired). Reviewers MUST find problems — if the
adversarial pass reports zero issues, re-run with a stronger model or escalate to `/audit`.

### Machine-readable output

```bash
forgeplan validate PRD-001 --json | jq '.must_violations'
```

## Output interpretation

- **MUST** — blocking. Activation will refuse. Fix before PR.
- **SHOULD** — warning. Track them, but they do not block the gate.
- **Rule aliases** apply (`## Motivation` == `## Problem`, `## Out of Scope` == `## Non-Goals`) — see the methodology guide for the full list.
- Exit code `0` = clean, `1` = MUST failure (in `--ci` mode only).

## CI mode (`--ci`)

Added in Sprint 11.3 as part of the methodology integrity work (PRD-034 / PRD-043),
`forgeplan validate --ci` turns the validator into a **hard pipeline gate**. In CI mode
the command exits 1 if **any MUST rule** fails on an `active` or `stale` artifact —
**drafts are intentionally excluded** so you are not blocked by work-in-progress stubs
that have not been activated yet.

### Behaviour

- `forgeplan validate --ci` → scans every artifact, filters to `status in {active, stale}`,
  runs the depth-aware MUST ruleset, and exits 1 if any artifact has at least one MUST
  violation.
- Drafts are still validated for human consumption (printed warnings) but do **not**
  cause a non-zero exit. The assumption is that a draft is explicitly a WIP state and
  activation is the real commitment point.
- Output stays human-readable by default; pair with `--json` if you want to post-process
  in CI (e.g. annotate a PR with findings).

### GitHub Actions snippet

Drop this into `.github/workflows/ci.yml` next to your `forgeplan health --ci` step:

```yaml
- name: Forgeplan validate gate
  run: |
    forgeplan scan-import
    forgeplan validate --ci
```

If this exits 1, the PR is blocked. Fix the MUST violations (or supersede / deprecate
the offending artifact if it is genuinely abandoned) before re-running.

Pair with [`forgeplan health --ci`](/docs/cli/health/#ci-mode---ci) for a two-layer gate:
`validate` catches structural completeness failures, `health` catches methodology debt
(blind spots, orphans, stale). Together they stop both "this PRD has no Problem section"
and "this PRD has no evidence" from landing in `dev`.

## How it fits the workflow

```
route → new → validate → reason (ADI) → code → evidence → review → activate
                 ↑                                           ↑
                 └───────── re-run after each edit ──────────┘
```

`validate` is the "did I fill the form" gate. `review` is the "am I ready to ship" gate
on top of it. `activate` refuses to run unless `validate` passes.

## See also

- [`forgeplan review`](/docs/cli/review/) — adds lifecycle checklist on top of validation
- [`forgeplan activate`](/docs/cli/activate/) — the gate `validate` feeds
- [`forgeplan score`](/docs/cli/score/) — quality beyond structural rules
- [Quality Gates](/docs/methodology/evidence/) — MUST/SHOULD/adversarial philosophy
- [CLI overview](/docs/cli/)
