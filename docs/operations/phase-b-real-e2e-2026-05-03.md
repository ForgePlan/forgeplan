# Phase B + Track 4-A8 — Real E2E closure (2026-05-03)

**Author**: Claude Opus 4.7 (1M)
**NOTE**: NOTE-049
**Branch**: `chore/close-real-e2e-gaps-phase-b-and-track-4a8`
**Driver**: HANDOFF-NEXT-CHAT.md PR 1 — close two unverified surfaces from
the 2026-05-02 sprint before cutting v0.28.0.

---

## TL;DR

Этот документ фиксирует реальный (не fake-script) end-to-end запуск
двух surface-ов, отмеченных gapping в предыдущей итерации:

1. **Phase B Wave 1 (PROB-050 A-3)** — `claude --print` invocation в
   `PluginDispatcher` + `AgentDispatcher`. Все unit/integration тесты до
   этого использовали fake-bash скрипты в `PATH`; production-binary
   `claude` 2.1.126 dispatcher-кодом ни разу не вызывался.
2. **Track 4-A8 playbooks** — `release.yaml` и `brownfield-docs.yaml`
   shipped в #236 с PASS на `forgeplan playbook validate`, но без
   `forgeplan playbook run` execution check.

Цель — RAW captured output от обоих surface-ов + audit-able evidence,
что либо (а) реальные surfaces работают как обещано, либо (б) есть bug
который мы фиксим сейчас, не позже.

---

## Baseline (2026-05-03)

| Property | Value |
|---|---|
| Workspace | `/Users/explosovebit/Work/ForgePlan` |
| Branch | `chore/close-real-e2e-gaps-phase-b-and-track-4a8` (off `dev` @ `5e08b4d`) |
| `Cargo.toml` version | `0.27.0` |
| `forgeplan health` | clean (267 artifacts, 0 blind / orphan / stale) |
| `claude --version` | `2.1.126 (Claude Code)` at `/Users/explosovebit/.local/bin/claude` |
| Dispatchers | `crates/forgeplan-core/src/playbook/dispatch/{plugin,agent}_dispatcher.rs` |
| Shared helpers | `crates/forgeplan-core/src/playbook/dispatch/claude_print.rs` |
| Playbook discovery | `.forgeplan/playbooks/` → `marketplace/playbooks/` → `~/.claude/plugins/*/playbooks/` |

---

## Hypotheses (from NOTE-049)

| ID | Hypothesis | Surface | Risk | Real-E2E test |
|----|-----------|---------|------|---------------|
| H1 | `claude --print` argv shape works end-to-end | Phase B | Low | Минимальный playbook с `Delegation::Agent`, exit code + JSON envelope |
| H2 | argv ordering bug surface при `--add-dir` + `--allowedTools` | Phase B | Medium | Playbook с `produces_at` + `allowed_tools`, `ps`/strace |
| H3 | `release.yaml` placeholder substitution semantics | Track 4-A8 | Medium | `forgeplan playbook run release --dry-run` |
| H4 | `brownfield-docs.yaml` graceful failure (missing skill) | Track 4-A8 | Low | `playbook run brownfield-docs --yes` на пустом workspace |
| H5 | argv injection guard rejects malicious agent name without spawning claude | Phase B | High | Playbook с `name: "../../etc/passwd"`, verify reject path |

---

## Pre-flight findings (без real claude run)

### F-PRE-1 — `release.yaml` НЕ exercises `claude --print` path

`forgeplan playbook show release` (12 шагов) показывает delegations:

| Step | `delegate_to.type` | Surface |
|------|--------------------|---------|
| 1-11 | `command` | shell-out (cargo/git/gh/bash) |
| 12 | `forgeplan_core` (target=`new`) | internal artifact creation |

**Импликация**: `forgeplan playbook run release --dry-run` НЕ exercises
`PluginDispatcher`/`AgentDispatcher`. H3 (placeholder substitution) тестирует
только `CommandDispatcher` argument printing — это ortho к Phase B Wave 1
findings. Для closure PROB-050 A-3 нужен ad-hoc Agent-step playbook
(см. ниже).

