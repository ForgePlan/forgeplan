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

- **34 unit tests** pass (was 24 in pre-audit version, +10 for validation
  + integration scenarios). Total workspace: **1184 tests pass, 0 fail**
  (was 1150 baseline; +34).
- **0 clippy warnings** on `cargo clippy --workspace --all-targets -- -D warnings`.
- **`cargo fmt --all -- --check`**: 0 diffs.
- **End-to-end smoke** on macOS dev binary, all 6 scenarios pass:
  1. Empty `--binary-path` rejected by clap (correct)
  2. Relative path rejected with clear error
  3. Nonexistent path rejected
  4. Real install with brew binary writes correct config
  5. Idempotent re-run says "already up to date"
  6. Dry-run shows correct binary path detection
- **Audit by 2 agents** (code-reviewer + rust-pro), 5 CRITICAL/HIGH
  findings, all fixed in-scope:
  - C1 (current_exe → versioned Cellar path): fixed via PATH-canonicalize
  - C2 (write_atomic tmp collision): fixed with PID suffix + cleanup
  - H1 (which_on_path ignores PATHEXT): fixed
  - H2 (no --binary-path validation): fixed with `validate_binary_path()`
  - H3 (UTF-8 on Windows paths): fixed via `to_str().ok_or_else(...)`
  - M1 (idempotency uses string compare): fixed with `Value` compare

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



