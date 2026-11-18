---
assigned_number: 205
author: null
created: 2026-11-13
depth: tactical
id: EVID-216
kind: evidence
links:
- target: spec-contextpack-bounded-federated-evidence-kb8bac5ddcb6edda3c6a10447bff91441
  relation: informs
parent_epic: null
predicted_number: 205
slug: evidence-contextpack-qualified-closure-and-k2b4866be40cdc2710f8946cc315cc2b6
status: draft
tags: []
title: ContextPack qualified closure and bounded CLI MCP delivery
updated: 2026-11-13
valid_until: null
---

---
assigned_number: 205
created: 2026-11-13
predicted_number: 205
slug: evidence-contextpack-qualified-closure-and-k2b4866be40cdc2710f8946cc315cc2b6
updated: 2026-11-13
---

# ContextPack qualified closure and bounded delivery

## Structured Fields

evidence_type: test
verdict: supports
congruence_level: 3

## Measurement

Observed on macOS/arm64 against an uncommitted implementation. These are development binaries with production features, not optimized release binaries.

- Source manifest SHA-256: `ba96d17436a2a5f90f318d5c4847d82c1d78edeb19fd3c5164268360d870a2a6`.
- CLI SHA-256: `5dd1dde2a3f0220aa1c0fcf90765c6a9c58b4f856dc8d34eaceef2feea508c8e`.
- Standalone MCP build SHA-256: `c0775ab5b80641e9f61c8c94320e1f66eb074d39f44d969c54ac5f9997cae227`.
- The native read-profile exercise launches `forgeplan serve --profile constellation-read`; the standalone executable was built, not exercised in this probe.
- No result commit, installation, push or release is asserted. Source manifests, failed attempts, complete command records, payloads and integration hashes remain in private local proof files.

## Result

- Build-7: formatting, production workspace check, strict Clippy and production-feature development builds passed.
- Behavioral suites: Core 21, read MCP 5 and CLI 5 passed; zero failed or ignored.
- Evaluation-accounting regressions: 9 passed, including snapshot timestamp serialization and returned ambiguity identity without request/body echo credit.
- Native-6: 94 checks passed over 38 recorded calls. Checks cover actual full-result byte caps, exact boundaries, whole-unit continuation, an independent declared-link oracle, page graph union, unresolved Mermaid navigation, source boundaries, historical Git bytes, stale/tampered cursors and clean EOF.
- Legacy MCP negotiated 2024-11-05. The future-version proposal negotiated the supported 2025-11-25 fallback; the actual negotiated result shape was measured. This does not claim negotiated 2026-07-28 support.
- Fifteen owned implementation/documentation files match the checked worker. All 424 Rust/Cargo files match the frozen build-7 source. EN/RU CLI and MCP technical blocks were captured from the checked binary.
- Terra narrow code review and bounded runtime-evidence review: PASS, with no remaining ContextPack-only blocker identified. The reviewer inspected records and did not rerun execution.

## Fixed-corpus evaluation

The final pack-only run executed the unchanged thirty tasks twice against one recorded Git snapshot. It produced sixty rows, zero source-boundary/version-substitution/incomplete-negative/pack-contract violations, and six explicit budget refusals. All tasks using the original 2000-token budget retrieved their required content. T21 (350), T26 (500) and T27 (500) refused on both passes; the next whole response required approximately 1744, 875 and 853 estimated tokens respectively. Budgets and tasks were not increased or rewritten.

The benchmark exited nonzero and is not reported as green. Required-content recall was 42/48 (87.5%). It recorded 676 extra references and 228 relevance-level wrong-source hits against narrow task expectations; these are not source-admission violations. Aggregate payload p50 was 39670.5 bytes and latency p50/p95 was 1203.8/1706.8 ms for complete automatically continued multi-page tasks, not individual pages. Each emitted page was checked against its own actual cap.

The earlier three-mode BGE-M3 baseline and this pack-only run have identical frozen corpus content but different Git vectors. They do not establish a new same-snapshot three-mode F10 acceptance. The expensive baseline was not repeated.

## Interpretation

The observations support SPEC-010's exercised declared closure, whole-body citations, bounded delivery, graph navigation and source boundaries. Whole-body units are the chosen conservative unit profile; a refusal does not prove that no smaller permissible whole unit could fit. Broad lexical retrieval still produces relevance noise. This evidence does not accept the complete SPEC or the wider Constellation release: same-snapshot F10 comparison, F11 and optimized release-binary acceptance remain separate, unclosed criteria. Windows local execution was explicitly waived by the user.

## Congruence Level Justification

CL3 means the same native platform and identified implementation context, not formal proof or exhaustive coverage. Build-6's 2000-token fixture failure is retained; compact JSON fixed it in build-7 without changing budgets. Native-5 failed because the private probe used `mermaid` instead of the actual `mermaid_navigation` field; native-6 corrected only that probe and passed against the same frozen binary. Earlier source and protocol failures are retained rather than overwritten.

## Related Artifacts

- SPEC-010: bounded federated evidence units and continuation; not fully accepted by this evidence alone.
- EPIC-010: wider Constellation program; not accepted here.

## Private Evidence Locations

- `.local/contextpack-c7/build-7/result.json` and `source-freeze.json`.
- `.local/contextpack-c7/native-6/result.json`.
- `.local/contextpack-c7/f10-pack-2/report-20261113-174843.md`, raw JSONL and metadata.
- `.local/contextpack-c7/integration-proof-7.json`.
- `.local/contextpack-c7/review-4.json` and `review-5.json`.
