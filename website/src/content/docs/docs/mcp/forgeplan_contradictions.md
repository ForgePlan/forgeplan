---
title: forgeplan_contradictions
description: "Issue #287 Phase C - detect cross-artifact contradictions (v1: hypothesis_duplicate via Jaccard ≥ 0.6 on titles). Three other classes (invariant_conflict, glossary_divergence, scenario_vs_invariant) require LLM judgement and are deferred - buckets included in response with empty arrays + limitations list."
---

Issue #287 Phase C - detect cross-artifact contradictions (v1: hypothesis_duplicate via Jaccard >= 0.6 on titles). Three other classes (invariant_conflict, glossary_divergence, scenario_vs_invariant) require LLM judgement and are deferred - buckets included in response with empty arrays + limitations list.

Read the description literally before you trust the answer: **v1 detects exactly one class**. The response is honest about the other three, but honesty here means a `limitations` array, not a detection. Treating this tool as "contradiction detection" and assuming the other three names are covered is the one mistake this page exists to prevent.

**Category**: Other

## What it detects

### Covered in v1: `hypothesis_duplicate`

Two `HYP-` artifacts whose **titles** are lexically near-identical.

- Tokenisation: lowercase the title, map every non-alphanumeric ASCII character to a space, split on whitespace. Punctuation and case are therefore irrelevant.
- Score: Jaccard similarity `|A ∩ B| / |A ∪ B|` over the two token sets.
- Threshold: `>= 0.6` (`HYPOTHESIS_DUPLICATE_JACCARD_THRESHOLD` in `crates/forgeplan-core/src/brownfield.rs`). Two empty or fully disjoint titles score `0.0` and are never flagged.
- Severity: `high` at `>= 0.85`, otherwise `medium`.
- Scope: titles only. Bodies, sections and artifacts of other kinds are not read.

### Not covered - deferred, needs an LLM

These three names exist in the report as constants and are listed in `limitations` on every call. They return nothing today:

| Class | Status | What it would need |
|---|---|---|
| `invariant_conflict` | deferred | LLM judgement over INV- bodies |
| `glossary_divergence` | deferred | LLM judgement over GLOS- definitions |
| `scenario_vs_invariant` | deferred | LLM judgement across SCEN- and INV- text |

They ship with the marketplace brownfield plugin (#79). Until then, an empty `contradictions` array means "no duplicate hypothesis titles", not "this workspace is internally consistent".

### One more limit: the DoS cap

The detector is O(N²) over hypotheses, so it short-circuits above `CONTRADICTIONS_MAX_HYPOTHESES = 1000` hypotheses: the pass is skipped entirely, `contradictions` comes back empty and `limitations` names the cap. In the unified anomaly view that cap is actionable - see below.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `workspace` | `string` | no | Resolve the workspace from this directory instead of the MCP server's startup CWD. Absolute or tilde-expanded. Useful when the agent runs in a git worktree separate from the server. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ContradictionsParams` (the shared DTOs live in `crates/forgeplan-mcp/src/types.rs`)_

There is no filter for severity, kind or artifact id - the tool always reports every duplicate pair it finds.

## Returns

Two top-level keys plus the protocol hint:

```json
{
  "contradictions": [
    {
      "left_artifact": "HYP-003",
      "right_artifact": "HYP-011",
      "kind_of_contradiction": "hypothesis_duplicate",
      "severity": "medium",
      "suggested_resolution": "Review titles — Jaccard similarity 0.67. Either merge into one hypothesis or sharpen distinguishing wording."
    }
  ],
  "limitations": [
    "invariant_conflict requires LLM judgement (v1 deferred)",
    "glossary_divergence requires LLM judgement (v1 deferred)",
    "scenario_vs_invariant requires LLM judgement (v1 deferred)"
  ],
  "_next_action": "Next: review 1 contradiction(s) — `forgeplan_get <left>` then `forgeplan_get <right>`."
}
```

Field notes (shapes taken from `Contradiction` and `ContradictionsReport`):

- `kind_of_contradiction` is a string, not an enum, so future classes can land without a wire-shape break. In v1 it is always `hypothesis_duplicate`.
- `severity` is `high` or `medium`; it hides the score rather than exposing it. When the same finding arrives through [`forgeplan_anomalies`](/docs/mcp/forgeplan_anomalies/) it is re-expressed as `jaccard_score` and a resolution tier.
- Pairs are emitted in scan order, each pair once and lower index first, so the report is deterministic for a fixed workspace.
- `limitations` always carries the three deferred classes. The cap entry appears only when the cap fired.
- `_next_action` is either `Done. - no contradictions detected.` or a count plus the review instruction.

## Example invocation

Whole workspace:

```json
{}
```

Against a worktree:

```json
{ "workspace": "~/work/forgeplan-feature-branch" }
```

## Typical sequence

1. `forgeplan_contradictions` - collect duplicate-title pairs after a discovery ingest.
2. `forgeplan_get` both sides of each pair and decide: same claim (merge) or genuinely different (sharpen the wording).
3. Merge by superseding the redundant hypothesis, then re-run to confirm the pair is gone.
4. Only then promote the surviving hypotheses with [`forgeplan_hypothesis_promote`](/docs/mcp/forgeplan_hypothesis_promote/) - promoting duplicates first means the same claim gets triangulated twice.

## Safety notes

- Read-only: the tool takes a snapshot of the records and never writes an artifact.
- The snapshot read happens under the workspace lock; the O(N²) comparison pass runs lock-free afterwards.
- Bounded work: at most 1000 hypotheses enter the pass, so a workspace stuffed with hypothesis stubs cannot stall the server.
- Lexical, not semantic: two differently-worded titles making the same claim will not be flagged, and two similar titles making different claims will be. The signal is a prompt to look, not a verdict.
- No LLM call and no network access.
- There is no CLI equivalent - this is an MCP-only surface. The same findings reach the terminal through [`forgeplan anomalies`](/docs/cli/anomalies/) as the `hypothesis_duplicate` kind.

## See also

- [`forgeplan_anomalies`](/docs/mcp/forgeplan_anomalies/) - the canonical envelope, including this finding with its resolution tier.
- [`forgeplan_hypothesis_status`](/docs/mcp/forgeplan_hypothesis_status/) - lifecycle distribution across HYP- artifacts.
- [`forgeplan_hypothesis_promote`](/docs/mcp/forgeplan_hypothesis_promote/) - advance a hypothesis once duplicates are resolved.
- [`forgeplan_orphans`](/docs/mcp/forgeplan_orphans/) - the other brownfield inspector from the same phase.
- [Methodology guide](/docs/methodology/overview/) - where brownfield inspectors sit in the cycle.
