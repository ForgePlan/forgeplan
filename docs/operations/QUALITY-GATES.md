[English](QUALITY-GATES.md) · [Русский](QUALITY-GATES.ru.md)

# CI Quality Gates — Infrastructure Reference (v0.28.0+)

Reference for all automated quality gates that run in Forgeplan CI and local
hooks. For each gate: **what it checks**, **when it runs**, **how to run
locally**, **how to fix common failures**.

Scope: this describes required checks and source-defined triggers, not a current
green CI receipt or live branch-rules inspection. A failed job blocks merging
only when applicable required checks/rules enforce it. Hooks require host wiring.
Under `AGENTS.md`, one admitted tester/build owner runs expensive verification
under the shared lease after writers land; doc workers do not launch parallel
Cargo checks. The commands below are reference, not authorization to run them.

> **Related document**: [`docs/methodology/QUALITY-GATES.md`](../methodology/QUALITY-GATES.md)
> covers methodology gates (Verification Gate, Adversarial Review, R_eff scoring).
> This document covers CI/CD infrastructure, not methodology.

---

## Gate Overview

| Gate | Trigger | Blocks |
|---|---|---|
| `cargo fmt --check` | configured Claude PreToolUse hook + CI | matching commit attempt; required CI |
| `cargo check` / `cargo clippy` | CI; owner-run local checks | required CI |
| nextest + doc-tests | CI (PR, push dev) | required CI |
| `forgeplan health` | CI; advisory local hook | required CI thresholds, not a universal local commit block |
| `forgeplan validate` | CI | required CI |
| MCP tool count drift detector | CI (PR, push dev) | required CI |

Hooks in `.claude/hooks/` are a local safety net before CI. Health/validation
run in `.github/workflows/forgeplan-health.yml`; tool-count drift runs separately
in `.github/workflows/ci.yml`, including docs-only PRs.

---

## 1. Formatting: `cargo fmt --check`

**Purpose:** verifies Rust source code is formatted according to `rustfmt`
(no uncommitted diff after auto-format).

**When it runs:**
- Configured Claude Code `PreToolUse` hook `pre-commit-fmt.sh` inspects `Bash` commit-command strings and can reject the tool call.
- CI — PRs to `dev`/`main` and pushes to `dev`.

**Hook file:** `.claude/hooks/pre-commit-fmt.sh`

**How to run locally:**
```bash
cargo fmt                    # auto-fix (applies changes in place)
cargo fmt -- --check         # dry-run — shows diff without changes; exit 1 on drift
```

**How to fix:**
```bash
cargo fmt                    # fix all formatting
cargo fmt -- --check         # must return exit 0, empty stdout
```

Formatting is MANDATORY before every commit (CLAUDE.md §Rust coding rules pt.3).
The configured hook is not a standard Git hook and cannot intercept commits from
every client. Preserve the formatting requirement; the admitted verification
owner runs checks under the repository's build/test lease.

---

## 2. Static analysis: `cargo check` and `cargo clippy`

**Purpose:** `cargo check` verifies compilability without building binaries.
`cargo clippy` adds lint rules with `-D warnings`; warnings make the command fail nonzero, not necessarily with exit 1.

**When it runs:**
- Locally: the designated verification owner, according to task scope and lease.
- CI: PRs to `dev`/`main` and pushes to `dev`.

**Full CI command:**
```bash
RUSTFLAGS=-Dwarnings cargo clippy --workspace --all-targets
RUSTFLAGS=-Dwarnings cargo clippy --workspace --all-targets --features semantic-search
```

**How to run locally:**
```bash
cargo check --workspace                                    # fast compilation check
cargo clippy --workspace --all-targets -- -D warnings      # full lint
```

**How to fix:**

Most clippy warnings can be auto-fixed:
```bash
cargo clippy --fix --workspace --allow-dirty --allow-staged
```

If a warning cannot be auto-fixed and is a false positive, add a targeted
suppression with an explanation:
```rust
#[allow(clippy::too_many_arguments)]  // No way to split without losing readability
pub fn complex_init(/* ... */) { /* ... */ }
```

