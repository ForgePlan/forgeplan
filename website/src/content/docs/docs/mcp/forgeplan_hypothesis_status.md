---
title: forgeplan_hypothesis_status
description: "Issue #287 Phase C - query hypothesis lifecycle state. Returns workspace-wide state distribution (verified / strong-inferred / inferred / refuted / parked) plus best-effort recent transitions (last 30 days, audit-line extraction from `## Lifecycle State` sections). Optional `id` filter narrows to a single HYP- artifact."
---

The read side of the hypothesis state machine maintained by
[`forgeplan_hypothesis_promote`](/docs/mcp/forgeplan_hypothesis_promote/). One call answers "how
much of this brownfield model is actually verified?" by counting every `kind: hypothesis` artifact
into its lifecycle state, and by scraping the last 30 days of promotion audit lines out of the
artifact bodies so you can see how recently things moved.

It is a pure report: no artifact is created, mutated or activated.

**Category**: Other

## When an agent calls this

- **Brownfield triage.** Before proposing new work, find out how many claims are `parked` (waiting
  on a domain interview) rather than `verified` (safe to build on).
- **After a promote.** Confirm the state landed and that the audit line is in the body, especially
  when a batch of promotions ran across several agents.
- **Triangulation check.** A large `inferred` + `strong-inferred` bucket with a small `verified`
  bucket is the signal to gather independent evidence rather than to write scenarios.
- **Coverage context.** [`forgeplan_coverage_business`](/docs/mcp/forgeplan_coverage_business/)
  scores a domain model; this report explains *why* its hypothesis term is low.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `id` | `string \| null` | no | Optional HYP- id to narrow the report to a single hypothesis. When `None`, returns the workspace-wide state distribution. Schema default: `null`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## What it does

1. Resolves the workspace through the worktree-aware read path, which does not take the write gate.
2. Acquires the workspace lock only around the snapshot read, then drops it before the report is
   built - the record listing is already snapshot-consistent, and holding the exclusive lock
   through the CPU pass would needlessly serialise other tools.
3. Keeps only artifacts whose kind is `hypothesis` (case-insensitive); records of other kinds are
   skipped silently, so passing the whole workspace is harmless.
4. Reads `hypothesis_state:` from each kept artifact's frontmatter. A missing, null, non-string or
   unparseable value counts as `parked`; a body without parseable frontmatter also counts as
   `parked`.
5. Scans each body's `## Lifecycle State` section for audit lines and keeps the ones dated inside
   the last 30 days.
6. Returns the report with a `_next_action` hint.

## Returns

Success is JSON encoded in MCP `content[0].text`. Illustrative values (assuming the example dates fall within the cutoff):

```json
{
  "_next_action": "Next: promote a hypothesis — `forgeplan_hypothesis_promote` (1 tracked).",
  "by_state": {
    "inferred": 0,
    "parked": 0,
    "refuted": 0,
    "strong_inferred": 0,
    "verified": 1
  },
  "recent_transitions": [
    {
      "date": "2026-09-16",
      "id": "HYP-001",
      "new_state": "inferred",
      "old_state": "parked"
    },
    {
      "date": "2026-09-16",
      "id": "HYP-001",
      "new_state": "strong-inferred",
      "old_state": "inferred"
    },
    {
      "date": "2026-09-16",
      "id": "HYP-001",
      "new_state": "verified",
      "old_state": "strong-inferred"
    }
  ],
  "total": 1
}
```

| Field | Meaning |
|---|---|
| `total` | Hypotheses counted after the optional `id` filter. |
| `by_state` | Per-state counter over the five states. Always all five keys, including the zeros. |
| `recent_transitions` | Audit entries in record/body scan order, not globally date-sorted. Dates earlier than UTC today minus 30 days are skipped; future-dated lines are not rejected. |

## Notes

- **Key naming is mixed, on purpose.** The `by_state` counters are snake_case (`strong_inferred`)
  while the state values in `old_state` / `new_state` and in the frontmatter are kebab-case
  (`strong-inferred`). The counters are flattened so JSON consumers can address them as fields.
- **The distribution trusts the frontmatter.** Counts read `hypothesis_state:`, not the audit lines
  - so a hand-edited body with stale lines will not distort `by_state`. The transitions list is the
  opposite: it is body-derived and therefore best-effort.
- **Transition extraction is strict.** Only the canonical line shape written by
  [`forgeplan_hypothesis_promote`](/docs/mcp/forgeplan_hypothesis_promote/) is recognised. Lines
  that use `to` instead of the arrow, or that lack the leading dash, are skipped silently.
- **The 30-day window is computed from the current UTC date**, not from any artifact timestamp.
- **A filter that matches nothing is not an error.** Asking for an id that does not exist returns
  `total: 0` with every counter at zero, and the hint then reads
  `Done. - no hypothesis artifacts in workspace.` even when the workspace is full of hypotheses.
  Treat that hint as "nothing matched this filter", and check `total` before concluding the
  workspace is empty.
- **Reading is not enumerating.** Only counts and recent transitions come back; call
  [`forgeplan_get`](/docs/mcp/forgeplan_get/) on an id when you need the claim itself.
- **Errors are not empty reports.** Workspace, lock, or record-read failures return plain-text MCP tool errors (`isError: true`). A successful unmatched filter instead returns the zero-count report described above.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

```text
forgeplan_hypothesis_status                       ← distribution + recent movement
forgeplan_list kind=hypothesis                    ← the ids behind the counts
forgeplan_get HYP-001                             ← the claim itself
forgeplan_hypothesis_promote (…, new_state=…)     ← the returned `_next_action`
```

## Related commands

- [`forgeplan graph --brownfield-only`](/docs/cli/graph/) - the same states rendered as a subgraph.
- [`forgeplan anomalies`](/docs/cli/anomalies/) - per-artifact findings, including
  `untriangulated_hypothesis`, where this report is aggregate.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_hypothesis_promote`](/docs/mcp/forgeplan_hypothesis_promote/) - the write side that produces these states
- [`forgeplan_coverage_business`](/docs/mcp/forgeplan_coverage_business/) - domain-model extraction score that consumes the distribution
- [`forgeplan_orphans`](/docs/mcp/forgeplan_orphans/) - the per-artifact view of unverified brownfield claims
- [`forgeplan_get`](/docs/mcp/forgeplan_get/) - read one hypothesis in full
