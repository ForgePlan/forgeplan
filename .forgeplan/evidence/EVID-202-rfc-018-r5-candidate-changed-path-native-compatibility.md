---
assigned_number: 202
author: null
created: 2026-10-27
depth: tactical
id: EVID-202
kind: evidence
links:
- target: RFC-018
  relation: informs
parent_epic: null
predicted_number: 202
slug: evid-rfc-018-r5-candidate-changed-path-native-compatibility
status: draft
tags: []
title: 'RFC-018 R5 candidate: changed-path native compatibility'
updated: 2026-10-27
valid_until: null
---

---
assigned_number: 202
created: 2026-10-27
predicted_number: 202
slug: evid-rfc-018-r5-candidate-changed-path-native-compatibility
updated: 2026-10-27
---

## Verdict

verdict: supports
review_verdict: CONCERNS
congruence_level: 3
evidence_type: test
claim: The R5 frozen semantic-capable candidate passes the user-prioritized changed-path checks for disposable pilots.

**Candidate smoke passed; not full acceptance, formal activation, or release.** Parent RFC-018 remains draft. Pin `5a7523e32a66bdf9de7d80205619c197dae688a726b4a09ca45cd3c310e02785`, 442 inputs. Actual verifier: openai-codex/gpt-6.1-sol, configured max, fallback disabled.

## Executed evidence

- Full default MCP `constellation_local_compat`: **22 passed / 0 failed / 0 ignored**, one nonempty test binary; routing/init amendments actually executed. No duplicate target run.
- Frozen fmt check and already-started semantic workspace all-targets check: **exit 0**.
- Fresh no-helper production CLI + standalone MCP: default and semantic builds **exit 0**, immutable SHA/profile copies preserved. Default build had already completed before the semantic-candidate correction, so the additional semantic build was necessary; the selected candidate is `production-semantic/`, not the default-only copies. Both semantic executables have semantic-search enabled, debug assertions enabled, debuginfo=0. CLI reports 0.39.0+constellation.s1.1; standalone version is captured from native initialize.
- Actual fresh semantic CLI reads/searches genuinely indexed nonempty Y and preserves X.
- Both fresh semantic full stdio hosts: actual absolute and bare-name Y native playbook searches return only Y PRD-001/Auth rewrite; same literal PRD-001 claim A/conflict B/release succeeds; outside and escaping-symlink targets and missing consent are refused. Eight request events per routing host appear only in Y (4 ok/4 tool_err), arguments private. Full X inventory including logs, directories, symlinks and lock files is unchanged **after completed startup baseline**.
- Both full hosts also pass init with startup X and valid env Y, separately with no workspace argument and with schema-unsupported raw workspace:Y. Physical workspace/event belong to X; all post-startup Y bytes/logs remain unchanged. **6 authoritative native full-host sessions / 32 JSON-RPC requests / 20 tool calls; all EOF exit 0.**

## Measurement qualifications

Two raw failed probes are preserved, not suppressed: an overlong manual actor label correctly hit the 64-byte guard; an initial inventory was taken before the asynchronous startup scan created its owner-lock entry. Corrected valid input and an explicit completed-zero-commit-scan barrier produced the authoritative passes. No file/lock/log was excluded from the final comparison. Raw total: 8 sessions, all EOF0; do not confuse the two invalid probes with the six authoritative sessions or with product regressions. See `runtime-cases.json` and each raw `runtime/*.json` plus stdout/stderr.

## Exact command and provenance receipts

`candidate-result.json` contains every exact Cargo command, native exit, duration and native df before/after. Raw Cargo stdout/stderr: `01-fmt.*`, `02-mcp-local-default.*`, `03-check-semantic.*`, `04-build-default-production.*`, `05-build-semantic-production.*`. Native argv/environment/responses/exits and source+binary+profile bindings are retained in `runtime-ledger.json` and `runtime/*.json`. Fresh binary SHA/profile records: both build `*-preserved-binaries.json` and `SAFELEASE_RELEASE.json`. No 0/0 run is labelled a test pass; no failing test was retried unchanged.

## Deferred and unavailable

Direct user speed-priority change defers strict clippy, full default/semantic unit/bin/doc/integration/schema/boundary matrices, unchanged core/KB/CLI-FPF/process suites, and the fresh BGE/reindex matrix to **DEFERRED_TO_PRE_RELEASE**. Semantic capability is built, but no fresh automatic-embedding runtime proof is claimed: these short-smoke fixtures have no active model profile. R4 68PASS and R3 real BGE/both-host autoembedding remain correctly pinned historical evidence, not R5 runs. Provider usage, LLVM coverage (AC >=80%; actual/delta unmeasured), and G1/G2/G3 remain unavailable/unwaived. No model download, paid call, tool installation, live-project/config change, canonical publication, activation or rollout occurred.

## Safe release

**SAFELEASE_RELEASE:** 442/442 hashes and aggregate pin unchanged; **0 compiler / 0 owned runtime processes**; global model hashes and preserved candidate binary hashes unchanged. Raw native process and disk proof are in `SAFELEASE_RELEASE.json`; source hash proof is `source-hashes-release.json`. Parent owns any later publication.
