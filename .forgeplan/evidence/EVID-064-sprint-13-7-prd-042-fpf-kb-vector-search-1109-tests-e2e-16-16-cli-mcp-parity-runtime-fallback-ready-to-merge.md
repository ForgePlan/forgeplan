---
depth: tactical
id: EVID-064
kind: evidence
links:
- target: PRD-042
  relation: informs
- target: EPIC-003
  relation: informs
status: active
title: Sprint 13.7 PRD-042 FPF KB Vector Search — 1109 tests, E2E 16/16, CLI+MCP parity, runtime fallback, READY TO MERGE
---

# EVID-064: Sprint 13.7 PRD-042 Implementation Evidence

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Summary

Sprint 13.7 implemented PRD-042 FPF Knowledge Base Vector Search in full across both CLI and MCP surfaces. All 3 FRs shipped on branch `feat/sprint-13.7-prd-042-kb-vector-search` over `release/v0.17.0`. Full /forge-cycle executed with multi-agent team: 2 parallel implementers (W1+W2) → 1 MCP parity specialist → 4 parallel auditors (Rust/Sec/Arch/Tests) → 1 fixer (11 fixes) → 1 completer (wave 2: 4 items) → team-lead manual UX verification → closeout. Supersedes PRD-018 (false-active stub from Sprint 12).

## Commits (5)

| Commit | Scope | LOC |
|---|---|---|
| `1384fcb` | W1 core: schema embedding column + migrate_fpf_spec + insert_fpf_chunks embeddings + search_fpf_by_vector via LanceDB native vector_search with Cosine distance | +382/-6 |
| `ef932ec` | W2 CLI: --semantic flag + ingest embedding pipeline feature-gated + run_search dispatch | +219/-6 |
| `ce8bc8d` | MCP parity: forgeplan_fpf_search with semantic: Option<bool> param + graceful fallback closing Arch L5 | +136/-21 |
| `9bf382d` | Audit fixes: 11 fixes — FpfStorage trait extension (Arch H1), critical test gaps (Tests C1-C3), length mismatch + mixed-dim tests, CLI input validation + ANSI strip, rustdoc feature-flag contract, vector_search error logging, E2E ingest assertion | +280/-30 |
| `805f93a` | Wave 2 completion: types.rs typed FpfSearchResponse with warning field, CLI runtime fallback parity with MCP via try_semantic_search helper, 4 fallback tests via closure injection, 8 corner case tests (NaN/Inf/off-by-one/empty/limit edges/unicode), NaN/Inf validation in insert_fpf_chunks | +405/-30 |

## FR mapping

### FR-001 — [System] can search FPF KB sections by semantic similarity using EmbedDriver

**Core implementation:**
- `forgeplan_core::db::store::search_fpf_by_vector(query_vec: &[f32], limit: usize) -> Result<Vec<FpfChunk>>` — validates dim=1024, uses LanceDB native `table.vector_search().distance_type(DistanceType::Cosine).limit(limit).execute()`, returns Ok(empty) gracefully on all-null column (migration path) or LanceDB errors (logged to stderr for debugging).
- `forgeplan_core::db::store::insert_fpf_chunks(&[FpfChunk], Option<&[Vec<f32>]>)` — accepts optional embeddings, validates length match + per-vec dim=1024 + **NaN/Inf rejection** at insert boundary.

**Schema + migration:**
- `forgeplan_core::db::schema::fpf_spec_schema()` — new `embedding` column `FixedSizeList<Float32, 1024>`, nullable. Rustdoc documents feature-flag contract.
- `forgeplan_core::db::migrate::migrate_fpf_spec()` — `NewColumnTransform::AllNulls`, idempotent, preserves pre-existing rows.

**CLI surface:**
- `crates/forgeplan-cli/src/commands/fpf.rs::run_search(query, limit, semantic: bool)` — wired to `--semantic` flag, defensive chain via `try_semantic_search` helper.

**MCP surface:**
- `crates/forgeplan-mcp/src/server.rs::forgeplan_fpf_search` — extended with `semantic: Option<bool>` param, graceful fallback on runtime failures, typed `FpfSearchResponse` with `warning` field.

### FR-002 — [User] can force semantic search with `forgeplan fpf search <query> --semantic`

