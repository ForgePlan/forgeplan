---
title: forgeplan_coverage_business
description: "Issue #287 Phase C - compute extract_score (0.0-1.0) for a Domain Model. Reads the DM's ## Composition section to derive expected counts of glossary / use_cases / invariants / scenarios / hypotheses; counts actuals across the workspace; returns weighted extract_score. Canonical fields (DDL/SDL/pseudo-code) default to 0/false until marketplace#79 Phase E lands."
---

Issue #287 Phase C - compute extract_score (0.0-1.0) for a Domain Model. Reads the DM's `## Composition` section to derive expected counts of glossary / use_cases / invariants / scenarios / hypotheses; counts actuals across the workspace; returns weighted extract_score. Canonical fields (DDL/SDL/pseudo-code) default to 0/false until marketplace#79 Phase E lands.

One call answers "how much of this domain model has actually been extracted into artifacts". It is a coverage ratio, not a correctness measure - a workspace can score 1.0 and still be wrong, and it can score 0.4 legitimately while the model is still being discovered.

**Category**: Quality & Validation

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `domain_model_id` | `string` | yes | The `DM-` artifact to measure. Must resolve to an artifact of kind `domain_model`. |
| `workspace` | `string` | no | Resolve the workspace from this directory instead of the MCP server's startup CWD. Absolute or tilde-expanded. Useful when the agent runs in a git worktree separate from the server. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::CoverageBusinessParams` (the shared DTOs live in `crates/forgeplan-mcp/src/types.rs`)_

Two failure modes are worth knowing, because both are returned as MCP errors with a fix hint rather than empty data:

- The id resolves to a different kind - the response says which kind it found and tells you to pass a `DM-` id.
- The id does not exist - the response is an artifact-not-found error.

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

```json
{
  "glossary": { "count": 12, "expected": 14, "rate": 0.857 },
  "use_cases": { "count": 9, "expected": 12, "rate": 0.75 },
  "invariants": { "count": 7, "expected": 9, "rate": 0.778 },
  "scenarios": { "per_use_case_avg": 1.44, "coverage_rate": 0.667 },
  "hypotheses": {
    "by_state": {
      "parked": 1,
      "inferred": 4,
      "strong_inferred": 2,
      "verified": 3,
      "refuted": 1
    },
    "total": 11
  },
  "canonical": {
    "ddl_compile": false,
    "sdl_parse": false,
    "pseudo_code_coherence": 0.0,
    "reproducibility": 0.0
  },
  "extract_score": 0.71,
  "_next_action": "Next: extract_score = 0.71. Improve coverage by adding artifacts referenced in DM-001's Composition section."
}
```

Field notes:

- `canonical` is a placeholder block in v1. All four fields return their defaults (`false`, `false`, `0.0`, `0.0`) until the marketplace brownfield plugin (#79, Phase E) populates the DDL compile, SDL parse, pseudo-code coherence and reproducibility checks. Do not read them as results.
- `hypotheses.by_state` keys are the five lifecycle states in their field spelling (`parked`, `inferred`, `strong_inferred`, `verified`, `refuted`); each value is parsed from a hypothesis's `## Lifecycle State` section, where the canonical form is kebab-case (`strong-inferred`). A body whose state cannot be parsed counts as `parked`.
- `glossary`, `use_cases` and `invariants` share the same triple shape: `count`, `expected`, `rate`.
- `_next_action` always states the score and names the composition section as the place to add artifacts.

## Example invocation

Score a domain model in the default workspace:

```json
{ "domain_model_id": "DM-001" }
```

Score a domain model in a worktree rather than the server's startup CWD:

```json
{ "domain_model_id": "DM-002", "workspace": "~/work/forgeplan-feature-branch" }
```

## When an agent calls this

- **After `forgeplan_discover_complete`** - turn a finished discovery session into one number for the report.
- **Before declaring a domain model extracted** - the score is the honest answer to "is anything missing".
- **While draining the orphan buckets** - re-run after linking scenarios or triangulating hypotheses to see the score move.
- **In a dashboard or CI trend** - the same workspace yields a deterministic score, so deltas are meaningful.

## Typical sequence

1. `forgeplan_orphans` - the four buckets tell you *which* artifacts are missing; this tool tells you *how much*.
2. `forgeplan_coverage_business` with the `DM-` id - record the baseline score.
3. Add the artifacts the composition expects, then link the scenarios.
4. Re-run and compare - `extract_score` should move, and the per-component `rate` tells you which component moved.
5. Keep `scenarios.coverage_rate` and `hypotheses.by_state` in the report: they expose gaps the score alone hides.

## Safety notes

- Read-only: the domain model, the record list and the relations are read as a snapshot under the workspace lock, which is released before the pure report builder runs. Nothing is written.
- No LLM call and no network access.
- `expected` is self-declared. A composition listing one term is satisfied by one term, so a small denominator can produce a high score cheaply - always read the component rates, not just `extract_score`.
- The score says nothing about whether the extracted artifacts are *correct*, or whether the terms and invariants are meaningful. Pair it with [`forgeplan_contradictions`](/docs/mcp/forgeplan_contradictions/) and [`forgeplan_orphans`](/docs/mcp/forgeplan_orphans/) before calling a domain model done.
- `forgeplan coverage` is a different tool: it reports decision coverage per code module. There is no CLI equivalent for this business-coverage score.

## See also

- [`forgeplan_orphans`](/docs/mcp/forgeplan_orphans/) - the four buckets behind this score.
- [`forgeplan_anomalies`](/docs/mcp/forgeplan_anomalies/) - the same gaps as actionable anomalies.
- [`forgeplan_coverage`](/docs/mcp/forgeplan_coverage/) - decision coverage per code module, a different measure.
- [`forgeplan_discover_complete`](/docs/mcp/forgeplan_discover_complete/) - close a discovery session before scoring it.
- [Methodology guide](/docs/methodology/overview/) - where brownfield inspectors sit in the cycle.
