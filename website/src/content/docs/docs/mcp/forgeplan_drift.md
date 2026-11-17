---
title: forgeplan_drift
description: "Check for drifted decisions - affected files that changed after ADR/RFC was created."
---

Detect "drifted decisions" - architectural decisions (ADR / RFC) whose declared affected files have been modified since the decision was recorded. Drift is a strong signal that the implementation has diverged from the documented rationale, and the decision may need a refresh, supersede, or new ADR.

**Category**: Quality

## When an agent calls it

- **Quarterly audit** - find ADRs that no longer match the code.
- **Before claiming an ADR is authoritative** - verify the affected files haven't been rewritten.
- **Before a refactor** - check if your target files are governed by a drifted ADR, and update it first.
- **Release prep** - confirm no architectural decisions are silently stale.

The detector reads paths/patterns from Markdown body sections `Affected Files`, `Affected Scope`, or `affected_files`, not a frontmatter array. For **active ADR/RFC** records it runs `git log --since <created_at> -- <pattern>` under the workspace's project root, then reads the last commit date. It does not use filesystem mtime, a LanceDB `modified_at`, uncommitted changes, or semantic comparison. A reported path can be the pattern itself rather than an expanded file.

## Input parameters

_No declared input fields. Call this tool with an empty object `{}`._

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP `content[0].text`: `total` (decisions checked), `stale` (drifted count), `reports`, `_next_action`. Reports include both drifted and clean checked decisions. Each report has `artifact_id`, `artifact_title`, `created_at`, `affected_files` (strings), `changed_files` (`path`, `last_modified` objects), `is_stale`. Here `is_stale` means detected git changes, not expiry or a stored status.

Illustrative partial payload (`_next_action` omitted):

```json
{
  "total": 1,
  "stale": 1,
  "reports": [
    {
      "artifact_id": "ADR-004",
      "artifact_title": "Storage design",
      "created_at": "2026-02-15T00:00:00Z",
      "affected_files": ["src/storage.rs"],
      "changed_files": [{"path": "src/storage.rs", "last_modified": "2026-04-07 10:00:00 +0000"}],
      "is_stale": true
    }
  ]
}
```

This is a report only. Workspace errors return plain-text tool errors. Core drift errors are suppressed to empty reports; failed git commands yield no changed files, so an empty/clean report is not proof of no drift. Records without an affected-path section are not checked.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

1. `forgeplan_drift` - list drifted decisions.
2. `forgeplan_get` each ADR - read original rationale.
3. Inspect whether the committed change actually invalidates the rationale. Renewing `valid_until` does not reset `created_at`, so it does not clear this git-history signal.
4. If needed, create a replacement and supersede the old decision under the lifecycle rules. Re-run and inspect checked and drifted counts rather than assuming every remediation removes a row.

## CLI equivalent

```bash
forgeplan drift
```

## See also

- [`forgeplan_coverage`](/docs/mcp/forgeplan_coverage/) - inverse view: which modules lack decisions.
- [`forgeplan_stale`](/docs/mcp/forgeplan_stale/) - time-based (not file-based) staleness.
- [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) - replace a drifted ADR.
- [Methodology guide](/docs/methodology/overview/)