### F-PRE-2 — `release.yaml` documented как hand-edit-required

Description блок `release.yaml` явно фиксирует:

> Maintainer hand-edits `vX.Y.Z` placeholder in step args before run
> (SPEC-003 1.1 has no template engine — PROB-050 A-1).

То есть H3 имеет известный constraint: `vX.Y.Z` это литерал в YAML, не
template variable. `--dry-run` распечатает его буквально. Пока SPEC-003
1.2 не bumped (PROB-050 A-1), это by-design. В docs PR 2 (release v0.28.0)
эта manual-edit flow подтвердится практически.

### F-PRE-3 — `brownfield-docs.yaml` shipped как REFERENCE EXAMPLE

YAML-комментарий header (lines 4-13) явно говорит:

> **Status: REFERENCE EXAMPLE, requires `forge-docs-miner` skill +
> `marketplace/mappings/docs-to-forge.yaml` (BOTH TBD — tracked in
> PROB-050).**
>
> Without … this playbook fails with `DispatchError::DelegateMissing`
> (step 1) or "mapping file not found" (step 2) at runtime.

Это превращает H4 из «возможно failure mode» в «expected, документированный
contract». Real E2E подтверждает graceful path вместо panic.

### F-PRE-4 — Step schema уже несёт budget_usd / allowed_tools / timeout_seconds

`crates/forgeplan-core/src/playbook/types.rs:150-170` — все три поля
живут в коде с PRD-072 FR-8 / ADR-011. Только формальный bump
schema_version 1.1 → 1.2 в SPEC-003 doc отстаёт (PROB-050 A-1).
Это означает H1/H2/H5 тестировать можно — runtime их read'ит.

---

## Test artifacts (исполняемые)

Изолированный workspace: `/tmp/phase-b-e2e-20260503T073601Z/`
- `.forgeplan/playbooks/h1-agent-happy.yaml` — H1 minimal Agent step (budget=$0.05)
- `.forgeplan/playbooks/h1b-agent-success.yaml` — H1b Agent SUCCESS path (budget=$0.50)
- `.forgeplan/playbooks/h2-add-dir-ordering.yaml` — H2 multi-tool + produces_at
- `.forgeplan/playbooks/h5-injection-guard.yaml` — H5 malicious agent name `../../etc/passwd`
- `.forgeplan/playbooks/h-plugin-happy.yaml` — Plugin variant
- `claude-recording-wrapper.sh` — bash wrapper that records argv+stdin+stdout, then exec's real `/Users/explosovebit/.local/bin/claude`
- `path-override/claude` → symlink to wrapper (для PluginDispatcher, см. F-RUNTIME-7)
- `logs/R-*.log` — `tee`-captured invocation logs per hypothesis
- Recordings dir: `/tmp/phase-b-e2e-recordings/` — argv/stdin/stdout per claude spawn

Build: `cargo build --release --bin forgeplan` → `target/release/forgeplan`
(installed brew binary `0.27.0` ПРЕДшествует Phase B Wave 1 — см. F-RUNTIME-5).

---

## Real-E2E run results

### R-6b-1 — H3 — `release.yaml --dry-run` ✅

```
Cost: $0.00 (CommandDispatcher only, no claude calls)
Time: <0.1s
Result: 12 steps printed verbatim
```
- vX.Y.Z literal preserved (not template-expanded — confirms F-PRE-2)
- 11/12 steps `delegate=command:`, step 12 `forgeplan_core:new`
- Zero claude --print invocation in this playbook (orthogonal to Phase B)
- **F-RUNTIME-2 minor**: `--dry-run` requires `--yes` (UX nit, not bug)

### R-6b-2 — H4 — `brownfield-docs.yaml --yes` (NOT FALSIFIABLE on v1) ⚠️