Do not suppress `clippy::all` or `warnings` globally — this masks real issues.
Rust 1.95 tightened several lints (CLAUDE.md §Rust coding rules pt.3).

---

## 3. Testing: `cargo test`

**Purpose:** runs the selected workspace test targets. Cargo reports failure with exit 101; this must not be confused with a script-specific exit 1.

Current CI uses `cargo nextest run --workspace --all-targets` and separately
`cargo test --workspace --doc`; its semantic-search oracle is a separate job.
The local commands below are alternatives, not a transcription of that CI job.

**When it runs:**
- CI: every PR + push to `dev`.
- Locally: mandatory before every commit (CLAUDE.md §Rust coding rules pt.3).

**How to run locally:**
```bash
cargo test --workspace                                     # standard run
cargo test --workspace --features test-helpers             # with test-helper escape-hatches
cargo test --workspace --features test-helpers -- --nocapture  # verbose output
```

**Smoke test from CLAUDE.md:**
```bash
cargo fmt && cargo fmt -- --check && cargo check && cargo test
```

These remain required pre-commit checks when applicable; the designated owner
runs them once under the shared lease, not every parallel writer. Keep fmt before
fmt-check, then compilation and tests. Report actual commands/outcomes, not a
generic claim that an unexecuted sequence passed.

**How to fix:** fix the failing test. If a test fails because of a code change,
it signals that the implementation broke expected behavior. Do not change the
test to make it pass without understanding why it failed.

---

## 4. Health gate: `forgeplan health`

**Purpose:** reports graph-orphans (no incoming or outgoing links), blind spots
(eligible decision artifacts without linked evidence), stale and risk signals.
CI `--ci --fail-on` exits nonzero when configured thresholds are exceeded.

**When it runs:**
- Configured `.claude/hooks/pre-commit-health.sh` — advisory PreToolUse check, not a standard Git pre-commit gate.
- `.github/workflows/forgeplan-health.yml` step `Health check` — CI.

**CI command:**
```bash
forgeplan health --ci --fail-on "orphans=10,blind_spots=5"
```

Thresholds are `orphans=10` and `blind_spots=5`. An orphan here is graph
disconnection, not a file/DB mismatch; index lag is a different diagnosis.
Whether the CI result blocks a merge depends on required-check configuration.

**How to run locally:**
```bash
forgeplan health                                              # state overview
forgeplan health --ci --fail-on "orphans=10,blind_spots=5"   # full CI check
```

**How to fix:**

*Graph-orphan artifacts*: inspect whether the artifact should have a real typed
relationship, add only the supported justified link, then inspect health again.
`scan-import` imports foreign Markdown; it does not repair missing graph edges.

*Blind spots*: gather actual evidence, create an EvidencePack with
`forgeplan new evidence "Verification of <ID>"`, fill its observations and scoring
fields through the supported authoring path, then link the returned EVID ID to
the decision with `forgeplan link <EVID-ID> <ID>`. `new --for` is not supported.
Do not manufacture or activate empty evidence merely to clear the dashboard.

*Expiry*: `forgeplan stale` lists overdue records; it does not rewrite status.
`forgeplan renew <ID> --reason "..." --until <YYYY-MM-DD>` requires a permitted
stored-state transition and a justified reassessment; extending a date is not
new measurement. An expired record still stored as active cannot be renewed
as active→active. If obsolete, use the justified deprecation workflow instead.

**Workflow file:** `.github/workflows/forgeplan-health.yml`

---

## 5. Methodology gates: `forgeplan validate` and `forgeplan score`

**Purpose:**
- `forgeplan validate <ID>` — reports implemented schema checks (sections,
  metadata presence, placeholders, depth-specific rules); `--ci` exits 1 on MUST failures.
- `forgeplan score <ID>` — computes R_eff (weakest-link). Does not block CI by
  itself, but feeds into the health check.
- `forgeplan blocked` — shows artifacts blocked by unclosed dependencies.
- `forgeplan order` — topological ordering of work by dependencies.

