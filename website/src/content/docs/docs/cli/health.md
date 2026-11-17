---
title: forgeplan health
description: "Inspect workspace health at session start, before a PR, or after bulk changes. The dashboard reports blind spots, orphans, expired evidence, at-risk artifacts, active stubs, duplicate signals, and advisory phase or gitignore drift."
---

Inspect workspace health at session start, before a PR, or after bulk changes. The dashboard reports blind spots, orphans, expired evidence, at-risk artifacts, active stubs, duplicate signals, and advisory phase or gitignore drift.

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
      --strict             Strict mode: exit 1 if verdict is NeedsAttention/Unhealthy or any of {orphans, blind_spots, active_stubs, at_risk} > 0. Designed for CI gates that want a single boolean signal. Empty workspaces and advisory-only signals (e.g. phase mismatches) keep exit 0
  -h, --help               Print help
  -V, --version            Print version
```

## Examples

```bash
forgeplan health
forgeplan health --compact
forgeplan health --json
forgeplan health --ci --fail-on "orphans=0,blind_spots=0,stale=5"
forgeplan health --strict --json
```

<a id="ci-mode---ci"></a>

## CI thresholds

In normal text mode, `--ci` exits 1 when a configured count is exceeded. Defaults are zero orphans and zero blind spots; stale and at_risk are unlimited unless explicitly set. `--fail-on` accepts comma-separated `orphans=N`, `blind_spots=N`, `stale=N`, and `at_risk=N`. After a fresh checkout, run `reindex` before the gate; `scan-import` does not refresh tracked artifacts.

## Strict mode and JSON caveat

`--strict` exits 1 for critical signals: orphans, blind spots, active stubs, at-risk findings, or a needs_attention/unhealthy verdict. Duplicate and stale counts affect this through verdict thresholds, not as unconditional direct failures. Advisory phase mismatches alone do not fail it. In 0.38.0 the JSON path returns before `--ci` thresholds are evaluated; do not use `--json --ci` as a gate. `--strict --json` does enforce strict mode and adds `exit_code`. Compact mode also returns before these gates.

## Triage

For blind decisions, inspect `score` and structured evidence fields; zero R_eff is not proof that no evidence link exists. Orphans have no incident relations. Re-review expired work before `renew` or `reopen`. Use `context` or `validate` for focused findings. A health report does not automatically repair, activate, or verify code.

## Machine output

JSON uses the shared health-report schema, including `total`, `verdict`, `blind_spots`, `orphans`, `stale_count`, `at_risk`, and additional diagnostic fields. It adds project identity and `_next_action`. Do not parse illustrative text dashboards as a stable schema.

## See also

- [`forgeplan status`](/docs/cli/status/) - raw counts without the health lens
- [`forgeplan blindspots`](/docs/cli/blindspots/) - blind-spot-only view
- [`forgeplan gaps`](/docs/cli/gaps/) - missing-artifact detection
- [`forgeplan stale`](/docs/cli/stale/) - bulk stale artifact review
- [`forgeplan context`](/docs/cli/context/) - per-artifact deep-dive
- [Methodology: quality gates](/docs/methodology/overview/)
