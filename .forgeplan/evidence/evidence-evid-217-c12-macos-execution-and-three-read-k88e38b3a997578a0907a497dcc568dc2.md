---
assigned_number: 206
author: null
created: 2026-11-13
depth: tactical
id: EVID-217
kind: evidence
links:
- target: epic-constellation-contextpack
  relation: informs
parent_epic: null
predicted_number: 206
slug: evidence-c12-macos-execution-and-three-read-k88e38b3a997578a0907a497dcc568dc2
status: draft
tags: []
title: C12 macOS execution and three read-boundary release blockers
updated: 2026-11-13
valid_until: null
---

---
assigned_number: 206
created: 2026-11-13
predicted_number: 206
slug: evidence-c12-macos-execution-and-three-read-k88e38b3a997578a0907a497dcc568dc2
updated: 2026-11-13
---

# C12 verification and release-blocking read defects

## Structured Fields

evidence_type: measurement
verdict: refutes
congruence_level: 3

## Measurement

This records the frozen, uncommitted C12 candidate, not the subsequent corrective work. C12 source manifest SHA-256: `64105034f5026a4f0cdcb4b93697965f6d0aed3a9746b016cdf78614ba459a86`. Optimized production CLI/MCP binaries were built from C11; C12 changes only an external fault-test fixture. No result commit or release is asserted.

Sol executed the macOS/arm64 checks. Independent final reviewer `openai-codex/gpt-6-astra:high` inspected source and retained receipts without rerunning execution.

## Result

- Optimized native and semantic builds and their real CLI/MCP probes passed. Targeted regressions passed: 65 migration, 21 pack, six actual cached-model oracle, five CLI and five read-MCP tests.
- C12 fault target: 11 passed, zero failed or ignored. Earlier failed runs remain preserved. SIGKILL recovery is not power-loss verification.
- Real semantic CLI in Y versus full MCP started in X with explicit Y returned 12 matching vector results and scores; clean EOF. Read-profile clients separately proved lexical scope and direct-write refusal, not vector search.
- The same-snapshot 180-row F10 experiment exited 1: six explicit insufficient-budget refusals, pack required-content delivery 42/48 versus 48/48 in local and federated modes. Zero measured boundary/version/cap/citation violations does not make delivery green. Relevance noise remains visible.
- Current scoped site build/browser checks passed: 608 HTML files, 124644 internal links, zero broken links; desktop and mobile surfaces inspected.
- Astra final whole-result verdict: **BLOCKER**. Three supported static defects remain: B1 scope-bound get/search/graph/impact check before reading but do not bind the actual served capture; B2 live graph/impact bypass the existing source-root guard; B3 successful/ambiguous federated get discards unavailable-source information. These are static findings, not newly executed reproductions.

## Interpretation

C12 is not accepted as release-ready. Correct B1-B3 and verify their behavioral regressions before a new verdict. Historical C6 results remain qualified evidence, not a fresh C12 full-suite run. Genuine unexecuted acceptance subcriteria, F10 outcomes and applicable formal coverage/review conditions remain open. Windows local execution was owner-waived. Publication and installation are not authorized.

## Congruence Level Justification

CL3 identifies the same macOS implementation and frozen inputs. It does not imply exhaustive coverage or erase the final BLOCKER. Raw commands, source/binary bindings and full reviews remain in private `.local/contextpack-c7/final-sol-5`, `final-sol-7`, `docs-runtime-final` and `final-review-inputs/astra-c12-whole-result.md`.

## Related Artifacts

- EPIC-010: current Constellation release slice only; no claim that the entire program is accepted.