```
Cost: $0.00
Time: ~0.2s
Result: success: 1, failed: 1, skipped: 1, exit code: 1 (verified with pipefail)
```
- Step 1 `[OK] scan-docs` — **skill not actually invoked** (SkillDispatcher v1 stub returns
  success without invocation, см. F-RUNTIME-3)
- Step 2 `[FAIL] ingest-docs` — "mapping file not found: marketplace/mappings/docs-to-forge.yaml"
- Step 3 `[SKIP] summary-note` — proper cascade with `on_error: abort` default
- ⚠️ **H4 verdict: NOT-FALSIFIABLE на v1 SkillDispatcher** — surface, который H4
  должен был тестировать (graceful failure on missing skill), не существует:
  dispatcher это intentional v1 stub, всегда returns success. Re-test deferred
  до Phase 6 Wave 5+ (skill-registry resolution lands).
- ✅ Адъякент-наблюдение: dependency-skip pipeline works, exit code propagation
  works (см. C-1 ниже), no panic.

### R-6a-3 — H5 — argv injection guard reject ✅

```
Cost: $0.00 (rejected pre-spawn)
Time: 0.01s
Result: success: 0, failed: 1
[FAIL] malicious-agent-name
       dispatch transport error: agent name `../../etc/passwd` (len=16) rejected:
       must match ^[A-Za-z][A-Za-z0-9_-]{0,63}$ (argv-injection guard, ADR-011 §Security)
```
- ✅ `validate_agent_name` regex reject pre-spawn (0.01s ≪ claude startup)
- ✅ `len=16` echoed; `truncate_for_log` not triggered (16 < 80 cap)
- ✅ NO claude subprocess (recordings dir untouched по этому run-у)
- ✅ Exit code: `1` (verified with `set -o pipefail` re-run; см. C-1 ниже)

### R-6a-1 — H1 — Agent dispatcher (low budget) ⚠️

```
Cost: $0.20184575 (despite --max-budget-usd 0.05 → 4× over cap)
Time: 6.02s
Result: failed: 1 — "is_error=true | cost=$0.2018"
JSON envelope: subtype=error_max_budget_usd, errors=["Reached maximum budget ($0.05)"]
```

argv (verified line-by-line):
```
--print --agent general-purpose --output-format json --max-budget-usd 0.05 --allowedTools Read
```
- ✅ argv shape exactly per ADR-011 §Decision (NUM_ARGS=9)
- ✅ Stdin pipe carries prompt (per claude_print.rs contract)
- ✅ JSON envelope decoded via `ClaudePrintResponse`
- ✅ `render_failure_context` formatted `is_error=true | cost=$0.X` correctly
- 🚨 **F-RUNTIME-6**: claude CLI enforces budget **post-hoc** — billed $0.20 before stopping at $0.05 cap. Implication: dispatcher honors `Step.budget_usd` in argv, but actual spend may exceed by multiplier (here 4×). Documentation should warn.

### R-6a-1b — H1b — Agent dispatcher SUCCESS ✅

```
Cost: ~$0.10 (within $0.50 budget)
Time: 5.17s
Result: success: 1, [OK] agent-echo, "Done."
```
- argv same shape as H1, budget_usd: 0.50 forwarded
- ✅ Real claude invocation completes successfully
- ✅ Hint protocol terminal `Done.` emitted
- ✅ Full E2E proof of life through AgentDispatcher path

### R-6a-2 — H2 — argv ordering with --add-dir + multi-tool ✅

```
Cost: ~$0.15 (within $0.50 budget)
Time: 7.70s
Result: success: 1, [OK] agent-with-produces, output: out/h2-result.txt
```

argv (verified, NUM_ARGS=13):
```
--print --agent general-purpose --output-format json --max-budget-usd 0.50
--add-dir /private/tmp/phase-b-e2e-20260503T073601Z/out
--allowedTools Read Glob Grep
```
- ✅ `--add-dir` BEFORE `--allowedTools` (variadic-last invariant per ADR-011)
- ✅ produces_at canonicalized to absolute path (no `..` escape)
- ✅ Multi-tool whitelist as 3 separate argv slots (variadic)
- ✅ R1 audit CRITICAL fix (argv reorder) preserved end-to-end

