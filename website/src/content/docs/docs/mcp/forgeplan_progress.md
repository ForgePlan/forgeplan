---
title: forgeplan_progress
description: "Show checkbox progress for artifacts. Parses markdown checkboxes (- [ ] / - [x]) and computes completion percentages."
---

Show checkbox-based completion progress for one or all artifacts. Parses standard Markdown task lists (`- [ ]` and `- [x]`) in the body, groups them by section (e.g. Implementation Phases, FR list), and returns per-group and overall percentages.

**Category**: Quality

## When an agent calls it

- **Session resume** — see how much of the PRD / RFC you've actually implemented.
- **Sprint check-in** — group-level breakdown shows which phase is lagging.
- **PR description generation** — quote the current completion % to communicate status.
- **After marking boxes** — confirm the parser saw your `- [x]` updates.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | no | Artifact ID. If omitted, shows progress for all artifacts that contain checkboxes. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ProgressParams`_

## Returns

```json
{
  "artifacts": [
    {
      "id": "RFC-006",
      "total": 18,
      "done": 12,
      "percent": 66.7,
      "groups": [
        { "section": "Phase 1 — Shape", "total": 5, "done": 5, "percent": 100 },
        { "section": "Phase 2 — Build", "total": 8, "done": 6, "percent": 75 },
        { "section": "Phase 3 — Evidence", "total": 5, "done": 1, "percent": 20 }
      ]
    }
  ]
}
```

## Example invocation

```json
{ "id": "RFC-006" }
```

Or for everything:

```json
{}
```

## Typical sequence

1. `forgeplan_progress` on a specific RFC — see where you left off.
2. Implement next unchecked item.
3. `forgeplan_update` to flip `- [ ]` → `- [x]`.
4. `forgeplan_progress` again — confirm the percent moved.

## CLI equivalent

```bash
forgeplan progress
forgeplan progress RFC-006
```

## See also

- [`forgeplan_estimate`](/docs/mcp/forgeplan_estimate/) — planned hours vs current progress.
- [`forgeplan_update`](/docs/mcp/forgeplan_update/) — mutate body to tick a box.
- [Methodology guide](/docs/methodology/overview/)
