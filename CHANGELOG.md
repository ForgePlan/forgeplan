# Changelog

All notable changes to Forgeplan are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/). Semver: `MAJOR.MINOR.PATCH`
with pre-1.0 minor bumps for breaking changes.

This file starts at v0.17.0. For prior releases, see git tags and the
corresponding sprint evidence under `.forgeplan/evidence/`.

## [0.17.0] — 2026-04-08 — EPIC-003: Search, Discovery, Intelligence

First release of EPIC-003. Adds keyword + semantic search, brownfield
discovery, scoring/routing intelligence, FPF rule surface, methodology
integrity gates, and reusable sprint checklist framework.

### Highlights

- **1109 tests passing** (+280 from v0.16.0), zero failures, zero warnings on
  both default and `--features semantic-search` builds
- **7 PRDs shipped** across 8 sprints (13.0 → 13.7 + post-closeout hotfix)
- **FPF Knowledge Base gains semantic vector search** via BGE-M3 embeddings
- **Methodology integrity gates** catch stub artifacts, duplicates, orphans
- **Sprint checklist framework** (NOTE-044) to prevent regression in future
  releases

### Added

**Smart Search v2** — PRD-039, Sprint 13.2
- BM25 ranking replaces substring scoring in `forgeplan search`
- Composable filter DSL (`--status`, `--depth`, `--since`, `--with-evidence`)
- 1-hop graph neighbor expansion (opt-out via `--no-expand`)
- Extended MCP `search` tool parameters

**Brownfield Discovery** — PRD-035, Sprints 13.3 + 13.4
- Tags system in frontmatter + LanceDB schema (v3→v4 migration)
- `forgeplan tag` / `untag` commands + `list --tag key=value` filter
- SourceTier → Congruence Level mapping (T1→CL3, T2→CL2, T3→CL1)
- `forgeplan discover` CLI command (session state machine)
- MCP tools: `forgeplan_discover_start`, `_scan`, `_next`, `_status`

**Scoring & Routing Intelligence** — PRD-040, Sprint 13.5
- Routing Skills Memory with exponential decay (90-day half-life)
- R_eff confidence intervals heuristic (widens with sparse/stale evidence)
- `forgeplan score` displays `[low — high]` interval alongside point estimate

**FPF Rules Surface** — PRD-041, Sprint 13.6
- `forgeplan fpf rules` — action-grouped tree (EXPLORE/INVESTIGATE/EXPLOIT)
  with `--flat` and `--json` modes
- `forgeplan fpf check <id>` — per-artifact rule match introspection
  with `--verbose` (unmatched list) and `--json` (canonical shape)
- MCP tools: `forgeplan_fpf_rules` (with `action`/`name`/`summary`/`source`
  filters) and `forgeplan_fpf_check`

**FPF KB Vector Search** — PRD-042, Sprint 13.7 (supersedes PRD-018)
- `embedding` column (`FixedSizeList<Float32, 1024>`) added to `fpf_spec`
  table, backward-compatible migration via `NewColumnTransform::AllNulls`
- `LanceStore::search_fpf_by_vector(query_vec, limit)` using LanceDB native
  `vector_search` with `DistanceType::Cosine`
- `forgeplan fpf search <query> --semantic` CLI flag
- MCP `forgeplan_fpf_search` gains `semantic: Option<bool>` param
- **Two-layer graceful fallback** — compile-time (feature off) + runtime
  (Embedder init fail / encode fail / vector search fail) → warning +
  keyword fallback
- NaN/Inf rejection at `insert_fpf_chunks` boundary
- Runtime `Embedder::dim() == EMBEDDING_DIM` assertion

**Methodology Integrity** — PRD-043, Sprint 13.1
- Duplicate guard (`forgeplan new` detects existing similar artifacts)
- Stub detection (blocks `activate` on unfilled templates)
- Health detection (`forgeplan health --ci` exits non-zero on blind spots)
- MCP warning envelope for methodology violations
- State machine: `Phase` enum with `validate_transition` enforcing
  Idle → Routing → Shaping → Coding → Evidence → PR for Standard+ depth