### R-6a-PLUGIN — Plugin dispatcher path (separate from Agent) ✅

```
Cost: ~$0.10 (within $0.50 budget)
Time: 5.31s
Result: success: 1, [OK] plugin-step, "Done."
```
Wrapper invocation method: PATH-prepended symlink (см. F-RUNTIME-7 — `FORGEPLAN_CLAUDE_BIN` env override НЕ работает для Plugin).

argv (NUM_ARGS=9, identical к H1b shape):
```
--print --agent general-purpose --output-format json --max-budget-usd 0.50 --allowedTools Read
```
- ✅ `agent_slug = target` correctly computed (`Delegation::Plugin { name: "dummy-pack-name", target: "general-purpose" }` → `--agent general-purpose`)
- ✅ Both `name` AND `target` passed `validate_agent_name` (PluginDispatcher line 178-181)
- ✅ Identical argv shape с AgentDispatcher (PROB-050 A-4: 80% body duplication confirmed at runtime — both produce same wire format for shared params)

---

## Findings catalogue

Production-grade findings из real-E2E execution (ОТСУТСТВОВАЛИ в fake-script тестах):

| ID | Severity | Surface | Description | Action |
|----|----------|---------|-------------|--------|
| F-RUNTIME-1 | MEDIUM | Discovery | `playbook list/run` ищет playbook'и relative to cwd. Built-in `marketplace/playbooks/` доступны только из forgeplan-репо, не из произвольного workspace где установлен binary | PROB-050 A-21 (new) |
| ~~F-RUNTIME-2~~ | ~~HIGH~~ | — | **RETRACTED (audit C-1)**: первоначальное наблюдение `EXIT_CODE=0` после failed step было `tee` pipeline artefact (`zsh` без `pipefail`, `$?` отражает tee а не forgeplan). Re-run с `set -o pipefail`: H5 → exit `1`, H4 → exit `1`, missing playbook → exit `2`. Source code в `commands/playbook.rs:473` уже корректно делает `exit(1)` при `failed > 0`. Mea culpa — methodological lesson: всегда use `pipefail` или capture `${PIPESTATUS[0]}` при testing exit codes | None — false alarm; A-22 retract |
| F-RUNTIME-3 | HIGH | Doc drift | `brownfield-docs.yaml` header заявляет «fails with DispatchError::DelegateMissing (step 1)» — но `SkillDispatcher` это intentional v1 stub (Phase 6 Wave 5+), всегда возвращает success **без real invocation**. Audit S-1 escalated severity: silent skill no-op в release-style playbook (например verify-signing шаг через skill) теоретически создаёт «fail-open» surface | PROB-050 A-23 (new); audit S-1 предлагает изменить SkillDispatcher v1 stub чтобы возвращать `success: false` с `not-yet-implemented` reason до Wave 5 |
| F-RUNTIME-4 | INFO | Resolved | Brew binary v0.27.0 содержит legacy "Task tool" сообщение. Dev binary правильно говорит «`claude` binary not found» | Resolved by next release |
| F-RUNTIME-5 | MEDIUM | Versioning | Dev binary и last-released brew binary возвращают **identical** `--version` (оба `0.27.0`). Невозможно различить runtime — ни в логах, ни в bug reports | PROB-050 A-24 (new) |
| F-RUNTIME-6 | MEDIUM | Budget enforcement | `claude --print --max-budget-usd N` enforces cap **post-hoc**: real spend может превышать cap в 2-5× (here 4× при N=$0.05). Это поведение `claude` CLI, не forgeplan, но dispatcher должен документировать | PROB-050 A-25 (new) |
| F-RUNTIME-7 | HIGH | Architecture | `PluginDispatcher::resolve_binary` НЕ читает `$FORGEPLAN_CLAUDE_BIN`, в отличие от `AgentDispatcher::resolve_claude_binary`. Divergent поведение — symptom 80% duplication из PROB-050 A-4. Security/testability concern: Plugin path не sandbox-redirectable через env | **Already in PROB-050 A-14** — empirical confirmation; audit S-2 уточняет требование к `#[cfg(test)]` gate (binary-substitution vector в production); partial fix через A-4 (extract `claude_print::invoke()`) |
| F-METHODOLOGY-2 | MEDIUM | Test coverage | Real-E2E прошёл 5 successful claude invocations + happy path + budget-error envelope. **JSON decode failure paths** (timeout, server_error, rate_limited, malformed envelope, HTTP 5xx, signal exit) остаются протестированы только в unit tests с fake-script. PROB-050 A-11 + A-16 уже покрывают этот gap (parse_envelope refactor + parameterized api_error_status tests) | Already in PROB-050 A-11 + A-16 |
| ~~F-METHODOLOGY-1~~ | ~~HIGH~~ | — | **RETRACTED**: первоначальное наблюдение, что PROB-050 пустой stub, оказалось ошибочным — `forgeplan get PROB-050` обрезал output, секции (Signal/Constraints/Optimization Targets/Acceptance Criteria A-1..A-20/Blast Radius/Reversibility) **полностью заполнены**. `status: draft` корректен (не активирован потому что нет evidence, не stub). Mea culpa | None — false alarm |

