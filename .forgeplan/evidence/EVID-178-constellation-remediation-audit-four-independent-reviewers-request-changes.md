---
depth: standard
id: EVID-178
kind: evidence
last_modified_at: 2026-10-19T07:09:39.866303+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: RFC-015
  relation: informs
status: draft
title: 'Constellation remediation audit: four independent reviewers, REQUEST_CHANGES'
---

---
assigned_number: 178
created: 2026-10-19
predicted_number: 178
slug: evid-constellation-remediation-audit-four-independent-reviewers-request-changes
updated: 2026-10-19
---

## Structured Fields

verdict: weakens
congruence_level: 3
evidence_type: code_review

## Scope

2026-10-19. HEAD 925111ec20fc036ead92a61d19636515497d6cc1 plus UNCOMMITTED source: 26 tracked changes and new workspace_read.rs, constellation_embedding_preservation.rs, constellation_pinned_search.rs, constellation_migrated_read.rs. Independent parallel reviewers AuditLogic, AuditArchitecture, AuditRust (constellation-minimax-reviewer) and AuditSecurity (security-reviewer). Reports available as agent://AuditLogic, agent://AuditArchitecture, agent://AuditRust, agent://AuditSecurity during this session; substantive findings preserved below. Review preceded subsequent fixes; fixes do not retroactively change this negative verdict. EVID-176/177 unchanged.

## Verdict

REQUEST_CHANGES. Static review only. Not a tested or accepted release. No cargo check/test/clippy/fmt or CLI/MCP E2E was run. Disk observed 9.2G, below approved >=10GiB prerequisite.

## Findings

1. BLOCKER Rust: facade.rs:1439,1585 compare profile.dimensions u32 against EMBEDDING_DIM i32. Source-confirmed mismatch, no compiler invocation.
2. HIGH publication: completion/failure can restore prepared profile A after explicit adoption B because adoption leaves A pending jobs and completion/failure overwrite descriptor.profile_id from batch. facade.rs:2424-2432,2524-2546. Regression must exercise adopt B then late A without intervening prepare B. Failure path also lacks config fence.
3. MEDIUM queue: admission retains pending only for captured.entries, deleting unreadable/malformed keys and resetting retry history on recovery (facade.rs:1742-1759).
4. HIGH runtime: actual inference channels unbounded; timeout leaves old queued jobs/results, subsequent drain aborts on old tokens; failed disconnected worker is parked and reused; submit failure swallowed; shutdown unconditionally joins hung native calls (runtime.rs:213-215,233-272,308-309,384-395,843-848,1064-1103).
5. HIGH model identity: profile resolution follows refs/main while engine find_snapshot can choose another directory; explicit/background document inference not tied to prepared profile snapshot (jobs.rs:672,790; runtime.rs:198-200,1076-1103; CLI embed.rs:51-63,117).
6. HIGH source boundary remains check-then-open through mutable absolute paths; export/pack/impact/snapshot canonicalize aliases before capture (export.rs:148; context_pack.rs:174; impact.rs:229; snapshot.rs:262; capture.rs:1010). Git inherited routing variables are not cleared (snapshot.rs:115-118).
7. HIGH GC: generations.rs:157-160 drops directory enumeration errors via flatten/is_file; incomplete listing can omit protected manifest and produce unsafe version floor.
8. HIGH cutover incomplete: no MCP mutation handler migrated; MutationContext still requires LanceStore (projection/context.rs:96); ~17 read handlers still use legacy-opening resolver; cached legacy handle is not invalidated on cutover (server.rs:644,732,978,2737 and other mutation callers). Migrated operations may fail or use retired handles.
9. HIGH runtime integration: watcher supervisor exists but serve/MCP does not register a continuing supervisor; one-worker-per-index differs from promised one-per-process; watcher synchronous drain can delay full scan.
10. HIGH coverage: federation limit can skip required source below top-K; snapshot/capture coverage may report hidden incomplete results (federation.rs:368-402,476-482; snapshot.rs:411-419).
11. MEDIUM retained handles: constellation_read.rs:36-44,246-253,353-389 retains source specs rather than frozen bytes, without TTL/count/byte limits, and mutex spans I/O.
12. HIGH resource limits: whole-file capture/config reads unbounded; config FIFO may block indefinitely (capture.rs:71-74,856-863,1010,1082-1084).
13. HIGH MCP semantic limit based on artifact count rather than artifact+section rows (server.rs:305-311); requires ranked-query reproduction.
14. MEDIUM new migrated-read fixture constructs maintenance capability for parent root rather than .forgeplan workspace (constellation_migrated_read.rs:62-92); precondition expected to fail, unexecuted.

## Verified Static Improvements

Preserve-map readback and metadata-only reuse are connected; explicit descriptor profile propagation, republish shared-tracker durability barrier, original-job binding, owner-free inference split, incarnation/tombstone continuity and shared-lock pins exist in source. These are source observations, not executed PASS claims. Windows physical identity uses size/mtime rather than a proven NTFS identity; readonly-mount and two-process GC evidence absent.

## Execution And Revisit

Zero test runs in this audit. No binary/source correspondence established. Findings require source fixes, targeted tests, full sequential no-feature/semantic matrix, actual CLI/MCP stdio and model oracle. Revisit after fixes with frozen source fingerprint and real logs. Approved plan steps for captured handles, source capability boundary, renderer-aware pack budget, full mutation cutover and pinned F10 remain acceptance obligations; do not equate this subset review with plan completion.


