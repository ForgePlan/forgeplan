---
depth: tactical
id: PROB-065
kind: problem
links:
- target: PROB-051
  relation: informs
- target: PROB-063
  relation: informs
- target: PROB-064
  relation: informs
status: draft
title: forgeplan_guard.target_phase enum semantically overlaps forgeplan_phase artifact phase enum
---

# PROB-065 — `forgeplan_guard.target_phase` enum semantically overlaps `forgeplan_phase` artifact phase enum

## Signal

W4 adversarial security audit (Wave 4A, v0.31.0 sprint, 2026-05-11) flagged that the MCP `forgeplan_guard` tool accepts a `target_phase: PhaseKind` argument whose enum (`idle | routing | shaping | coding | evidence | pr`) **lexically overlaps** the artifact lifecycle phase enum used by `forgeplan_phase_advance` (`shape | validate | adi | code | test | audit | evidence | done`). Both enums contain a valid `evidence` variant with different runtime meaning: in `forgeplan_guard` it means "the methodology session is in its evidence-collection phase", in `forgeplan_phase_advance` it means "the artifact has reached its evidence lifecycle stage". An agent (or operator) reading the JSON schema of one tool and applying the same input shape to the other passes type validation and gets a verdict against the wrong state object — silently.

The new W3 coverage test `c43_forgeplan_guard_smoke` (`crates/forgeplan-mcp/tests/integration_full_coverage.rs:720-724`) embeds a comment that reads:

> `forgeplan_guard`'s `target_phase` is the methodology-session phase enum (idle/routing/shaping/coding/evidence/pr), NOT the artifact phase enum from `forgeplan_phase` (shape/validate/adi/code/test/audit/evidence/done). Don't confuse them.

The comment is the admission of risk that motivates this PROB — guidance buried in a test file does not protect production agents.

## Context

- **Workspace state at detection**: `chore/v031-dependabot-bump` branch, dev-based sprint, no related artifacts in `draft`.
- **Reproducibility**: 100% deterministic — call `forgeplan_guard {"target_phase": "evidence"}` immediately after `forgeplan_new {"kind": "prd", "title": "X"}`. Guard responds against the session phase machine (likely returning `pass` if session has progressed past routing). Artifact is still in `shape` lifecycle phase. An agent that interpreted the guard `pass` as "artifact-phase = evidence is OK to proceed" will skip the Code → Test → Audit → Evidence ordering of the artifact lifecycle.
- **Code paths**:
  - `crates/forgeplan-mcp/src/server.rs:466-475` — `PhaseKind` enum (session)
  - `crates/forgeplan-mcp/src/server.rs:692-696` — `GuardParams.target_phase: PhaseKind`
  - `crates/forgeplan-mcp/src/server.rs:5285-5325` — `forgeplan_guard` impl, maps `PhaseKind` → `core::session::Phase`
  - `crates/forgeplan-core/src/phase/mod.rs::Phase` — artifact lifecycle phase enum (8 variants)
  - `crates/forgeplan-mcp/tests/integration_full_coverage.rs:720-724` — coverage test with the "don't confuse them" comment

## Root cause

Two unrelated state machines (methodology session vs artifact lifecycle) were named with the same conceptual word ("phase") and one accidentally-overlapping variant (`evidence`). The MCP tool argument schemas describe each parameter as `target_phase` without naming which phase machine it gates. Serde schema validation cannot distinguish "evidence (session)" from "evidence (artifact)" because they share spelling; both calls succeed and the consumer must mentally track which state object each tool acts on. This is a classic naming collision between two bounded contexts — methodology session machine (defined in `forgeplan-core/src/session`) and artifact phase machine (defined in `forgeplan-core/src/phase`) — surfaced through identically-named MCP parameters.

## Why now

Discovered during W4 adversarial security audit of v0.31.0 sprint accumulated changes (`dev..chore/v031-dependabot-bump`, +2038 LOC, focus on PROB-064 dual-key fix + coverage extensions). The "don't confuse them" comment was added by the W3 coverage worker writing `c43_forgeplan_guard_smoke` — they hit the footgun during test authoring and documented it inline rather than fixing the surface. Wave 4A flagged the deferred-fix pattern as MED-severity finding F-4 in the security audit report.