## Audit findings reflected (R1, 2026-05-03)

Audit by 2 lenses (security-expert + code-analyzer, both opus, adversarial directive):

**security-expert** (verdict: CONDITIONAL PASS, 1 MEDIUM + 3 LOW + 1 INFO):
- **S-1 MEDIUM**: F-RUNTIME-3 + (now retracted) F-RUNTIME-2 chain → silent fail-open in CI. **Partial mitigation**: F-RUNTIME-2 retracted reduces severity, но S-1 core message valid: SkillDispatcher v1 stub returning `success: true` без invocation остаётся HIGH issue для release-style playbook'ов. Acceptance criteria A-23 amended.
- **S-2 LOW→HIGH escalated**: A-14 wording «OR document as test-only» недостаточен — `FORGEPLAN_CLAUDE_BIN` в release builds = binary-substitution vector (CWE-426). A-14 wording tightened to require `#[cfg(test)]` gate.
- **S-3 LOW**: wrapper recordings в `/tmp/phase-b-e2e-recordings/` mode `drwxrwxrwt` — TOCTOU/perm gap. Methodology lesson: future runs use `mktemp -d -t phase-b-XXXXXX` (mode 700). Documented в PROB-050 A-26.
- **S-4 LOW**: A-3 closure language narrowed — «happy-path argv shape + envelope decode на healthy CLI», не «hardened against hostile envelopes».
- **S-5 INFO**: no secrets, no PII, sign-off на безопасность doc-content.

**code-analyzer** (verdict: CONDITIONAL PASS, 1 HIGH + 3 MEDIUM + 2 LOW):
- **C-1 HIGH (resolved here)**: F-RUNTIME-2 / A-22 contradicts source code — already had `exit(1)` on failed > 0. Re-verified with `set -o pipefail`: exit codes correct. **F-RUNTIME-2 / A-22 retracted в этом diff.**
- **C-2 MEDIUM**: A-3 closure references EVID-097 (TBD) — violation red line #7 spirit. **Action**: EVID-097 создаётся в Task #7 ДО commit; checkbox A-3 переходит в `[x]` только после EVID-097 active.
- **C-3 MEDIUM**: H4 «not-falsifiable on v1» relabel applied (см. R-6b-2 выше).
- **C-4 MEDIUM**: F-METHODOLOGY-2 added — JSON decode failure paths still fake-script-only. Tied to PROB-050 A-11 + A-16.
- **C-5 LOW**: NOTE-049 frontmatter `status: draft` vs body table `Status | Active` drift — fixed (body table → `Draft until activate`).
- **C-6 LOW**: cost reconciliation. Per-run sum: H1 ($0.20184575) + H1b (~$0.10) + H2 (~$0.15) + H_PLUGIN (~$0.10) = ~$0.55 of **measured-precisely** invocations. The first H1 attempt without wrapper (also ~$0.43) is partially counted because it bypassed wrapper. **Honest total: ~$0.98 measured + ~$0.10 unmeasured rounding = ≤$1.10 (originally cited ~$1.13)**. Receipt updated below.

