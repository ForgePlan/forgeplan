---
title: forgeplan anomalies
description: "Issue #289 - detect pipeline anomalies (stuck drafts, orphan links, mis-typed based_on, etc.). Each entry carries severity + tier + suggested_resolution so orchestrators can dispatch fixes"
---

Detect pipeline anomalies across the workspace: stuck drafts, orphan links,
mis-typed `based_on` relations, missing MUST sections, expired evidence, and
so on. Every entry carries a **severity** and a **resolution tier** plus a
`suggested_resolution`, so an orchestrating agent can dispatch a fix without
re-classifying anything itself.

`anomalies` is the CLI mirror of the
[`forgeplan_anomalies`](/docs/mcp/forgeplan_anomalies/) MCP tool: same
inputs, same semantics, and the same JSON payload under `--json`.

## Usage

```text
forgeplan anomalies [OPTIONS]
```

## Options

```text
      --kind <KIND>          Filter by anomaly kind (snake_case: stuck_draft, orphan_link, mistyped_based_on, missing_must_section, expired_evidence, weakest_link_unresolvable, phase_mismatch, circular_dependency, duplicate_artifact)
      --severity <SEVERITY>  Filter by severity (low, medium, high)
      --since <SINCE>        Only return anomalies observed at or after this RFC 3339 timestamp
      --json                 Output as JSON instead of human-readable table
  -h, --help                 Print help
  -V, --version              Print version
```

## What it does

1. Opens the workspace store and runs the canonical detector from
   `forgeplan_core::anomalies`.
2. Compares the detected set against `.forgeplan/anomalies-journal.jsonl`,
   the append-only record of every anomaly id and the timestamp it was first
   seen at.
3. Applies the `--kind`, `--severity`, and `--since` filters.
4. Persists the refreshed journal (atomic tempfile + rename) so the next run
   can report a diff rather than a full snapshot.
5. Renders the human-readable table, or the structured payload with `--json`.

## Anomaly catalog

The catalog is defined in `crates/forgeplan-core/src/anomalies.rs`. The
first nine kinds are the v1 pipeline catalog from issue #289; the next five
are the brownfield extension from Epic #287; `unbacked_displacement` is the
ADR-020 score-laundering guard.

| Kind | What it means |
|---|---|
| `stuck_draft` | Evidence is complete and linked, but the artifact is still `draft` after 24h |
| `orphan_link` | A relation points at an artifact id that does not exist in the workspace |
| `mistyped_based_on` | Evidence linked to a PRD/RFC/ADR via `based_on` instead of `informs` |
| `missing_must_section` | A `status=active` artifact still carries stub markers (`TODO`, `TBD`, `placeholder`, `XXX`) |
| `expired_evidence` | `valid_until` is in the past while the evidence still informs an active artifact |
| `weakest_link_unresolvable` | R_eff collapsed to 0 through a hard CL penalty further up the chain |
| `phase_mismatch` | `status=active` but the lifecycle phase is still Shape or Validate |
| `circular_dependency` | Two artifacts are `based_on` each other |
| `duplicate_artifact` | Two artifacts with equivalent titles and overlapping bodies |
| `hypothesis_duplicate` | Two hypotheses with title-token Jaccard of 0.6 or more |
| `uncovered_use_case` | A use case has no linked scenario |
| `unverified_invariant` | An invariant has no linked scenario |
| `orphan_glossary_term` | A glossary term is never referenced from a use case or invariant body |
| `untriangulated_hypothesis` | A hypothesis references fewer than two distinct evidence packs |
| `unbacked_displacement` | Hostile evidence was terminated without a `supersedes` successor while its target is still live |

Use `--kind <snake_case>` to narrow to a single kind. It is a single-value
filter - one kind per invocation, matched exactly. An unknown value is
rejected rather than silently returning nothing:

```bash
forgeplan anomalies --kind bogus
```

Typical output:

```text
Error: Unknown anomaly kind: bogus
Valid kinds (pipeline): stuck_draft, orphan_link, mistyped_based_on, missing_must_section, expired_evidence, weakest_link_unresolvable, phase_mismatch, circular_dependency, duplicate_artifact
Valid kinds (brownfield, Epic #287): hypothesis_duplicate, uncovered_use_case, unverified_invariant, orphan_glossary_term, untriangulated_hypothesis
```

## Severity and tier

`--severity` takes `low`, `medium`, or `high` and matches exactly. Anything
else fails fast:

```bash
forgeplan anomalies --severity critical
```

Typical output:

```text
Error: Unknown severity: critical
Valid severities: low, medium, high
```

`--tier` does **not** exist. Tier is a field on each entry, not a filter, and
it is meant to be read by the orchestrator that consumes the report:

| Tier | Meaning | Who acts |
|---|---|---|
| `auto` | Low severity, unambiguous | The orchestrator fixes it silently |
| `adi` | Medium ambiguity | An FPF ADI loop in the orchestrator or agent |
| `user` | High severity, irreversible, or ambiguous | Escalate with a question |

Passing `--tier` is a hard error:

```text
$ forgeplan anomalies --tier auto
error: unexpected argument '--tier' found
```

## The `--since` diff filter

`--since` accepts an RFC 3339 timestamp and compares it against the journal
entry for each anomaly, not against the current scan time. An anomaly first
seen yesterday keeps yesterday's `observed_at`, so a poll loop with
`--since <last-poll>` sees only what is genuinely new. Any timezone notation
works (`Z`, `+00:00`, `+05:00`) because the comparison is on parsed
timestamps. A malformed value is rejected with an example:

