[English](QUALITY-GATES.md) · [Русский](QUALITY-GATES.ru.md)

# CI Quality Gates — Infrastructure Reference (v0.28.0+)

Reference for all automated quality gates that run in Forgeplan CI and local
hooks. For each gate: **what it checks**, **when it runs**, **how to run
locally**, **how to fix common failures**.

> **Related document**: [`docs/methodology/QUALITY-GATES.md`](../methodology/QUALITY-GATES.md)
> covers methodology gates (Verification Gate, Adversarial Review, R_eff scoring).
> This document covers CI/CD infrastructure, not methodology.

---

## Gate Overview

| Gate | Trigger | Blocks |
|---|---|---|
| `cargo fmt --check` | pre-commit hook + CI | commit, PR merge |
| `cargo check` / `cargo clippy` | pre-commit hook + CI | commit, PR merge |
| `cargo test` | CI (PR, push dev) | PR merge |
| `forgeplan health` | CI + pre-commit hook | PR merge |
| `forgeplan validate` | CI | PR merge |
| MCP tool count drift detector | CI (PR, push dev) | PR merge |

Hooks in `.claude/hooks/` are a local safety net before CI. CI gates in
`.github/workflows/forgeplan-health.yml` are the final barrier before merge.

---

## 1. Formatting: `cargo fmt --check`

**Purpose:** verifies Rust source code is formatted according to `rustfmt`
(no uncommitted diff after auto-format).

**When it runs:**
- `pre-commit-fmt.sh` — pre-commit hook. Runs `cargo fmt --check`; if dirty
  diff is found, aborts `git commit` with an explanation.
- CI — every PR to `dev` or `main`.

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
The `.claude/hooks/pre-commit-fmt.sh` hook blocks commits automatically,
but do not rely on the hook alone — run `cargo fmt` manually at the end of
every coding session.

---

## 2. Static analysis: `cargo check` and `cargo clippy`

**Purpose:** `cargo check` verifies compilability without building binaries.
`cargo clippy` adds lint rules with `-D warnings` — any warning causes exit 1.

**When it runs:**
- Locally: after each code change (recommended practice).
- CI: every PR to `dev` or `main`.

