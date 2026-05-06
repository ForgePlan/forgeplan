---
depth: standard
id: EVID-104
kind: evidence
last_modified_at: 2026-05-05T22:53:14.636851+00:00
last_modified_by: claude-code/2.1.128
links:
- target: PROB-053
  relation: informs
- target: PRD-074
  relation: informs
- target: RFC-008
  relation: informs
status: active
title: PROB-053 / PRD-074 closure — shell-exec gate, 1985 tests, Round 7 audit
---

# EVID-104: PROB-053 / PRD-074 closure — shell-exec gate, 1985 tests, Round 7 audit

## Summary

Shell-execution gate for `Delegation::Command` (PROB-053 / PRD-074) shipped через PRD-074 Phase 1-4 + Round 7 adversarial audit fixes (3 parallel agents — architect, code-reviewer, security). Default-deny: `--allow-shell` CLI flag OR `[playbook] allow_shell = true` workspace config opt-in required перед shell exec. User-visible stderr warning (`! shell-exec: <full argv>`) с `escape_debug` sanitization (CWE-117 / CWE-150 mitigation). MCP parity through `PlaybookRunParams.allow_shell`.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Evidence

### Real E2E на release binary (target/release/forgeplan v0.29.0, 2026-05-06)

5-cell test matrix, все cells PASS:

| Cell | Invocation | Expected | Actual |
|---|---|---|---|
| A | `forgeplan playbook run test-shell` (no flags) | refuse + Fix hint includes `--yes --allow-shell` | ✅ Error: ADR-009 + Fix: includes both flags + Note about dropping `--allow-shell` если no Command steps |
| B | `forgeplan playbook run test-shell --yes` | step refused, hint mentions `--allow-shell` OR workspace config | ✅ `[FAIL] e — step uses delegate_to: command — pass --allow-shell (or set [playbook] allow_shell = true ...)` |
| C | `forgeplan playbook run test-shell --yes --allow-shell` | success + stderr `! shell-exec: <full argv>` | ✅ `! shell-exec: /bin/echo hello --allow-shell` (escape_debug-sanitized; full argv визible, не truncated) |
| D | `forgeplan playbook run test-shell --allow-shell` (no `--yes`) | refuse — flags независимы (Round 7 HIGH-A) | ✅ same Error: ADR-009 — `--allow-shell` does NOT imply `--yes` after Round 7 |
| E | `[playbook] allow_shell = true` в config + `--yes` only | success + dedicated AUTO-APPROVED banner | ✅ `!! shell-exec: AUTO-APPROVED via [playbook] allow_shell=true in .forgeplan/config.yaml. To force-deny ...` followed by per-step `! shell-exec:` warning |

### Unit + integration tests

- `cargo test --workspace --features test-helpers` → **1985 PASS / 0 fail / 5 ignored** (38 suites)
- Pre-PROB-053 baseline: 1977 passed
- Net: +8 PROB-053-specific tests:
  - `format_shell_exec_warning_renders_seconds_for_whole_durations`
  - `shell_exec_warning_escapes_ansi_in_program` (HIGH-F regression — CWE-117)
  - `shell_exec_warning_escapes_control_chars_in_args` (HIGH-F regression)
  - `shell_exec_warning_renders_full_argv` (MED-D regression)
  - `shell_exec_warning_truncates_pathological_argv` (MED-D bound)
  - `test_config_yaml_omitted_playbook_defaults_to_none` (MED-5 forward-compat)
  - `test_config_yaml_playbook_allow_shell_true_roundtrips` (HIGH-2 FR-2 coverage)
  - `test_config_yaml_empty_playbook_defaults_allow_shell_false` (HIGH-2 default-deny)
  - `command_step_without_allow_shell_is_refused` (executor security gate, renamed)
  - `security_refusal_message_documents_both_paths` (PRD-074 §FR-5)
  - `security_refuses_command_without_allow_shell` (helper rename)

### Quality gates на release branch

- `cargo fmt --check` → 0 diffs
- `cargo clippy --workspace --all-targets --features test-helpers -- -D warnings` → 0 warnings
- Workspace version stays 0.29.0 (this PR ships в `[Unreleased]` block для v0.30.0)

### Round 7 adversarial audit (2026-05-06)

3 parallel agents (architect-reviewer, code-reviewer, security-expert) — все вернули **FIX-FIRST**, 9+ findings закрыто в этом PR:

