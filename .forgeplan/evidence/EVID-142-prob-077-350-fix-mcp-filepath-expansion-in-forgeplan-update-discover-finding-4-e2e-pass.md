---
depth: tactical
id: EVID-142
kind: evidence
links:
- target: PROB-077
  relation: informs
status: draft
title: 'PROB-077/#350 fix: MCP @filepath expansion in forgeplan_update + discover_finding — 4 e2e PASS'
---

## Summary

Fix evidence for **PROB-077 / GitHub #350** — MCP `@filepath` silent data loss. The MCP `forgeplan_update` and `forgeplan_discover_finding` tools now expand a `@/path` body into the file's content (CLI parity) instead of persisting the literal `@path` string.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

CL3: direct e2e measurement of the exact shipped fix on the real MCP JSON-RPC surface (the same surface the bug was reported on).

## Fix

`expand_body_filepath()` helper added to `crates/forgeplan-mcp/src/server.rs`:
- leading `@` → read the file (CLI parity: strips YAML frontmatter when present);
- anything else → returned verbatim.

Applied in both `body`-accepting handlers — `forgeplan_update` and `forgeplan_discover_finding`. The DoS body-length cap (`mcp_max_body_len`) now measures the **expanded** content, so a short `@path` cannot smuggle an oversized file past the limit. A read failure routes through `safe_invalid_params` (no `$HOME`/absolute-path leak) and surfaces as `invalid_params` — loud, never silent.

## Tests

4 e2e tests in `crates/forgeplan-mcp/tests/mcp_update_filepath_e2e.rs` (real JSON-RPC via McpFixture):

| Test | Asserts |
|------|---------|
| `mcp_update_at_filepath_expands_into_body` | `@file` → body is file CONTENT, not the `@path` string (the #350 repro) |
| `mcp_update_at_filepath_strips_frontmatter` | YAML frontmatter stripped from a `@file` body |
| `mcp_update_at_nonexistent_file_errors_not_silent` | missing `@file` → `invalid_params` error, NOT a silent literal write |
| `mcp_update_literal_body_without_at_is_verbatim` | plain body (incl. inline `a@b.com`) stored verbatim |

`forgeplan_discover_finding` uses the same `expand_body_filepath` helper → covered transitively.

Gate: forgeplan-mcp **259 passed / 0 failed**, clippy 0 (`--all-targets`), fmt 0.

## Provenance

- Branch: `fix/issue-350-mcp-update-filepath` (off `origin/dev`)
- Fix commit: `0e3d8f6`
- Files: `crates/forgeplan-mcp/src/server.rs` (helper + 2 handler call-sites), `crates/forgeplan-mcp/tests/mcp_update_filepath_e2e.rs` (new)