```text
$ forgeplan anomalies --since yesterday
Error: --since is not a valid RFC 3339 timestamp (yesterday): premature end of input
Example: 2026-05-19T00:00:00Z
```

If the journal is missing or unreadable, every anomaly counts as fresh for
that scan - the filter degrades to full-snapshot semantics instead of
failing.

## Entry shape

| Field | Type | Description |
|---|---|---|
| `id` | string | Deterministic id derived from kind and affected artifacts, stable across runs |
| `kind` | string | Canonical snake_case kind from the catalog |
| `severity` | string | `low`, `medium`, or `high` |
| `tier` | string | `auto`, `adi`, or `user` |
| `affected` | array of string | Artifact ids, primary artifact first |
| `observed_at` | string | RFC 3339 timestamp, persisted across scans via the journal |
| `description` | string | Human-readable summary, at most 200 characters |
| `evidence` | object | Per-kind supporting fields (for example `age_hours`, `jaccard_score`) |
| `suggested_resolution` | object or null | `tier`, `action`, `target`, and a one-line `rationale` |

In text mode a non-null resolution renders as an indented
`-> {action} {target}` line under the anomaly; when `action` is absent the
renderer prints `(manual)`.

## Examples

### Example 1: a healthy workspace

```bash
forgeplan anomalies
```

Typical output:

```text
No pipeline anomalies detected. ✓
Done.
```

### Example 2: an active artifact left full of template markers

```bash
forgeplan anomalies
```

Typical output:

```text
forgeplan anomalies — 1 detected
  by severity: 1 high, 0 medium, 0 low
  by tier:     0 auto, 0 adi, 1 user

  ✗  [user] missing_must_section — PRD-001: active artifact has stub markers (TODO/TBD/placeholder)

Next: forgeplan get <affected-id>  # review user-tier anomalies
```

The trailing hint is chosen from the tier counts: `auto` wins first, then
`user`, then `adi`.

### Example 3: the same report as JSON

```bash
forgeplan anomalies --json
```

```json
{
  "_next_action": "1 anomaly(ies) detected — 1 require user input. Run `forgeplan_get <id>` on affected artifacts to review.",
  "anomalies": [
    {
      "affected": ["PRD-001"],
      "description": "PRD-001: active artifact has stub markers (TODO/TBD/placeholder)",
      "evidence": {
        "markers_found": 6,
        "message": "Body appears to be unfilled template (6 markers found)",
        "title": "Tighten the widget pipeline"
      },
      "id": "anom-missing-must-PRD-001",
      "kind": "missing_must_section",
      "observed_at": "2026-09-16T13:50:13.040412+00:00",
      "severity": "high",
      "suggested_resolution": null,
      "tier": "user"
    }
  ],
  "by_severity": { "high": 1, "low": 0, "medium": 0 },
  "by_tier": { "adi": 0, "auto": 0, "user": 1 },
  "total": 1
}
```

### Example 4: only the high-severity slice

```bash
forgeplan anomalies --severity high
```

Typical output:

```text
forgeplan anomalies — 1 detected
  by severity: 1 high, 0 medium, 0 low
  by tier:     0 auto, 0 adi, 1 user

  ✗  [user] missing_must_section — PRD-001: active artifact has stub markers (TODO/TBD/placeholder)

Next: forgeplan get <affected-id>  # review user-tier anomalies
```

### Example 5: one kind at a time

```bash
forgeplan anomalies --kind stuck_draft
```

Typical output:

```text
No pipeline anomalies detected. ✓
Done.
```

### Example 6: poll only what changed since the last sweep

```bash
forgeplan anomalies --since 2026-09-16T00:00:00Z
```

Typical output:

```text
No pipeline anomalies detected. ✓
Done.
```

## Notes

- **Exit code is always 0 on a successful scan**, whether or not anomalies
  were found. This is an advisory surface for orchestrators, not a gate; use
  [`forgeplan health --ci`](/docs/cli/health/) when you need a non-zero exit
  to block a pipeline.
- The journal lives at `.forgeplan/anomalies-journal.jsonl` - one JSON object
  per line (`id`, `observed_at`). It is a heuristic store, so an unparseable
  line is skipped rather than aborting the scan.
- The report goes to stdout, so `--json` stays clean for piping into `jq`.
- Anomaly ids are deterministic (`anom-missing-must-PRD-001`), which makes
  them safe to use as dedupe keys or ticket references.

## Related commands

- [`forgeplan health`](/docs/cli/health/) - the session-start dashboard that
  wraps the same detections in blind-spot / orphan / stale language.
- [`forgeplan blindspots`](/docs/cli/blindspots/) - the blind-spot-only view.
- [`forgeplan gaps`](/docs/cli/gaps/) - missing-artifact detection.
- [`forgeplan stale`](/docs/cli/stale/) - the bulk `valid_until` sweep.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan_anomalies`](/docs/mcp/forgeplan_anomalies/) - the MCP mirror with identical output
- [`forgeplan health`](/docs/cli/health/) - session-start dashboard
- [`forgeplan score`](/docs/cli/score/) - investigate the R_eff signals behind the adi tier
- [Methodology guide](/docs/methodology/overview/)
