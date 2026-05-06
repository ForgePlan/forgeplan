---
depth: standard
id: PROB-052
kind: problem
last_modified_at: 2026-05-05T20:38:02.387245+00:00
last_modified_by: claude-code/2.1.128
links:
- target: PROB-050
  relation: based_on
status: active
title: PR-E Round 6 deferred — TOCTOU + symlink follow в which_in_path
---

## Signal

PR-E Round 6 adversarial security audit (3 parallel agents, 2026-05-05) flagged
`crates/forgeplan-core/src/playbook/dispatch/helpers.rs::which_in_path` as a
TOCTOU + symlink-follow vulnerability:

```rust
pub(super) fn which_in_path(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(program);
        if candidate.is_file() { return Some(candidate); }
    }
    None
}
```

`is_file()` follows symlinks (no `symlink_metadata`), no `canonicalize()`, no
executable-bit check. Window between `is_file()` and `Command::spawn` allows
TOCTOU swap on a writable PATH directory (`/usr/local/bin` is group-writable
on default Homebrew installs). Combined with PATH being inherited (no
allowlist on which dirs to scan), a user with write access to *any* PATH dir
earlier than the legitimate `claude` location can plant a hijacking binary.

This is a **pre-existing surface** (existed before PR-E refactor — surfaced
because the audit re-examined the path-resolution chain). Not a v0.29.0
regression.

## Constraints

- MUST NOT regress the cross-platform PATH lookup (Windows lacks executable-bit
  semantics; canonicalize MUST be the variant that does not require the file
  to exist on Windows yet still resolves Unix symlinks).
- MUST NOT cache the resolved path indefinitely — claude binary upgrades land
  via brew during a session, cache invalidation rules need design.
- MUST keep `which_in_path` callable from both `AgentDispatcher` and
  `PluginDispatcher` after refactor.

## Optimization Targets (1-3 max)

- **Resolve-once-per-dispatcher**: dispatcher caches the canonicalised path
  on first successful resolve, eliminating TOCTOU window between
  `is_file()` and the next `Command::spawn`.
- **Permissions check**: parent dir must be owned by uid 0 or current uid;
  binary file MUST NOT have group/other write bits (Unix).
- **Symlink resolution**: use `std::fs::canonicalize` instead of plain
  `is_file()` so the dispatcher knows the real path being executed.

## Observation Indicators (Anti-Goodhart)

- `cargo test --workspace --features test-helpers` stays at ≥ 1977 PASS.
- `forgeplan health` clean (0 blind spots, 0 orphans).
- Cross-platform CI green (Linux + macOS + Windows).

## Acceptance Criteria

- [x] **AC-1** `which_in_path` calls `canonicalize` on first match; returns `Option<PathBuf>` of the real path. **Closed** — `resolve_safe_path` invokes `std::fs::canonicalize` and caller spawns the canonical PathBuf.
- [x] **AC-2 (partial)** Unix path: file must not have group/other write bits. **Closed** for write-bit clause (`mode & 0o022 != 0`). **Re-scoped**: parent-dir *ownership* check (uid 0 OR current uid) deferred — single-user threat model already covered by parent-dir mode gate; multi-user shared workstation is out of PROB-052 scope. Tracked: follow-up if multi-tenant deployment lands.
- [x] **AC-3** Windows path: skip permission check. **Closed** — `cfg(unix)` gates the perm clause; Windows still gets canonicalize + non-file rejection. PRD §AC-3 explicitly documents the Windows ACL skip.
- [~] **AC-4** Dispatcher-level cache. **Re-scoped**: per-dispatch resolution accepted because `AgentDispatcher` / `PluginDispatcher` are recreated per `dispatch()` call (constraint named in original AC body — "recreated per `dispatch()` call — current behavior — so cache invalidation is implicit"). Adding a `OnceCell<PathBuf>` here would create the staleness risk the AC's own MUST-NOT prohibits ("MUST NOT cache the resolved path indefinitely"). Re-scoping makes the AC's intent explicit rather than silently dropping the field.
- [x] **AC-5** +3 unit tests. **Closed + Round 7 hardening** — 7 tests:
  1. `which_in_path_canonicalizes_symlink_to_real_target`
  2. `which_in_path_rejects_group_writable_binary`
  3. `which_in_path_rejects_group_writable_parent_dir`
  4. `which_in_path_skips_empty_path_entries` (Round 7 audit MED-4)
  5. `resolve_safe_path_rejects_group_writable_override` (Round 7 audit HIGH-1)
  6. `resolve_safe_path_canonicalizes_safe_override` (Round 7 audit HIGH-1)
  7. `which_in_path_windows_skips_permission_gate` (cfg(not(unix)))
- [x] **AC-6** CHANGELOG entry under **Security**. **Closed** in same sprint commit.

## Round 7 Audit — Consumer-Side Bypass Closure (HIGH-1)

Round 7 adversarial audit (2 parallel agents — security + code-reviewer, 2026-05-06) caught **HIGH-1 transport asymmetry**: PROB-052 closed the PATH-search surface but **left override branches unguarded**:

- `AgentDispatcher::resolve_claude_binary` — `claude_binary` field used bare `is_file()`.
- `PluginDispatcher::resolve_binary` — `claude_binary` field returned without ANY check.
- `resolve_forgeplan_binary` — `FORGEPLAN_BIN` test override + `target/release/forgeplan` workspace fallback used bare `is_file()`.

Closed by promoting `resolve_safe_path` to `pub(super)` and routing all 4 override branches through it. Same canonicalize + perm gate now applies whether the binary is found via PATH search OR explicit override. Pre-Round-7 a Homebrew operator setting `claude_binary = /usr/local/bin/claude` (group=admin 0o775) would bypass the gate entirely — Round 7 closes that vector.

**Round 7 also closed**: MED-1/MED-2 log-injection (CWE-117/CWE-150) hardening — `tracing::warn!` rejection messages now use `escape_debug` mirroring PROB-053 shell-exec warning pattern; new `eprintln!` operator-visible channel so rejections surface без `RUST_LOG=warn`. MED-4 empty-PATH-entry skip. HIGH-3 docstring mode-bit precision. Residual TOCTOU between metadata() and Command::spawn documented as acceptable trade-off (full closure requires non-portable `O_NOFOLLOW + fexecve`).

**Deferred to follow-up sprint**: typed-error refactor (`String` → `enum RejectionReason` per PROB-049 typed-errors lineage), setuid/setgid bit rejection (CWE-250 adjacent — narrow exploit window), sync-mutex variant of `DISPATCH_ENV_LOCK` (test infrastructure cleanup).

## Refs

- PR-E Round 6 audit (2026-05-05): security-expert agent MED-1
- Round 7 audit (2026-05-06): security + code-reviewer parallel agents, 2 HIGH closures + 4 MED closures
- CHANGELOG.md (v0.29.0): "Deferred to v0.30.0" section
- CHANGELOG.md (Unreleased): PROB-052 Security section
- EVID-XXX (next sprint commit): closure evidence with full audit transcript