**Sprint Checklist Framework** — NOTE-044 (post-closeout deliverable)
- Reusable quality gate for every future sprint, 7 phases with red flags
- Encodes lessons from Sprint 13.7 retrospective
- Explicit "what not to skip" checklist for planning / implementation /
  audit / fixer / re-audit / manual UX / closeout / meta phases

### Changed

- **FPF KB schema**: backward-compatible migration adds `embedding` column
  (nullable). Existing workspaces work unchanged; re-ingest to populate
  embeddings.
- **MCP tool registry expanded** from ~37 to ~47 tools
- **CI linter**: `forgeplan health --ci` + `validate --ci` land (Sprint 11.3)
- **FpfStorage trait extended** — `insert_fpf_chunks` now accepts optional
  embeddings; `search_fpf_by_vector` added to trait (no default impl,
  forcing explicit backend choice per Sprint 13.7 hotfix re-audit)
- **CLI `fpf search` input validation** — empty / oversized (>8192 chars)
  queries rejected before store access
- **MCP param length bounds** on `forgeplan_fpf_search` and
  `forgeplan_fpf_rules` (id ≤128, name ≤128, action ≤64, source ≤16)
- **ANSI strip** on user-supplied query echoed in error messages
  (`No FPF sections match '{}'` sanitized against control chars)

### Deprecated / Superseded

- **PRD-018 "FPF Knowledge Base — semantic search"** — superseded by PRD-042.
  PRD-018 was a false-active stub with R_eff=1.0 but no real implementation,
  flagged by Sprint 13.1 methodology integrity work. PRD-042 closes the gap
  with actual BGE-M3 integration + supersedes PRD-018 to terminal state.

### Fixed

- **Sprint 13.1.5 hardening**: LazyLock<Regex> for `check_stub`, typed
  `StubReport` return, `forgeplan import` gate for active stubs (security
  bypass closed), configurable `IntegrityConfig` MCP limits
- **Sprint 13.1.7 integrity config wiring**: `IntegrityConfig::validate()`
  now called by CLI command path; `forgeplan health` no longer crashes on
  minimal configs via `#[serde(default)]` on top-level `Config` fields
- **Sprint 13.6 FPF Rules canonical JSON**: CLI and MCP now emit identical
  `{artifact_id, kind, status, matched, unmatched, winning, summary}` shape
  via typed `RuleCheckResult`, replacing hand-rolled `serde_json::json!`
- **Sprint 13.7 post-closeout hotfix** (PR #156):
  - `FpfStorage::search_fpf_by_vector` added to trait (closes asymmetry)
  - MCP handler integration harness at `crates/forgeplan-mcp/tests/`
  - Real BGE-M3 end-to-end test (`#[ignore]`, feature-gated)
  - Real v3 workspace migration test
  - Runtime dim assert + `fpf_spec_schema` rustdoc tying 1024 → BGE-M3
  - `InMemoryStore::search_fpf_by_vector` returns `Err` (not silent empty)
  - Wave 2 completer work re-audited (was originally skipped)

### Stats

- 1109 tests passing (+280 from v0.16.0)
- Core crate: 897 tests; CLI: 99 + 40 integration; MCP: 15 unit + 7 handler
- 42 MB release binary (strip + lto + opt-level=z)
- ~56 CLI commands, ~47 MCP tools
- 7 new PRDs activated, 1 superseded (PRD-018 → PRD-042)
- Sprint retrospective: 19 debts found, 11 fixed in hotfix, 8 backlog
  (NOTE-045), 6 process lessons (NOTE-044)

### Methodology lessons captured

- **Dependent sprint branch base verification** — new CLAUDE.md section
  covering the Sprint 13.1.5 rebase hell that taught us to verify parent
  branches contain expected commits before spawning teammates
- **Sprint Checklist Framework (NOTE-044)** — reusable 7-phase gate to
  prevent planning gaps (was: "user had to ask 'what did we miss'")
- **Sprint 13.7 Deferred Debts (NOTE-045)** — backlog tracking for the
  8 non-blocking items that rolled forward from the retrospective

### Related PRs
PRs #141 → #156. See `git log main..release/v0.17.0` for full list.

[0.17.0]: https://github.com/ForgePlan/forgeplan/releases/tag/v0.17.0
