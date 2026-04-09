---
depth: standard
id: PRD-045
kind: prd
links:
- target: PROB-029
  relation: based_on
- target: PRD-043
  relation: informs
status: active
title: Health verdict aggregator reads all warning signals (v0.17.1 hotfix)
---

# PRD-045: Health verdict aggregator — read ALL warning signals

## Progress

```
FR-001   ████████████████████████  1/1  Verdict aggregator reads PRD-043 signals (stubs+dups) ✓ v0.17.1
FR-002   ░░░░░░░░░░░░░░░░░░░░░░░░  0/1  Three-level verdict enum — deferred to v0.18
FR-003   ████████████████████████  1/1  Next-actions include stubs/dups with concrete IDs     ✓ v0.17.1
FR-004   ████████████████████████  1/1  CHANGELOG v0.17.1 Fixed entry                         ✓ v0.17.1
─────────────────────────────────────────────────
TOTAL                               3/4  ( 75%) — v0.17.1 hotfix

Deferred from this PRD:
- FR-002: explicit Verdict enum (Healthy / NeedsAttention / Unhealthy).
  v0.17.1 keeps the existing "next_actions is non-empty" implicit
  signal — no "looks healthy" string when stubs/dups present. Making
  it explicit via typed enum is cleaner but requires HealthReport
  shape change which would break MCP JSON contract. Deferred to
  v0.18 minor release. See NOTE-045 entry.
```

**v0.17.1 delivered (commit b6f478e):**
- `generate_next_actions` signature extended with 2 new params
- Compute reordered: stubs/dups before next_actions
- Stubs action includes concrete ID with supersede/deprecate recipe
- Duplicates action includes concrete pair with deprecate recipe
- 3 new unit tests covering stub/dup/healthy branches
- Verified on dogfood: health now reports 3 concrete actions instead
  of "Project looks healthy"
- See EVID-067 for implementation evidence.

## Problem

`forgeplan health` currently prints a verdict line that contradicts its
own warnings. Observed on dogfood workspace 2026-04-08 during v0.17.0
final audit:

```
  duplicate pairs (5):
    EVID-001 duplicates EVID-003 (100%)
    EVID-002 duplicates EVID-004 (100%)
    ...

  Active stubs (8):
    PRD-008 ... 6 markers
    PRD-009 ... 6 markers
    ...

  Next actions:
    1. Project looks healthy. Continue implementation.

  Project looks healthy!
```

The verdict aggregator was written before Sprint 13.1 PRD-043 added
stub detection and duplicate detection. When those signals were wired
into display, the verdict roll-up was not updated to include them.
This is a classic "feature added in detection but forgotten in summary
aggregator" bug. The detection path prints warnings correctly; the
verdict path reads only the older `orphans` and `blind_spots` signals
and misses the Sprint 13.1 additions.

Impact: users trust the verdict line and ignore real warnings sitting
above it. CI gates pass when they should fail if `--fail-on stubs` is
not explicitly set. AI agents calling `forgeplan_health` through MCP
receive contradictory state and make wrong decisions downstream.

## Goals

- Verdict line reflects every displayed warning category
- Three-level gradient verdict (healthy / needs attention / unhealthy)
  rather than binary so light warnings do not equal heavy warnings
- Next actions list non-empty whenever verdict is not healthy
- Existing CI gates (`--fail-on`) continue working without changes
- JSON output for `health --json` gains a verdict field without
  breaking existing fields

## Non-Goals

- Changing exit codes of `--ci` mode without explicit user threshold
- Adding new warning categories (this PRD is only plumbing, not new
  detection)
- Telemetry, metrics upload, or persistent health history
- Configurable verdict thresholds (hard-coded for v0.17.1; can be
  revisited in a future PRD if users request)

## Target Users

- CLI users running `forgeplan health` interactively and reading the
  verdict line to decide whether to act
- AI agents calling the `forgeplan_health` MCP tool and parsing the
  verdict to decide next tool invocation
- CI pipelines using `forgeplan health --ci --fail-on` to gate merges
  on workspace health

## User Journeys

### Journey 1 — Dogfood maintainer running health

1. Run `forgeplan health` on workspace with 8 active stubs + 5 duplicate pairs
2. Output shows all warnings as today
3. Verdict line at bottom: `Verdict: needs attention — 8 stubs, 5 duplicate pairs`
4. Next actions section lists concrete commands per warning type

### Journey 2 — CI pipeline with thresholds

1. CI runs `forgeplan health --ci --fail-on stubs=0`
2. Workspace has 1 stub
3. Exit code 1; stderr includes `Workspace has 1 active stub which exceeds threshold 0`
4. Verdict line: `Verdict: unhealthy — 1 stub exceeds --fail-on threshold`