- `crates/forgeplan-cli/src/main.rs::FpfCommands::Search { query, limit, semantic }` — clap derive flag
- Dispatch at line ~802 passes `semantic` to `commands::fpf::run_search`

### FR-003 — [System] gracefully falls back to keyword when EmbedDriver unavailable OR feature disabled

**Two layers of fallback** (stronger than PRD minimum):

1. **Compile-time gate** (feature off):
   - CLI: `#[cfg(not(feature = "semantic-search"))]` branch prints yellow ⚠ warning "semantic-search feature not compiled in; falling back to keyword search" to stderr, runs keyword path, exit 0
   - MCP: same logic, sets `warning` field in response

2. **Runtime failures** (feature on but something breaks):
   - Embedder::new() fails (no internet, HuggingFace down, disk full) → warning + keyword fallback
   - embedder.embed() fails (tokenizer panic on pathological input) → warning + keyword fallback
   - search_fpf_by_vector() errors (corrupted index, IO error) → warning + keyword fallback
   - CLI: via `try_semantic_search` helper + main caller's `match Err(_) => eprintln + store.search_fpf(keyword)`
   - MCP: handler's defensive `match` chain + `warning` field populated

Any runtime failure degrades gracefully — zero hard errors for the user, all paths return results.

## Core API additions

- `RuleSource::Config | Default` — not in this sprint (was Sprint 13.6)
- `FpfStorage::insert_fpf_chunks(chunks, Option<&[Vec<f32>]>)` — **trait extended** (Arch H1 fix)
- `FpfSearchResponse { query, semantic, count, results, warning }` — typed MCP response
- `FpfSearchHit { id, section_id, title, snippet, line_count }` — typed hit
- `try_semantic_search<F: FnOnce(&str) -> Result<Vec<f32>>>(store, query, limit, encoder) -> Result<Vec<FpfChunk>>` — CLI helper with closure injection for testability

## Audit cycle

### Round 1: 4 parallel auditors

| Auditor | Crit | High | Med | Low |
|---|---|---|---|---|
| Rust | 0 | 0 | 2 | 2 |
| Security | 0 | 0 | 2 | 7 |
| Architecture | 0 | **2** | 4 | 7 |
| Tests | **3** | **5** | 7 | 3 + 6Q |

**Merge blockers identified:**
- Tests C1: `search_fpf_by_vector` ordering assertion weak (only checks results[0])
- Tests C2: all-null-embedding-column case NOT tested (migration path for every existing user!)
- Tests C3: migration idempotency test doesn't verify data preservation
- Arch H1 + Tests H5: FpfStorage trait forwarder hardcodes None, architectural lie
- Arch H2: feature-flag contract implicit, needs rustdoc
- Tests H1-H4: length mismatch, mixed dim, CLI fallback, ingest feature-off untested

### Round 2: Fixer (commit 9bf382d) — 11 FIXes applied

1. FIX 1 (Tests C2): all-null-embedding-column test
2. FIX 2 (Tests C3): `migrate_fpf_spec_preserves_pre_existing_rows` with legacy rows
3. FIX 3 (Tests C1): strengthened ordering test (len + no duplicates)
4. FIX 4 (Arch H1+Tests H5): `FpfStorage` trait signature extended with `Option<&[Vec<f32>]>`; forwarders in `lance.rs` and `in_memory.rs` updated
5. FIX 5 (Arch H2): rustdoc feature-flag contract on schema.rs + store.rs
6. FIX 6 (Tests H1+H2): length mismatch + mixed-dim validation tests
7. FIX 7 (Tests H3+H4): CLI unit tests for empty/whitespace/oversized query
8. FIX 8 (Sec M1): query length bound + empty check before store access
9. FIX 9 (Sec M2): ANSI strip on echoed query
10. FIX 10 (Rust M2+L1): log swallowed vector_search errors to stderr + hint parity
11. FIX 11 (Tests Q3): E2E ingest exit-0 assertion + specific fallback grep

Result: 0 critical code findings, 0 high findings, all merge blockers addressed.

### Round 3: MCP parity agent (commit ce8bc8d) — Arch L5 closed

