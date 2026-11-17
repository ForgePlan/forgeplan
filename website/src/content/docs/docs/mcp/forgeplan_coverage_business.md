---
title: forgeplan_coverage_business
description: "Issue #287 Phase C - compute extract_score (0.0-1.0) for a Domain Model. Reads the DM's ## Composition section to derive expected counts of glossary / use_cases / invariants / scenarios / hypotheses; counts actuals across the workspace; returns weighted extract_score. Canonical fields (DDL/SDL/pseudo-code) default to 0/false until marketplace#79 Phase E lands."
---

Compute `extract_score` (0.0–1.0) for a Domain Model from its `## Composition` expectations, workspace-wide artifact counts, scenario links, and hypothesis states. The core handler always returns zero/false canonical-readiness fields; installing a marketplace pack does not turn these defaults into compiler or reproducibility results.

One call answers "how much of this domain model has actually been extracted into artifacts". It is a coverage ratio, not a correctness measure - a workspace can score 1.0 and still be wrong, and it can score 0.4 legitimately while the model is still being discovered.

**Category**: Quality & Validation

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `domain_model_id` | `string` | yes | Required DM- artifact id. The Composition section of that Domain Model artifact provides the `expected` counts; this tool counts what's actually present in the workspace. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Where `expected` comes from

The denominator is self-declared by the domain model itself. The `## Composition` section is parsed for distinct id-shaped tokens per prefix:

| Expected count | Prefix counted in `## Composition` |
|---|---|
| `glossary.expected` | `GLOS-` |
| `use_cases.expected` | `UC-` |
| `invariants.expected` | `INV-` |
| not used by the score | `SCEN-` and `HYP-` |

- Counting is deduplicated, so an id listed twice counts once.
- Template placeholders are skipped: a literal `GLOS-XXX` never counts, so a never-edited domain model does not look like it has one of each.
- A domain model with no `## Composition` section yields zero expected counts, and the report still comes back as a valid envelope with `rate = 0.0` per term.
- The actuals are counted workspace-wide by kind (`glossary`, `use_case`, `invariant`, `hypothesis`), so artifacts that the composition does not mention still push `count` up.

## How `extract_score` is computed

`extract_score` is a weighted sum of five clamped ratios, each component capped at 1.0:

| Component | Weight | Ratio used |
|---|---|---|
| `glossary` | 0.15 | `count / expected` |
| `use_cases` | 0.25 | `count / expected` |
| `invariants` | 0.25 | `count / expected` |
| `scenarios` | 0.20 | `coverage_rate` - use cases with at least one linked scenario, over all use cases |
| `hypotheses` | 0.15 | `(verified + strong_inferred + 0.5 * inferred) / total` |

Details that change what a score of, say, 0.62 means:

- A ratio with `expected = 0` is `0.0`, never `NaN`, so an empty composition contributes nothing rather than poisoning the JSON.
- The scenario component uses the *coverage rate*, not the average scenarios per use case. `per_use_case_avg` is reported as information; the DM template's target of at least two scenarios per use case is not part of the score.
- The hypothesis component is a population average, and it is `0.0` when the workspace holds no hypotheses - not `1.0`.
- Only the five positive coverage relations (`informs`, `based_on`, `refines`, `demonstrates`, `covers`) count towards scenario coverage.

## Returns

Success is JSON in MCP `content[0].text`. Illustrative complete report:

```json
{
  "glossary": { "count": 1, "expected": 1, "rate": 1.0 },
  "use_cases": { "count": 1, "expected": 1, "rate": 1.0 },
  "invariants": { "count": 1, "expected": 1, "rate": 1.0 },
  "scenarios": { "per_use_case_avg": 1.0, "coverage_rate": 1.0 },
  "hypotheses": {
    "by_state": {
      "parked": 0,
      "inferred": 0,
      "strong_inferred": 0,
      "verified": 1,
      "refuted": 0
    },
    "total": 1
  },
  "canonical": {
    "ddl_compile": false,
    "sdl_parse": false,
    "pseudo_code_coherence": 0.0,
    "reproducibility": 0.0
  },
  "extract_score": 1.0,
  "_next_action": "Next: extract_score = 1.00. Improve coverage by adding artifacts referenced in DM-001's Composition section."
}
```

Field notes:

- `canonical` always contains defaults (`false`, `false`, `0.0`, `0.0`) in this core implementation. They are not executed checks.
- `hypotheses.by_state` uses `parked`, `inferred`, `strong_inferred`, `verified`, `refuted`. Counts read `hypothesis_state` from frontmatter, not the audit text in `## Lifecycle State`; missing/unparseable states count as `parked`.
- `glossary`, `use_cases` and `invariants` share the same triple shape: `count`, `expected`, `rate`.
- `_next_action` always states the score and names the composition section as the place to add artifacts.
- Reported `rate` can exceed 1.0; only its contribution to `extract_score` is clamped. Counts include terminal artifacts and artifacts outside the selected DM's Composition.
- A missing DM returns a plain-text tool error; an existing record of a different kind returns a tool error explaining that `domain_model` is required. Workspace, lock, and snapshot-read failures also return tool errors, not zero-coverage reports.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "domain_model_id": "DM-001"
}
```

## When an agent calls this

- **After `forgeplan_discover_complete`** - turn a finished discovery session into one number for the report.
- **Before declaring a domain model extracted** - the score is the honest answer to "is anything missing".
- **While resolving orphan findings** — compare the complementary reports, but do not expect every orphan fix to change the score.
- **In a dashboard or CI trend** - the same workspace yields a deterministic score, so deltas are meaningful.

## Typical sequence

1. Read `forgeplan_orphans` for unsupported use cases/invariants, unused terms, and textual triangulation gaps. These are complementary findings, not the components of `extract_score`.
2. `forgeplan_coverage_business` with the `DM-` id - record the baseline score.
3. Add the artifacts the composition expects, then link the scenarios.
4. Re-run and compare component values. Artifact counts, use-case scenario coverage, and hypothesis frontmatter states affect the score; glossary usage, invariant-to-scenario links, and textual triangulation references are not independently scored. Fixing those orphan findings can leave `extract_score` unchanged.
5. Keep `scenarios.coverage_rate` and `hypotheses.by_state` in the report: they expose gaps the score alone hides.

## Safety notes

- Read-only: the domain model, the record list and the relations are read as a snapshot under the workspace lock, which is released before the pure report builder runs. Nothing is written.
- No LLM call and no network access.
- `expected` is self-declared. A composition listing one term is satisfied by one term, so a small denominator can produce a high score cheaply - always read the component rates, not just `extract_score`.
- The score says nothing about whether the extracted artifacts are *correct*, or whether the terms and invariants are meaningful. Pair it with [`forgeplan_contradictions`](/docs/mcp/forgeplan_contradictions/) and [`forgeplan_orphans`](/docs/mcp/forgeplan_orphans/) before calling a domain model done.
- `forgeplan coverage` is a different tool: it reports decision coverage per code module. There is no CLI equivalent for this business-coverage score.

## See also

- [`forgeplan_orphans`](/docs/mcp/forgeplan_orphans/) — complementary four-bucket heuristic report, not a breakdown of this score.
- [`forgeplan_anomalies`](/docs/mcp/forgeplan_anomalies/) — unified findings with suggested resolution tiers.
- [`forgeplan_coverage`](/docs/mcp/forgeplan_coverage/) - decision coverage per code module, a different measure.
- [`forgeplan_discover_complete`](/docs/mcp/forgeplan_discover_complete/) - close a discovery session before scoring it.
- [Methodology guide](/docs/methodology/overview/) - where brownfield inspectors sit in the cycle.