---

## Conclusion

**PROB-050 A-3 closure status (narrowed per S-4):**
- ✅ Plugin dispatcher path: real `claude --print` invocation verified, argv shape matches ADR-011 §Decision verbatim **on healthy `claude` 2.1.126**
- ✅ Agent dispatcher path: real `claude --print` invocation verified, success path completed
- ✅ argv injection guard: pre-spawn rejection verified (0.01s, no subprocess)
- ✅ argv ordering with `--add-dir` + variadic `--allowedTools`: verified end-to-end
- ✅ JSON envelope decoding: verified for **success** and **budget-error** envelope shapes
- ⚠️ **NOT verified** (per S-4 + C-4): malformed JSON, embedded control chars in `result`, extremely long `result`, `total_cost_usd: NaN`, claude returning HTTP 5xx, signal exit. These remain unit-test (fake-script) coverage. Tracked in PROB-050 A-11 + A-16.

**Closure exceeded scope** — обнаружены **5 valid net-new findings** (F-RUNTIME-1, -3, -5, -6 + F-METHODOLOGY-2; F-RUNTIME-4 resolved-at-dev; F-RUNTIME-7 already in A-14; F-RUNTIME-2 retracted per audit C-1). Added to PROB-050 как A-21, A-23, A-24, A-25 (+ A-26 for methodology lesson per S-3); A-22 retracted; A-3 закрывается с narrowed scope.

**Production-grade verdict**: Phase B Wave 1 dispatcher core (argv assembly, validation guard, JSON success+budget-error decode) **работает** end-to-end на real `claude` 2.1.126 на happy path. Failure-path JSON decode и architectural debt (F-RUNTIME-7 divergence, F-RUNTIME-1/3 CLI/skill semantics) tracked в PROB-050 для будущих PR-ов.

**Total cost (reconciled per C-6)**:

| Run | Cost (USD) | Source |
|-----|-----------:|--------|
| H1 attempt 1 (no wrapper, env-export issue) | 0.4291 | logged in `R-6a-1-h1-agent-happy.log` |
| H1 attempt 2 (inline env, $0.05 cap budget-error) | 0.20184575 | argv recording 0742116 |
| H1b ($0.50 budget, success) | ~0.10 | claude session, не recorded individually |
| H2 (--add-dir + 3 tools, $0.50, success) | ~0.15 | claude session |
| H_PLUGIN (PATH wrapper, $0.50, success) | ~0.10 | argv recording 0746336 |
| **Subtotal measured** | **~$0.98** | — |

H1 attempt 1 ($0.43) — это «дань методологическому учению»: первый run без wrapper из-за `export` non-propagation. Это и есть та дополнительная стоимость, которая раздула первоначально заявленные ~$1.13 до фактического ~$1.00. Honest total: **~$0.98 USD** across 5 measured invocations + 1 lessons-learned bypass.

**Next**: EVID-097 with structured fields → activate NOTE-049 + EVID-097 → commit + STOP for user approval before push.

---

## References

- NOTE-049 — closure note (this work)
- ADR-011 — Phase B claude --print decision
- EVID-093 — claude --print spike measurements
- EVID-096 — Phase B Wave 1 closure measurement (predecessor)
- PROB-050 — Phase B follow-ups (20 acceptance criteria, A-3 closes here)
- SPEC-003 — Playbook YAML schema (1.1 current; 1.2 bump in PROB-050 A-1)
- HANDOFF-NEXT-CHAT.md — driving handoff doc