CLI `validate` works on legacy and migrated Constellation layouts; full MCP
`forgeplan_validate` was also exercised successfully on migrated artifacts.
See the [pilot compatibility notes (RU)](CONSTELLATION-PILOT.ru.md) for the tested scope,
not a guarantee of every registered tool's behavior.
Zero MUST findings does not guarantee activation: length/evidence/stub gates
also apply. Schema recommendations are not all automatically enforced.

**When it runs:**
- `forgeplan validate --ci` — CI step `Validate artifacts`
  (`.github/workflows/forgeplan-health.yml`).
- Methodology smoke test from CLAUDE.md (after every sprint):
  ```bash
  forgeplan validate PRD-XXX && forgeplan score PRD-XXX
  forgeplan blocked && forgeplan order
  ```

**CI command:**
```bash
forgeplan validate --ci   # validates all artifacts; exit 1 on any MUST error
```

**How to run locally:**
```bash
forgeplan validate PRD-001          # single artifact
forgeplan validate --ci             # all artifacts
forgeplan score PRD-001             # R_eff score
forgeplan blocked                   # what is blocked
forgeplan order                     # work order by dependencies
```

**How to fix:**

Validate outputs the list of missing MUST sections or fields with wrong format.
Fix the indicated sections. The validator accepts aliases (CLAUDE.md §Validator aliases):
- `## Problem` = `## Motivation` = `## Problem Statement`
- `## Goals` = `## Success Criteria`
- etc.

If validate fails on a correctly written artifact, the alias used may be
unrecognized — add a standard heading or check the alias list in CLAUDE.md.

**Workflow file:** `.github/workflows/forgeplan-health.yml`

---

## 6. Drift detector: `scripts/check-mcp-tool-count.sh`

**Purpose:** compares the **actual MCP tool count** in source code
(`crates/forgeplan-mcp/src/server.rs`) with **numbers cited in documentation**
(README, CLAUDE.md, website, docs). If documentation diverges from code —
exit 1 (CI failure).

**Background (PROB-050):** during the v0.28.0 audit, an external OpenAI agent
found ~18 locations in documentation with stale MCP-tool counters (several
distinct historical values against the actual count at v0.28.0). This script
checks recognized numeric claims in its configured scan paths. This is not a
complete documentation/registration audit: unrecognized wording, skipped lines
and other profile modules can remain outside the detector. Inspect its actual
scope and output rather than infer universal coverage from a green job.

**Source of truth:** the detector counts `#[tool(...)]` registration attributes
in `crates/forgeplan-mcp/src/server.rs`, excluding undecorated test functions.
It does not count the separate read-profile module.

**Script file:** `scripts/check-mcp-tool-count.sh`

**When it runs:**
- CI job `check-mcp-tool-count-drift` in `.github/workflows/ci.yml`.
  It is separate from the path-filtered health workflow so docs-only PRs run it.
- Not run as a pre-commit hook (too slow for every commit); recommended as a
  pre-push check or manual run before opening a PR.

**What it scans:**

The script searches these paths:
- `CLAUDE.md`
- `README.md`
- `TODO.md`
- `website/src/` (all `.md`, `.tsx`, `.astro`, `.mdx`)
- `docs/` (all `.md`)

Pattern: lines containing `<N> MCP tools`, `<N> инструментов`, `<N> tools`
(only numbers ≥ 10, to avoid matching "3 tools cover..." and similar).

Excluded from checks:
- `CHANGELOG` files and lines — historical numbers are intentionally preserved.
- Lines containing the comment `# mcp-count-drift: ignore`.
- TODO.md lines matching `Previous: v0.*`.

**How to run locally:**
```bash
# Strict mode (same as CI)
./scripts/check-mcp-tool-count.sh

# Warn-only (shows drift without exiting with error)
./scripts/check-mcp-tool-count.sh --warn

# Help
./scripts/check-mcp-tool-count.sh --help
```

**Illustrative output on success** (concrete numbers vary per release —
`<ACTUAL>` is the registration count reported by the detector):
```
Actual MCP tool count (src): <ACTUAL>

No drift — all docs are consistent with src (<ACTUAL>).
```

