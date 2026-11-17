---
title: forgeplan_contradictions
description: "Detect duplicate hypothesis titles using Jaccard ≥ 0.6. Returns contradictions and limitations; invariant conflicts, glossary divergence and scenario-versus-invariant checks remain deferred, with no separate result buckets."
---

Detect duplicate hypothesis titles using Jaccard similarity >= 0.6. The other three named classes require LLM judgement and remain unimplemented in this core handler. They are listed in `limitations`; there are no separate empty-array buckets for them.

Read the description literally before you trust the answer: **v1 detects exactly one class**. The response is honest about the other three, but honesty here means a `limitations` array, not a detection. Treating this tool as "contradiction detection" and assuming the other three names are covered is the one mistake this page exists to prevent.

**Category**: Other

## What it detects

### Covered in v1: `hypothesis_duplicate`

Two `HYP-` artifacts whose **titles** are lexically near-identical.

- Tokenisation retains ASCII letters/digits only, lowercases ASCII, and splits on other characters. Non-ASCII letters (including Cyrillic) are discarded, not morphologically normalised.
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

Installing a marketplace pack does not change this Rust handler's scope. An empty `contradictions` array means no title duplicates were emitted by this heuristic (or the cap skipped the pass), not that the workspace is internally consistent.

### One more limit: the DoS cap

The detector is O(N²) over hypotheses, so it short-circuits above `CONTRADICTIONS_MAX_HYPOTHESES = 1000` hypotheses: the pass is skipped entirely, `contradictions` comes back empty and `limitations` names the cap. In the unified anomaly view that cap is actionable - see below.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP `content[0].text` with two report keys plus `_next_action`. Illustrative values:

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

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

1. `forgeplan_contradictions` - collect duplicate-title pairs after a discovery ingest.
2. `forgeplan_get` both sides of each pair and decide: same claim (merge) or genuinely different (sharpen the wording).
3. Record the resolution and distinguish genuinely different titles. Superseding a hypothesis does not remove it from this detector: all hypothesis records are compared regardless of lifecycle status, so re-running can still report the pair.
4. Only then promote the surviving hypotheses with [`forgeplan_hypothesis_promote`](/docs/mcp/forgeplan_hypothesis_promote/) - promoting duplicates first means the same claim gets triangulated twice.

## Safety notes

- Read-only: the tool takes a snapshot of the records and never writes an artifact.
- The snapshot read happens under the workspace lock; the O(N²) comparison pass runs lock-free afterwards.
- Bounded work: at most 1000 hypotheses enter the pass, so a workspace stuffed with hypothesis stubs cannot stall the server.
- Lexical, not semantic: two differently-worded titles making the same claim will not be flagged, and two similar titles making different claims will be. The signal is a prompt to look, not a verdict.
- No LLM call and no network access.
- Workspace, lock, and snapshot-read failures return plain-text MCP tool errors (`isError: true`), not a successful empty report.
- There is no CLI equivalent - this is an MCP-only surface. The same findings reach the terminal through [`forgeplan anomalies`](/docs/cli/anomalies/) as the `hypothesis_duplicate` kind.

## See also

- [`forgeplan_anomalies`](/docs/mcp/forgeplan_anomalies/) - the canonical envelope, including this finding with its resolution tier.
- [`forgeplan_hypothesis_status`](/docs/mcp/forgeplan_hypothesis_status/) - lifecycle distribution across HYP- artifacts.
- [`forgeplan_hypothesis_promote`](/docs/mcp/forgeplan_hypothesis_promote/) - advance a hypothesis once duplicates are resolved.
- [`forgeplan_orphans`](/docs/mcp/forgeplan_orphans/) - the other brownfield inspector from the same phase.
- [Methodology guide](/docs/methodology/overview/) - where brownfield inspectors sit in the cycle.
