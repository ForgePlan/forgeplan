---
depth: tactical
id: EVID-140
kind: evidence
links:
- target: PRD-078
  relation: informs
- target: ADR-016
  relation: informs
- target: ADR-015
  relation: informs
- target: PROB-072
  relation: informs
status: draft
title: 'PRD-078 closure: 7-layer verify (3130 tests) + Layer-7 audit (0 HIGH+) + all real findings fixed'
---

## Summary

Closure evidence for **PRD-078** (MCP worktree-aware projection routing) and its structure record **ADR-016** (single resolution chain + `DetectionPolicy` + collapsed store). Records the independent 7-layer verification and the Layer-7 adversarial workflow audit (25 agents) plus remediation of every confirmed real finding. Branch `feat/prd-078-integration` @ `478e35a`, 22 commits ahead of `origin/dev`.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

CL3: these are direct measurements of the exact shipped implementation (same context). `verdict: supports` covers the PRD's functional + backward-compat thesis (AC-1..4, SC-1, SC-3); the one open performance NFR (SC-2) is explicitly flagged below as *not measured*, not refuted.

## Verification — 7 layers, each re-run independently

Agent self-reports were explicitly distrusted; every gate below was re-executed by the orchestrator. This caught real regressions agents had reported green (prob060 env-collision, e2e P2 flaky, 4 config-axis leaks).

| Layer | Gate | Result |
|---|---|---|
| 1 Compile | `cargo clippy --workspace --all-targets` (compiles every target incl. core integration tests) | exit 0 |
| 2 Lint | `cargo clippy --workspace --all-targets -- -D warnings` | **0 warnings** |
| 3 Full suite | `cargo test --workspace` (ambient API keys unset → CI-like) | **3130 passed, 0 failed** |
| 3′ Affected surface | `cargo test -p forgeplan-mcp` | 255 passed, 0 failed |
| 4 Code read (grep) | 0 residual `require_workspace()`; mutating handlers untouched; no second resolution chain | confirmed |
| 5 Journey-1 e2e | `worktree_routing_e2e.rs` (write) + `worktree_read_e2e.rs` (read) + `worktree_error_e2e.rs` (error gate) — real JSON-RPC | green |
| 6 Backward-compat | same 3130 suite + P2 `no-param-uses-default` e2e | green |
| 7 Adversarial audit | workflow, 4 dimensions × refute × synthesis | see below |
| fmt | `cargo fmt --check` | 0 diffs |

## Layer-7 adversarial audit outcome

- Dynamic workflow: 25 agents, ~2.7M tokens, 4 review dimensions (security / architecture / correctness / tests); each finding refuted by an independent skeptic, then synthesized.
- Verdict: **CONCERNS / fix-first. 10 confirmed / 10 refuted. 0 confirmed HIGH+** (MEDIUM ceiling) — refactor core is sound.
- Two HIGH-claimed findings refuted to LOW/INVALID by skeptics: score-weights store leak (refuted), "6 paramless boundaries unsafe" (INVALID — documented boundary).
- Every real finding remediated before this evidence (each gate re-run + committed):
  - **ARCH-3**: `detect_multi_worktree` submodule false-positive → fixed via `git --git-dir` ≠ `--git-common-dir` semantics + 2 regression edge tests (submodule, symlink). Commit `a47d5e7`.
  - **4 config-axis cross-workspace leaks** (`forgeplan_score`, `fpf_check`, `estimate`, `fpf_rules`): config now loaded from the RESOLVED worktree, not server default; dead leak-source helper removed. Commit `70f5503`.
  - **SEC-1 / MED-4**: `resolve_workspace_core` errors routed through the `$HOME`/scratch-path sanitizer (`safe_invalid_params`) — no path/username leak in `-32602` responses. Commit `12b2f1f`.
  - **ARCH-2**: ADR-016 amended (Implementation Note) to honestly reconcile the shipped in-place variant (methods on `ForgeplanServer`) vs the literal `WorkspaceResolver` type. Commit `478e35a`.

## Acceptance-criteria mapping

| AC / SC | Evidence | Status |
|---|---|---|
| AC-1 worktree write (param → resolved, no main file) | `worktree_routing_e2e.rs` | MET |
| AC-2 backward-compat (cwd, no error) | `worktree_read_e2e` P2 + 3130 suite | MET |
| AC-3 multi-worktree no param → `-32602` + suggestion | `worktree_error_e2e.rs` | MET |
| AC-4 CI strict via env → `resolved_via=env` | `worktree_read_e2e` E3 | MET |
| SC-1 silent fallback eliminated | e2e Journey-1 (write+read same worktree) | MET |
| SC-3 regression count 3084 → ≥ | **3130 passed** | MET (exceeded) |
| NFR-004 `resolved_via` in response | `server.rs` field present | MET |
| **SC-2 / NFR-001 latency < 5ms p95** | criterion bench (`benches/workspace_detection.rs`) | **NOT MEASURED — deferred to PROB-073** |
| AC-5 / SC-4 deprecate user workaround | empirical on user branch | DEFERRED — post-activate |

## Open items (honest)

1. **SC-2 / NFR-001 (latency bench)** — the PRD lists this as pre-activate; it is NOT measured. The bench file does not exist (`crates/forgeplan-core/benches/` absent). Deferred to **PROB-073** (paired sprint, shared bench infra). Accept-with-deferral rationale: detection is 2 `git rev-parse` subprocess calls, run only on the mutating-call error-gate when `workspace` is omitted; expected well under 5ms, but unmeasured. Tracked: task #5.
2. **AC-5 / SC-4** — empirical verification on the user's pipeline branch; cannot be closed from this repo. Post-activate.
3. Deferred LOW test-hardening (TC-1..4) + SEC-3 (empty-`$HOME`) → follow-up issue per audit synthesis.

## Provenance

- Branch: `feat/prd-078-integration` @ `478e35a` (22 commits ahead of `origin/dev`)
- Test command: `env -u GEMINI_API_KEY -u OPENAI_API_KEY -u ANTHROPIC_API_KEY cargo test --workspace`
- Shared target: `target-prd078-shared` (disk-constrained multi-worktree env, 4 worktrees)
- Verifier discipline: independent re-run of every gate; agent self-reports distrusted by policy.





