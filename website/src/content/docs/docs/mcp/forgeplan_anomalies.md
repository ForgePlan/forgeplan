---
title: forgeplan_anomalies
description: "Issue #289 - detect pipeline anomalies (stuck drafts, orphan links, mis-typed based_on, missing MUST sections, expired evidence, cycles, duplicates, etc.). Returns structured list with severity + tier + suggested_resolution so orchestrators (`/forge-cycle`, `/forge-cleanup`) can dispatch auto/adi/user tiers without re-classifying."
---

Issue #289 - detect pipeline anomalies (stuck drafts, orphan links, mis-typed based_on, missing MUST sections, expired evidence, cycles, duplicates, etc.). Returns structured list with severity + tier + suggested_resolution so orchestrators (`/forge-cycle`, `/forge-cleanup`) can dispatch auto/adi/user tiers without re-classifying.

Every entry is a self-describing work order: a deterministic `id`, the `affected` artifact ids, a `severity`, a resolution `tier` (`auto` / `adi` / `user`) and, when a canonical fix exists, a `suggested_resolution` naming the exact tool to call. The caller sorts and dispatches; it does not re-classify.

**Category**: Other

## When an agent calls this

- **Start of a self-healing pass** - `/forge-cycle` polls this before deciding whether the tree is workable.
- **Cleanup sweeps** - `/forge-cleanup` reads `tier` to decide what it may fix without asking the operator.
- **After a bulk ingest or `scan-import`** - `orphan_link` catches relations whose target id does not exist in the workspace.
- **After an activation** - `missing_must_section` catches `active` artifacts whose bodies still carry stub markers (`TODO`, `TBD`, `placeholder`, `XXX`).
- **Long sessions and CI, diff mode** - pass `since` to get only the anomalies first observed after the previous scan instead of re-reading the entire list.

## What it detects

Fifteen kinds are detected in total: ten pipeline-side detectors run inside `detect_anomalies`, and five brownfield detectors are bridged in from `crates/forgeplan-core/src/brownfield.rs`. The v1 catalog from issue #289 names the first nine pipeline kinds; `unbacked_displacement` joined later as the ADR-020 guard. Severity and tier are fixed per kind:

| Kind | Severity | Tier | Suggested resolution |
|---|---|---|---|
| `stuck_draft` | medium | `auto` | `forgeplan_activate` on the affected EVID- |
| `orphan_link` | high | `user` | none - needs an operator decision |
| `mistyped_based_on` | medium | `adi` | `forgeplan_link` with `replace=true` and relation `informs` |
| `missing_must_section` | high | `user` | none |
| `expired_evidence` | medium | `user` | none |
| `weakest_link_unresolvable` | low | `adi` | `forgeplan_score` on the weakest link |
| `phase_mismatch` | low | `auto` | `forgeplan_phase_advance` |
| `circular_dependency` | high | `user` | none |
| `duplicate_artifact` | medium | `adi` | `forgeplan_supersede` the redundant artifact |
| `unbacked_displacement` | medium | `adi` | `forgeplan_score` on the first live target |
| `hypothesis_duplicate` | high, medium | `adi` | none - `target` points at the left hypothesis |
| `uncovered_use_case` | medium | `adi` | `forgeplan_link` the demonstrating SCEN- |
| `unverified_invariant` | medium | `adi` | `forgeplan_link` |
| `orphan_glossary_term` | low | `adi` | none - reference the term or deprecate the GLOS- |
| `untriangulated_hypothesis` | medium | `adi` | `forgeplan_link` a second evidence pack |

A few detection rules worth knowing before you act on the output:

- `unbacked_displacement` fires when an evidence pack with a `refutes` / `weakens` verdict was terminated (superseded or deprecated) without a `supersedes` successor edge while a target it informed is still live.
- `duplicate_artifact` compares titles plus body overlap; `hypothesis_duplicate` is title-token Jaccard only and is the same finding that [`forgeplan_contradictions`](/docs/mcp/forgeplan_contradictions/) reports on its own.
- `uncovered_use_case`, `unverified_invariant`, `orphan_glossary_term` and `untriangulated_hypothesis` are the four buckets of [`forgeplan_orphans`](/docs/mcp/forgeplan_orphans/), re-shaped into the canonical anomaly envelope.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `kind` | `string` | no | Keep a single anomaly kind, by its snake_case name (for example `orphan_link`). An unknown value returns an error listing the valid kinds instead of silently widening the result. |
| `severity` | `string` | no | Exact-match severity filter: `low`, `medium` or `high`. Anything else returns an error naming the three valid values. |
| `since` | `string` | no | RFC 3339 timestamp. Only anomalies whose `observed_at` is at or after it are returned - see the journal notes under Returns. A non-RFC-3339 value returns an error with an example. |
| `workspace` | `string` | no | Resolve the workspace from this directory instead of the MCP server's startup CWD. Absolute or tilde-expanded. Useful when the agent runs in a git worktree separate from the server. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::AnomaliesParams` (the shared DTOs live in `crates/forgeplan-mcp/src/types.rs`)_

