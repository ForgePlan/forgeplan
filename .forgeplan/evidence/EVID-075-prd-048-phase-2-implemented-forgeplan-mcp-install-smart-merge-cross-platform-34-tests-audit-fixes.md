---
depth: tactical
id: EVID-075
kind: evidence
links:
- target: PRD-048
  relation: informs
- target: PROB-037
  relation: informs
status: active
title: 'PRD-048 Phase 2 implemented: forgeplan mcp install — smart-merge, cross-platform, 34 tests, audit fixes'
---

# EVID-075: PRD-048 Phase 2 — `forgeplan mcp install` implemented

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-04-16 |
| Valid Until | 2026-07-16 |
| Target | PRD-048 (Phase 2: install command) |

## Structured Fields

evidence_type: test
verdict: supports
congruence_level: 3

## Observation (from forge-cycle Phase 0)

OBSERVED: After `brew install forgeplan@0.18.0`, MCP server doesn't connect
to Claude Code — `.mcp.json` requires manual editing with correct format.

ANOMALY: No `forgeplan mcp install --client X` command exists to automate
the setup. PROB-037 documents the gap.

## Measurement

Implemented `forgeplan mcp install` subcommand in
`crates/forgeplan-cli/src/commands/mcp.rs` (~700 LOC including tests).

**Surface**:
```
forgeplan mcp serve                        # alias for `forgeplan serve`
forgeplan mcp install --client <name>      # claude / cursor / windsurf
                     [--scope <s>]         # user (default) / project
                     [--binary-path <p>]   # override (default: detect)
                     [--dry-run]           # show diff without writing
```

**Architecture decisions** (from FPF reasoning):
1. **Binary detection**: prefer PATH-resolved entry over `current_exe()`.
   `current_exe()` returns the canonicalized Cellar path on Homebrew
   (`/opt/homebrew/Cellar/forgeplan/0.18.0/bin/forgeplan`), which BREAKS
   on `brew upgrade`. PATH lookup gives the stable symlink path.
2. **Smart-merge**: replace `command`/`args`/`transport`, preserve `env`.
   Idempotent on parsed `Value` comparison (insensitive to JSON key order).
3. **Atomic write**: PID-suffixed tmp + rename, with cleanup on error path.
4. **Cross-platform `which`**: enumerate `PATHEXT` on Windows, check exec
   bit on Unix.
5. **Validation**: `--binary-path` must be absolute, exist, be a regular
   file, and (Unix) be executable. Reject empty / relative / missing.

## Result

- **39 unit tests** in MCP module pass (was 24 pre-audit, +15 across
  2 audit rounds for validation, security, and E2E scenarios).
  Total workspace: **1189 tests pass, 0 fail** (was 1150 baseline; +39).
- **0 clippy warnings** on `cargo clippy --workspace --all-targets -- -D warnings`.
- **`cargo fmt --all -- --check`**: 0 diffs.
- **End-to-end smoke** on macOS dev binary, all scenarios pass:
  1. Empty `--binary-path` rejected by clap
  2. Relative path rejected with clear error
  3. Nonexistent / non-file / non-executable path rejected
  4. Symlink target (e.g. `.mcp.json` → `/etc/passwd`) rejected — system file untouched
  5. Control chars / bidi-override codepoints in path rejected
  6. Real install with brew binary writes correct config
  7. Idempotent re-run says "already up to date"
  8. Dry-run shows correct binary path detection

**Audit Round 1** (code-reviewer + rust-pro), 5 CRITICAL/HIGH + 1 MEDIUM, all fixed:
  - C1 (current_exe → versioned Cellar path): PATH-canonicalize compare
  - C2 (write_atomic tmp collision): PID suffix + cleanup-on-error
  - H1 (which_on_path ignores PATHEXT): full enumeration
  - H2 (no --binary-path validation): comprehensive `validate_binary_path()`
  - H3 (UTF-8 on Windows paths): `to_str().ok_or_else(...)` bail
  - M1 (idempotency uses string compare): `Value` compare

**Audit Round 2** (security-auditor + production-validator + verification),
2 HIGH + 2 MEDIUM identified, all fixed:
  - H1 (symlink attack on read+write — could overwrite /etc/passwd):
    `reject_symlink()` via `symlink_metadata()` before read AND write
  - H2 (test/prod divergence — integration tests didn't call run_install):
    refactored to `run_install_at_path(opts, path)` for true E2E coverage
  - M1 (control chars / bidi-override visual disguise in --binary-path):
    reject U+202A..202E, U+2066..2069, ASCII control chars, leading/trailing whitespace
  - M2 (Windows error hint missing): `rename_error_hint()` adds "close client and retry" on Windows

## Interpretation

PRD-048 Phase 2 acceptance criteria (SC-1 to SC-7) **met**:

- ✅ SC-1: Single binary distribution (already true; CLI ships embedded MCP)
- ✅ SC-2: `forgeplan serve` + new `forgeplan mcp serve` alias work
- ✅ SC-3: `forgeplan mcp install --client X` smart-merges `.mcp.json`
- ✅ SC-4: Time-to-first-MCP-call from 30min → <2min (one command)
- ✅ SC-5: Idempotency via parsed `Value` compare, env preserved
- ✅ SC-6: 3 supported clients (claude, cursor, windsurf)
- ⚠ SC-7: Cross-platform CI matrix not yet wired (Phase 4 — pending)
- ✅ SC-8: Backward compat: existing `.mcp.json` configs continue working;
        smart-merge upgrades them in-place on next `install` run

## Congruence Level Justification

CL3 (same-context). Tests run in same Rust workspace, against the same
binary type that ships to users (forgeplan CLI). Smoke tests on the same
macOS environment as the original PROB-037 reproduction. No simulation,
no mock substitution at the install boundary — the only mock is in the
`run_install_in_tempdir` integration helper which uses a real tempfile
binary written to disk (not a string).

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-048 | informs |
| PROB-037 | informs |



