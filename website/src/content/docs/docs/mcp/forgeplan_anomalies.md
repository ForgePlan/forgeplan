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
- `duplicate_artifact` compares tokenized titles of same-kind records, excluding deprecated/superseded records; it does not compare bodies. `hypothesis_duplicate` separately uses hypothesis title-token Jaccard and is also reported by [`forgeplan_contradictions`](/docs/mcp/forgeplan_contradictions/). Inspect both artifacts before acting on either heuristic.
- `uncovered_use_case`, `unverified_invariant`, `orphan_glossary_term` and `untriangulated_hypothesis` are the four buckets of [`forgeplan_orphans`](/docs/mcp/forgeplan_orphans/), re-shaped into the canonical anomaly envelope.
- `expired_evidence` checks expiry and outgoing `informs`/`based_on` links but does not actually filter the evidence or targets by lifecycle status, despite its “active parent(s)” description.
- `circular_dependency` detects reciprocal `based_on` pairs, not arbitrary longer cycles. Use the structural graph tools for broader cycle analysis.
- `weakest_link_unresolvable` reads cached R_eff and walks at most 16 hops. It can report no named weakest link when the walk revisits a node, reaches its cap, or finds only skipped/positive ancestors; that is not proof of a cycle.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `kind` | `string \| null` | no | Issue #289: filter by anomaly kind. Accepts the same snake_case names emitted in the response (`stuck_draft`, `orphan_link`, …). When `None`, all detected anomalies are returned. Schema default: `null`. |
| `severity` | `string \| null` | no | Filter by severity (`low` / `medium` / `high`). Exact match. Schema default: `null`. |
| `since` | `string \| null` | no | ISO 8601 cutoff - only return anomalies observed at or after this timestamp. Useful for diff-style polling. Schema default: `null`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP `content[0].text`. The values below are illustrative, not a test result from this review. Invalid `kind`, `severity`, or RFC 3339 `since` returns an `isError: true` tool result containing plain text and a recovery hint. Counts describe the filtered report.

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

The MCP tool and CLI share the report builder and `_next_action` helper. Field notes:

- `id` is derived from the kind plus the affected ids, so re-running the scan yields the same id - that is your dedupe and tracking key.
- Most `affected` arrays start with the primary artifact; the brownfield detector-cap anomaly has an empty array and no concrete target.
- `description` is human-readable; the struct comment suggests 200 characters, but construction does not enforce a length cap.
- `evidence` is free-form per kind; the shape is documented per variant in `crates/forgeplan-core/src/anomalies.rs`.
- `suggested_resolution` is `null` for the kinds that need operator judgement, and otherwise carries `tier`, `action`, `target` and a one-line `rationale`.
- Entries are sorted by severity (high first), then by `id`, so the output is stable across runs.
- `_next_action` is one of four strings: `No anomalies. Done.`, an auto-fixable summary, a user-input summary (as above), or an ADI-investigation summary.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

1. `forgeplan_anomalies` - read `by_tier`, not just `total`.
2. `by_tier.auto > 0` — inspect the suggested action and apply the calling agent's authorization policy. The tier is a recommendation, not permission, and this tool never executes the fix.
3. `by_tier.adi > 0` - run the FPF ADI loop: several plausible fixes exist, pick one and record why.
4. `by_tier.user > 0` - escalate with `NEED_USER_INPUT`; `forgeplan_get` the affected ids first so the question carries context.
5. Re-run with `since` set to the start of the cycle to confirm nothing new landed while you worked.

## Safety notes

- Read-only for the artifact store: nothing is activated, superseded or edited by this call.
- It does read and write `.forgeplan/anomalies-journal.jsonl` and takes the workspace lock for it, so concurrent calls serialise rather than lose journal entries.
- The journal is what gives `since` its diff semantics: an anomaly first seen in an earlier scan keeps its original `observed_at`. Entries for anomalies that no longer appear are garbage-collected.
- A missing or corrupt journal degrades gracefully - every anomaly is treated as freshly observed that scan (pre-0.33 behaviour), and journal write failures are warned, never fatal.
- The journal is a heuristic store, not a source of truth. The deterministic `id` is the identity of an anomaly.
- No LLM call. The handler reads the configured workspace store and local journal; it does not invoke the suggested MCP tools.

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
