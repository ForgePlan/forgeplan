---
title: forgeplan_health
description: "Show project health dashboard - gaps, risks, blind spots, orphans, stale evidence, and recommended next actions. Returns aggregate `verdict` field (\"healthy\" | \"needs_attention\" | \"unhealthy\") + `verdict_summary` for one-line human rendering. Branch on `verdict` for CI gates and agent decision logic. No LLM needed."
---

Show project health dashboard - gaps, risks, blind spots, orphans, stale evidence, and recommended next actions. This is the single most important MCP tool for an AI agent: call it at session start to understand project state before doing any work.

**Category**: Dashboards & Graph

## When an agent calls it

- **Session start** - understand the workspace's current state. Optional external memory is separate from this tool.
- **Before starting new work** - check if there are debts (blind spots, orphans, stale artifacts) that should be fixed first.
- **After a sprint** - verify nothing regressed (new blind spots, orphaned evidence).
- **Before release** - confirm health is clean.

The tool requires no LLM. Its report is an entry point for inspection, not a substitute for reading the affected artifacts.

## Input parameters

_No declared input fields. Call this tool with an empty object `{}`._

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The runtime report uses these keys:

| Field | Meaning |
|---|---|
| `total`, `by_kind`, `by_status` | Artifact count and count arrays by kind and stored status. |
| `by_derived_status`, `stale_count` | Derived freshness state and number of expired artifacts; `stale` is not a stored lifecycle status. |
| `blind_spots` | Objects with `id`, `title`, `issue` for decisions missing evidence. |
| `orphans` | Artifact ID strings, not objects. |
| `at_risk` | Objects with `id`, `title`, `reason`. |
| `possible_duplicates`, `active_stubs`, `stale_drafts` | Additional quality findings to inspect. |
| `advisory_phase_mismatches`, `phase_read_errors` | Advisory phase findings and read-error count. `phase_mismatches` is also emitted as a compatibility alias. |
| `gitignore_drift` | Paths and reasons for ignore-rule drift. |
| `next_actions` | Suggested remediation actions. |
| `verdict`, `verdict_summary` | Machine verdict (`empty`, `healthy`, `needs_attention`, `unhealthy`) and human-readable summary. Do not branch on the obsolete value `debt`. |
| `active_claims`, `active_claim_count`, `skipped_claim_files` | MCP claim-ledger details. Claims are advisory and do not enforce write exclusion in other tools. |
| `_next_action` | A concrete suggested next call. |

There is no `total_artifacts`, `risks` or per-artifact `stale` array in this response. Use [`forgeplan_stale`](/docs/mcp/forgeplan_stale/) for expiry details.

Success is JSON encoded in MCP `content[0].text`. `by_kind` entries are `{kind, count}`; both status count arrays use `{status, count}`. `possible_duplicates` entries contain `id_a`, `id_b`, `similarity`, `title_a`, `title_b`, `kind`; `active_stubs` contain `id`, `kind`, `title`, `markers_found`, `message`; `stale_drafts` contain `id`, `kind`, `age_hours`, `verdict_set`, `has_links`, `recommendation` (`ready_to_activate` or `incomplete_or_orphan`). Phase entries contain `id`, `title`, `status`, `current_phase`, `advisory`; gitignore entries contain `path`, `reason`; active claims contain `id`, `agent_id`, `expires_at`.

`at_risk` and the evidence scores used for `by_derived_status` are computed fresh with the **flat evidence minimum**, not the recursive score cached by `forgeplan_score`. Health does not cascade dependency scores or apply recursive evidence-type penalties. `at_risk` requires linked evidence and a flat score strictly below `0.3`; deprecated/superseded target artifacts are excluded. A target with no linked evidence is not flagged by this risk check (it may be a blind spot instead). A recursive dependency failure therefore need not produce a corresponding health risk.

The report does not repair findings, activate drafts, renew evidence, or enforce a CI exit code. An external caller must parse `verdict` and implement its own gate. Advisory phase mismatches are reported but excluded from verdict promotion. An empty initialized workspace returns `verdict: "empty"`, not `healthy`. Workspace failures can return plain-text tool errors; store/report failures can be JSON-RPC errors. Do not parse errors as report JSON.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

1. `forgeplan_health` - read the current verdict and findings.
2. If `verdict` is `needs_attention` or `unhealthy`, inspect the listed artifacts and choose the appropriate remediation.
3. Re-run after changes, then use `forgeplan_route` for the next task.

## CLI equivalent

```bash
forgeplan health
```

## See also

- [`forgeplan_status`](/docs/mcp/forgeplan_status/) - simpler kind × status breakdown.
- [`forgeplan_blindspots`](/docs/mcp/forgeplan_blindspots/) - focused blind-spot list.
- [`forgeplan_stale`](/docs/mcp/forgeplan_stale/) - expired `valid_until` list.
- [Methodology guide](/docs/methodology/overview/)