No prior PROB or ADR captures this — the methodology session machine landed in EPIC-005 / PRD-052 era, the artifact phase machine landed in PRD-051 / RFC-006 era, neither cross-referenced the other for naming collisions.

## Decision — proposed fix (Option A is the recommended path)

### Option A (recommended) — rename MCP-side argument and enum to make the bounded context explicit

1. Rename `GuardParams.target_phase` → `target_session_phase` (serde `#[serde(rename = "target_session_phase")]` to preserve JSON shape on the wire).
2. Rename `enum PhaseKind` → `enum SessionPhaseKind` in `server.rs:466-475`.
3. Update `forgeplan_guard` tool docstring (`#[tool(description = "...")]`) to explicitly state: *"This guards the **methodology session** phase machine (`idle → routing → shaping → coding → evidence → pr`). For the **artifact lifecycle** phase machine (`shape → validate → adi → code → test → audit → evidence → done`), use `forgeplan_phase_advance` / `forgeplan_phase`."*
4. Move the "don't confuse them" warning from `integration_full_coverage.rs:720-724` into the tool's user-facing schema description so agents reading the MCP catalog see it.
5. Mirror the rename in CLI: `forgeplan guard --target-phase` → `forgeplan guard --target-session-phase` (legacy `--target-phase` accepted with deprecation warning for one major version).

Breaking-change profile: MCP JSON arg name change (additive — keep legacy `target_phase` accepted via `#[serde(alias = "target_phase")]` with deprecation note); CLI flag breaking but mitigatable via alias. No code-data path changes.

### Option B (lighter — docs-only) — keep names, sharpen documentation

1. Add explicit "DO NOT confuse with `forgeplan_phase_advance.target` artifact-phase enum" in `forgeplan_guard` tool description.
2. Add a cross-reference link in the `forgeplan_phase_advance` description back to `forgeplan_guard` for the reverse direction.
3. Author a methodology doc page (`docs/methodology/PHASE-DISAMBIGUATION.ru.md`) covering both phase machines side-by-side with example flows.

Option B is faster but leaves the naming collision in place. Future agents that don't read docs first reproduce the footgun. Option A closes the class.

## Acceptance criteria

1. **A-1** — MCP `forgeplan_guard` tool description string explicitly names "session phase" and cross-references `forgeplan_phase_advance` (Option A or B).
2. **A-2** — MCP `forgeplan_phase_advance` tool description string explicitly names "artifact lifecycle phase" and cross-references `forgeplan_guard` (Option A or B).
3. **A-3** (Option A only) — JSON schema for `forgeplan_guard` arguments shows parameter name `target_session_phase` (or `target_phase` with description text disambiguating), with `target_phase` accepted as deprecated alias for ≥ one minor release.
4. **A-4** — Regression test asserts that the tool descriptions returned from MCP `tools/list` contain the disambiguation phrase "session phase" / "artifact lifecycle" verbatim.
5. **A-5** — `crates/forgeplan-mcp/tests/integration_full_coverage.rs:720-724` "don't confuse them" comment removed (now redundant — disambiguation lives in user-facing schema).

## Linked artifacts

- **Informs**: PROB-051 (CLI/MCP parity for `advisory_phase_mismatches`), PROB-063 (verdict aggregator clarity), PROB-064 (dual-key emission — same cross-surface-asymmetry class)
- **Related**: PRD-052 (methodology session phase machine), PRD-051 / RFC-006 (artifact phase lifecycle), EPIC-005 (advisory phase tracker)
- **Discovered by**: Wave 4A security audit, v0.31.0 sprint (2026-05-11), finding F-4 / MED-2

## References

- `crates/forgeplan-mcp/src/server.rs:466-475` (PhaseKind enum)
- `crates/forgeplan-mcp/src/server.rs:692-696` (GuardParams)
- `crates/forgeplan-mcp/src/server.rs:5285-5325` (forgeplan_guard impl)
- `crates/forgeplan-core/src/phase/mod.rs` (artifact Phase enum)
- `crates/forgeplan-core/src/session/` (methodology session phase machine)
- `crates/forgeplan-mcp/tests/integration_full_coverage.rs:720-724` (deferred-fix comment)
- W4 audit report: HIGH-1 (paired with this MED-2), see sprint-v031-cleanup teammate log




