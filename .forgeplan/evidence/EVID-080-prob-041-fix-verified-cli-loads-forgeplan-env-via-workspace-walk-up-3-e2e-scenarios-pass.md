---
created: 2026-04-20
depth: tactical
id: EVID-080
kind: evidence
links:
- target: PROB-041
  relation: supports
status: draft
title: PROB-041 fix verified — CLI loads .forgeplan/.env via workspace walk-up, 3 E2E scenarios PASS
updated: 2026-04-20
---

# EVID-080: PROB-041 fix verified — 3 E2E scenarios PASS

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Measurement

Branch `fix/prob-041-dotenv-workspace-discovery` на актуальном `dev` (после merge #199). Fix в `crates/forgeplan-cli/src/main.rs`: добавлен `load_workspace_env()` helper который через `forgeplan_core::workspace::find_workspace(cwd)` walk-up'ит до `.forgeplan/` и вызывает `dotenvy::from_path(ws.join(".env"))` перед стандартным `dotenvy::dotenv()`.

Method:
1. `cargo fmt --all --check` — clean
2. `cargo clippy --workspace --all-targets -- -D warnings` — clean
3. `cargo test --workspace` — 1405/1405 PASS (0 regressions)
4. `cargo build --release --bin forgeplan` — success
5. Real binary E2E из 3 директорий с реальным `NEURALDEEP_API_KEY` в `.forgeplan/.env`:
   - workspace root
   - subdir `crates/forgeplan-core/` (тест walk-up)
   - `/tmp` (outside workspace — graceful fallback)

## Result

**Все 3 E2E scenario PASSED**:

| Scenario | Expected | Actual |
|---|---|---|
| AC-1: workspace root + valid `.forgeplan/.env` | Level 2 (LLM) | **Level 2 (FPF reasoning)** ✅ |
| AC-2: subdir `crates/forgeplan-core/` (walk-up) | Level 2 | **Level 2 (FPF reasoning)** ✅ |
| AC-3: outside workspace (`/tmp`), no env | Level 0 fallback, no crash | **Level 0 (keywords)** ✅ |
| AC-4: shell env var overrides workspace `.env` | Shell wins | Confirmed — `dotenvy::from_path()` не override'ит уже set env var |
| AC-6: Все тесты PASS | 1405 tests pass | **1405/1405 PASS, 0 failed** ✅ |

AC-5 (MCP server `forgeplan serve` inherits env) — implicit следствие: `forgeplan serve` проходит через CLI main.rs → `load_workspace_env()` вызывается → subprocess получает env. Требует end-to-end MCP call для explicit проверки (deferred, не блокер).

**Diff size**: 14 LOC added (1 new helper + 3-line call site change in `main.rs`).

**Tooling quality**:
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo fmt --all --check` — clean
- 0 new warnings

## Interpretation

PROB-041 root cause (dotenvy ищет только cwd) закрыт с наименьшим возможным blast radius: один файл, один helper, zero new dependencies (forgeplan_core::workspace уже в graph). Precedence design (shell > workspace .env > cwd .env) сохраняет backward compat для кейсов где users уже `export`'или env vars в shell.

Fix также unblocks параллельно наблюдавшуюся проблему из другой сессии (aod-worker brownfield migration) где user тратил время на выяснение почему Level 0 всегда. Теперь zero-friction: положил `.env` в `.forgeplan/`, CLI сразу видит.

## Congruence Level Justification

CL3 (same-context measurement): тесты запущены на actual branch under fix, actual binary (release profile), actual workspace с actual API key. 3 из 3 AC scenarios проверены на боевом endpoint (neuraldeep.ru — gpt-oss-120b). Не research, не proxy — прямое измерение.

Penalty CL3 = 0.0 (exact match).

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| PROB-041 | Problem | supports (verifies fix) |
| PROB-022 | Problem | informs (brownfield onboarding unblocked) |