Added `semantic` param to MCP `forgeplan_fpf_search` tool with defensive runtime fallback — closed the "MCP-first tool" asymmetry gap flagged by architecture auditor.

**Notable**: mcp-parity agent added **runtime** fallback protection (Embedder init / encode / search failures) beyond what was in the original task spec, which was only compile-time fallback. This made MCP more robust than CLI — creating an inconsistency that Wave 2 then resolved.

### Round 4: Completer Wave 2 (commit 805f93a) — 4 items

**C1 types.rs cleanup** — Returned MCP handler to typed `FpfSearchResponse` with `warning: Option<String>` field. Eliminated dead code in types.rs. External JSON contract unchanged (verified via serialization tests).

**C2 CLI runtime fallback parity** — Extracted `try_semantic_search<F>(store, query, limit, encoder: F)` helper that takes an `FnOnce` encoder closure. Main `run_search` catches runtime failures in the semantic path (Embedder init / encode / search errors) and degrades to keyword with ⚠ stderr warning, matching MCP's robustness.

**C3 Fallback tests** (Approach B — closure injection) — 4 tests:
- `semantic_fallback_on_embedder_init_fail`
- `semantic_fallback_on_encode_fail`
- `semantic_fallback_on_search_fail`
- `semantic_success_returns_results`

**C4 Corner cases** — 8 tests + NaN/Inf validation in `insert_fpf_chunks` production code:
- `search_fpf_by_vector_nan_query_handled`
- `search_fpf_by_vector_inf_query_handled`
- `search_fpf_by_vector_off_by_one_dim_errors` (1023, 1025)
- `search_fpf_by_vector_empty_slice_errors`
- `search_fpf_by_vector_limit_zero`
- `search_fpf_by_vector_limit_max`
- `search_fpf_by_vector_unicode_query_chunks`
- `insert_fpf_chunks_nan_in_embedding_rejected`

## Test results

- **Total: 1109 tests pass, 0 failed, 1 ignored** (baseline ~1075 + 34 net new)
- Test count by crate:
  - `forgeplan-core` lib: 894 tests (+12 from 882 in W1, +8 in fixer, +8 in completer)
  - `forgeplan-cli`: 99 tests (+3 in fixer CLI unit, +4 in completer fallback tests)
  - `forgeplan-mcp`: 15 tests (+4 in fp_param_validation from mcp-parity)
  - Other crates: 101 tests (unchanged)
- `cargo fmt -- --check`: clean
- `cargo check --workspace` (default features): 0 warnings, 0 errors
- `cargo check --workspace --features semantic-search`: 0 warnings, 0 errors
- `cargo build --release` (default): 1m 56s, 42MB binary
- E2E regression `tests/e2e/sprint-13.7-regression.sh`: **exit 0, all 16 checks pass**:
  - 13.1 duplicate guard
  - 13.2 BM25 search
  - 13.3 tags (key=value)
  - 13.4 discover subcommand
  - 13.5 score with evidence
  - 13.6 fpf rules tree + flat + json
  - 13.6 fpf check styled + json + missing-artifact error
  - 13.7 fpf ingest exit 0 (NEW)
  - 13.7 fpf search --semantic specific fallback warning (NEW)
  - 13.7 fpf search --semantic graceful exit 0 (NEW)
  - 13.7 fpf search keyword path still works (NEW)

## Manual UX verification (team-lead on release binary)

| # | Command | Quality | Notes |
|---|---|---|---|
| 1 | `fpf search "trust"` (keyword, default) | ★★★★★ | 3 relevant results, backward compat preserved |
| 2 | `fpf search "trust" --semantic` (default build) | ★★★★★ | `⚠ semantic-search feature not compiled in; falling back to keyword search` + results + exit 0 |
| 3 | `fpf check PRD-001` (13.6 regression) | ★★★★★ | Winning ★ blind-spot rule, 13.6 surface intact |
| 4 | `fpf rules` (13.6 regression) | ★★★★★ | Action tree EXPLORE/INVESTIGATE/EXPLOIT with `link_count=0` (not `==0`, polish preserved) |
| 5 | `fpf search ""` (empty query validation) | ★★★★★ | Clean "Error: Search query cannot be empty" |
| 6 | E2E 13.7 regression script | ★★★★★ | 16/16 pass on release binary |

