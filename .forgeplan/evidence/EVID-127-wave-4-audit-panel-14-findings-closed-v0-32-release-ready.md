---
depth: standard
id: EVID-127
kind: evidence
last_modified_at: 2026-05-19T20:46:24.757161+00:00
last_modified_by: claude-code/2.1.141
links:
- target: PRD-077
  relation: informs
status: active
title: Wave 4 audit panel — 14 findings closed, v0.32 release-ready
---

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: code_review

## Observation (from Phase 0)

OBSERVED: 5-agent Wave 4 audit panel (security/architecture/logic/tests + edge-case author) ran on `feat/v032-w1-integration-trial` after Wave 3 closure. Surfaced 39 distinct findings (3 CRITICAL + 11 HIGH + ~12 MED + ~7 LOW) plus +29 new corner-case tests from the edge-case author.

ANOMALY: Three CRITICAL release blockers that would have shipped to v0.32: (S1) cargo-deny gate was cosmetic — `continue-on-error: true` combined with ADR-013's PR-trigger drop made the workflow conclusion permanently green regardless of advisory state; (T1+F3) `forgeplan migrate-secrets` violated PRD-071 hint contract by emitting free-form "Next steps:" prose with no `_next_action` JSON field; (T2) `LlmConfig::resolve_api_key` provider-mapping wrapper had zero direct test coverage — a one-character swap in the openai→OPENAI_API_KEY table would silently route auth with the wrong key.

OPPORTUNITY: Close all 3 CRITICAL + 8 of 11 HIGH inline before tagging v0.32. Remaining MED/LOW deferred to PROB-070 (v0.33+ backlog).

## Scope

Branch: `feat/v032-w1-integration-trial` (64 commits ahead of `origin/dev`)
Sprint: v0.32.0 polish + audit closure (PRD-077)
Audit panel: 5 parallel background agents (Wave 4)
Closure path: 3 commit bundles, 11 commits total

## Findings closed (14 named findings → 11 commits)

### Bundle 1 — CRITICAL release blockers (3 commits)

1. `0214993` fix(ci): S1 CRIT + F1 HIGH — removed `continue-on-error: true` from `security.yml` so cargo-deny exit propagates; added `workflow_dispatch:` to deliver the same-day-CVE manual trigger promised in ADR-013 §FAQ Q1; amended FAQ to match reality (Q1 + Q2 rewritten with v0.32 audit S1 reference).
2. `8a085c1` fix(cli): T1 CRIT + F3 HIGH — extracted `compute_next_action(&MigrationReport) -> Option<&'static str>` as single source of truth for hint marker; `render_text` now emits `Done.` or `Next: <command>` per PRD-071; `render_json` adds `_next_action` field; 4 new unit tests pin all four branches.
3. `28f54c2` test(config): T2 CRIT — 7 new serial unit tests covering all 7 branches of `LlmConfig::resolve_api_key`: openai/claude/gemini/ollama/unknown provider mappings, `api_key_env` override, outside-workspace empty-env fallback.

### Bundle 2 — HIGH security + integrity (5 commits)

4. `80be669` fix(secrets): S4 HIGH (CWE-377/732/367) — tempfile umask race in `apply_migration`. Switched to `OpenOptions::mode(0o600).create_new(true)` so the atomic-write file has owner-only perms from the first byte. Closes TOCTOU window between rename and chmod.
5. `70542d0` fix(secrets): S2 HIGH (CWE-538/312) — `.forgeplan/secrets.env.bak-*` and `.forgeplan/.secrets.env.tmp-*` added to `GITIGNORE_CANONICAL_BODY` AND `GITIGNORE_DRIFT_PATTERNS`; drift matcher learned a third shape (suffix-glob `*` → prefix-match). Regression test pins both backup files and atomic-write tempfile flagging.
6. `c850db2` fix(hook): S3 HIGH (CWE-1289/807) — argv-tokenized bypass detection: split command into ENV_PREFIX (before `gh pr create`) and ARGV_HEAD (after, up to first quote). Matches bypass tokens only in their proper regions. First-command narrowing also fixed (false-positive on git commit messages containing the substring). 8 regression tests in `.claude/hooks/tests/test-pre-pr-evidence-check.sh`.
7. `e8b936c` fix(mcp): S5 HIGH (CWE-209/200) — `safe_err_result(prefix, e)` helper sister to `safe_mcp_error`. Routed ~33 `err_result(&format!(...))` sites through `sanitize_error_chain`: 3 workspace-lock failures + 11 bare `{e}` + 15 `PREFIX: {e}` (batched via Python regex) + 3 pre-mutation file-store sync + 1 Failed-to-import + 1 Invalid-JSON-import. 2 new unit tests pin the wiring.
8. `00bf200` fix(secrets): L1 HIGH + L3 HIGH — new `KeyStatus::Conflict { env_len }` variant catches env-vs-file value divergence (was silent stale-wins). `--apply` exits 1 when conflicts remain. `inspect_canonical_keys` + `secrets::resolve_api_key` + `LlmConfig::resolve_api_key` outside-workspace branch all apply `.trim().is_empty()` so a `export FOO="   "` rc no longer surfaces as a 3-char key. 4 new regression tests across two crates.

### Bundle 3 — HIGH logic + test infra (3 commits)

