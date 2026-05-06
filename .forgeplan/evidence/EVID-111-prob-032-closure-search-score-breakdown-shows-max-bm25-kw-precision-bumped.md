---
depth: standard
id: EVID-111
kind: evidence
links:
- target: PROB-032
  relation: informs
status: active
title: PROB-032 closure search score breakdown shows max(bm25, kw) precision bumped
---

# EVID-111: PROB-032 closure — search score breakdown coherent с total

## Summary

Closes PROB-032 — `forgeplan search` displayed `kw=0.0 sem=0.0 r=0.0 g=0.0` while total score was non-zero (e.g. 0.57), violating "sum ≈ total" expectation и lying к user about ranking composition. Two-part fix в `crates/forgeplan-cli/src/commands/search.rs:393-405`: (a) display `max(bm25_score, keyword_score)` instead of `keyword_score` (substring) since `combined_score()` actually uses the max; (b) bump precision `{:.1}` → `{:.2}` so contributions of 0.02–0.09 no longer round-down к 0.0.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Evidence

### Root cause

`SmartSearchResult` carries TWO keyword channels:
- `keyword_score` — substring match score (raw `record.body.contains(query)` etc.)
- `bm25_score` — normalized BM25 token-match score (PRD-039 Smart Search v2)

`combined_score()` в `crates/forgeplan-core/src/search/smart.rs:153` uses
`keyword_channel = bm25_norm.max(keyword_score)` as the base. When match is
via BM25 tokenization (e.g. "auth" matches "authentication" via stemming
but not as substring), `keyword_score` = 0 but `bm25_score` > 0. CLI
display только showed `keyword_score`, hence `kw=0.0` despite non-zero
total.

### Fix

```rust
let kw_channel = r.bm25_score.max(r.keyword_score);
let signals = format!(
    "kw={:.2} sem={:.2} r={:.2} g={:.2}",
    kw_channel, r.semantic_score, r.r_eff, r.graph_centrality
);
```

### Real E2E (target/release/forgeplan, fresh tempdir)

```
$ forgeplan init -y
$ forgeplan new prd "Error handling for API endpoints"
$ forgeplan new prd "Authentication system"
$ forgeplan search "api error"

Found 1 result(s) for "api error" (smart search):

  0.36  PRD-001 [prd|draft] "Error handling for API endpoints"
        kw=0.36 sem=0.00 r=0.00 g=0.00
```

Pre-fix: `kw=0.0 sem=0.0 r=0.0 g=0.0` (lying — total 0.36 без visible contributor).
Post-fix: `kw=0.36 sem=0.00 r=0.00 g=0.00` (truthful — kw is the contributor; total = kw × boost).

Math: `combined_score = max(kw, sem) × (1.0 + r×0.2 + active×0.1 + graph×0.1)`. With kw=0.36, sem=0, r=0, draft (no active boost), graph=0: `0.36 × 1.0 = 0.36` ✓.

### AC tracking

- AC-1 ✅ kw component shows non-zero value when contributing (was lying as 0.0)
- AC-2 ✅ sum of components ≈ total (within rounding via {:.2} precision)
- AC-3 N/A (sensible breakdown DOES exist — fix preserved breakdown line)
- AC-4 ✅ existing search tests still pass (0 failures across 38 suites)

### Quality gates

```
cargo fmt --check                                                  clean
cargo clippy --workspace --all-targets --features test-helpers
  -- -D warnings                                                   clean
cargo test --workspace --features test-helpers                     0 failures (38 suites)
```

## Hindsight

Bug-class lesson: **multi-channel scoring exposes display/computation drift**. When `combined_score` uses `max(A, B)` but display shows only `A`, users see incoherent breakdowns. Audit prompt template: «when scoring formula uses `max()` / `mean()` / `weighted_sum()` over multiple channels, display MUST show the actually-contributing channel(s), not arbitrary subset».

Mirrors PROB-029 verdict aggregator pattern: hidden fold logic + display path that lies about the fold = same shape, different surface (search vs health).

## Related Artifacts

| Artifact | Relation |
|---|---|
| PROB-032 | informs (this evidence demonstrates closure) |
| PRD-039 | informs (Sprint 13.2 BM25 introduced the breakdown duality) |
| PROB-030 | informs (sibling BM25 search regression — same audit batch) |



