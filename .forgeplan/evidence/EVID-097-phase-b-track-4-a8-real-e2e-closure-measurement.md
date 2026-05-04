---
depth: standard
id: EVID-097
kind: evidence
last_modified_at: 2026-05-03T07:57:40.660475+00:00
last_modified_by: claude-code/2.1.126
links:
- target: ADR-011
  relation: informs
- target: PROB-050
  relation: informs
- target: NOTE-049
  relation: informs
- target: EVID-096
  relation: informs
status: draft
title: Phase B + Track 4-A8 real E2E closure measurement
---

# EVID-097: Phase B + Track 4-A8 real E2E closure measurement

| Field | Value |
|-------|-------|
| Status | Draft (activate after link → score > 0) |
| Created | 2026-05-03 |
| Valid Until | 2026-08-01 |
| Target | ADR-011 (Phase B Wave 1 claude --print decision) + PROB-050 A-3 (real E2E gap) |

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3

## Measurement

End-to-end empirical verification of `claude --print` invocation through both
`PluginDispatcher` and `AgentDispatcher` (ADR-011 / Phase B Wave 1) on real
production binary `claude` 2.1.126 (`/Users/explosovebit/.local/bin/claude`).

**Setup (production-grade, isolated):**
- Dev binary: `target/release/forgeplan` built from `dev` HEAD `5e08b4d`
  (post-PR #235 + #236, with all Phase B Wave 1 + Track 4-A8 changes).
  Note: `--version` reports `0.27.0` (см. F-RUNTIME-5 / A-24).
- Isolated workspace: `/tmp/phase-b-e2e-20260503T073601Z/`,
  initialised via `forgeplan init -y` against fresh dir.
- Recording wrapper: `claude-recording-wrapper.sh` — bash script that
  records argv (one per line), stdin pipe, and stdout to per-PID logs at
  `/tmp/phase-b-e2e-recordings/`, then `exec`s real claude with same args.
  Wrapper is functionally transparent — claude never sees the wrapper, the
  argv it receives is byte-identical to what the dispatcher emitted.
- Wrapper invocation paths: AgentDispatcher honours `$FORGEPLAN_CLAUDE_BIN`
  → wrapper used directly. PluginDispatcher does NOT (см. F-RUNTIME-7 /
  A-14) → PATH-prepended symlink trick used (`$WS/path-override/claude` →
  wrapper).

**Test playbooks** at `$WS/.forgeplan/playbooks/`:
- `h1-agent-happy.yaml` — minimal Agent step, `budget_usd: 0.05`
- `h1b-agent-success.yaml` — Agent step, `budget_usd: 0.50` (real success)
- `h2-add-dir-ordering.yaml` — Agent + `produces_at` + 3-tool whitelist
- `h5-injection-guard.yaml` — Agent with malicious name `../../etc/passwd`
- `h-plugin-happy.yaml` — Plugin variant, target=general-purpose

**Hypotheses tested (per NOTE-049):**
- H1: argv shape end-to-end (Agent path)
- H2: argv ordering with `--add-dir` + variadic `--allowedTools`
- H3: `release.yaml --dry-run` (CommandDispatcher orthogonal to Phase B)
- H4: `brownfield-docs.yaml` graceful failure (proven not-falsifiable on v1)
- H5: `validate_agent_name` argv injection guard rejects pre-spawn
- H_PLUGIN: PluginDispatcher path (separate from Agent)

**Test re-run discipline (post audit C-1):** all exit-code observations
re-verified with `set -o pipefail` to eliminate `tee` pipeline artefact.

## Result

**Argv shape — verified line-by-line против ADR-011 §Decision:**

H1b (Agent, NUM_ARGS=9):
```
--print --agent general-purpose --output-format json
--max-budget-usd 0.50 --allowedTools Read
```

H2 (Agent + produces_at + 3 tools, NUM_ARGS=13):
```
--print --agent general-purpose --output-format json
--max-budget-usd 0.50
--add-dir /private/tmp/phase-b-e2e-20260503T073601Z/out
--allowedTools Read Glob Grep
```

H_PLUGIN (Plugin, target→agent_slug, NUM_ARGS=9):
```
--print --agent general-purpose --output-format json
--max-budget-usd 0.50 --allowedTools Read
```

**Invariants empirically held:**
- ✅ `--add-dir` BEFORE `--allowedTools` (variadic-last, R1 audit CRITICAL fix)
- ✅ `produces_at` canonicalized to absolute path (no `..` escape, R1 fix)
- ✅ `--max-budget-usd` formatted as fractional `0.50` (R1 format_budget parity fix)
- ✅ Multi-tool whitelist as separate argv slots (variadic)
- ✅ Stdin pipe carries prompt (per claude_print docs contract)
- ✅ JSON envelope decoded for both `is_error: false` and `is_error: true / subtype: error_max_budget_usd` shapes
- ✅ `validate_agent_name` rejects `../../etc/passwd` in 0.01s WITHOUT spawning subprocess (regex `^[A-Za-z][A-Za-z0-9_-]{0,63}$` enforced before binary resolution per agent_dispatcher.rs:172-174)
- ✅ Both name AND target validate in PluginDispatcher (plugin_dispatcher.rs:178-181)
- ✅ Plugin variant correctly computes `agent_slug = target` when present, `name` otherwise

**Exit codes (re-verified with pipefail):**
- H5 (failed step): exit `1` ✅
- H4 (partial fail): exit `1` ✅
- missing playbook: exit `2` ✅
- H1b/H2/H_PLUGIN (success): exit `0` ✅

**Total measured cost:** ~$0.98 USD across 5 measured invocations + 1
methodology-bypass attempt:
- H1 attempt 1 (no wrapper, env-export issue): $0.4291
- H1 attempt 2 (inline env, $0.05 cap budget-error): $0.20184575
- H1b ($0.50 budget, success): ~$0.10
- H2 (--add-dir + 3 tools, success): ~$0.15
- H_PLUGIN (PATH wrapper, success): ~$0.10

**Findings discovered (см. ops doc + PROB-050 amendments):**
- F-RUNTIME-1 / A-21: cwd-relative playbook discovery (built-ins not globally available)
- F-RUNTIME-3 / A-23: `brownfield-docs.yaml` doc claim vs SkillDispatcher v1 stub (audit S-1 escalates to fail-safe redesign)
- F-RUNTIME-5 / A-24: dev/release binary share `--version` string
- F-RUNTIME-6 / A-25: claude `--max-budget-usd` is post-hoc (4× overrun observed)
- F-RUNTIME-7: empirical confirmation of A-14 (PluginDispatcher ignores `$FORGEPLAN_CLAUDE_BIN`)
- F-METHODOLOGY-2 / A-26: failure-path JSON decode still fake-script only

**Findings retracted post-audit:**
- F-RUNTIME-2 / A-22: `tee` pipeline artefact, not a CLI bug
- F-METHODOLOGY-1: PROB-050 not a stub, output truncation false alarm

## Interpretation

**Phase B Wave 1 dispatcher core works as designed** на real `claude`
2.1.126 in happy-path и в budget-error scenarios. Argv shape, validation
guard, JSON envelope decode, и hint protocol terminal markers (`Done.`)
все verified end-to-end. R1 audit CRITICAL fixes (path traversal reject,
allowed_tools regex, argv ordering, format_budget parity) preserved
through real binary spawn.

**Production-grade verdict** support for ADR-011 §Decision: claude --print
is a viable invocation mechanism, dispatcher correctly translates step
config to argv, JSON envelope provides actionable failure context.

**Caveats** (S-4 narrowing):
1. Failure-path JSON decode (timeout, server_error, malformed envelope,
   HTTP 5xx, signal exit) verified только in unit tests — A-11 + A-16
   track this gap.
2. `--max-budget-usd` is post-hoc — claude billed $0.20 before stopping
   at $0.05 cap. Не блокер, но dispatcher должен документировать (A-25).

**PROB-050 A-3 ✅ closes** with this evidence pack. Ten of remaining
24 acceptance criteria (A-21..A-26 added в этой sprint, A-22 retracted)
form scope для PR 4.

## Congruence Level Justification

CL3 — same context, direct measurement on the same dispatcher code, same
binary that production users would invoke. No transformations, no
proxies, no model-driven reasoning. Recording wrapper preserves byte-
identical argv shape (verified by capturing stdin/stdout/argv to file
and reading them back). The only synthesis is selecting which cost
fractions are individually-attributable vs lessons-learned overhead;
that's an honest accounting choice not a measurement artefact.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| ADR-011 | informs (closes PROB-050 A-3 acceptance criterion of the parent decision) |
| EVID-093 | informs (predecessor — claude --print spike measurements; this evidence builds on its argv-shape contract) |
| EVID-096 | informs (predecessor — Phase B Wave 1 closure; this evidence promotes its closure from fake-script to real binary) |
| PROB-050 | informs (acceptance criteria A-3 closure + A-21..A-26 amendments) |
| NOTE-049 | informs (parent verification note, hypotheses H1..H5 + H_PLUGIN) |




