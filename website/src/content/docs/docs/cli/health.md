---
title: forgeplan health
description: "Project health dashboard - blind spots, orphans, stale, at-risk artifacts"
---

`forgeplan health` summarizes workspace debt, including **blind spots**, **orphans**, overdue validity dates, **at-risk** evidence, active stubs, duplicate candidates, and advisory phase signals. It uses the owning read source: canonical captures on migrated workspaces, the legacy store otherwise. These categories are not interchangeable with structural validation or acceptance of the implementation.

The **Unified Workflow Protocol** makes `forgeplan health` mandatory at the
start of every session - before you write code, route a task, or open a new
PR. If health shows debt, you clear it before starting new work. Do not
accumulate debt.

## When to use

- **Session start** - always, no exceptions. Part of the standard protocol
- Before creating a PR - make sure your branch did not introduce new debt
- In CI/CD - use `--ci` mode to block pipelines on regressions
- After `forgeplan scan-import` or any bulk ingestion
- Weekly sprint review - check aggregate project health trends
- Before a release - `--ci --fail-on "orphans=0,blind_spots=0"` as a release gate

## When NOT to use

- In a tight edit-compile loop - too noisy for per-save runs
- For a single artifact - use `forgeplan context <ID>` or `forgeplan validate <ID>`
- As a replacement for `forgeplan status` - health flags problems; status gives raw counts

## Usage

```text
forgeplan health [OPTIONS]
```

## Options

```text
      --compact            Compact one-line output for hooks/scripts
      --json               Output as JSON for machine consumption
      --ci                 Text-mode gate: fail when configured debt thresholds are exceeded
      --fail-on <FAIL_ON>  Fail thresholds for --ci (e.g., "orphans=5,blind_spots=3,stale=2")
      --strict             Fail on critical health signals; also applies with --json
  -h, --help               Print help
  -V, --version            Print version
```

## Examples

### Example 1: Session start check

```bash
forgeplan health
```

Illustrative healthy summary, not a captured transcript or an exhaustive output schema:

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

### Example 2: Health with debt - clear it before new work

```bash
forgeplan health
```

Illustrative debt summary, not a captured runtime transcript:

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

Exits 1 for any orphans, any blind spots, or more than five stale records. Run after the pipeline's intended ingestion or canonical index maintenance; `scan-import` is document ingestion, not routine Constellation recovery.

### Example 4: Machine-readable output

```bash
forgeplan health --json
```

The JSON object includes `total`, `verdict`, `by_kind`, `by_status`, `blind_spots`, `orphans`, `stale_count`, `at_risk`, `active_stubs`, duplicate candidates and advisory fields. Counts and list fields differ: stale validity is a count, not a `stale` array. `--strict --json` also adds `exit_code`.

### Example 5: Compact mode for shell prompts

```bash
forgeplan health --compact
# Output includes project, artifact count, blind spots, stale count and at-risk count.
```

Use inside a zsh/bash prompt or tmux status bar to keep health visible.

## CI mode (`--ci`)

Added in Sprint 11.3 (PRD-034, methodology integrity gates), `forgeplan
health --ci` exits non-zero when the workspace exceeds configured debt
thresholds. This turns the health dashboard into a **PR gate** or **pipeline
blocker** rather than a purely advisory check at session start.

### Behaviour

- Plain `health --ci` fails on any `orphans` or `blind_spots`; omitted `stale` and `at_risk` thresholds are unbounded.
- `--fail-on` accepts comma-separated `metric=N` thresholds for `orphans`, `blind_spots`, `stale`, and `at_risk`; failure means a count exceeds its threshold.
- In this selected implementation `--json` and `--compact` return before the `--ci` branch. Do not rely on `--ci` with those output modes. Use the separately implemented `--strict` gate for JSON automation; `--strict` and `--ci` are different policies.

### GitHub Actions snippet

Drop this into `.github/workflows/ci.yml` after your `cargo test` job:

```yaml
- name: Forgeplan health gate
  run: |
    # Run after the owning index has been maintained for the intended sources.
    forgeplan health --ci --fail-on "blind_spots=0,orphans=0"
```

These thresholds reject any counted blind spots or orphans; they do not gate stale or at-risk counts unless explicitly included. They are not the strictest possible health policy. Choose the required policy deliberately; do not loosen an existing project gate merely to make CI pass.

Pair this with [`forgeplan validate --ci`](/docs/cli/validate/) to check methodology debt and structural completeness separately.

## Output interpretation

- **Blind spots** - non-draft, non-terminal decision artifacts without linked evidence. A zero score with linked refuting evidence is not the same condition. Add real EvidencePacks with declared `verdict`, `congruence_level`, and `evidence_type`.
- **Orphans** - no incoming or outgoing links. Either unfinished or forgotten.
  Fix: link to parent/child or deprecate.
- **Stale** - a validity date has passed; detection does not itself rewrite persisted status. Inspect the stored status and use the applicable `renew --reason --until` transition, or `reopen --reason` to create a new draft and deprecate the original.
- **At risk** - a live artifact has linked evidence whose flat R_eff is below 0.3. Structural validation and depth calibration are separate checks.

Thresholds for concern:

- Any blind spots = immediate fix (you are claiming active decisions with no proof)
- Orphans > 3 = workflow leak, find where artifacts are getting abandoned
- Stale > 5 = review overdue records with `forgeplan stale`; there is no `forgeplan refresh` command.

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

- [`forgeplan status`](/docs/cli/status/) - raw counts without the health lens
- [`forgeplan blindspots`](/docs/cli/blindspots/) - blind-spot-only view
- [`forgeplan gaps`](/docs/cli/gaps/) - missing-artifact detection
- [`forgeplan stale`](/docs/cli/stale/) - inspect expired validity dates
- [`forgeplan context`](/docs/cli/context/) - per-artifact deep-dive
- [Methodology: quality gates](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan health --help`.

```text
Show project health dashboard — gaps, risks, blind spots, next actions

Usage: forgeplan health [OPTIONS]

Options:
      --compact            Compact one-line output for hooks/scripts
      --json               Output as JSON for machine consumption
      --ci                 CI mode: exit code 1 if issues found (for pipeline gates)
      --fail-on <FAIL_ON>  Fail thresholds for --ci (e.g., "orphans=5,blind_spots=3,stale=2")
      --strict             Strict mode: exit 1 if verdict is NeedsAttention/Unhealthy or any of {orphans, blind_spots, active_stubs, at_risk} > 0. Designed for CI gates that want a single boolean signal. Empty workspaces and advisory-only signals (e.g. phase mismatches) keep exit 0
  -h, --help               Print help
  -V, --version            Print version
```
<!-- forgeplan-runtime:end -->
