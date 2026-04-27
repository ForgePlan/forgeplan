---
created: 2026-04-26
depth: standard
domain: general
id: PRD-070
kind: prd
links:
- target: PROB-045
  relation: based_on
priority: P1
projectType: cli_tool
status: draft
title: CLI parity for v0.21-v0.24 features — full first-class CLI commands for activity, undo-last, restore, phase, dispatch, claim, release
updated: 2026-04-26
---

# PRD-070: CLI parity for v0.21-v0.24 features

## Problem

10 features shipped in v0.21-v0.24 (PRD-055/056/057) only expose MCP tools — no CLI commands. Terminal users cannot:
- undo a delete or restore an artifact without spinning up Claude Code
- inspect the activity log
- read or advance phase state
- run multi-agent dispatch as a dry-run plan
- claim/release work locks

This breaks the project invariant "every MCP tool has a CLI counterpart". 47 of 55 MCP tools have CLI surface; these 10 are the only gap.

Full detail in PROB-045.

## Target Users

| Persona | Need |
|---|---|
| CLI power user | Wants `forgeplan undo-last` from terminal without launching Claude Code |
| CI / scripts | Need `forgeplan activity --json` and `forgeplan claims --json` for automation |
| Orchestrator agent | Already uses MCP — CLI is bonus, not blocker |
| Sub-agent | Already uses MCP — CLI is bonus |
| Course student | Tutorial uses CLI consistently — having to switch to MCP for these features breaks the flow |

## Goals

| ID | Criterion | Metric | Target |
|---|---|---|---|
| SC-1 | Command count | new CLI subcommands in `forgeplan --help` | +10 |
| SC-2 | Test coverage | integration tests for each new command | 10/10 |
| SC-3 | Logic duplication | new code in `forgeplan-cli` per command | < 100 LOC each (thin wrapper) |
| SC-4 | Build | `cargo build --release` exit 0 | clean |
| SC-5 | Lint | `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings |
| SC-6 | Format | `cargo fmt --check` | 0 diff |
| SC-7 | Docs regen | website CLI pages regenerated | +30 (EN+RU+overview) |

## Non-Goals

- Rewriting any MCP tool — they continue to work as-is
- Adding new business logic — CLI is a thin wrapper over existing core/mcp logic
- Touching CLI commands that already exist
- Refactoring command dispatch architecture — follow existing patterns
- Adding TUI / interactive mode

## Functional Requirements

| ID | Priority | Requirement |
|---|---|---|
| FR-001 | Must | User can run `forgeplan activity` to query the activity log with optional `--since-hours`, `--tool`, `--status`, `--limit`, `--json` filters |
| FR-002 | Must | User can run `forgeplan activity-stats` to see aggregate stats with optional `--since-hours` and `--json` |
| FR-003 | Must | User can run `forgeplan undo-last` to reverse the most recent destructive op with optional `--within-hours` and `--json` |
| FR-004 | Must | User can run `forgeplan restore <ID>` to restore a soft-deleted artifact with optional `--json` |
| FR-005 | Must | User can run `forgeplan phase <ID>` to read advisory phase state with optional `--json` |
| FR-006 | Must | User can run `forgeplan phase-advance <ID> --to <PHASE>` with optional `--reason` and `--json` |
| FR-007 | Must | User can run `forgeplan dispatch --agents N` with optional `--epic`, `--kind`, `--status`, `--overlap-threshold`, `--json` |
| FR-008 | Must | User can run `forgeplan claim <ID>` with optional `--agent`, `--ttl-minutes`, `--note`, `--json` |
| FR-009 | Must | User can run `forgeplan claims` to list active claims with optional `--json` |
| FR-010 | Must | User can run `forgeplan release <ID>` with optional `--agent`, `--force`, `--json` |
| FR-011 | Should | Each command's `--help` text matches the description from the corresponding MCP tool |
| FR-012 | Should | Text output mode is consistent with existing CLI commands (Forge tone, no emoji, table for lists) |
| FR-013 | Could | `forgeplan --help` groups new commands under existing categories (Multi-agent, History, Phase) |

## Technical Approach

**Pattern**: Each new CLI command follows the existing pattern from e.g. `forgeplan validate`:

1. Add variant to `Commands` enum in `crates/forgeplan-cli/src/main.rs`
2. Create handler module `crates/forgeplan-cli/src/commands/<cmd>.rs`
3. Handler calls existing core function (same one MCP tool calls)
4. Format output as text or JSON based on `--json` flag
5. Add integration test in `crates/forgeplan-cli/tests/cli_<cmd>.rs`

**No new core logic** — every MCP handler in `forgeplan-mcp/src/server.rs` already calls a core function. CLI handlers call the same core function and format output differently.

**Multi-agent dispatch** to parallelize: 3 agents claim 3-4 commands each via `forgeplan_dispatch` + `forgeplan_claim`.

## Dependencies

| Dependency | Type | Status |
|---|---|---|
| Forgeplan v0.24.0 binary | Internal | Released |
| Existing core functions (activity log, soft-delete, phase, dispatch) | Internal | Shipped |
| `clap` derive | External | Already used by all CLI |

## Risks

| Risk | Mitigation |
|---|---|
| Output format diverges between CLI text and JSON | JSON output mirrors MCP response shape; text is human-readable summary of same fields |
| Two agents touch the same handler module | `forgeplan_dispatch` Jaccard threshold + claim protocol — that's literally what we're testing |
| Integration tests flaky due to global state (claims dir, trash dir) | Each test uses `tempfile::TempDir` for isolated workspace |
| Help text drifts from MCP descriptions | FR-011: copy MCP tool description verbatim into clap doc comment |

## Acceptance Criteria

- All 10 commands produce identical JSON output to their MCP counterparts (verified by integration test)
- Text output is concise and copy-pasteable into a script
- `forgeplan --help` shows all new commands; `forgeplan <cmd> --help` shows full options
- Cargo test green, clippy green, fmt clean
- Website docs regenerated and CLI/MCP coverage stays at 100%

## Related Artifacts

| Artifact | Relation |
|---|---|
| PROB-045 | based_on |
| PRD-055 | informs (source of activity, undo, restore) |
| PRD-056 | informs (source of phase, phase-advance) |
| PRD-057 | informs (source of dispatch, claim, claims, release) |