**Typical output on drift** (`<ACTUAL>` ≠ `<STALE>`):
```
Actual MCP tool count (src): <ACTUAL>

Drift detected (3 lines):
  DRIFT: README.md:42:...<STALE> MCP …    (number=<STALE> context="<STALE> MCP …")
  DRIFT: CLAUDE.md:28:...<STALE> MCP …    (number=<STALE> context="<STALE> MCP …")
  DRIFT: website/src/content/index.mdx:17:...<STALE> MCP …

Resolution: update each location to actual count (<ACTUAL>) OR add a
comment explaining why the historical number is preserved (e.g. CHANGELOG).
```

**How to fix drift:**

1. Read the count and flagged lines using the detector itself:
   ```bash
   ./scripts/check-mcp-tool-count.sh --warn
   ```

2. Update current claims to the matching build/profile count. Do not rewrite
   historical release facts or treat full/read-profile counts as interchangeable.

3. If a specific location must preserve a historical count (e.g., a
   release-notes line referencing the count at a prior version), append a
   drift-ignore comment to that exact line so the script skips it:
   `... <!-- mcp-count-drift: ignore -->` (the marker MUST be on the same
   line as the number; `grep -v` is line-anchored).

4. Re-run the script:
   ```bash
   ./scripts/check-mcp-tool-count.sh   # should return exit 0
   ```

5. If CHANGELOG.md was changed — regenerate the website mirror:
   ```bash
   cd website && node scripts/copy-changelog.mjs
   ```

**Rule for tool authors:** when changing MCP registrations, update current
build/profile claims in documentation before opening a PR. A function named
`forgeplan_*` without `#[tool(...)]` is not itself a registered tool.

The command-drift detector scans `CLAUDE.md` and the shipped forge skill, not
README or all of `docs/**`. It checks command/flag existence, not positional
arguments or runtime success. A green detector does not certify all examples.
CLI/MCP reference regeneration is an explicit maintenance step, not a substitute
for exercising examples; preserve manual explanations and distinguish profiles.

---

## 7. Pre-commit hooks overview (`.claude/hooks/`)

These are configured Claude Code `PreToolUse` handlers, not automatically
installed `.git/hooks`. They complement CI and apply only to matching tool calls.

| Hook file | Blocks | When |
|---|---|---|
| `forge-safety-hook.sh` | Literal blacklist match; not a shell parser/sandbox | configured PreToolUse, `Bash` only |
| `pre-commit-fmt.sh` | Matching commit tool call if fmt drifts | configured PreToolUse, `Bash` |
| `commit-test-check.sh` | Added public functions and zero added `#[test]`; smaller ratios only warn, no function↔test coverage proof | configured PreToolUse, commit command |
| `pr-todo-check.sh` | Matching `gh pr create`/`submit` with open P0 tasks; not pushes | configured PreToolUse, `Bash` |
| `pre-commit-health.sh` | Advisory warnings, exit 1; missing tool/workspace skips | configured PreToolUse, commit command |

**Note:** hooks are in `.claude/hooks/` (for Claude Code integration), not in
the standard `.git/hooks/`. They are activated by the Claude Code environment
as `PreToolUse` hooks. Standard git pre-commit hooks operate independently
(if present).

**Detailed guide:** [`docs/operations/AGENT-HOOKS.md`](AGENT-HOOKS.md)

---

## Gate order (developer contract)

Correct order before committing/opening a PR — critical. Wrong order masks
errors and makes them harder to isolate:

```
Before every commit:
  1. cargo fmt                              # fix
  2. cargo fmt -- --check                   # verify
  3. cargo check --workspace                # 0 warnings
  4. cargo test --workspace                 # 0 failures
  5. forgeplan health                       # no critical blind spots

Before PR (additionally):
  6. cargo clippy --workspace --all-targets -- -D warnings   # strict lint
  7. forgeplan validate --ci                # all artifacts pass
  8. ./scripts/check-mcp-tool-count.sh     # no drift
```

