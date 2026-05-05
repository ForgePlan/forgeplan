---
depth: standard
id: RFC-008
kind: rfc
last_modified_at: 2026-05-05T22:10:15.648882+00:00
last_modified_by: claude-code/2.1.128
links:
- target: PRD-074
  relation: based_on
status: active
title: Shell-execution gate implementation — phases for --allow-shell + config + warning
---

# RFC-008: Shell-execution gate implementation

## Summary

Implement the shell-execution gate scoped by [PRD-074](../prds/PRD-074-shell-execution-gate-for-delegation-command.md) as a four-phase, single-PR change to `forgeplan-cli`, `forgeplan-core`, and `forgeplan-mcp`. The gate is a boolean toggle (CLI flag OR workspace config), default deny, paired with a mandatory user-visible stderr warning before each shell exec. Marketplace signing remains explicitly out-of-scope (separate follow-up). Reference playbook YAMLs (audit/release/brownfield-docs) updated with `--allow-shell` documentation.

## Motivation

PR-E Round 6 adversarial security audit (2026-05-05) flagged `Delegation::Command` as an unguarded CWE-78 / CWE-94 surface. The `tracing::warn!` is silent unless `RUST_LOG` enables it, the YAML loader has no signing or allowlist, and the marketplace plugin distribution channel (PRD-067/068/069) is in active design. Without a default-deny gate, a malicious playbook YAML downloaded over the network can execute arbitrary shell. PROB-053 tracks the issue. Shipping the safety net first, before designing the (much bigger) signing scheme, is the standard "make the change easy, then make the easy change" pattern.

## Options Considered

### Option A — boolean gate at dispatcher level (recommended)

Add `allow_shell: bool` to `PlaybookRunOptions` (or equivalent struct passed to the dispatcher). Wire from CLI flag and workspace config. `RoutingDispatcher` checks the bool before delegating to `CommandDispatcher`; rejects with `DispatchError::Transport` + `Fix:` hint otherwise.

**Pros**: minimal scope, easy to test (matrix of 4 cells), reversible. Stderr warning is dispatcher-side so MCP tool also gets it.

**Cons**: boolean is coarse — операторы которые хотят разрешить только `git status` но не `curl` лишены. Future per-command allowlist would need a config evolution.

### Option B — feature flag at compile time (`#[cfg(feature = "shell-exec")]`)

Gate `Delegation::Command` execution behind a Cargo feature flag. Default builds REJECT compile-time; opt-in builds enable.

**Pros**: strongest possible boundary — release binary literally cannot exec shell unless rebuilt. Mirrors the PROB-050 A-14 cfg(test) pattern.

**Cons**: breaks the local audit.yaml use case unless we ship two binaries (audit-binary + production-binary). Distribution complexity. Operators can't toggle per-invocation. Heavyweight for a problem that boolean solves.

### Option C — signing-only (defer entire gate until signing ships)

Don't gate at all in v0.30; treat operator vigilance as the only mitigation; ship signing in v0.31+ as the actual fix.

**Pros**: avoids two-step migration (gate now, then signing).

**Cons**: marketplace risk window stays open для всего v0.30 cycle. PR-E Round 6 audit explicitly called out the absence of user-visible warning as MED-2; deferring leaves a known security gap shippable.

## Proposed Direction

**Option A**. Boolean gate at dispatcher level, paired with mandatory stderr warning. Signing scheme deferred to follow-up PROB/PRD when marketplace fetch ships (per PRD-074 §Non-Goals).

Rationale: closes the immediate marketplace risk by refusing default; preserves existing local-trusted workflows via opt-in; provides a clean upgrade path to per-command allowlist (config schema evolves, boolean stays default for unrecognized configs); does not couple the safety net release to a much larger signing design conversation.

## FR → Phase mapping (PRD-074)

| FR | Phase | Where |
|----|-------|-------|
| **FR-1** (`--allow-shell` flag, default refuse + Fix: hint) | Phase 2 | `forgeplan-cli/src/commands/playbook.rs` |
| **FR-2** (workspace config `[playbook] allow_shell`) | Phase 3 | `forgeplan-core/src/config.rs` |
| **FR-3** (stderr `! shell-exec:` warning) | Phase 1 | `forgeplan-core/src/playbook/dispatch/routing.rs` |
| **FR-4** (audit/release/brownfield-docs YAML headers) | Phase 4 | `marketplace/playbooks/*.yaml` |
| **FR-5** (Hint Protocol: `Error:` + `Fix:` markers) | Phase 1 + 2 | dispatcher refuse path + CLI exit handler |
| **FR-6** (MCP `forgeplan_playbook_run` parity, if exists) | Phase 4 | `forgeplan-mcp/src/server.rs` |
| **FR-7** (4-cell test matrix) | Phase 1 + 2 | unit tests in dispatcher + integration test in CLI |

## Implementation Phases

### Phase 1 — Core dispatcher gate (`forgeplan-core`) — closes FR-3, FR-5 (refuse path), FR-7 (matrix cells 1-2)

- Extend `RoutingDispatcher::new` (or equivalent) to accept `allow_shell: bool`.
- In the dispatch path, when `step.delegate_to == Delegation::Command { .. }` and `!allow_shell`, return:
  ```rust
  Err(DispatchError::Transport(
    "shell execution refused — Delegation::Command requires \
     explicit --allow-shell or [playbook] allow_shell = true \
     in workspace config".into()
  ))
  ```