### Journey 3 — Clean workspace

1. Run `forgeplan health` on fresh workspace with no warnings
2. Verdict line: `Verdict: healthy — no warnings`
3. Next actions: `No action needed.`

## Functional Requirements

- **FR-001** — Health verdict aggregator reads the active-stubs count,
  duplicate-pairs count, orphans count, blind-spots count, and stale
  count signals, rolling them into a single verdict level. All five
  signals must contribute.
- **FR-002** — Three verdict levels:
  - `healthy` — all signal counts are zero
  - `needs attention` — any signal count greater than zero but within
    default warning thresholds (stubs under five, duplicates under
    three, orphans under three, stale under twenty percent of active)
  - `unhealthy` — any signal count exceeds the default critical
    threshold OR any explicit user `--fail-on` threshold is breached
- **FR-003** — Next actions list is generated from the non-zero signals.
  One actionable command per warning category, with placeholders filled
  from the actual offending IDs. Examples:
  - stubs: suggest `forgeplan deprecate ID reason` or
    `forgeplan supersede ID by NEW`
  - duplicates: suggest `forgeplan deprecate ID reason duplicate of OTHER`
  - orphans: suggest `forgeplan link ID OTHER relation informs`
  - stale: suggest `forgeplan renew ID until DATE` or
    `forgeplan refresh`
- **FR-004** — Help text, CHANGELOG, and methodology guide updated per
  NOTE-044 rule. The `forgeplan health help` output describes the three
  verdict levels. CHANGELOG has an entry under v0.17.1 Fixed section.
  CLAUDE.md workflow mentions the new verdict semantics.

## Non-Functional Requirements

- **NFR-001 Backward compat** — `--ci` mode exit codes remain unchanged
  unless the user explicitly sets stricter `--fail-on` thresholds. CI
  pipelines passing on v0.17.0 must continue passing on v0.17.1 with
  the same config.
- **NFR-002 Output parseable** — JSON mode adds a new verdict field but
  retains all existing fields and their types unchanged. Downstream
  tools parsing `health --json` should not break.
- **NFR-003 Deterministic** — Same workspace state always produces the
  same verdict. No randomness, no mtime dependence, no network calls.

## Design note — three-level verdict

One design question I resolved without user input: how many verdict
levels to expose. Chose three (healthy, needs attention, unhealthy)
rather than binary (healthy vs unhealthy).

Reasoning: binary would over-alarm on normal workspaces where one stub
should not equal fifty stubs in severity. Three levels match common
traffic-light UX patterns and let CI gates progressively escalate via
`--fail-on` thresholds. Thresholds between attention and unhealthy are
hard-coded for v0.17.1 to avoid shipping a configuration surface in a
patch release; making them configurable can happen in a future PRD if
real usage shows the defaults are wrong.

## Acceptance Criteria

- FR-001: verdict function signature takes all five signal counts as
  input parameters; test coverage exercises each parameter independently.
- FR-002: unit test — zero signals produces `healthy` verdict; one stub
  produces `needs attention`; ten stubs produces `unhealthy`.
- FR-003: next actions list is empty only when verdict is healthy.
  Otherwise contains at least one actionable command per warning type.
  Commands use concrete IDs from the actual warning, not placeholders.
- FR-004: `forgeplan health help` text describes the three levels.
  CHANGELOG v0.17.1 has a Fixed entry for this bug. CLAUDE.md workflow
  section mentions the new verdict semantics.
- Integration test: create workspace with one stub PRD and one duplicate
  pair, run health, assert verdict contains `needs attention` and next
  actions include concrete remediation commands.
- Unit test: verdict aggregator function exhaustively exercised with
  every combination of signal presence.
- E2E: release binary on dogfood workspace prints `Verdict: needs
  attention` given the existing 8 stubs + 5 duplicates.

## Affected Files

- `crates/forgeplan-core/src/health/mod.rs` — verdict function
- `crates/forgeplan-cli/src/commands/health.rs` — display + CI glue
- `CHANGELOG.md` — v0.17.1 Fixed entry
- `CLAUDE.md` — health workflow mention
- `crates/forgeplan-cli/tests/health_test.rs` — integration tests

## Related

| Artifact | Relation |
|---|---|
| PROB-029 | based_on (this closes PROB-029) |
| PRD-043 | informs (PRD-043 detection whose signals this aggregates) |
| EVID-058 | informs (Sprint 13.1 PRD-043 implementation evidence) |
| PRD-044 | sibling (paired v0.17.1 hotfix work) |