For a methodology smoke check, create a separate sandbox outside existing
workspaces, create a PRD and fill its required sections **before** validation.
Record real evidence rather than a sample test title; do not expect an unfilled
template to pass. Select the compatible CLI/MCP operation for the layout.
FPF ingest/search is a separate check with its own corpus prerequisite.

---

## CI workflow anatomy: `forgeplan-health.yml`

**File:** `.github/workflows/forgeplan-health.yml`

**Trigger:** PR to `dev` or `main`, on changes in `.forgeplan/**` or `crates/**`.

**Steps (in execution order):**

1. Pinned `actions/checkout` — code checkout
2. Install system dependencies (`protobuf-compiler`)
3. `dtolnay/rust-toolchain@stable` — Rust toolchain
4. `Swatinem/rust-cache@v2` — Cargo cache
5. `cargo build -p forgeplan` — CLI build
6. **Rebuild index** — `forgeplan reindex` from tracked canonical Markdown;
   no workspace deletion, partial copy-back or `scan-import` round-trip
7. **Health check** — `forgeplan health --ci --fail-on "orphans=10,blind_spots=5"`
8. **Validate artifacts** — `forgeplan validate --ci`

Each step is an independent exit-code barrier. If the health check fails,
validation is not reached. MCP count drift is a separate job in `ci.yml`.

---

## Common scenarios and solutions

### "PR blocked by drift-check after adding a new MCP tool"

```bash
# 1. Read the current registration count from the detector
./scripts/check-mcp-tool-count.sh --warn

# 2. Find all locations to update
./scripts/check-mcp-tool-count.sh --warn

# 3. Update each flagged location (CLAUDE.md, README.md, website/, docs/)

# 4. If CHANGELOG.md was changed — regenerate website
cd website && node scripts/copy-changelog.mjs

# 5. Verify
./scripts/check-mcp-tool-count.sh   # exit 0
```

### "forgeplan health fails in CI but passes locally"

An out-of-date derived index is one possible cause, not a diagnosis. CI rebuilds
from tracked Markdown using `reindex`. Preserve canonical data while comparing:

1. Stop competing writers; back up the **entire** `.forgeplan/` and MCP config
   outside the workspace. An export alone is not a full backup.
2. Check the discovered workspace path and local config. Parent discovery and
   missing config can explain a different result; `init -y` is not a reset.
3. Run the matching binary's `forgeplan reindex` from the workspace root.
   On migrated layout it delegates to Constellation; explicit `forgeplan index`
   is also available. Run `forgeplan embed` if semantic coverage is required.
4. Inspect expected artifacts and diagnostic messages, then run the applicable
   health gate. Check layout compatibility before CLI `validate`.

Do not delete `.forgeplan/`: it contains the canonical documents, config and
local state, not just a disposable index. `init --force` keeps artifact bodies
but activates default config; custom edits (including `embedding`) remain only
in backup until restored. See the [portable recovery/rollback runbook](CONSTELLATION-PILOT.ru.md).

### "`cargo fmt --check` fails in CI but passes locally"

Check rustfmt version:
```bash
rustfmt --version   # should match CI (stable)
cargo +stable fmt -- --check
```

If versions differ — update the toolchain:
```bash
rustup update stable
```

---

## Cross-references

- [`scripts/check-mcp-tool-count.sh`](../../scripts/check-mcp-tool-count.sh) — drift detector (source with inline comments)
- [`.github/workflows/forgeplan-health.yml`](../../.github/workflows/forgeplan-health.yml) — full CI workflow
- [`docs/operations/AGENT-HOOKS.md`](AGENT-HOOKS.md) — detailed pre-commit hooks guide
- [`docs/methodology/QUALITY-GATES.md`](../methodology/QUALITY-GATES.md) — methodology gates (Verification Gate, R_eff, Adversarial Review)
- [`docs/methodology/release-workflow.md`](../methodology/release-workflow.md) — release pre-conditions checklist
- [`CLAUDE.md`](../../CLAUDE.md) §Hooks enforcement — short hooks reference table
