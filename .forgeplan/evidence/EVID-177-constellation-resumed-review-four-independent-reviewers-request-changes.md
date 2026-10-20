---
depth: standard
id: EVID-177
kind: evidence
last_modified_at: 2026-10-18T20:47:22.819358+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: RFC-015
  relation: informs
status: draft
title: Constellation resumed review — four independent reviewers, REQUEST_CHANGES
---

---
assigned_number: 177
created: 2026-10-18
predicted_number: 177
slug: evid-constellation-resumed-review-four-independent-reviewers-request-changes
updated: 2026-10-18
---

## Structured Fields

verdict: weakens
congruence_level: 3
evidence_type: code_review

## Scope

2026-10-18. HEAD 925111ec plus UNCOMMITTED facade/jobs/publisher/reconciler and tests/constellation_embedding_preservation.rs. Four independent read-only reviewers: CoreLogic (reviewer), ProfileSafety (reviewer), ReadBoundary (security-reviewer), AcceptanceReview (architect-review). Skills: smith, audit, team, rust-expert, restore. Source unchanged by this review. REQUEST_CHANGES. Historical EVID-176 is not overwritten.

## Findings

Paths below relative to crates/forgeplan-core/src/constellation unless qualified.

1. BLOCKER static: publisher.rs:1334-1339 artifacts_batch and :1622-1625 sections_batch require preserved maps, but facade.rs:939,977-981 and publisher.rs:731-745 retain old arity. Semantic-column readback/maps not connected. This is signature analysis, not a compiler run. C01 incomplete.
2. HIGH static: facade.rs:1173-1182 and publisher.rs:433 rebuild descriptor; manifest.rs:393-397 derives profile only from pending jobs. Prior descriptor.profile_id is not preserved. C01 open.
3. MEDIUM static: publisher.rs:1290-1292,1381-1387 requires source_digest equality for preservation, invalidating metadata-only artifact vectors despite unchanged input (L01).
4. HIGH static, consensus CoreLogic/ReadBoundary/AcceptanceReview: publisher.rs:368-373 republish_current still lacks finish_writes/sync_written_set before CURRENT. New fault tests are not the implementation. C04 open.
5. HIGH static: generations.rs:184-202 writes reader leases in source index; export.rs:163-252 bypasses lease protection. C06 read-only guarantee open; concurrent export/GC failure is inference, not reproduced here.
6. HIGH static: pack/impact/export/snapshot canonicalize replaced roots before capture, unlike search/get symlink checks. context_pack.rs:191-201, impact.rs:235-239, export.rs:150-154, snapshot.rs:264-275. C10 open.
7. MEDIUM static: MCP constellation_read.rs:36-45,225-253 stores SourceSpec not captured working bytes; handles reread changed bytes and lack TTL/cap. C08 open.
8. MEDIUM static: context_pack.rs:365-420 budgets excerpts/title, not entire serialized task/metadata/omissions/escaping or MCP text representation. C09 open.
9. HIGH static contract gap: facade.rs:1411-1588 holds physical owner during inference. Explicit CLI embed passes vectors without immutable original binding; facade.rs:420-472 recaptures source/profile after compute. Stale vector receiving new binding is INFERENCE requiring runtime reproduction. Drain reads baseline before owner at :1396-1412; concurrent pointer regression is INFERENCE. C07 only partially addressed; no late-A test.
10. MEDIUM static: full MCP server.rs:409-454,709 and ordinary get/search/new/update/link/score still require legacy store on migrated layout. Separate constellation tools do not close C12.
11. Acceptance gaps: CI runs embedding_reference for real but not semantic workspace tests. eval/run_eval.py:470-503 does not use created snapshot; local version validation is not fail-closed (:304-320,366-368). Historical F10 zero substitutions cannot prove pinned-version fidelity.

## Existing fixes

C02 old-key retirement is wired statically, but old pinned generation and crash recovery remain unverified. C03 reindex migration guard exists in committed W-alpha. C05 explicitly suppresses Git lazy fetch. C11 snapshot qualifier is passed. None is claimed independently runtime-verified for current UNCOMMITTED source here.

## Execution

CWD /Users/explosovebit/Work/ForgePlan-Constellation-P0. Command ./target/debug/deps/constellation_embedding_preservation-aac3db0555cc231d --test-threads=1. Exit 101, one binary, 2 passed, 1 failed, 0 ignored. a_reconcile_with_one_edit_preserves_embeddings_on_unchanged_rows failed at test line320: prd-alpha unavailable vs ready. Profile migration and canonical rekey passed in that binary. Binary correspondence to current source NOT established; no current-source GREEN claim. Free disk observed 8.9G; no cargo/build/full matrix or CLI/MCP E2E run. Memory recall aborted twice; not used as evidence.

## Remaining work / handoff

Smith Row4 RIPER-5, not new feature or live incident. First complete selective preservation wiring and descriptor profile, then immutable computation bindings/owner protocol and durability barrier. Independently verify C02/C03/C05/C11. Resolve read-only lease/GC protocol, root checks, captured working handles, serialized pack cap and full MCP migrated cutover. Freeze source, establish one build owner and sufficient disk before sequential no-feature/semantic matrix, real CLI/MCP stdio L01-L16/F01-F10, pinned F10 eval and resource measurements. F11 external adapter remains separately unaccepted. No activation, push or merge performed. Revisit: after these source changes and current-source runtime evidence, rerun affected independent reviews and guardian gate.