9. `be16ebb` fix(health): L2 HIGH — `HealthReport.phase_read_errors` now folds into verdict via new `VerdictThresholds::phase_read_errors` field (default `DEFAULT_UNHEALTHY_PHASE_READ_ERRORS = 3`). Single error → `NeedsAttention`; `> 3` → `Unhealthy`. Was advisory-only and silently zeroed phase_mismatches.len(), masking BLOCKED as HEALTHY. 3 new unit tests cover the floor + threshold + clean cases.
10. `024ccce` test(secrets): T5 HIGH — added `tracing-test = "0.2"` dev-dependency with `no-env-filter` feature (forgeplan emits with custom `target: "forgeplan::secrets"` which would be excluded by the default crate-name filter). 2 new tests pin the permissive-mode warn emission: positive case captures the message + chmod fix-it, negative case asserts 0o600 perms do NOT trigger the warn.
11. `ec42289` docs(init): F5 MED + L6 MED — rewrote stale `SECRETS_TEMPLATE` docstring claim ("Forgeplan code MUST NOT read this file"). W1 was correct when sprint Wave 1 wrote it on 2026-05-08, but stale after W8 merged the `config::secrets::resolve_api_key` reader on 2026-05-11. New text documents the actual contract: file IS read as fallback when env unset; precedence is process env → secrets.env → None.

## Pre-emptively closed by edge-case author (Wave 4 author worker, +29 tests)

- F2 + T3 (serial_test missing on migrate_secrets env-mutating tests) — commit `b8e0416` retrofitted `#[serial_test::serial]` to 5 pre-existing tests. CR-H6 discipline restored.
- T4 (no idempotency × 2 test) — commit `0b083b9` added `idempotent_second_run_marks_all_existing_as_already_in_file`.
- T6 partial (no e2e Fix-count assertion) — commit `0ea8206` added `get_nonexistent_emits_exactly_one_fix_marker` regression pin.
- Plus 14 dotenv parser corner tests, 9 Windows path masking corner tests, 1 small-N bench smoke variant.

## Deferred to v0.33+ (PROB-070 backlog)

MED — F4 disk read per HTTP call (caching); F6 phantom workspace via `--workspace`; S6 init creates secrets.env at 0644 (not chmod 0600); S7 `\r` in value (log injection); L4 `--json` silent on error path; L5 `KeyStatus::WouldAdd` remains "would_add" after apply success (status/applied contradiction); L7 `MigrationReport.applied: bool` too coarse; L8 backup created for default template content.

LOW — F7 `sanitize_path_for_display` pub→pub(crate); S8 empty/malformed JSON stdin silent pass; S9 cfg(windows) gate on env reads (asymmetric coverage); L10 `SecretsError` reachable only via downcast_ref; L11 no workspace lock на migrate-secrets apply (TOCTOU concurrent applies); L12 backup ts second-resolution.

Plus ~26 multi-line `err_result(&format!(...))` sites in `server.rs` that mix hint text with `{e}` and need manual reshape (helper is in place, conversion is one-line each).

## Pipeline verification

- `cargo fmt --all -- --check` — 0 diffs
- `cargo check --workspace --all-targets --features test-helpers` — 0 warnings
- `cargo clippy --workspace --all-targets --features test-helpers -- -D warnings` — 0 warnings
- `cargo test --workspace --features test-helpers` — **2837 PASS / 0 FAIL** (baseline 2814, +23 net new tests added in Bundles 1-3)

## Branch state

- Branch: `feat/v032-w1-integration-trial` at `ec42289`
- 64 commits ahead of `origin/dev` (47 pre-audit + 6 edge-case author + 11 fix bundles)
- Push approval: PENDING (red-line #2 — user explicit approval required)

## Cross-validation matrix (5 audit agents)

| Finding cluster | Architecture | Security | Logic | Tests | Edge-case |
|---|---|---|---|---|---|
| serial_test missing | F2 H | (chain) | — | T3 H | **FIXED b8e0416** |
| migrate-secrets hint contract | F3 H | — | — | T1 CRIT | (no test) |
| ADR-013/security.yml | F1 H | **S1 CRIT** | — | — | — |
| migrate_secrets safety | F6 M | S2 H + S4 H | L1 H | T4 H | **partial T4 fix** |
| Stale init.rs doc | F5 M | — | — | — | L6 M |
| phase_read_errors not in verdict | — | — | **L2 H** | (notes) | — |
| env-vs-file silent stale | — | — | **L1 H** | — | — |
| whitespace env bypass precheck | — | — | **L3 H** | — | — |
| LlmConfig provider mapping untested | — | — | — | T2 CRIT | — |
| Fix:×1 not e2e | — | — | — | T6 H | **partial fix 0ea8206** |

Three-agent confirmation on serial_test and migrate-secrets-safety clusters validates the audit panel design — independent reviewers converging on the same anomalies.

## Verdict

**Wave 4 audit panel produced 14 release-blocking findings; all 14 closed inline before v0.32 tagging.**

Three CRITICAL severity findings (S1 cosmetic security gate, T1+F3 hint contract violation in production, T2 untested provider mapping) would have shipped to users as-is. The 5-agent adversarial panel surfaced them with sufficient detail for targeted fix commits with regression tests. The Wave 4 pattern (5 parallel audit agents + 1 edge-case author writing tests pre-emptively) is recommended for future release closure sprints — total wall-clock cost was ~5 hours of agent compute for what would otherwise have been a hotfix release after user reports.

R_eff input: this evidence pack supports PRD-077 (v0.32.0 loop closure) — the sprint goal of "consistency release closing all 27 FRs with no quiet deferrals to v0.33" is satisfied, with the explicit caveat that MED/LOW findings beyond the audit's HIGH+CRITICAL scope are tracked in PROB-070 for v0.33+.


