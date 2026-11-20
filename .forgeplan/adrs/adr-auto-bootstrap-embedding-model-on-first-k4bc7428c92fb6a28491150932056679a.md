---
author: Eli Rum
depth: standard
id: ADR-032
kind: adr
links:
- target: PRD-092
  relation: based_on
assigned_number: 32
parent_epic: null
predicted_number: 32
slug: adr-auto-bootstrap-embedding-model-on-first-k4bc7428c92fb6a28491150932056679a
status: active
tags: []
title: Auto-bootstrap embedding model on first semantic use
valid_until: 2027-05-18
---

# ADR-032: Auto-bootstrap embedding model on first semantic use

## Context

Semantic search needs the BGE-M3 embedding model (~2.1 GB). Today the model is fetched only by `setup --warm-model` (explicit prefetch) or implicitly when `Embedder::new()` runs inside `embed` — but the first *semantic operation* a user actually performs (`forgeplan search --semantic`, `index --embed-profile`) either requires that prior manual step or leaves the embedding queue stranded pending forever. Related: PRD-092 (self-maintaining index), RFC-021 (implementation), PROB-088 (release builds without semantic-search feature).

## Decision

The first permitted semantic operation downloads the model automatically with a visible progress indicator and a resumable cache, instead of requiring `setup --warm-model` up front. `setup --warm-model` remains the explicit prefetch path; a build without the `semantic-search` feature keeps the existing honest `Fix:` error and never downloads.

**Selected**: Lazy bootstrap inside the permitted model-obtainer path (`embed`, `index --embed-profile`, `search --semantic`, and MCP equivalents through the same facade).

**Why Selected**: It removes the dead-end first-run experience without surprising anyone — the download happens only when the user explicitly asked for a semantic operation, progress is shown, the cache is resumable and checksum-verified, and offline/metered users keep `--skip-model`/explicit-`Fix` escape hatches. Eager download at `init` would punish every install that never uses semantic search; keeping it manual guarantees stranded queues.

## Alternatives Considered

| Option | Verdict | Why |
|--------|---------|-----|
| A: Eager download during `forgeplan setup`/`init` | Rejected | Forces ~2.1 GB on users who never run semantic search; init must stay fast and offline-tolerant |
| B: Lazy auto-download on first semantic use | **Chosen** | Download only when semantic use is actually requested; single bootstrap point behind `Embedder::new()` covers every obtainer |
| C: Keep manual-only (`setup --warm-model` + `Fix:` hint) | Rejected | Confirmed gap: pending embedding jobs strand silently and the first semantic call dead-ends |

## Consequences

### Positive
- First `search --semantic` works end-to-end on a fresh install.
- Embedding queue stops stranding: the drain and the download share one obtainer path.
- `setup --warm-model` remains useful for CI/prefetch but is no longer required.

### Negative (trade-offs)
- First semantic call carries the ~2.1 GB download latency — visible via progress, interruptible.
- A failed/partial download must not corrupt the cache; resume and checksum verification are required, not optional.

### Risks
- Metered/offline users get a surprise download: mitigated by the honest no-semantic `Fix:` path and a clear offline error naming the URL and `setup --warm-model`.
- Concurrent obtainers racing the download: serialize through the same lock that guards embedding drains; the cache write is atomic (temp file + rename).

## Invariants

- Model download happens only inside a permitted model-obtainer call (`embed`, `index --embed-profile`, `search --semantic`, `setup --warm-model`) — never during plain `index`, `watch`, or a non-semantic command.
- A build without `semantic-search` never attempts a download and always fails with the `Fix:` hint.
- A partially downloaded cache is never served; resume or restart until the checksum verifies.
- The embedding queue stays pending (not dropped) whenever the model is absent or the drain is refused.

## Evidence Requirements

- Cold `search --semantic` on a model-absent workspace completes after one download and a second call loads from cache in under 1 second.
- SIGINT mid-download leaves a resumable, non-corrupt cache.
- `index --embed-profile` with pending jobs and no model reports `embedding drain skipped: model not present` and leaves the queue intact.
- Non-semantic build: `search --semantic` exits non-zero with the `semantic-search` `Fix:` hint.

## Valid Until

**Дата**: `2027-05-18`

**Обоснование TTL**: six months — revisit when the embedding engine changes (PRD-084 tract migration), when section-level embedding lands (queue granularity changes), or if download reliability proves a measured defect.

**Refresh Triggers**:
- the embedding model or runtime changes (BGE-M3 → another engine);
- section-level embedding jobs ship and change queue semantics;
- measured download-failure rate justifies a bundled-model release;
- PROB-088-style packaging defects make semantic builds unavailable again.

## Rollback Plan

If auto-bootstrap proves harmful (surprise downloads, corrupted caches, metered-link incidents), revert to manual-only by gating `Embedder::new()` behind the existing `Fix: forgeplan setup --warm-model` error — the obtainer seam is the single place that decides, so rollback is a one-point change. The embedding queue stays pending either way, so no data migration is needed.

## Affected Files

- `crates/forgeplan-core/src/embed/**` — `Embedder::new()` bootstrap, cache resume/checksum
- `crates/forgeplan-cli/src/commands/embed.rs`, `index.rs`, `search*.rs`, `setup.rs` — obtainer surfaces and progress/Fix output
- `crates/forgeplan-core/src/constellation/**` — embedding queue drain, profile admission
- `crates/forgeplan-mcp/src/**` — semantic search through the same facade

