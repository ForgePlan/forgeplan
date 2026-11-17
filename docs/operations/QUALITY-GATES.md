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
| `cargo fmt --check` | Claude Code commit hook + CI | agent commit, PR merge when required |
| `cargo check` / `cargo clippy` | CI (default + `semantic-search`) | PR merge when required |
| Tests (`nextest` + doc-tests) | CI (PR, push dev) | PR merge when required |
| Embedding correctness oracle | CI with cached model | PR merge when required |
| `forgeplan health` | path-filtered CI; local hook warns | PR merge when required |
| `forgeplan validate --ci` | path-filtered CI | PR merge when required |
| MCP tool count drift detector | `ci.yml` (every PR, push dev) | PR merge when required |
| Website build, command drift, CLI surface smoke | `ci.yml` | PR merge when required |
| `cargo-deny check` | push `dev`/`main`, weekly schedule, manual dispatch | post-merge security signal; not a PR job |

Hooks in `.claude/hooks/` intercept configured Claude Code tool calls, not
arbitrary shell or Git operations. CI is defined in `ci.yml`,
`forgeplan-health.yml`, and `security.yml`; GitHub rulesets determine
which checks actually block merging.

`security.yml` runs `cargo deny check advisories licenses bans sources`
after pushes, not on `pull_request`. Inspect the security result for the
release SHA before tagging; a green PR alone does not include that scan.

---

## 1. Formatting: `cargo fmt --check`

**Purpose:** verifies Rust source code is formatted according to `rustfmt`
(no uncommitted diff after auto-format).

**When it runs:**
- `pre-commit-fmt.sh` — Claude Code `PreToolUse` hook for Bash commands
  containing `git commit`; exits 2 when `cargo fmt --check` fails.
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

Formatting is mandatory before every commit (CLAUDE.md §Rust coding rules).
The configured Claude Code hook blocks intercepted commits, but does not
install a native Git pre-commit hook or cover commits from other clients.

---

## 2. Static analysis: `cargo check` and `cargo clippy`

**Purpose:** `cargo check` verifies compilability without building binaries.
`cargo clippy` adds lint rules with `-D warnings` — any warning causes exit 1.

**When it runs:**
- Locally: after each code change (recommended practice).
- CI: every PR to `dev` or `main`.

**CI commands:** `ci.yml` sets `RUSTFLAGS="-Dwarnings"` and runs:
```bash
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets
cargo check --workspace --all-targets --features semantic-search
cargo clippy --workspace --all-targets --features semantic-search
```

**How to run locally:** use the same commands with `RUSTFLAGS="-Dwarnings"`.
`test-helpers` is not the CI feature configuration; compiling the semantic
configuration is separate from running the embedding correctness oracle.

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

**Purpose:** runs workspace tests. CI uses `nextest` for test binaries and
`cargo test --doc` separately; semantic inference has its own model-backed job.

**When it runs:**
- CI: every PR + push to `dev`.
- Locally: mandatory before every commit (CLAUDE.md §Rust coding rules pt.3).

**How to run locally:**
```bash
cargo nextest run --workspace --all-targets                 # CI test binaries
cargo test --workspace --doc                               # CI doc-tests
cargo test --workspace                                    # alternative local runner
```

The `Embedding correctness oracle` job separately runs the feature-gated
reference tests with a cached ~2.1 GB model and a guard that fails instead
of silently skipping missing weights. Default workspace tests do not
exercise that configuration.

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

**Purpose:** reports artifact-graph health. Orphans have no incoming or
outgoing links; blind spots are active decisions without supporting evidence.
These are not file/index mismatch counts. `--ci --fail-on` returns exit 1
when the configured counts exceed their thresholds.

**When it runs:**
- `.claude/hooks/pre-commit-health.sh` — advisory Claude Code hook, not a blocking Git hook.
- `.github/workflows/forgeplan-health.yml` step `Health check` — CI.

**CI command:**
```bash
forgeplan health --ci --fail-on "orphans=10,blind_spots=5"
```

Thresholds: `orphans=10` tolerates up to 10 unlinked artifacts;
`blind_spots=5` tolerates up to 5 active decisions without evidence.
Exceeding either fails this CI step.

