---
depth: tactical
id: PROB-077
kind: problem
status: deprecated
title: 'MCP forgeplan_update and discover_finding write literal @filepath as body — silent data loss (GH #350)'
---

## Signal

GitHub issue #350 (2026-05-27, reporter explosivebit), confirmed on two independent sessions (user's `gerts-hub` project + a marketplace sandbox repro). MCP `forgeplan_update` called with `body="@/path/to/file.md"` wrote the **literal string** `@/path/to/file.md` into the artifact body instead of the file's content — silently overwriting whatever was there. The call returned `Updated successfully`; the loss was invisible until a later `forgeplan_get`.

## Root Cause

The CLI (`forgeplan-cli/src/commands/update.rs`) expands a leading `@` into file content; the MCP `forgeplan_update` and `forgeplan_discover_finding` handlers did **not** — they persisted `p.body` verbatim. A CLI/MCP asymmetry: skills document the `--body @file` pattern, agents mirror it through MCP, and the MCP side wrote the path string.

## Impact

CRITICAL silent data loss. Any agent mirroring the documented CLI `@file` pattern through MCP destroys the target artifact's body. Worst failure mode — `Updated successfully` + invisible loss until someone re-reads the artifact.

## Resolution

Option (a) symmetry (the issue-preferred outcome): add an `expand_body_filepath()` helper to the MCP server — a leading `@` reads the file (CLI parity: strips YAML frontmatter), the DoS body-length cap applies to the **expanded** content, and a read error surfaces as `invalid_params` (loud, not silent) via the `$HOME`-sanitizing error path. Applied in both handlers (`forgeplan_update` + `forgeplan_discover_finding`). Fixed on branch `fix/issue-350-mcp-update-filepath`; evidence EVID-142.

## Related

- GitHub #350 (source signal)
- Adjacent CLI/MCP asymmetry class: #353 (claim `--agent` `/` handling)




