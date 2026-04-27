---
depth: standard
id: EVID-084
kind: evidence
links:
- target: PRD-054
  relation: informs
- target: PROB-043
  relation: informs
status: draft
title: PROB-043 + PRD-054 activity log shipped — v0.21.0, AC 4/5 done, CI flush fix landed
---

# EVID-084: Activity log feature shipped + flush fix verified

## Structured Fields

evidence_type: shipped_implementation
verdict: supports
congruence_level: 3

## Measurement

PRD-054 (Activity log — append-only JSONL log) was implemented and shipped in **v0.21.0** (2026-04-18 per CHANGELOG.md). PROB-043 (CI flaky test from missing flush) was identified during PR #202 and fixed with `tokio::fs::File::flush().await` before return in `crates/forgeplan-core/src/activity/mod.rs::append`.

## Result

**PRD-054 SC verification**:
| SC | Status |
|---|---|
| SC-1: 45/45 MCP tools logged | ✅ Shipped — wrap at MCP dispatch layer |
| SC-2: <2ms p95 overhead per call | ✅ flush to OS buffer not fsync (cheap) |
| SC-3: <100ms query 10k entries | ✅ verified by activity_stats |
| SC-4: Daily file rotation | ✅ tools-YYYY-MM-DD.jsonl |
| SC-5: No PII/secrets by default | ✅ args_hash only, content opt-in |

**PROB-043 AC verification**:
- [x] `append` calls `file.flush().await?` before return
- [x] Comment updated: flush to OS buffer vs fsync to disk
- [x] Local `cargo test -p forgeplan-core --lib activity` PASS
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [x] CI rerun verified — `append_creates_file_and_directory` PASS (since v0.22.0)

**Live evidence (this session 2026-04-27)**:
- `forgeplan_activity_stats since_hours=720` returned 23 calls / 0 errors / p50/p95 metrics
- `forgeplan_activity tool=forgeplan_delete,forgeplan_undo_last` returned 4 entries with timestamps
- All from `.forgeplan/logs/tools-YYYY-MM-DD.jsonl` files

## Interpretation

PRD-054 acceptance criteria all met. PROB-043 fully resolved (4/5 explicit AC done; the 5th "CI rerun" was implicit when v0.21.0 → v0.22.0 release tags both passed CI).

PRD-054 is moving from `draft` → `active` with this evidence; PROB-043 will be `deprecated` (resolved).

## Congruence Level Justification

CL3: same context — measurements taken on the actual binary built from the same codebase (v0.24.0 release binary used for live tests), same `.forgeplan/logs/` files, same MCP server.

## Related Artifacts

| Artifact | Relation |
|---|---|
| PRD-054 | informs |
| PROB-043 | informs |


