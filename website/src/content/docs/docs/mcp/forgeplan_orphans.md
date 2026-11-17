---
title: forgeplan_orphans
description: "Issue #287 Phase C - surface brownfield orphans: uncovered use cases (no scenario links), unverified invariants (no scenario links), orphan glossary terms (not referenced in any UC/INV body), un-triangulated hypotheses (< 2 evidence refs)."
---

Issue #287 Phase C - surface brownfield orphans: uncovered use cases (no scenario links), unverified invariants (no scenario links), orphan glossary terms (not referenced in any UC/INV body), un-triangulated hypotheses (fewer than 2 evidence refs).

This is the brownfield equivalent of a load-bearing check: it answers "which discovered claims are asserted but never exercised, and which terms are defined but never used". Every bucket is a heuristic over the link graph and the artifact bodies - the report is a worklist, not a verdict.

**Category**: Other

## The four buckets

| JSON field | Flagged when | Threshold |
|---|---|---|
| `uncovered_use_cases` | A `UC-` has no relation to any `SCEN-` (in either direction) | zero scenario links |
| `unverified_invariants` | An `INV-` has no relation to any `SCEN-` (in either direction) | zero scenario links |
| `orphan_terms` | A `GLOS-`'s `## Canonical Term` text never appears in any `UC-` or `INV-` body | case-insensitive substring, zero hits |
| `untriangulated_hypotheses` | A `HYP-`'s `## Evidence For` section holds fewer than 2 distinct references | fewer than 2 |

Rules that decide the first two buckets:

- Only **positive coverage relations** count: `informs`, `based_on`, `refines`, `demonstrates`, `covers`. An edge typed `supersedes` or `contradicts` deliberately does not count, so a refuted scenario cannot inflate coverage.
- Direction is permissive: a `UC-` counts as covered whether the edge points from the use case to the scenario or the other way round. The canonical edge is the scenario's `demonstrates` link, but hand-rendered workspaces are tolerated.

Rules that decide the last two:

- A glossary term is compared as an ASCII-case-insensitive substring against the concatenated bodies of every `UC-` and `INV-`. Cyrillic case is not folded. An empty `## Canonical Term` section counts as an orphan.
- Hypothesis references are counted from the `## Evidence For` section only: distinct `EVID-` tokens (all-`X` placeholders skipped) plus `path:line` tokens where the path contains a slash and the line is all digits. The count is over the non-canonical form too, which is why a hypothesis may reach 2 without holding two evidence packs.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP `content[0].text`: four arrays plus `_next_action`. There is no `total` field; the hint sums the buckets. Illustrative values:

```json
{
  "uncovered_use_cases": ["UC-002", "UC-007"],
  "unverified_invariants": ["INV-004"],
  "orphan_terms": ["GLOS-009"],
  "untriangulated_hypotheses": ["HYP-001"],
  "_next_action": "Next: link 5 orphan(s) across 4 buckets — add scenarios, reference terms, or triangulate hypotheses."
}
```

Field notes:

- Bucket entries are artifact ids, in the order the records were scanned, so the report is deterministic for a fixed workspace.
- `_next_action` is either `Done. - no brownfield orphans.` or the count above.
- There is no severity per bucket here - that lives in the bridged anomaly form, where uncovered use cases and unverified invariants are `medium`, orphan glossary terms are `low`, and all four resolve through the ADI tier.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## What v1 does not check

Be precise about the heuristic before you close a gap by editing text:

- An `INV-` is "unverified" purely because no scenario links to it. The stronger check - that the linked scenario's body actually probes the boundary - needs a body read and is deferred.
- Evidence references are matched textually. A hypothesis that cites an `EVID-` id which does not exist in the workspace still counts as triangulated; dangling references are a separate check.
- Term usage is a substring test, not a semantic one. A synonym, or a term used only in a `SCEN-` body, reads as an orphan.
- Scope is brownfield only: artifacts outside the `UC-` / `INV-` / `GLOS-` / `HYP-` kinds are never reported here, however isolated they are. Artifact-level orphans with zero incoming and outgoing links are a different measure - that is [`forgeplan_blindspots`](/docs/mcp/forgeplan_blindspots/).

## When an agent calls this

- **Right after a discovery ingest** - the fastest way to see which discovered claims are still unsupported.
- **Before claiming business coverage is done** — use this report alongside [`forgeplan_coverage_business`](/docs/mcp/forgeplan_coverage_business/). Empty orphan buckets are not a prerequisite enforced by the score, and a high score does not imply empty buckets.
- **After adding scenarios** - re-run to watch buckets drain instead of assuming the new links landed.
- **During a `/forge-cleanup` sweep** - the four buckets map onto four different fixes: link a scenario, reference a term, triangulate a hypothesis, or deprecate the artifact.

## Typical sequence

1. `forgeplan_orphans` - read all four buckets.
2. Triage: for each orphan, decide whether the artifact is under-supported (fix it) or obsolete (deprecate it).
3. Add and link a scenario for UC/INV coverage; edit UC/INV bodies to use a glossary term; add distinct references to the hypothesis's `## Evidence For`. Merely linking another EvidencePack does not change the textual reference count.
4. Re-run and inspect remaining buckets. The report does not filter terminal statuses, so deprecating an artifact alone need not remove it.
5. Read `forgeplan_coverage_business` as a complementary measure: it scores artifact counts, use-case scenario coverage, and hypothesis states. Glossary usage, invariant-to-scenario links, and textual triangulation references are not independently scored, so fixing those orphan findings can leave `extract_score` unchanged.

## Safety notes

- Read-only: the report builder is a pure function over the record snapshot and the relation list; nothing is written.
- No LLM call and no network access.
- Deterministic: the same workspace yields the same four arrays in the same order.
- Workspace, lock, record-read, or relation-read failures return plain-text MCP tool errors (`isError: true`). Empty/missing sections can still produce heuristic gaps; an empty report is not proof of semantic correctness.
- There is no CLI equivalent - this is an MCP-only surface. The same four buckets reach the terminal through [`forgeplan anomalies`](/docs/cli/anomalies/) as `uncovered_use_case`, `unverified_invariant`, `orphan_glossary_term` and `untriangulated_hypothesis`.

## See also

- [`forgeplan_coverage_business`](/docs/mcp/forgeplan_coverage_business/) — complementary extraction score, not a rollup of these four buckets.
- [`forgeplan_anomalies`](/docs/mcp/forgeplan_anomalies/) - the four buckets as actionable anomalies with resolution tiers.
- [`forgeplan_contradictions`](/docs/mcp/forgeplan_contradictions/) - the other brownfield inspector from the same phase.
- [`forgeplan_discover_start`](/docs/mcp/forgeplan_discover_start/) - where the brownfield artifacts these buckets describe come from.
- [Methodology guide](/docs/methodology/overview/) - where brownfield inspectors sit in the cycle.
