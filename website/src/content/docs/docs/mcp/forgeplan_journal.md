---
title: forgeplan_journal
description: "Decision timeline for ADR, Note, PRD, RFC, Epic, Spec, Problem and Solution by default, with flat R_eff and evidence counts. The explicit kind filter accepts only adr, note, problem or solution."
---

Show the decision journal: without `kind`, the timeline includes ADR, Note, PRD, RFC, Epic, Spec, Problem and Solution artifacts, annotated with flat `R_eff` and evidence counts. The explicit `kind` filter accepts only `adr`, `note`, `problem` or `solution`; its four-value enum does not limit the unfiltered default. Use `forgeplan_get` to read an entry's rationale and supporting links.

**Category**: Dashboards & Graph

## When an agent calls it

- **Session start after a long absence** - walk the last N decisions to rebuild context.
- **Before making a related decision** - check whether a prior Note / ADR / Problem already covers the topic.
- **At-risk audit** - call with `risk: true` to surface only decisions with weak or expired evidence.
- **Onboarding** - generate a narrative of the project's decision history.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `kind` | `string \| null` | no | Filter by artifact kind (decision-kinds only) string: Enum: `"adr"`, `"note"`, `"problem"`, `"solution"`. |
| `risk` | `boolean \| null` | no | Show only at-risk decisions Schema default: `null`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP `content[0].text`: `entries`, `total`, and `_next_action`. Each entry contains exactly `id`, `title`, `kind`, `created_at`, `r_eff`, `evidence_count`, `has_stale_evidence`. The internal entry has a status, but the MCP serializer does not expose it. There are no evidence-ID arrays or per-entry `at_risk` fields.

Entries are ordered by `created_at` string descending, with all lifecycle statuses included. `risk` defaults to false; true keeps entries with no evidence, flat `r_eff < 0.3`, or expired eligible evidence. The hint separately counts risk using `< 0.5`, so its count can differ from the risk-filter definition. `evidence_count` includes terminal linked packs, although their scores/expiry are excluded from the flat aggregate. This is a fresh flat-evidence calculation, not the cached recursive score; it does not persist scores or mutate artifacts.

Illustrative response for one ADR without evidence:

```json
{
  "entries": [
    {"id": "ADR-005", "kind": "adr", "title": "Storage design", "created_at": "2026-04-03T00:00:00Z", "r_eff": 0.0, "evidence_count": 0, "has_stale_evidence": false}
  ],
  "total": 1,
  "_next_action": "1 at-risk of 1 entries. Score the worst: `forgeplan_score ADR-005`."
}
```

Workspace errors return plain-text tool errors; journal/store failures can be JSON-RPC errors, not empty successful timelines.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "kind": "adr",
  "risk": true
}
```

## Typical sequence

1. `forgeplan_journal` with `risk: true` - list at-risk decisions.
2. `forgeplan_get` for each entry - read full body.
3. `forgeplan_new` evidence → `forgeplan_link` → `forgeplan_score` - refresh R_eff.

## CLI equivalent

```bash
forgeplan journal --type adr --risk
```

## See also

- [`forgeplan_blindspots`](/docs/mcp/forgeplan_blindspots/) - decisions with no evidence at all.
- [`forgeplan_decay`](/docs/mcp/forgeplan_decay/) - R_eff degradation from aging evidence.
- [Methodology guide](/docs/methodology/overview/)