**Full CI command:**
```bash
cargo clippy --workspace --all-targets --features test-helpers -- -D warnings
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

**Purpose:** runs the full workspace test suite. Any test failure causes exit 1.

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

This sequence is mandatory before any commit. Order matters: fmt first
(otherwise fmt-check fails on the next step), then check (faster than test),
then test.

**How to fix:** fix the failing test. If a test fails because of a code change,
it signals that the implementation broke expected behavior. Do not change the
test to make it pass without understanding why it failed.

---

## 4. Health gate: `forgeplan health`

**Purpose:** checks the workspace artifact health — no orphan artifacts
(in DB but not in files), blind spots (active without evidence), stale artifacts.
In CI mode (`--ci --fail-on`) returns exit 1 when thresholds are exceeded.

**When it runs:**
- `.claude/hooks/pre-commit-health.sh` — pre-commit hook (locally).
- `.github/workflows/forgeplan-health.yml` step `Health check` — CI.

**CI command:**
```bash
forgeplan health --ci --fail-on "orphans=10,blind_spots=5"
```

Thresholds: `orphans=10` (tolerates up to 10 orphans — lance rebuild may lag),
`blind_spots=5` (more than 5 active without evidence — methodological debt,
blocks merge).

**How to run locally:**
```bash
forgeplan health                                              # state overview
forgeplan health --ci --fail-on "orphans=10,blind_spots=5"   # full CI check
```

**How to fix:**

*Orphan artifacts* (in files, not in DB):
```bash
forgeplan scan-import   # reindex markdown → LanceDB
forgeplan health        # verify orphans=0
```

*Blind spots* (active without evidence):
```bash
forgeplan health        # see blind spots list
# For each blind spot:
forgeplan new evidence --for <ID> "Smoke evidence"  # or full EVID
forgeplan activate <EVID-ID>
```

*Stale artifacts* (TTL expired):
```bash
forgeplan stale                    # list stale artifacts
forgeplan renew <ID> --reason "..." --until 2026-08-01
# or if completely obsolete:
forgeplan deprecate <ID> --reason "obsolete"
```

**Workflow file:** `.github/workflows/forgeplan-health.yml`

---

## 5. Methodology gates: `forgeplan validate` and `forgeplan score`

**Purpose:**
- `forgeplan validate <ID>` — checks an artifact against its schema (MUST sections,
  frontmatter, format). Returns exit 1 on errors.
- `forgeplan score <ID>` — computes R_eff (weakest-link). Does not block CI by
  itself, but feeds into the health check.
- `forgeplan blocked` — shows artifacts blocked by unclosed dependencies.
- `forgeplan order` — topological ordering of work by dependencies.

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
found 18 locations in documentation with stale tool counts (28 / 37 / 45 / 47
tools vs. actual 63). This script prevents recurrence: every PR that adds or
removes an MCP tool will fail CI until documentation is updated.

**Source of truth:** count of async functions matching `async fn forgeplan_*(`
in `crates/forgeplan-mcp/src/server.rs`.

**Script file:** `scripts/check-mcp-tool-count.sh`

**When it runs:**
- CI step `MCP tool count drift check` in `.github/workflows/forgeplan-health.yml`.
  Runs **last** in the health-gate sequence: fmt → build → reindex →
  health → validate → drift-check.
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

**Typical output on success:**
```
Actual MCP tool count (src): 63

No drift — all docs are consistent with src (63 tools).
```

**Typical output on drift:**
```
Actual MCP tool count (src): 65

Drift detected (3 lines):
  DRIFT: README.md:42:...63 MCP tools...  (number=63 context="63 MCP tools")
  DRIFT: CLAUDE.md:28:...63 MCP tools...  (number=63 context="63 MCP tools")
  DRIFT: website/src/content/index.mdx:17:...63 MCP tools...

Resolution: update each location to actual count (65) OR add a
comment explaining why the historical number is preserved (e.g. CHANGELOG).
```

**How to fix drift:**

1. Find the actual count:
   ```bash
   grep -cE 'async fn forgeplan_' crates/forgeplan-mcp/src/server.rs
   ```

2. Update all locations listed in the drift output:
   ```bash
   # Example: new count = 65
   # Edit CLAUDE.md "## Current status" line:
   # "63 MCP tools" → "65 MCP tools"
   ```

3. If a specific location should preserve a historical count (e.g., "there were
   37 tools before v0.22.0"), add a drift-ignore comment:
   ```markdown
   <!-- mcp-count-drift: ignore -->
   there were 37 tools before v0.22.0, now 65
   ```

4. Re-run the script:
   ```bash
   ./scripts/check-mcp-tool-count.sh   # should return exit 0
   ```

5. If CHANGELOG.md was changed — regenerate the website mirror:
   ```bash
   cd website && node scripts/copy-changelog.mjs
   ```

**Rule for tool authors:** when adding a new MCP tool (`async fn forgeplan_<name>`),
**always** update the tool count in CLAUDE.md `## Current status` and README.md
before opening a PR. Otherwise CI will fail at the drift-check step.

---

## 7. Pre-commit hooks overview (`.claude/hooks/`)

Hooks are a local safety net that run before `git commit`. They do not replace
CI but catch issues earlier.

| Hook file | Blocks | When |
|---|---|---|
| `forge-safety-hook.sh` | Destructive commands (`rm -rf /`, `cargo publish`, `DROP TABLE`, `git push --force`) | pre-tool-use in Claude Code |
| `pre-commit-fmt.sh` | Commit if `cargo fmt --check` shows drift | git pre-commit |
| `commit-test-check.sh` | Commit if diff contains a new `pub fn` without a test | git pre-commit |
| `pr-todo-check.sh` | Push if PR has unclosed P0 tasks | git pre-push |
| `pre-commit-health.sh` | Commit if `forgeplan health` reports critical issues | git pre-commit |

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

Full smoke test from CLAUDE.md:
```bash
cargo fmt && cargo fmt -- --check && cargo check && cargo test
forgeplan init -y && forgeplan new prd "Smoke" && forgeplan validate PRD-XXX
forgeplan score PRD-XXX && forgeplan blocked && forgeplan order
forgeplan fpf ingest && forgeplan fpf search "trust"
```

---

## CI workflow anatomy: `forgeplan-health.yml`

**File:** `.github/workflows/forgeplan-health.yml`

**Trigger:** PR to `dev` or `main`, on changes in `.forgeplan/**` or `crates/**`.

**Steps (in execution order):**

1. `actions/checkout@v4` — code checkout
2. Install system dependencies (`protobuf-compiler`)
3. `dtolnay/rust-toolchain@stable` — Rust toolchain
4. `Swatinem/rust-cache@v2` — Cargo cache
5. `cargo build -p forgeplan` — CLI build
6. **Rebuild index** — `forgeplan init -y` + copy markdown + `scan-import`
   (reconstructs LanceDB from tracked markdown files)
7. **Health check** — `forgeplan health --ci --fail-on "orphans=10,blind_spots=5"`
8. **Validate artifacts** — `forgeplan validate --ci`
9. **MCP tool count drift check** — `./scripts/check-mcp-tool-count.sh`

Each step is an independent exit-code barrier. If step 7 fails, steps 8 and 9
do not run (GitHub Actions fail-fast behavior).

---

## Common scenarios and solutions

### "PR blocked by drift-check after adding a new MCP tool"

```bash
# 1. Get the current count
grep -cE 'async fn forgeplan_' crates/forgeplan-mcp/src/server.rs

# 2. Find all locations to update
./scripts/check-mcp-tool-count.sh --warn

# 3. Update each flagged location (CLAUDE.md, README.md, website/, docs/)

# 4. If CHANGELOG.md was changed — regenerate website
cd website && node scripts/copy-changelog.mjs

# 5. Verify
./scripts/check-mcp-tool-count.sh   # exit 0
```

### "forgeplan health fails in CI but passes locally"

Cause: local LanceDB may contain stale data from previous init-s. CI always
does a clean init + scan-import. To reproduce locally:

```bash
forgeplan export --output backup-$(date +%Y%m%d).json
cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)
rm -rf .forgeplan && forgeplan init -y
# Copy tracked markdown back:
cp -r .forgeplan-backup-*/prds/ .forgeplan/
cp -r .forgeplan-backup-*/rfcs/ .forgeplan/
# ... etc.
forgeplan scan-import
forgeplan health --ci --fail-on "orphans=10,blind_spots=5"
```

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
