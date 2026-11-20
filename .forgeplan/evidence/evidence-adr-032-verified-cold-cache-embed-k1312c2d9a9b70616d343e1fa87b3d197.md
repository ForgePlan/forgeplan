---
author: null
depth: tactical
id: EVID-219
kind: evidence
links:
- target: adr-auto-bootstrap-embedding-model-on-first-k4bc7428c92fb6a28491150932056679a
  relation: informs
parent_epic: null
predicted_number: 219
slug: evidence-adr-032-verified-cold-cache-embed-k1312c2d9a9b70616d343e1fa87b3d197
status: active
tags: []
title: 'ADR-032 verified: cold-cache embed bootstrap ships with SIGKILL-durable queue'
valid_until: null
---

# EVID-219: ADR-032 verified: cold-cache embed bootstrap ships with SIGKILL-durable queue

| Field | Value |
|-------|-------|
| Status | Active |
| Created | 2026-11-19 |
| Target | ADR-032 |

## Structured Fields

evidence_type: test
verdict: supports
congruence_level: 3

base_sha: 51f1c2bc
result_sha: 1196f9b0
changed_paths: crates/forgeplan-cli/src/commands/embed.rs, crates/forgeplan-cli/src/commands/index.rs, crates/forgeplan-cli/src/commands/common.rs, crates/forgeplan-cli/tests/cli_lazy_sync.rs, crates/forgeplan-cli/tests/cli_negative_contracts.rs

## Measurement

Cold-cache bootstrap exercised in an isolated workspace with
`FORGEPLAN_MODEL_CACHE` pointed at an empty home and `HF_HUB_OFFLINE=1`.
`forgeplan embed` and `forgeplan index --embed-profile` on a cold cache
route through the download-capable `Embedder::new()` before
`ResolvedProfile::open()`, so the first semantic use downloads the model
instead of refusing with `setup --warm-model`. Read-only paths
(`search --semantic`, watch, git-sync, MCP queries) still never download.

Queue durability: durable `pending_jobs` in the generation manifest are
asserted stable across a hard SIGKILL (self-reexec test: child admits a
job, parent kills, the pending set is identical afterward) — test
`cli_lazy_sync.rs` family + manifest assertion.

## Result

- `cli_lazy_sync` suite: PASS (reconcile before read lock,
  `reconciled: N file(s)` on stderr only on real change, digest-gated
  no-op).
- `cli_negative_contracts`: 17/17 PASS including
  `status_json_emits_parseable_report` and
  `cold_cache_embed_bootstraps_not_warm_model_hint`.
- `cargo check -p forgeplan --all-targets`, `cargo clippy`, `cargo fmt`:
  clean. Commits `7961b034` (P1/P3) + `51f1c2bc` (P2) on
  `fix/reindex-bootstrap-564`, merged to `dev` via #566.

## Interpretation

ADR-032's decision — permitted obtainers bootstrap the model on first
semantic use, query paths never download — is implemented exactly as
specified and holds under SIGKILL and cold-cache conditions.

## Congruence Level Justification

CL3: same-context — measured on the exact binaries and code paths the
ADR governs (`Embedder::new()` ordering, `spawn_resolved`,
`cached_query_embedding`, manifest `pending_jobs`).

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| ADR-032 | informs |
| RFC-021 | informs |
