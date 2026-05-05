---
depth: standard
id: PROB-052
kind: problem
last_modified_at: 2026-05-05T20:38:02.387245+00:00
last_modified_by: claude-code/2.1.128
links:
- target: PROB-050
  relation: based_on
status: draft
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

- [ ] `which_in_path` calls `canonicalize` on first match; returns
  `Option<PathBuf>` of the real path.
- [ ] Unix path: parent dir ownership check (uid 0 OR current uid); file
  must not have group/other write bits.
- [ ] Windows path: skip permission check (Windows ACL is out of scope —
  document explicitly).
- [ ] `AgentDispatcher` + `PluginDispatcher` cache the resolved path
  for the lifetime of the dispatcher instance (recreated per `dispatch()`
  call — current behavior — so cache invalidation is implicit).
- [ ] +3 unit tests: TOCTOU symlink swap detected, group-writable binary
  rejected, cross-platform skip.
- [ ] CHANGELOG entry under **Security** section.

## Refs

- PR-E Round 6 audit (2026-05-05): security-expert agent MED-1
- CHANGELOG.md (v0.29.0): "Deferred to v0.30.0" section