| ID | Finding | Source | Status |
|---|---|---|---|
| HIGH-A | `--allow-shell` shadow yes + bad refuse hint | code H1 + sec F6 + arch H2/H3 | ✅ closed: drop shadow, hint includes both flags |
| HIGH-B | Config OR-semantics defeats default-deny | sec F2 + arch H1 | ✅ closed: dedicated banner emits when config-only path |
| HIGH-C | Test matrix gap — FR-2 workspace-config uncovered | code H2 + arch MED-3 | ✅ closed: +3 PlaybookConfig tests + +4 warning tests |
| HIGH-D | `load_config` silent swallow на parse error | code H3 | ✅ closed: explicit Warning to stderr (CLI) + tracing::warn (MCP) |
| HIGH-E | MCP tool description omits `allow_shell` | arch H4 | ✅ closed: description updated с PRD-074 reference |
| HIGH-F | Terminal-injection в stderr warning (CWE-117/150) | sec F1 | ✅ closed: `escape_debug` + 4 regression tests |
| MED-C | `SecurityError::ShellRequiresYes` misleading | code M4 + arch MED-4 | ✅ closed: renamed `ShellRequiresAllowShell` + deprecated alias |
| MED-D | stderr argv truncation hides forensic detail | arch MED-2 + sec F1 | ✅ closed: full argv с 4 KiB cap + truncation marker |
| LOW-2 | Redundant `find_workspace` в playbook.rs | code L2 | ✅ closed: cached fp_dir_opt |

### Deferred to follow-up (not in v0.30.0 PROB-053 scope)

- F3 — `ForgeplanCore::Ingest` path-traversal — CWE-1284, separate adjacent surface, новый PROB candidate
- F4 — MCP shell-exec stderr unreachable to user — requires rmcp protocol design, defer
- F5 — TOCTOU once-per-run config sample — documented, defer (single-user low risk)
- F7 — step_id shell-quote в Fix: hint — informational LOW (loader already validates step_id chars)
- MED-E — `ExecutorConfig {yes_flag, allow_shell}` independent fields can construct illegal state — refactor scope, defer
- MED-1 — eprintln warning fires before subprocess spawn check — minor honesty fix, defer
- LOW-1, LOW-3, LOW-4 cosmetic items

### Files changed (12 files)

- `crates/forgeplan-core/src/config/types.rs` — `PlaybookConfig { allow_shell: bool }` + 3 serde tests
- `crates/forgeplan-core/src/playbook/dispatch/mod.rs` — `validate_command_delegate_security` parameter rename + `SecurityError::ShellRequiresAllowShell` + deprecated alias + 2 new tests
- `crates/forgeplan-core/src/playbook/dispatch/command_dispatcher.rs` — `format_shell_exec_warning` helper + 4 HIGH-F/MED-D regression tests
- `crates/forgeplan-core/src/playbook/executor.rs` — `ExecutorConfig.allow_shell` field
- `crates/forgeplan-cli/src/main.rs` — `--allow-shell` CLI flag
- `crates/forgeplan-cli/src/commands/playbook.rs` — effective allow_shell resolution + Round 7 HIGH-A/B/D fixes (no shadow, banner, log-on-error)
- `crates/forgeplan-cli/tests/integration_phase6_e2e.rs` — 2 existing E2E tests updated с `--allow-shell`
- `crates/forgeplan-mcp/src/types.rs` — `PlaybookRunParams.allow_shell`
- `crates/forgeplan-mcp/src/server.rs` — MCP tool description + effective resolution + tracing::warn
- `marketplace/playbooks/release.yaml` — header doc updated (`--allow-shell` requirement)
- `CHANGELOG.md` — `[Unreleased]` block с full Round 7 closure summary
- `.forgeplan/problems/PROB-053-...md` — AC checked, status active, closure summary

## Reproducibility

```bash
# Clone the branch (after merge)
git checkout dev
git pull
cargo build --release --bin forgeplan

# Reproduce 5-cell matrix (manual)
FRESH=$(mktemp -d)
cd "$FRESH"
forgeplan init -y
mkdir -p .forgeplan/playbooks
cat > .forgeplan/playbooks/test-shell.yaml << 'EOF'
schema_version: "1.0"
name: test-shell
title: Shell test
steps:
  - id: e
    delegate_to:
      type: command
      command: ["/bin/echo", "hello"]
EOF

# Cell A: refuse + hint
forgeplan playbook run test-shell  # exit 2

# Cell B: --yes alone refuses
forgeplan playbook run test-shell --yes  # exit 1, [FAIL] step

# Cell C: success + warning
forgeplan playbook run test-shell --yes --allow-shell  # exit 0, ! shell-exec: ...

# Cell D: --allow-shell alone refuses (Round 7 HIGH-A)
forgeplan playbook run test-shell --allow-shell  # exit 2

# Cell E: workspace config path + banner
echo -e "playbook:\n  allow_shell: true" >> .forgeplan/config.yaml
forgeplan playbook run test-shell --yes  # !! AUTO-APPROVED banner + ! shell-exec
```

## R_eff calculus

- Verdict: **supports** (PRD-074 §FR-1..FR-7 verified end-to-end)
- Congruence Level: **CL3** — same context (PROB-053 + PRD-074 + RFC-008 + Round 7 audit + real E2E + 1985 unit tests + сequential 4-cell + workspace-config cell)
- Evidence type: **test** (live release binary execution proof + 1985 cargo test pass + 4 dedicated regression tests за HIGH-F/MED-D)
- Decay: `valid_until` not set — closure evidence (point in time, не time-bounded measurement)





