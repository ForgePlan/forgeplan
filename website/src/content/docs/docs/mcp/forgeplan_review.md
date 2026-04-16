---
title: forgeplan_review
description: "Review an artifact — run validation and show lifecycle checklist. Shows MUST/SHOULD findings and whether artifact can be activated."
---

Review a single artifact — runs validation (depth-aware MUST / SHOULD rules), checks lifecycle prerequisites (evidence, R_eff, linked relations), and returns a clear verdict on whether the artifact is ready to activate. This is the combined "can I activate?" gate used before calling `forgeplan_activate`.

**Category**: Quality

## When an agent calls it

- **Before activation** — confirm all gates pass to avoid activation errors.
- **PR review** — run on each touched artifact to catch missing sections or MUST failures.
- **Author self-check** — faster than running `validate` + `score` + manual lifecycle inspection.
- **Automated quality hooks** — pre-commit / CI can call this and fail the build if any artifact regresses.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to review. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ReviewParams`_

## Returns

```json
{
  "artifact_id": "PRD-042",
  "kind": "prd",
  "depth": "standard",
  "status": "draft",
  "validation": {
    "must_errors": [],
    "should_warnings": ["density < 50 words in section Goals"]
  },
  "lifecycle": {
    "r_eff": 0.72,
    "has_evidence": true,
    "ready_to_activate": true
  },
  "verdict": "PASS — ready to activate"
}
```

If blocked:

```json
{
  "verdict": "FAIL",
  "validation": { "must_errors": ["Missing section: Problem"] },
  "lifecycle": { "ready_to_activate": false }
}
```

## Example invocation

```json
{ "id": "PRD-042" }
```

## Typical sequence

1. `forgeplan_review` → if `FAIL`, fix issues.
2. `forgeplan_update` to patch the body.
3. `forgeplan_review` again → expect `PASS`.
4. `forgeplan_activate` — flip draft → active.

## CLI equivalent

```bash
forgeplan review PRD-042
```

## See also

- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) — validation only, without lifecycle check.
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) — R_eff recomputation only.
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) — the action gated by this review.
- [Methodology guide](/docs/methodology/overview/)