**How to run locally:**
```bash
forgeplan health                                              # state overview
forgeplan health --ci --fail-on "orphans=10,blind_spots=5"   # full CI check
```

**0.38.0 JSON gate limitation (core #545):** `health --json --ci`
returns from the JSON branch before evaluating the `--ci --fail-on`
thresholds. Keep the text-mode commands above for that gate; JSON output
alone is not its exit-code equivalent. `health --strict --json` has a
different, stricter contract (including failure on any orphan or blind
spot), not a replacement that preserves the configured thresholds.

**How to fix:**

*Orphan artifacts* (no links): inspect `forgeplan get <ID>` and add a
meaningful relation to the relevant decision or evidence. Reindexing does
not create missing relationships. For a stale file-derived index, use:
```bash
forgeplan reindex       # sync existing .forgeplan markdown → LanceDB
forgeplan health
```
`scan-import` discovers new artifacts in arbitrary markdown; it is not an
index rebuild command.

*Blind spots* (active decisions without evidence): collect real results,
create `forgeplan new evidence "Verification results"`, and fill the pack
with the actual methods, results, `verdict`, `congruence_level`, and
`evidence_type` through `forgeplan update`. Link the returned ID:
```bash
forgeplan link <EVID-ID> <DECISION-ID> --relation informs
forgeplan validate <EVID-ID> --ci
forgeplan activate <EVID-ID>
forgeplan score <DECISION-ID>
```
Creating or activating an empty template is not evidence.

*Stale artifacts* (TTL expired):
```bash
forgeplan stale                    # list stale artifacts
forgeplan renew <ID> --reason "Reverified against current sources" --until <YYYY-MM-DD>
# or if completely obsolete:
forgeplan deprecate <ID> --reason "obsolete"
```

**Workflow file:** `.github/workflows/forgeplan-health.yml`

---

## 5. Methodology gates: `forgeplan validate` and `forgeplan score`

**Purpose:**
- `forgeplan validate <ID>` — checks an artifact against its schema (MUST sections,
  frontmatter, format). Use `--ci` to return exit 1 on MUST failures.
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
found ~18 locations in documentation with stale MCP-tool counters (several
distinct historical values against the actual count at v0.28.0). This script
prevents recurrence: every PR that adds or removes an MCP tool now fails CI
until documentation catches up. Numbers intentionally kept abstract here so
this section does not itself become a drift target — see the live count
emitted by `scripts/check-mcp-tool-count.sh`.

**Source of truth:** registered `#[tool(...)]` attributes in
`crates/forgeplan-mcp/src/server.rs`. Counting `async fn forgeplan_*`
also counts undecorated test functions and can overstate the tool total.

**Script file:** `scripts/check-mcp-tool-count.sh`

**When it runs:**
- Independent `Check MCP tool count drift` job in `.github/workflows/ci.yml`,
  on every PR to `dev`/`main` and push to `dev`, with no path filter.
  It first self-tests its extraction regex, then scans the documentation.
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

**Typical output on success** (concrete numbers vary per release —
`<ACTUAL>` is the registered-tool count emitted by the script):
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

1. Find the actual count:
   ```bash
   grep -cE '^[[:space:]]*#\[tool\(' crates/forgeplan-mcp/src/server.rs
   ```

2. Update all locations listed in the drift output. Use the actual count
   from step 1 (replace `<NEW>` with the value `grep` returned), and update
   each line in CLAUDE.md, README.md, etc. that names the MCP tool count.

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

**Rule for tool authors:** when adding a new MCP tool (`async fn forgeplan_<name>`),
**always** update the tool count in CLAUDE.md `## Current status` and README.md
before opening a PR. Otherwise CI will fail at the drift-check step.

---

## 7. Pre-commit hooks overview (`.claude/hooks/`)

Hooks intercept matching Claude Code tool calls when wired in
`.claude/settings.json`. They do not run for every native Git operation
and do not replace CI or remote rulesets.

| Hook file | Blocks | When |
|---|---|---|
| `forge-safety-hook.sh` | Destructive commands (`rm -rf /`, `cargo publish`, `DROP TABLE`, `git push --force`) | pre-tool-use in Claude Code |
| `pre-commit-fmt.sh` | Commit if `cargo fmt --check` shows drift | Claude Code Bash `git commit` |
| `commit-test-check.sh` | Commit when its new-function/test heuristic fails | Claude Code Bash `git commit` |
| `pr-todo-check.sh` | PR creation with unchecked P0 TODO items | Claude Code Bash `gh pr create` / `gh pr submit` |
| `pre-commit-health.sh` | Advisory warning about blind spots (exit 1, not blocking exit 2) | Claude Code Bash `git commit` |
| `pre-pr-evidence-check.sh` | Recognized referenced artifacts without detected linked evidence | Claude Code Bash recognized `gh pr create` |

**Note:** hooks are in `.claude/hooks/` (for Claude Code integration), not in
the standard `.git/hooks/`. They are activated by the Claude Code environment
as `PreToolUse` hooks. Standard git pre-commit hooks operate independently
(if present).

`pre-pr-evidence-check.sh` is registered in `.claude/settings.json`.
It recognizes `gh pr create` only as the first command after whitespace
and simple leading environment assignments. It extracts numeric artifact
IDs from the branch and last 20 commits, skips EVID/NOTE IDs, and checks
evidence-link presence (graph lookup, then a heuristic relations-section
fallback), not evidence quality or completeness. No recognized refs means
pass-through; a missing Forgeplan binary also warns and skips the check.

Its bypasses are `FORGEPLAN_SKIP_EVIDENCE=1` and branches matching
`docs/*`, `chore/sync-*`, `chore/dependabot-*`, `release/v*`, or `hotfix/*`.
It also recognizes a hook-only `--no-evidence-check` token in the unquoted
argument prefix; the hook does not remove it or make it a supported `gh`
option. Detected missing evidence blocks with exit 2; an allowed call
does not certify that every relevant artifact or result was checked.

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

For CLI end-to-end smoke, use the repository's disposable-workspace
scripts rather than creating a half-filled PRD in the working project:
```bash
bash scripts/smoke-test.sh --verbose
bash scripts/check-doc-command-drift.sh --bin target/debug/forgeplan
bash scripts/cli-surface-exercise.sh --bin target/debug/forgeplan
```
Build the binary first. Model- or LLM-dependent surfaces may be reported
as `EXTERNAL` by the surface script; that is not a passed functional check.

---

## CI workflow anatomy: `forgeplan-health.yml`

**File:** `.github/workflows/forgeplan-health.yml`

**Trigger:** PR to `dev` or `main`, on changes in `.forgeplan/**` or `crates/**`.

**Steps (in execution order):**

1. Checkout (pinned action SHA)
2. Install system dependencies (`protobuf-compiler`)
3. Install stable Rust and configure Cargo cache
4. `cargo build -p forgeplan`
5. **Rebuild index** — `./target/debug/forgeplan reindex`
6. **Health check** — `./target/debug/forgeplan health --ci --fail-on "orphans=10,blind_spots=5"`
7. **Validate artifacts** — `./target/debug/forgeplan validate --ci`

Each step is an exit-code barrier; later ordinary steps are skipped after
a failure. Formatting, tests, and MCP count drift are separate `ci.yml`
jobs, not later steps in this health job.

---

## Common scenarios and solutions

### "PR blocked by drift-check after adding a new MCP tool"

```bash
# 1. Get the current count
grep -cE '^[[:space:]]*#\[tool\(' crates/forgeplan-mcp/src/server.rs

# 2. Find all locations to update
./scripts/check-mcp-tool-count.sh --warn

# 3. Update each flagged location (CLAUDE.md, README.md, website/, docs/)

# 4. If CHANGELOG.md was changed — regenerate website
cd website && node scripts/copy-changelog.mjs

# 5. Verify
./scripts/check-mcp-tool-count.sh   # exit 0
```

### "forgeplan health fails in CI but passes locally"

One possible cause is a stale local index. CI rebuilds the tracked
markdown with `reindex`, not `init` plus `scan-import`. Compare the exact
PR checkout in a disposable clone, then run:

```bash
./target/debug/forgeplan reindex
./target/debug/forgeplan health --ci --fail-on "orphans=10,blind_spots=5"
./target/debug/forgeplan validate --ci
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