## Returns

```json
{
  "_next_action": "1 anomaly(ies) detected — 1 require user input. Run `forgeplan_get <id>` on affected artifacts to review.",
  "anomalies": [
    {
      "id": "anom-orphan-link-EVID-001-informs-PRD-999",
      "kind": "orphan_link",
      "severity": "high",
      "tier": "user",
      "affected": ["EVID-001", "PRD-999"],
      "observed_at": "2026-09-16T13:50:38.262275+00:00",
      "description": "EVID-001 --informs--> PRD-999 (target does not exist)",
      "evidence": { "source": "EVID-001", "target": "PRD-999", "relation": "informs" },
      "suggested_resolution": null
    }
  ],
  "total": 1,
  "by_severity": { "low": 0, "medium": 0, "high": 1 },
  "by_tier": { "auto": 0, "adi": 0, "user": 1 }
}
```

That payload was captured from `forgeplan anomalies --json` in a throwaway workspace holding a single link to a non-existent target; the MCP tool returns the same report object, and both surfaces inject the same `_next_action` string (PRD-071 parity). Field notes:

- `id` is derived from the kind plus the affected ids, so re-running the scan yields the same id - that is your dedupe and tracking key.
- `affected[0]` is the primary artifact (the draft itself for `stuck_draft`, the source for `mistyped_based_on`).
- `description` is human-readable and capped at 200 chars.
- `evidence` is free-form per kind; the shape is documented per variant in `crates/forgeplan-core/src/anomalies.rs`.
- `suggested_resolution` is `null` for the kinds that need operator judgement, and otherwise carries `tier`, `action`, `target` and a one-line `rationale`.
- Entries are sorted by severity (high first), then by `id`, so the output is stable across runs.
- `_next_action` is one of four strings: `No anomalies. Done.`, an auto-fixable summary, a user-input summary (as above), or an ADI-investigation summary.

## Example invocation

All anomalies in the default workspace:

```json
{}
```

Only orphan links:

```json
{ "kind": "orphan_link" }
```

Only high-severity findings:

```json
{ "severity": "high" }
```

Diff mode - everything first observed after a known timestamp:

```json
{ "since": "2026-09-16T00:00:00Z" }
```

Reporting against a worktree instead of the server CWD:

```json
{ "workspace": "~/work/forgeplan-feature-branch" }
```

## Typical sequence

1. `forgeplan_anomalies` - read `by_tier`, not just `total`.
2. `by_tier.auto > 0` - the orchestrator resolves those without prompting (a complete stuck draft is safe to activate).
3. `by_tier.adi > 0` - run the FPF ADI loop: several plausible fixes exist, pick one and record why.
4. `by_tier.user > 0` - escalate with `NEED_USER_INPUT`; `forgeplan_get` the affected ids first so the question carries context.
5. Re-run with `since` set to the start of the cycle to confirm nothing new landed while you worked.

## Safety notes

- Read-only for the artifact store: nothing is activated, superseded or edited by this call.
- It does read and write `.forgeplan/anomalies-journal.jsonl` and takes the workspace lock for it, so concurrent calls serialise rather than lose journal entries.
- The journal is what gives `since` its diff semantics: an anomaly first seen in an earlier scan keeps its original `observed_at`. Entries for anomalies that no longer appear are garbage-collected.
- A missing or corrupt journal degrades gracefully - every anomaly is treated as freshly observed that scan (pre-0.33 behaviour), and journal write failures are warned, never fatal.
- The journal is a heuristic store, not a source of truth. The deterministic `id` is the identity of an anomaly.
- No LLM call and no network access. This is a pure scan of the local workspace.

## CLI equivalent

```bash
forgeplan anomalies --kind orphan_link --severity high --since 2026-09-16T00:00:00Z --json
```

See [`forgeplan anomalies`](/docs/cli/anomalies/) for the terminal surface.

## See also

- [`forgeplan_health`](/docs/mcp/forgeplan_health/) - aggregate verdict before you dive into the anomaly list.
- [`forgeplan_contradictions`](/docs/mcp/forgeplan_contradictions/) - the hypothesis-duplicate detector on its own.
- [`forgeplan_orphans`](/docs/mcp/forgeplan_orphans/) - the four brownfield buckets in their native shape.
- [`forgeplan_get`](/docs/mcp/forgeplan_get/) - read an affected artifact before deciding.
- [`forgeplan anomalies`](/docs/cli/anomalies/) - same detection engine from the terminal.