## Architecture honesty

The `FpfStorage` trait now accepts embeddings through its signature — no more "forwarder passes None" lie. Any consumer going through the driver abstraction (current: `LanceDriver`, `InMemoryStore`; future: remote drivers, test doubles) can legitimately write vector embeddings. This closes Arch H1 and prevents future trait drift.

The typed `FpfSearchResponse` / `FpfSearchHit` structs in `mcp/types.rs` are now the source of truth for the MCP JSON contract. Schema and handler are colocated in one place. The brief detour through raw `serde_json::json!()` during mcp-parity work has been undone — a legitimate architectural cleanup.

## Defensive programming — two layers

This sprint deliberately overshot PRD-042 FR-003 which only required compile-time fallback. Both CLI and MCP now handle:

1. Compile-time: `#[cfg(not(feature = "semantic-search"))]` branches
2. Runtime: `match Err(_) => warn + keyword fallback` on Embedder init, encode, and search errors

This is documented behavior, tested via 4 fallback tests using closure injection, and means users never see a hard error from the semantic path. The trade-off: slightly more complex code in `run_search` (one helper function) vs much stronger reliability in production. Worth it for a feature that depends on a 150MB model download from an external service.

## Deferred to backlog

Logged per-audit for future sprints, NOT blocking merge:

- **Rust M1**: Embedder cold load per search (future MCP hot path concern; single-call CLI unaffected)
- **Arch M1**: `run_migrations` signature refactor to `Tables<'_>` struct — do before Sprint 13.8 if adding another optional table
- **Arch M2**: Convenience helper for query encoding once second consumer (third surface) appears
- **Arch M3+M4**: `knowledge.rs` vs CLI ingest orchestration split — `FpfIngestBatch` refactor candidate
- **Arch L6**: IVF-PQ vector index creation for >10k sections (204 sections doesn't need it today)
- **Arch L7**: Zero-copy embedding pass via contiguous `&[f32]` vs `&[Vec<f32>]` — micro-optimization
- **Tests M1-M6 misc**: additional unicode edge cases, tie-breaking in ordering, MCP filter combinations in semantic path
- **`search_fpf_by_vector` addition to `FpfStorage` trait**: for symmetric surface — skipped because no consumer requires it yet

## Integration with prior Sprint 13.x work

- Sprint 13.1 (duplicate guard): still works, E2E verified
- Sprint 13.2 (BM25 smart search): still works, E2E verified
- Sprint 13.3 (tags + SourceTier): still works, E2E verified
- Sprint 13.4 (discover MCP): still works, E2E verified
- Sprint 13.5 (Skills Memory + R_eff CI): still works
- Sprint 13.6 (FPF Rules CLI+MCP): still works — `fpf check` winning rule highlighting verified on release binary
- **Sprint 12 FPF Engine + PRD-018 false-active stub**: PRD-018 is explicitly superseded by PRD-042

## Team execution pattern

Multi-agent team ran cleanly this sprint:
- 2 sequential implementers (core + CLI) — file ownership prevented parallelism but also prevented conflicts
- 1 parallel MCP parity specialist (different file ownership, zero overlap with fixer)
- 4 parallel auditors (read-only, classic fan-out)
- 1 sequential fixer (11 fixes, touched multiple crates safely)
- 1 sequential completer (wave 2, closes gaps from fixer + mcp-parity iteration)
- team-lead did manual UX directly (faster for 6 smoke checks vs spawning another agent)
- Total wall time: ~3 hours including fixer stall period

## Related Artifacts

| Artifact | Relation |
|---|---|
| PRD-042 | informs (this evidence supports FR-001..FR-003) |
| **PRD-018** | **superseded by PRD-042** (Sprint 12 false-active stub closed) |
| EPIC-003 | informs (Sprint 13 v0.17.0 series, last feature sprint) |
| RFC-001 | context (FPF Engine parent) |
| RFC-003 | context (Driver Layer with EmbedDriver trait) |
| EVID-063 | predecessor (Sprint 13.6 closeout) |
| NOTE-039 | closes (deferred Sprint 12 FPF KB vector search item) |
| sources/RuVector | external pattern source (vector search reference) |


