---
depth: tactical
id: NOTE-045
kind: note
status: active
title: Sprint 13.7 Deferred Debts — backlog tracking for post-closeout retrospective
---

# NOTE-045: Sprint 13.7 Deferred Debts

Comprehensive list of items found during Sprint 13.7 self-retrospective (2026-04-08) that were NOT fixed in the post-closeout hotfix. Each item is classified:
- **FIXED** — addressed in hotfix branch `fix/sprint-13.7-post-closeout-hardening`
- **BACKLOG** — legitimate future work, tracked here
- **PROCESS** — methodology lesson, captured in NOTE-044 checklist instead of code

## Sprint 13.7 retrospective summary

Found 19 debt items across 4 categories:
- Not tested (6 items)
- Architectural softening (4 items)
- Typing relaxations (3 items)
- Process / methodology (6 items)

Of these, 11 were **FIXED** in the post-closeout hotfix. 7 moved to **BACKLOG** below. 1 resolved automatically (#12 warning serialization — verified during retro).

## FIXED in hotfix (commits 8204d9c + bc4e7d0 + team-lead D2)

| # | Item | Fix |
|---|---|---|
| 1 | Real semantic path (end-to-end BGE-M3) 0 tested | `#[ignore]` test `real_semantic_roundtrip_with_bge_m3` — verified manually running 8s |
| 2 | `run_search --semantic` with feature ON 0 tested | Same as #1 covers it |
| 3 | No MCP handler integration harness (deferred 2 sprints) | NEW `crates/forgeplan-mcp/tests/fpf_search_handler.rs` with 7 integration tests |
| 4 | Real v3 migration path untested | NEW `migrate_real_v3_workspace_adds_fpf_spec_embedding_column` test with legacy fixture |
| 5 | Tests M6 semantic vs keyword ordering missing | NEW `semantic_and_keyword_top_result_agrees_for_clear_match` |
| 6 | E2E --semantic feature-on path 0 | NEW `SEMANTIC_E2E=1 SEMANTIC_BIN=...` opt-in path in `tests/e2e/sprint-13.7-regression.sh` |
| 8 | `FpfStorage::search_fpf_by_vector` not in trait | Added to trait + LanceDriver forwarder + InMemoryStore stub + dyn-compat test |
| 10 | Vector dim hardcoded 1024 no runtime check | Runtime assert `embedder.dim() == EMBEDDING_DIM` in `run_ingest` + rustdoc on `fpf_spec_schema` |
| 12 | `warning` field serialization shape unverified | Verified: no `skip_serializing_if`, emits `"warning": null` as contract specifies. Explicit test asserts this |
| 18 | Wave 2 ~400 LOC never re-audited | Re-auditor agent ran on 805f93a + hotfix commits |
| 11 | types.rs dead code from mcp-parity json! workaround | Completer wave 2 already fixed (restored typed struct). Meta-lesson captured in NOTE-044 |

## BACKLOG — real deferred debts

### Architecture

**D1. `run_migrations` signature not scalable** (Arch M1 from Sprint 13.7 audit)
- Current: `run_migrations(artifacts, relations, change_log: Option, fpf_spec: Option)` — 4 params, 2 Optional
- Problem: Sprint 13.8+ adds another table → another param. Linear growth.
- Fix: `struct Tables<'a> { artifacts, relations, change_log, fpf_spec }` + `run_migrations(&Tables<'_>)`
- Size: ~30 LOC diff
- When: **before next sprint that adds a table** (no fixed deadline)
- Severity: Medium (not functional, organizational)

**D2. `knowledge.rs` ingest ownership** (Arch M3/M4 from Sprint 13.7 audit)
- Current: CLI `run_ingest` orchestrates scan + parse + encode + insert (4 concerns in one function)
- PRD-042 originally said encoding should live in `knowledge.rs::ingest_fpf_directory()`
- Reason we deferred: CLI is thin enough, core is logical home but refactor adds risk without functional gain
- Fix: extract `FpfIngestBatch { chunks, embeddings: Option<..> }` + `LanceStore::ingest_fpf_batch(batch)` + move encoding to `knowledge.rs::encode_for_ingest()` feature-gated
- Size: ~80 LOC refactor
- When: opportunistic (when touching run_ingest for another reason)
- Severity: Low (deliberate deviation from PRD-042, documented in EVID-064)

**D3. Query encoding helper for future third surface** (Arch M2 from Sprint 13.7 audit)
- Current: CLI and MCP each have their own `Embedder::new() + embed(query)` path
- Problem: third surface (TUI, web) would duplicate the glue
- Fix: `#[cfg(feature = "semantic-search")] pub fn embed_query(query: &str) -> Result<Vec<f32>>` in `embed/mod.rs`
- Size: ~15 LOC
- When: when third surface appears (YAGNI until then)
- Severity: Low (just a DRY concern)

**D4. `FpfStorage` trait extension for remaining methods** 
- After fix #8 added `search_fpf_by_vector` to trait, there are still `has_fpf`, `clear_fpf`, `list_fpf_sections`, `get_fpf_section` on `LanceStore` but NOT on the trait
- Asymmetry: trait consumer can `insert_fpf_chunks`, `search_fpf`, `search_fpf_by_vector` but not `list` or `get`
- Fix: extend trait with these 4 methods + InMemory stubs
- Size: ~40 LOC
- When: when a second FpfStorage consumer (not LanceStore) needs full read access
- Severity: Low (still YAGNI)

### Performance

**D5. IVF-PQ vector index for scale** (Arch L6 from Sprint 13.7 audit)
- Current: LanceDB brute-force vector scan on 204 sections (fine)
- Threshold: performance degrades around 10k+ sections
- Fix: `table.create_index(Index::ivf_pq())` on `embedding` column during ingest
- When: when KB grows past ~2k sections or p50 search latency exceeds 50ms
- Severity: Low (scalability future)

**D6. Zero-copy embedding pass** (Arch L7)
- Current: `Option<&[Vec<f32>]>` — 204 heap allocations per batch (~850KB on 1024-dim × 204)
- Fix: refactor to `&[f32]` contiguous buffer (204 × 1024 = 209k floats = 836KB flat) with row-stride
- Size: ~50 LOC + tests
- Severity: Very Low (micro-optimization, invisible at current scale)

**D7. Embedder cold load per search invocation** (Rust M1)
- Current: `Embedder::new()` called fresh on every `run_search` with `--semantic`. BGE-M3 init ~100ms after first download.
- Impact: MCP server with many sequential semantic searches re-loads model each call
- Fix: `once_cell::sync::OnceCell<Mutex<Embedder>>` in MCP server OR lazy static in `embed/mod.rs`
- When: when MCP semantic search becomes a hot path (currently single-call use)
- Severity: Low (perf future)

### Design subjective

**D8. Runtime fallback adversarial review** (beyond-PRD scope extension)
- Current: 3-layer defense (feature-off → Embedder init fail → encode fail → search fail) all silently fall back to keyword with warning
- Concern: is silent fallback always right? Some users might want a hard error + explicit retry flag
- Not tested: what if user *wants* to know the semantic path failed and NOT fall back? No `--strict-semantic` flag
- Fix: consider `--strict-semantic` flag OR document decision explicitly (current: always prefer degraded result over failure)
- When: if users complain about silent fallback
- Severity: Low (UX design choice)

## PROCESS — captured in NOTE-044 checklist

**P1.** Fixer stale git view confusion (15 min)
**P2.** cli-impl идле час after W2
**P3.** MCP parity unplanned at sprint start
**P4.** Wave 2 unplanned at sprint start
**P5.** No re-audit after Wave 2 (skipped step in /forge-cycle)
**P6.** Manual UX was shallow (6 commands, not comprehensive)

All 6 encoded as checklist items or red flags in NOTE-044.

## Summary

- **11 items FIXED** in hotfix (6 hours of team work across 3 agents + team-lead)
- **8 items BACKLOG** with explicit severity and "when to fix" criteria
- **6 items PROCESS** captured in NOTE-044 for future prevention

**Net debt reduction**: Sprint 13.7 originally shipped with ~19 issues, now has 8 backlog items all classified Low/Very Low severity + 0 unfixed High/Critical. Quality is net-positive vs what was merged in PR #155.

## Related Artifacts

| Artifact | Relation |
|---|---|
| NOTE-044 | sibling (reusable checklist to prevent recurrence) |
| EVID-064 | context (Sprint 13.7 evidence) |
| PRD-042 | subject (Sprint 13.7 PRD that triggered debts) |
| EPIC-003 | context |