- When `allow_shell == true`, BEFORE spawning the subprocess, emit:
  ```rust
  eprintln!("! shell-exec: {} [{} args]", argv.first().unwrap_or(&String::new()), argv.len().saturating_sub(1));
  ```
- Existing `CommandDispatcher::dispatch` body unchanged (gate is at routing layer).
- Tests:
  1. `dispatch_command_refuses_when_allow_shell_false` — `DispatchError::Transport` matches expected message
  2. `dispatch_command_proceeds_when_allow_shell_true` — happy path
  3. `dispatch_command_emits_stderr_warning_when_allowed` — capture stderr, assert prefix `! shell-exec:`
  4. `dispatch_non_command_unaffected_by_flag` — `Delegation::Plugin` and `Delegation::Agent` ignore the bool

### Phase 2 — CLI flag (`forgeplan-cli`) — closes FR-1, FR-5 (CLI exit), FR-7 (matrix cells 3-4)

- Add `#[arg(long)] allow_shell: bool` to the `playbook run` subcommand (clap derive).
- Resolve effective `allow_shell` = `cli_flag || workspace_config_value` and pass to dispatcher.
- Update help text: `"Allow Delegation::Command shell execution (CWE-78 gate, default deny)"`.
- Tests:
  1. `playbook_run_without_flag_refuses_command_step` — exit code non-zero, stderr matches `Fix:` hint
  2. `playbook_run_with_flag_executes_command_step` — exit 0
  3. `playbook_run_help_documents_allow_shell` — `--help` output contains the description

### Phase 3 — Workspace config schema (`forgeplan-core/config`) — closes FR-2

- Extend `WorkspaceConfig` (or whatever the loader struct is named) with:
  ```rust
  #[serde(default)]
  pub playbook: PlaybookConfig,

  pub struct PlaybookConfig {
      #[serde(default)]
      pub allow_shell: bool,
  }
  ```
- Defaults preserved — workspaces without the key load identically to today.
- Document in `.forgeplan/config.yaml` reference (if such a doc exists) and in PRD-074 §FR-2.
- Test:
  1. `config_default_has_allow_shell_false` — fresh workspace
  2. `config_loads_allow_shell_from_yaml` — set `[playbook] allow_shell = true`, parse, assert
  3. `config_unknown_keys_warning_does_not_break_load` — forward-compatible

### Phase 4 — Reference playbook docs + MCP parity — closes FR-4, FR-6

- Update `audit.yaml`, `release.yaml`, `brownfield-docs.yaml` headers (top comment block) to mention `--allow-shell` requirement when invoked.
- If `forgeplan_playbook_run` MCP tool exists, expose `allow_shell: bool` parameter (same default false). If MCP doesn't have run, document the omission in PRD-074 follow-up.
- CHANGELOG entry under **Security** section + migration note for operators.
- Real E2E:
  1. Run `forgeplan playbook run audit.yaml` без флага → expect non-zero exit + Fix: hint
  2. Run `forgeplan playbook run audit.yaml --allow-shell` → expect normal flow + stderr warning visible

## Migration

**For operators**:

```bash
# Pre-v0.30:
$ forgeplan playbook run release.yaml

# v0.30+:
$ forgeplan playbook run release.yaml --allow-shell
# OR set in .forgeplan/config.yaml:
# [playbook]
# allow_shell = true
```

CHANGELOG release notes will surface this prominently. Existing CI scripts using `forgeplan playbook run` need a one-line update.

## Risk

| Risk | Impact | Mitigation |
|---|---|---|
| Existing CI scripts break silently — пользователь видит "shell execution refused" | Medium — operators must update CI workflows, but error message includes `Fix:` hint | CHANGELOG prominent migration note + test that existing audit.yaml works с `--allow-shell` |
| Workspace config opt-in encourages "set it once and forget" — moots the гард | Medium — но это на operator's risk, документировано | Doc: opt-in is for trusted-local; never set in marketplace-fetched workspaces |
| Boolean gate is too coarse — future per-command allowlist needs schema evolution | Low — boolean gate stays as default; allowlist becomes a richer schema atop boolean (e.g. `allow_shell: bool \| List<String>`) | RFC-008 explicitly notes this is upgrade-compatible |
| `eprintln` in test environments captured into test fixtures | Low — tests должны явно capture stderr (already common pattern) | Use `tracing::warn!` AS WELL для completeness, but keep `eprintln` as the load-bearing user-visible channel |

## Open Questions

- Should the `eprintln` line include a hash / fingerprint of the playbook source (file path + sha256)? Useful for forensic logs but adds dependency. **Default**: no, keep simple; revisit if marketplace ships with a `--source` provenance field.
- Should `forgeplan playbook run --dry-run` print the `! shell-exec:` warning even if not executing? **Default**: yes — dry-run is for safety review, the warning is exactly what the operator wants to see.

## Success Criteria

- 4-cell test matrix all pass (flag=t/f × config=t/f)
- `forgeplan health` clean on a workspace using audit.yaml with `--allow-shell`
- `cargo clippy --workspace --all-targets --features test-helpers -- -D warnings` clean
- Real E2E proof: malicious YAML refuses default; explicit `--allow-shell` works; warning visible
- CHANGELOG entry documented
- audit.yaml / release.yaml / brownfield-docs.yaml headers updated




