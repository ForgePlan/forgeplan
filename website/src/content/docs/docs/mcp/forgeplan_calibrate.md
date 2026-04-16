---
title: forgeplan_calibrate
description: "Suggest depth level (Tactical/Standard/Deep/Critical) for artifacts based on content analysis. Detects security sections, breaking changes, link count, body complexity."
---

Suggest an appropriate **depth** level (`Tactical` / `Standard` / `Deep` / `Critical`) for one or all artifacts, based on heuristic content analysis. Depth drives validator strictness (MUST rules per depth), ADI requirement, and review ceremony — so miscalibrated depth means either over-engineering a one-liner or shipping a critical change without adversarial review.

**Category**: Quality

## When an agent calls it

- **After creating a stub** — let calibration suggest the right depth before filling in sections.
- **During review** — sanity-check that the author chose a reasonable depth.
- **Audit sweep** — find artifacts marked `Tactical` that actually warrant `Deep` based on body content.
- **Onboarding agents** — teach them what factors push depth up (security, breaking changes, cross-team scope).

Heuristics considered: security sections present, breaking-change markers, outgoing link count, body length and section complexity.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | no | Artifact ID to calibrate. Omit to calibrate all artifacts in the workspace. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::CalibrateParams`_

## Returns

```json
{
  "suggestions": [
    {
      "id": "PRD-042",
      "current_depth": "tactical",
      "suggested_depth": "deep",
      "reasons": [
        "Security section present",
        "Breaking change detected (## Migration required)",
        "8 outgoing links"
      ]
    }
  ],
  "total_suggestions": 1
}
```

## Example invocation

```json
{ "id": "PRD-042" }
```

## Typical sequence

1. `forgeplan_calibrate` — scan the workspace.
2. For each mismatch: re-route the task with `forgeplan_route` or manually set depth in frontmatter.
3. `forgeplan_validate` — rerun validation under the new depth (stricter MUST rules).
4. `forgeplan_reason` — ADI is mandatory at Deep / Critical.

## CLI equivalent

```bash
forgeplan calibrate
forgeplan calibrate PRD-042
```

## See also

- [`forgeplan_route`](/docs/mcp/forgeplan_route/) — depth selection at task inception.
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) — depth-aware validation.
- [Methodology guide](/docs/methodology/overview/)
