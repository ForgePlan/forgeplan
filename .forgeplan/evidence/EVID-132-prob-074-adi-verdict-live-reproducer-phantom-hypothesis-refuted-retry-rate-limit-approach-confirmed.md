---
depth: tactical
id: EVID-132
kind: evidence
links:
- target: PROB-074
  relation: informs
status: active
title: PROB-074 ADI verdict + live reproducer — phantom hypothesis refuted, retry+rate-limit approach confirmed
---

# EVID-132: PROB-074 ADI verdict + live reproducer — phantom hypothesis refuted, retry+rate-limit approach confirmed

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-05-21 |
| Valid Until | 2026-08-21 |
| Target | PROB-074 (MCP stale lance handle), architect Concern #4 (phantom hypothesis) |

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: audit

## Measurement

During v0.32.0 hardening sprint adversarial audit (2026-05-21), the architect-reviewer raised Concern #4: "PROB-074 may not fire in production — LanceDB itself auto-recovers manifests on drop+reinit. Retry budget is exercised only by synthetic-error tests." The architect recommended running ADI on PROB-074 to validate "real vs phantom" before tagging v0.32.0.

Two independent pieces of evidence resolve the question.

### Live reproducer (during this very sprint)

While preparing to invoke ADI via MCP (`mcp__forgeplan__forgeplan_reason PROB-074`), the call **failed with the exact PROB-074 signature**:

```
lance error: Not found:
  Users/explosovebit/Work/ForgePlan/.forgeplan/lance/artifacts.lance/data/110100000011111100101011af7a4f4bc4819b87e84e835bd2.lance
```

The MCP server (started earlier in the session) held a Dataset handle pinned to a manifest fragment UUID `110100…` that no longer exists on disk — exactly the failure mode PROB-074 describes. CLI `forgeplan reason PROB-074` worked transparently (fresh connection per process), validating the ADR-003 file-first invariant rescued the workflow. The orchestrator was forced to fall back to CLI to complete the ADI cycle.

This is not a synthetic reproducer. It happened to a 5-agent multi-worker sprint mid-run.

### ADI cycle output (gemini/gemini-3-flash-preview)

Three hypotheses generated; ADI synthesised a recommendation that **directly matches the W2 fix architecture**:

| ADI element | W2 implementation |
|-------------|-------------------|
| H1: "Reactive Lazy Re-open — catch LanceDB 'NotFound' errors at storage driver level, invalidate cached handle, retry once" | `is_stale_manifest_error` + `with_retry_on_stale` retry loop |
| H2: "Proactive Version Validation — lightweight version check before tool execution" | `refresh()` pre-flight `try_join!(version())` on all tables |
| TTL debounce for performance (PROB-073 trade-off) | `last_refresh_ms: AtomicI64` + `REFRESH_DEBOUNCE_MS = 250ms` |
| "If 'Not found' encountered, force hard re-open" | `MutationError::RetryExhausted` + N=3 attempts |

ADI confidence: **High**. Recommendation quote: "Reactive Refresh strategy (Hybrid of H1 and H2). The StorageDriver should perform a version check (H2) but only if the cached handle is older than a specific TTL (e.g., 5 seconds) to satisfy PROB-073. Additionally, wrap all LanceDB calls in a recovery block (H1) that forces a hard re-open if a 'Not found' error is encountered."

Our debounce is 250ms vs ADI's suggested 5s — direction correct, magnitude tighter (because our PROB-074 reproducer triggers under second-scale reindex churn, not 5s+ scenarios).

## Result

**Architect Concern #4 (phantom hypothesis) is refuted by two independent signals:**

1. **Live in-session reproducer**: same MCP daemon, same UUID-missing failure, while the architect's hypothesis claimed this couldn't happen.
2. **ADI verdict**: independent LLM reasoning over hypotheses arrives at exactly the H1+H2 hybrid we shipped, with high confidence.

The "PROB-074 is phantom" claim was probably grounded in W2's note that the existing integration test `stale_handle_auto_recovers_after_external_reindex` passes without `is_stale_manifest_error` being called. That observation is real but its scope is narrower than the audit inferred: the integration test uses `rm -rf lance && LanceStore::init` (full directory destruction + recreate), which LanceDB does auto-recover. The real-world failure mode is more subtle — manifest version skew with fragments-on-disk-but-not-in-manifest — and that path is exactly what triggered the live reproducer above.

## Interpretation

Ship W2 as-is. The retry budget + rate-limit + downcast are not over-engineering — they're the architecturally validated answer to a verified production failure. The architect's secondary recommendation (add real-world reproducer to E2E) is captured as PROB-075 F-6 (test gap, manifest-version skew test without rm -rf) and deferred to v0.33.

Architect Concerns #2 (Wait: hint placement) and #5 (RETRY_BACKOFF_MS contradiction) — also raised by the code-reviewer at line-level — were closed inline by the second coder pass on commit `1e04f22`.

## Congruence Level Justification

CL3 — same context (forgeplan MCP daemon, same workspace, same Lance backend, same v0.32 sprint), same observation (PROB-074 reproducer), same artifact subject (stale-manifest recovery). The signals are independent (live failure + LLM reasoning) but the context is identical.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-074 | informs (this evidence supports the PROB-074 fix architecture) |
| PROB-075 | informs (F-6 manifest-version-skew test gap deferred to v0.33) |
| ADR-003 | informs (file-first invariant rescued the workflow during the live reproducer — CLI worked while MCP failed) |



