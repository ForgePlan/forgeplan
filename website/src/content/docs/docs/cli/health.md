---
title: forgeplan health
description: "Project health dashboard — blind spots, orphans, stale, at-risk artifacts"
---

`forgeplan health` is the session-start command. It scans every artifact in
the workspace and reports four categories of problems: **blind spots**
(active artifacts without evidence), **orphans** (artifacts with no links in
or out), **stale** (valid_until expired), and **at risk** (low R_eff or
failing validation). If nothing is wrong it prints "Project looks healthy!"

The **Unified Workflow Protocol** makes `forgeplan health` mandatory at the
start of every session — before you write code, route a task, or open a new
PR. If health shows debt, you clear it before starting new work. Do not
accumulate debt.

## When to use

- **Session start** — always, no exceptions. Part of the standard protocol
- Before creating a PR — make sure your branch did not introduce new debt
- In CI/CD — use `--ci` mode to block pipelines on regressions
- After `forgeplan scan-import` or any bulk ingestion
- Weekly sprint review — check aggregate project health trends
- Before a release — `--ci --fail-on "orphans=0,blind_spots=0"` as a release gate

## When NOT to use

- In a tight edit-compile loop — too noisy for per-save runs
- For a single artifact — use `forgeplan context <ID>` or `forgeplan validate <ID>`
- As a replacement for `forgeplan status` — health flags problems; status gives raw counts

## Usage

```text
forgeplan health [OPTIONS]
```

## Options

```text
      --compact            Compact one-line output for hooks/scripts
      --json               Output as JSON for machine consumption
      --ci                 CI mode: exit code 1 if issues found (for pipeline gates)
      --fail-on <FAIL_ON>  Fail thresholds for --ci (e.g., "orphans=5,blind_spots=3,stale=2")
  -h, --help               Print help
  -V, --version            Print version
```

## Examples

### Example 1: Session start check

```bash
forgeplan health
```

Typical healthy output:

```
Project Health
==============
  Artifacts:    147 (PRD: 42, RFC: 18, ADR: 12, Evidence: 56, ...)
  Active:       98
  Blind spots:  0
  Orphans:      0
  Stale:        0
  At risk:      0

Project looks healthy!
```

### Example 2: Health with debt — clear it before new work

```bash
forgeplan health
```

```
Project Health
==============
  Blind spots:  2
    - PRD-019 (active, 0 evidence, R_eff=0.00)
    - RFC-007 (active, 0 evidence, R_eff=0.00)
  Orphans:      1
    - NOTE-033 (no parent, no children, no links)
  Stale:        3
    - ADR-004 (valid_until 2026-03-01, expired 41 days ago)
  At risk:      0

Next actions:
  1. Create EvidencePack for PRD-019 and RFC-007
  2. Link NOTE-033 to its parent or deprecate
  3. Review and renew/supersede ADR-004
```

Fix each before starting new work.

### Example 3: CI pipeline gate

```bash
forgeplan health --ci --fail-on "orphans=0,blind_spots=0,stale=5"
```

Exits 1 if the workspace has any orphans, any blind spots, or more than 5
stale artifacts. Drop this in GitHub Actions after `forgeplan scan-import`
to prevent health debt from landing in `dev`.

### Example 4: Machine-readable output

```bash
forgeplan health --json
```

Emits `{ "artifacts": 147, "blind_spots": [...], "orphans": [...], "stale": [...], "at_risk": [...] }`
for dashboards, bots, or Hindsight memory ingestion.

### Example 5: Compact mode for shell prompts

```bash
forgeplan health --compact
# -> Forgeplan: 147 artifacts, 0 issues
```

Use inside a zsh/bash prompt or tmux status bar to keep health visible.

## CI mode (`--ci`)

Added in Sprint 11.3 (PRD-034, methodology integrity gates), `forgeplan
health --ci` exits non-zero when the workspace exceeds configured debt
thresholds. This turns the health dashboard into a **PR gate** or **pipeline
blocker** rather than a purely advisory check at session start.

### Behaviour

- `forgeplan health --ci` → exit 1 if **any** orphans, blind spots, stale,
  or at-risk artifacts are found.
- `forgeplan health --ci --fail-on <spec>` → per-metric thresholds, in the
  form `metric=N` joined by commas. Supported metrics are `orphans`,
  `blind_spots`, `stale`, and `at_risk`. Exit 1 if **any** metric exceeds
  its threshold.

### GitHub Actions snippet

Drop this into `.github/workflows/ci.yml` after your `cargo test` job:

```yaml
- name: Forgeplan health gate
  run: |
    forgeplan scan-import
    forgeplan health --ci --fail-on "blind_spots=0,orphans=0"
```

The strictest configuration — `blind_spots=0,orphans=0` — means **no**
active PRD/RFC/ADR without evidence and **no** unlinked artifacts are
allowed to land in `dev`. Loosen the thresholds if you need a grace period
while migrating a brownfield workspace.

Pair this with [`forgeplan validate --ci`](/docs/cli/validate/#ci-mode---ci)
to cover both methodology debt (health) and structural completeness
(validate) in the same pipeline.

## Output interpretation

- **Blind spots** — `status=active` but R_eff=0 (no linked evidence). Means
  you activated an artifact on trust alone. Fix: create an EvidencePack with
  `verdict`, `congruence_level`, `evidence_type` fields and link it.
- **Orphans** — no incoming or outgoing links. Either unfinished or forgotten.
  Fix: link to parent/child or deprecate.
- **Stale** — `valid_until` has passed. Fix: `forgeplan renew <ID>` to extend
  or `forgeplan reopen <ID>` to supersede with a new draft.
- **At risk** — low R_eff (<0.3), failing validation, or depth-mismatched.
  Fix: run `forgeplan context <ID>` and address the specific issue.

Thresholds for concern:

- Any blind spots = immediate fix (you are claiming active decisions with no proof)
- Orphans > 3 = workflow leak, find where artifacts are getting abandoned
- Stale > 5 = decay problem, run `forgeplan refresh` for a sweep

## How it fits the workflow

```
[health] → route → Shape → Validate → Reason → Code → Evidence → Activate → [health]
   ^                                                                           ^
session start                                                             end of sprint
```

- **Session start**: always run first. Fix debt before new work.
- **Pre-PR**: run again to confirm your branch is clean
- **Release gate**: CI mode with strict `--fail-on` thresholds

## See also

- [`forgeplan status`](/docs/cli/status/) — raw counts without the health lens
- [`forgeplan blindspots`](/docs/cli/blindspots/) — blind-spot-only view
- [`forgeplan gaps`](/docs/cli/gaps/) — missing-artifact detection
- [`forgeplan refresh`](/docs/cli/stale/) — bulk stale artifact review
- [`forgeplan context`](/docs/cli/context/) — per-artifact deep-dive
- [Methodology: quality gates](/docs/methodology/overview/)
