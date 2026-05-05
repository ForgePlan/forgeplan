---
title: Changelog
description: "Forgeplan release notes — every public version with added features, fixes, and breaking changes."
---

All notable changes to Forgeplan are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/). Semver is `MAJOR.MINOR.PATCH`
with pre-1.0 minor bumps for breaking changes.

The canonical source is [`CHANGELOG.md`](https://github.com/ForgePlan/forgeplan/blob/main/CHANGELOG.md)
in the repository. This page is generated from it at build time via `scripts/copy-changelog.mjs`.

---

All notable changes to Forgeplan are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/). Semver: `MAJOR.MINOR.PATCH`
with pre-1.0 minor bumps for breaking changes.

This file starts at v0.17.0. For prior releases, see git tags and the
corresponding sprint evidence under `.forgeplan/evidence/`.

## [Unreleased]

## [0.29.0] — 2026-05-05 — verdict aggregator + typed errors + claude --print refactor + CWE-426 hardening

Bundles 10 merge-PRs (#239..#248) since v0.28.0 (2026-05-03). Five
load-bearing themes:

**(1) PROB-029 — typed `Verdict` aggregator on `HealthReport`** —
`forgeplan health` теперь возвращает структурированный `Empty / Healthy /
NeedsAttention / Unhealthy` verdict с configurable thresholds, MCP +
CLI surfaces консистентны, banner driven off verdict (eliminates the
pre-fix "Project healthy!" disagreeing with `next_actions` printed
above it). Round 4 + Round 5 + Round 6 audit closures: `_next_action`
ladder checks active_stubs + possible_duplicates + phase_mismatches
before fallthrough; `Verdict::Empty` is now a proper 4th variant
(was deferred at Round 5 via manual overrides; resolved by-construction
in Round 6).

**(2) PROB-049 — typed errors H-class** —
`MutationError::StoreError` split into `StoreTransient` (recoverable) +
`StoreFatal` (not recoverable). `MutationContext<'_>` introduced for
all 17 projection helpers, replacing separate `(workspace, store)`
arguments. `# Errors` rustdoc added to all 17 helpers. Both new public
types are `#[non_exhaustive]`. **BREAKING for direct library consumers**
of `forgeplan-core`; CLI/MCP surfaces unaffected. Round 6 audit honesty
note: `is_recoverable()` is intentionally infrastructure-only in
v0.29.0; first MCP retry-loop consumer wires in v0.30.0.

**(3) PROB-050 — `claude --print` dispatch refactor (A-4..A-15)** —
Single source of truth in `playbook::dispatch::claude_print`:
`invoke()` (full 9-step orchestration), `build_argv()` (argv +
both security gates inline), `parse_envelope()` (UTF-8-trimmed JSON
decode — silent divergence закрыт), `format_timeout_msg()` (uniform
second/millisecond rendering — Round 6 audit closure: sub-second
durations now render `Nms` instead of truncating to "0s"). 
`helpers::which_in_path` consolidated; 3 identical local copies
removed. `DISPATCH_ENV_LOCK` (cfg(test)) closes cross-test PATH-
mutation race. **Argv shape byte-identical pre/post**; agent diagnostic
strings unified to plugin's pre-existing format (Round 6 audit
honesty correction — pre-claim of "no behaviour change" was incorrect
about diagnostic strings while correct about argv shape).

**(4) PROB-050 A-14 — CWE-426 binary-substitution mitigation
fully closed** (Round 6 broadens original mitigation):
- **Env-var path**: `$FORGEPLAN_CLAUDE_BIN` / `$FORGEPLAN_BIN` gated
  behind `#[cfg(test)]` — release binaries do not contain the
  symbol.
- **Struct-API path** (Round 6 audit HIGH-1): `pub claude_binary` field
  + `pub with_claude_binary` builder — equivalent compile-time injection
  surface — demoted to private field; builder gated behind
  `cfg(any(test, all(feature = "test-helpers", debug_assertions)))`.
  Symmetric across `AgentDispatcher` + `PluginDispatcher`. Real E2E:
  `strings target/release/forgeplan | grep -c FORGEPLAN_CLAUDE_BIN` →
  `0` (symbol absent from release binary).

**(5) PROB-051 — Wave-1 audit closures** (cherry-picked from
`integration/w1-audit-v3`): EVID-103 documents Round 4 + Round 5
audit on the integration branch (8 HIGH found+closed Round 4; 7 NEW
HIGH found Round 5 — 4 closed inline, 3 architectural deferred to
PROB-051 itself).

### Pre-conditions verified before cutting

- `cargo fmt --check` clean (0 diffs)
- `cargo clippy --workspace --all-targets --features test-helpers
  -- -D warnings` clean (0 warnings)
- `cargo test --workspace --features test-helpers` все PASS
  (1977 tests, 0 failed, 5 ignored, 38 suites)
- `forgeplan health` clean — verdict `Healthy`, 0 blind spots,
  0 orphans, 0 stale
- Real E2E #1 (Verdict::Empty release binary) PASS
- Real E2E #2 (CWE-426 strings binary check, 0 occurrences) PASS
- Real E2E #3 (real workspace health smoke) PASS
- Real E2E #4..#6 (format_timeout_msg, dispatch parity, validate) PASS

### Security — Dependabot (Round 4 triage, 2026-05-05)

- 17 of 18 alerts from v0.28.0 round 3 closed automatically with the
  v0.28.0 → main merge (rust patches + npm audit fix cycle).
- **1 alert remains open**: `lru 0.12.5` ([Alert #3], CVSS 0.0,
  Miri-only stacked-borrows in `IterMut`). Transitive via
  `tantivy → lance → lancedb`; no direct exploit surface. Carry-forward
  to v0.30.0 — accepted-with-justification, re-evaluate on
  tantivy 0.25 release. Full triage:
  [`docs/operations/dependabot-triage-2026-05-05.md`](docs/operations/dependabot-triage-2026-05-05.md).

### Added (CI infrastructure, methodology)

- **PROB-050 A-30 ✅ closes — `docs/operations/QUALITY-GATES.{md,ru.md}`
  documents all CI quality gates** (fmt, clippy, test, health, validate,
  drift detector). Cross-referenced from `CLAUDE.md §Hooks enforcement`
  and `docs/methodology/release-workflow.md §Pre-conditions`.

### Changed (forgeplan-core public API — BREAKING for direct library consumers)

- **PROB-050 A-7 ✅ closes — `playbook::dispatch::claude_print` symbol
  visibility tightened**. Following empirical verification (`rg <name>
  crates/`) that no in-tree consumer outside the dispatch module reads
  these symbols, the following `pub` items were tightened:
  - `DEFAULT_BUDGET_USD`, `DEFAULT_ALLOWED_TOOLS` → `pub(crate)`
  - `helpers::DEFAULT_TIMEOUT_SECS`, `helpers::MAX_OUTPUT_BYTES`,
    `plugin_dispatcher::DEFAULT_PLUGIN_TIMEOUT_SECS` → `pub(crate)`
  - `ClaudePrintResponse` (struct) + its methods → `pub(super)`
  - `assemble_prompt`, `add_dir_for_produces_at`,
    `effective_allowed_tools`, `effective_budget_usd` → `pub(super)`

  External crates that imported these symbols will fail to compile against
  v0.29.0. Recommended migration: invoke the dispatch module through its
  public surface (`AgentDispatcher` / `PluginDispatcher`) rather than
  reaching into helpers. If a use case requires a tightened symbol, open
  a PROB issue justifying the public contract.

### Changed (forgeplan-core public API — additive, but downstream library consumers should rebuild)

- **PROB-050 A-4 + A-5 + A-6 + A-11 + A-15 ✅ close — `claude --print`
  dispatch refactor**. Single source of truth in
  `playbook::dispatch::claude_print`:
  - `claude_print::invoke()` — full 9-step orchestration
    (argv + env + prompt + spawn + timeout + parse + render).
    AgentDispatcher and PluginDispatcher reduce to (a) variant unpack,
    (b) name validation, (c) binary resolution, (d) call invoke.
  - `claude_print::build_argv()` — argv construction with both security
    gates inline (`validate_allowed_tools` + `add_dir_for_produces_at`).
    Argv-shape parity between dispatchers now enforced by construction.
  - `claude_print::parse_envelope()` — UTF-8-trimmed JSON envelope decode.
    Plugin dispatcher previously had no `.trim()` — silent divergence
    from agent path closed.
  - `claude_print::format_timeout_msg()` — uniform second/millisecond
    rendering. Agent dispatcher previously leaked `Duration` Debug repr;
    plugin dispatcher used `.as_secs()` only. PR-E Round 6 audit closure:
    sub-second durations now render `Nms` (was: `0s` for any
    `< 1s` timeout, which confused operators chasing tight-loop timeouts).
    Production path is `Step.timeout_seconds: u32 ≥ 1`, so the
    common-case `Ns` rendering is byte-stable.
  - `helpers::which_in_path` promoted from `fn` to `pub(super) fn`;
    3 identical local copies removed from the dispatchers.
  - `claude_print::DISPATCH_ENV_LOCK` — `#[cfg(test)] pub(super) static
    tokio::sync::Mutex<()>` shared by `agent_dispatcher::tests`,
    `plugin_dispatcher::tests`, and `helpers::tests` (Round 5 audit
    Logic LOW-1 + PR-E audit HIGH-1: cross-test PATH-mutation race
    fully closed).
  - **Behaviour delta** (corrected from earlier honesty-of-claim audit):
    argv shape IS byte-identical pre/post; agent-side diagnostic strings
    were unified to plugin's pre-existing format — specifically
    `"failed to decode claude --print JSON envelope"` (was "produced
    unparseable JSON envelope" agent-side), `format_timeout_msg`
    output (was `Duration` Debug repr agent-side), and the new
    `stdout_preview=` failure-context block (added on agent path; plugin
    path always had it). Operators / scripts / log-grep regexes that
    matched the old agent-only strings need a one-line update.

- **PROB-049 H-1 ✅ closes — `MutationError::StoreError` split into typed
  variants `StoreTransient` (recoverable) and `StoreFatal` (not recoverable).**
  The legacy `StoreError(#[from] anyhow::Error)` collapse-everything variant is
  removed. Categorisation logic (`MutationError::from_store_err`) inspects the
  `anyhow::Error` chain (lancedb / std::io shapes) and routes between the two.
  Default fallthrough is `StoreTransient` — strict refinement of legacy
  recoverable=true. **Honesty note** (PR-E Round 6 audit HIGH-2):
  `is_recoverable()` is intentionally infrastructure-only in v0.29.0 —
  no MCP / CLI retry loop currently consumes it. The audit flagged this
  as a risk (variants drifting from real failure modes without a
  consumer). Mitigation: this CHANGELOG block is the load-bearing
  contract; the first MCP retry wiring (tracked as PROB-049 follow-up
  for v0.30.0, candidate: `forgeplan_health` cold-start LanceDB lock
  contention) will close the loop. Until then, downstream library
  consumers calling `is_recoverable()` should treat the boolean as a
  *hint* rather than a stable contract.
- **PROB-049 H-6 ✅ closes — `MutationContext` introduced for projection helpers.**
  All 17 file-first mutation helpers in `forgeplan_core::projection` now take
  `&MutationContext<'_>` instead of separate `(workspace, store)` arguments.
  47 call sites updated across `forgeplan-cli` + `forgeplan-mcp`. The struct
  is `#[non_exhaustive]` and constructed via `MutationContext::new(...)` —
  external library consumers may not use a struct literal.
- **PROB-049 H-4 ✅ closes — `# Errors` rustdoc on all 17 projection helpers.**
- **PROB-029 ✅ closes — typed `Verdict` aggregator (`Empty / Healthy /
  NeedsAttention / Unhealthy`) on `HealthReport`.** Pure
  `compute_verdict[_with]` functions with configurable
  `VerdictThresholds`. Both new public types are `#[non_exhaustive]`. CLI
  `forgeplan health` banner driven off the verdict (no longer disagrees
  with `next_actions`). `next_actions` rewritten to emit concrete
  remediation commands. MCP `forgeplan_health` and CLI `--json` both
  expose `verdict` + `verdict_summary` fields. **PR-E Round 6 audit MED
  closure**: `Verdict::Empty` is now a proper 4th variant (was deferred
  at Round 5 via manual `verdict_summary` overrides on CLI + MCP
  surfaces; both overrides removed in this release because
  `human_summary()` for `Empty` carries the right text by construction).
  CI gates that auto-promoted on `verdict == "healthy"` no longer
  promote uninitialized projects. **Round 5 audit closures (HIGH Logic +
  Documentation)**: MCP `_next_action` ladder now checks active_stubs +
  possible_duplicates + phase_mismatches before the "Project healthy"
  fallthrough (eliminates contradiction-via-different-field); MCP tool
  description advertises the `verdict` field for agent discovery.

### Security

- **PROB-050 A-14 ✅ closes — CWE-426 binary substitution mitigated**
  (PR-E Round 6 audit HIGH closure broadens the original mitigation).
  Two equivalent injection surfaces are now both closed:

  1. **Env-var path**:
     `AgentDispatcher::resolve_claude_binary` and the sibling
     `helpers::resolve_forgeplan_binary` gate their respective
     `$FORGEPLAN_CLAUDE_BIN` / `$FORGEPLAN_BIN` env-var overrides behind
     `#[cfg(test)]`. Release binaries silently ignore both env vars;
     only test builds honour them for fixture wiring. Closes the
     v0.28.0 release-notes promise (audit S-2 escalation, see
     [`docs/operations/phase-b-real-e2e-2026-05-03.md`](docs/operations/phase-b-real-e2e-2026-05-03.md)
     F-RUNTIME-7).

  2. **Struct-API path** (PR-E Round 6 audit HIGH-1, found by
     adversarial security review): `AgentDispatcher::claude_binary` was
     a `pub` field; `with_claude_binary` was a `pub` builder, both
     un-gated. A release-build caller could write attacker-controlled
     paths directly via the struct API, defeating the env-var
     hardening. Both `AgentDispatcher` and `PluginDispatcher` now:
     (a) keep `claude_binary` as a private field (only `new()` writes
     `None`), and (b) gate `with_claude_binary` (and the deprecated
     aliases `with_task_tool` / `with_task_tool_path`) behind
     `#[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]`.
     Pattern mirrors `LanceStore` test-helper gating in
     `crates/forgeplan-core/src/db/store.rs:361-384` —
     `debug_assertions` ensures a downstream consumer who accidentally
     enables `test-helpers` in a `--release` build still gets a
     compile error, not a silent activation. Both surfaces now
     symmetric (architectural audit HIGH-1: pre-fix PluginDispatcher
     had no env path while AgentDispatcher had both, asymmetric
     hardening).

  **Migration for operators** (CLI / brew / binary distributions):
  the env-var path was never a documented contract; operators relying on
  it for production override should pin `claude` via `$PATH` — that is
  the only supported binary-resolution surface at the CLI / playbook
  layer. There is no per-invocation override at the YAML schema (SPEC-003)
  today; tracked as PROB-050 A-31 if such a surface becomes needed.

  **Library consumers** embedding `forgeplan-core` directly: the
  `with_claude_binary(path)` builder is now feature-gated. For test
  wiring, build with `--features test-helpers` (and run in debug
  profile, or unit-test cfg). For production wiring, use `new()` and
  rely on `$PATH` resolution.

### Deferred to v0.30.0 (PR-E Round 6 audit findings — pre-existing surfaces, not v0.29.0 regressions)

- **TOCTOU + symlink-follow in `which_in_path`** (Sec MED-1):
  `is_file()` follows symlinks, no `canonicalize`, no executable-bit
  check. Window between resolve and `Command::spawn` allows TOCTOU
  swap on a writable PATH dir. Pre-existing surface (existed before
  PR-E refactor). Tracked as **PROB-052** — consider canonicalize +
  parent-dir ownership/mode check + path caching on dispatcher.
- **`Delegation::Command` CWE-78 surface** (Sec MED-2):
  `Delegation::Command { command: Vec<String> }` parses directly from
  YAML with no allowlist / signing / user-facing warning. Real shell
  injection vector if playbooks loaded from network/marketplace.
  Tracked as **PROB-053** — gate behind feature flag /
  `--allow-shell` CLI flag, or require signing for marketplace.
- **`assemble_prompt` produces_at injection** (Sec LOW-1):
  workspace-relative path is splice-formatted into natural-language
  prompt; backticks could close markdown code-fence and inject
  prompt-instructions to the agent. Pre-existing surface. Tracked
  as **PROB-054** — validate `produces_at` against
  `^[A-Za-z0-9._/-]+$` before splicing.
- **`claude_print` god-module split** (Arch MED-2):
  module is 1066 LOC with ~9 responsibilities (argv, env, prompt,
  validators, byte-truncation, JSON parsing, failure rendering,
  timeout formatting, test mutex). Tracked as **PROB-055** —
  refactor into `claude_print/{argv.rs, envelope.rs, validators.rs,
  invoke.rs, test_lock.rs}` keeping `mod.rs` as façade. Cosmetic /
  maintainability, not security.
- **MED-1 leaky-abstraction in `compute_verdict_with`** (Arch MED-1):
  stored `HealthReport.verdict` may disagree with MCP-computed
  verdict (which folds in `phase_mismatches`). By-design today;
  consider removing stored field in v0.30.0 or renaming to
  `partial_verdict`. Tracked as **PROB-056**.

## [0.28.0] — 2026-05-03 — file-first invariant compile-enforced + claude --print dispatchers + canonical playbooks

Bundles 14 merge-PRs (#224..#237) since v0.27.0 (2026-04-28). Three
load-bearing themes: **(1) PRD-073 file-first invariant compile-enforced**
(ADR-003 — `LanceStore::*` mutating methods are now `pub(crate)`,
file-first projection wrappers are the only mutation surface), **(2)
ADR-011 Phase B Wave 1** — Plugin and Agent dispatchers shell out to
`claude --print` on the real `claude` 2.1.126 binary, replacing the
fictional `task-tool` from ADR-010, **(3) Track 4-A8 canonical
playbooks** — `release.yaml` + `brownfield-docs.yaml` ship as runnable
templates for marketplace skill/mapping authors.

Real-E2E verification of Phase B Wave 1 (PR 1 / 2026-05-03,
NOTE-049 + EVID-097): 5 measured real `claude --print` invocations
(3 happy-path success + 1 budget-error envelope decode + 1 retracted
env-export attempt), byte-identical argv recording wrapper, validation
guard reject in 0.01s. ADR-011 R_eff = 0.70 grade B (3 evidence packs,
all CL3 supports).

Dependabot: 16 of 18 open alerts auto-close on this `release/v0.28.0
→ main` merge (lockfile in dev already at patch versions per round 2 +
round 3 triage). 2 carry-forward (lru transitive via tantivy, uuid
transitive via mermaid) с обоснованием в `docs/operations/dependabot-triage-2026-05-03.md`.

Pre-conditions verified before cutting: cargo fmt clean, cargo clippy
--workspace --all-targets --features test-helpers -- -D warnings clean,
cargo test --workspace --features test-helpers all PASS (1614+ tests),
forgeplan health clean.

### Added (CI infrastructure)

- **`scripts/check-mcp-tool-count.sh`** — drift detector: compares actual MCP
  tool count in `crates/forgeplan-mcp/src/server.rs` against all documentation
  locations (README, CLAUDE.md, website, docs). Introduced after a v0.28.0
  release audit (external OpenAI agent) found 18 stale references across the
  repo (counts 28 / 37 / 45 / 47 vs actual 63). Script exits 1 on any mismatch
  so CI blocks PRs that add/remove tools without updating docs. Supports
  `--warn` mode for local development and inline `# mcp-count-drift: ignore`
  escape hatch for intentional historical counts.
- **`.github/workflows/forgeplan-health.yml`** step `MCP tool count drift check`:
  wires the drift detector as the final gate of the Architecture Health workflow
  (after `forgeplan health` + `forgeplan validate`). Closes PROB-050 A-30
  "preventive value theoretical" finding (was doc-only, now enforced in CI).
- See [`docs/operations/QUALITY-GATES.ru.md`](docs/operations/QUALITY-GATES.ru.md)
  for full CI gate reference (fmt / clippy / test / health / validate /
  drift-check), including how to run each gate locally and fix common failures.

### Verification (PR 1 + PR 2.5 closures, 2026-05-03 / 2026-05-04)

- **NOTE-049** + **EVID-097**: real-E2E closure of Phase B Wave 1.
  Production `claude` 2.1.126 invoked through PluginDispatcher AND
  AgentDispatcher with byte-identical argv recording wrapper.
  Discovered 5 net-new findings (added to PROB-050 as A-21..A-26 + 1
  A-22 retract via audit C-1 pipefail discipline lesson). Total spent:
  ~$0.98 USD across 5 measured claude invocations.
- **PROB-050 A-3 ✅ closes** with narrowed scope (happy + budget-error
  envelope verified end-to-end on healthy CLI; failure-path JSON
  decode coverage tracked in A-11 + A-16).
- **PROB-050 A-14 wording tightened**: require `#[cfg(test)]` gate for
  `FORGEPLAN_CLAUDE_BIN` (audit S-2 escalates env-injection vector
  CWE-426 from documentation-only mitigation to compile-time gate).
- **PROB-050 A-28 ✅ closes** via YAML rewrite (`Delegation::Agent` →
  `Delegation::Plugin` split for colon-namespaced agent slugs in
  `audit.yaml` steps 1-3). Real-E2E proof on 2026-05-04: all 3 parallel
  agents successfully spawned, claude resolved bare slugs
  (`architect-reviewer`, `code-reviewer`, `security-expert`), 502s
  wall-clock real work + ~$3.50 spent — closing what would have been
  a guaranteed `DispatchError::Transport` reject pre-spawn.
- **PROB-050 A-29 NEW** (discovered during A-28 verification):
  `claude_print::DEFAULT_BUDGET_USD = $1.00` слишком низок для
  adversarial-review playbooks. All 3 audit agents hit `error_max_budget_usd`
  at $1.05-$1.25. Operational fix applied: `audit.yaml` steps 1-3 now
  carry explicit `budget_usd: 5.00`. Methodology fix tracked as A-29
  option (b) for next sprint (tier `DEFAULT_BUDGET_QUICK` /
  `DEFAULT_BUDGET_REVIEW`).

### Added (AI documentation discoverability)

- `website/public/robots.txt`: explicit `Allow: /` for 18 named AI
  crawlers (GPTBot, ClaudeBot, Google-Extended, CCBot, Applebot,
  PerplexityBot, и т.д.) — Forgeplan документация specifically built
  for agentic consumers, signal openness explicitly rather than rely
  on absence of `Disallow`.
- `website/public/llms.txt`: curated entry-point per emerging
  Anthropic/Mintlify convention (https://llmstxt.org/). Provides
  one-shot context for LLM agents discovering Forgeplan: methodology
  links, CLI/MCP reference, getting-started anchor. Without this, AI
  agents had to guess which paths matter.

### Detail — PRD-073 file-first invariant (EVID-094 R_eff=0.80 grade A)

Phase 3a → 3b → 3c → 4. Four adversarial audit rounds
(general / live-test / Rust-focused / final team-lead) closed
7 CRITICAL + 13 HIGH findings. PROB-048 deprecated as resolved.

### Added — file-first projection helpers (15 total)

- 9 mutation helpers: `create_artifact_with_projection`,
  `delete_artifact_with_projection`, `update_metadata_with_projection`,
  `update_body_with_projection`, `update_depth_with_projection`,
  `add_link_with_projection`, `delete_link_with_projection`,
  `add_tags_with_projection`, `remove_tags_with_projection`. Each does
  the {sync_before, mutate, render_after} triplet so callers can no
  longer forget projection.
- 6 sync-from-file helpers: `sync_artifact_from_file`,
  `sync_body_from_file`, `sync_metadata_from_file`,
  `sync_relation_from_file`, `delete_orphan_artifact`,
  `delete_orphan_relation`. For reindex / git_sync / watch where the
  file is already authoritative.
- `add_links_batch_with_projection`: deduplicates pre-sync + post-render
  per unique participant. 100-link bundle: ~600 LanceDB calls + 400 file
  ops → 2×U + N where U is unique IDs.
- `delete_artifact_after_soft_delete`: brief helper for the MCP
  soft-delete pattern (file already in trash, only DB row to drop).
- `MutationError` enum + `MutationResult<T>` alias introduced (typed
  errors); helper signature migration deferred to PRD-073 Phase 3c.
- `marketplace/playbooks/audit.yaml`: reference template for the
  multi-agent adversarial audit pattern. Updated header to reflect
  ADR-011 (claude --print via PluginDispatcher / AgentDispatcher);
  current YAML uses colon-namespaced agent slugs (`agents-pro:architect-reviewer`)
  which are pre-spawn-rejected by `validate_agent_name` until PROB-050 A-28
  introduces a colon-aware slug strategy.

### Changed (BREAKING for downstream library consumers)

- **`LanceStore::*` mutating methods are now `pub(crate)`**: 11 methods
  (`create_artifact`, `update_artifact`, `update_valid_until`,
  `update_depth`, `update_body`, `add_tags`, `remove_tags`,
  `delete_artifact`, `add_relation`, `delete_relation`,
  `delete_relations_for_artifact`) are no longer accessible from
  external crates. External callers must go through
  `forgeplan_core::projection::*` helpers. **Migration**: replace
  `store.create_artifact(&art)` with
  `projection::create_artifact_with_projection(&ws, &store, &art)`.
- **Slugify is now ASCII-only**: `is_ascii_alphanumeric` instead of
  `is_alphanumeric`. Workspaces with cyrillic/CJK slugs require
  `forgeplan reindex` after pulling this version; existing files
  remain on disk but get a fresh ASCII slug on next render.
- **`LanceStore::update_embedding` and `update_r_eff_score` stay `pub`**
  (Class A derived data, ADR-003 Amendment 1).
- **BREAKING (forgeplan-core lib only)**: 16 mutation helpers in
  `projection::*` migrated from `anyhow::Result<T>` to `MutationResult<T>`
  (PRD-073 Phase 3c, ADR-003 Amendment 2). CLI binary and MCP server
  surfaces unaffected. Library consumers see the same `?` ergonomics via
  anyhow's blanket `From<E: std::error::Error + Send + Sync + 'static>`
  impl. Variant taxonomy: `InvalidId`, `InvalidKind`, `EmptyField`,
  `FileNotFound`, `ProjectionMismatch`, `RowNotFound`, `StoreError`. Use
  `MutationError::is_recoverable()` to drive retry / warn-and-continue
  policy instead of string-matching on flattened error messages.
  Concrete migration example for downstream library consumers:
  ```rust
  // Before (anyhow::Result):
  let err = create_artifact_with_projection(...).await.unwrap_err();
  if err.to_string().contains("invalid id") { /* ... */ }

  // After (MutationResult):
  match create_artifact_with_projection(...).await {
      Err(MutationError::InvalidId(_)) => /* fatal input */,
      Err(e) if e.is_recoverable()     => /* transient — retry ok */,
      Err(_)                           => /* fatal — surface to user */,
      Ok(path) => /* happy path */,
  }
  ```
  See ADR-003 Amendment 2 (`.forgeplan/adrs/ADR-003-*.md`) for the full
  before/after error matrix and Phase 3d reserved-variant notes.
- **`sync_artifact_from_file` and `sync_body_from_file` signatures take
  `workspace: &Path`** to enable `FileNotFound { id, path }` typed errors
  with the actual on-disk location. CLI callers (`reindex`, `git_sync`,
  `watch`) updated. (PRD-073 Phase 3c)
- **`update_body_with_projection` now returns `RowNotFound`** (not
  `StoreError`) for the missing-id case — fixes Wave 1A audit finding
  where `is_recoverable() == true` would have mislabeled an
  unrecoverable input error as a transient I/O failure.

### Changed (behavioral — visible to CLI users)

- **All 22 CLI mutation handlers now hold an exclusive workspace lock**
  (30 s timeout) for the duration of the operation. Concurrent
  `forgeplan update` invocations that previously raced now serialize
  cleanly. Scripts using `&` or `xargs -P` against the same workspace
  may see lock-contention errors that were previously silent races.
- **`forgeplan delete` now creates a soft-delete receipt** (parity
  with MCP). Recoverable via `forgeplan undo-last` or
  `forgeplan restore <id>` within 30 days.
- **All markdown writes are atomic** (tempfile + rename). Kill -9
  mid-write no longer leaves zero-length projection files.
- **File frontmatter `title:` now preserves non-ASCII titles verbatim**
  (PRD-073 Phase 3c R2 audit M-R2-3 / security). Previously, an
  artifact created with a Cyrillic / CJK / emoji title (anything that
  slugifies to empty) was rendered with `title: untitled` in the file
  frontmatter — losing the user's original title from the on-disk
  representation while the DB row preserved it. The Phase 3c
  `projection_slug` helper now applies the `untitled` fallback only
  to the on-disk filename (e.g. `prds/PRD-001-untitled.md`), and the
  frontmatter receives the original title. Operators with non-ASCII
  confidential titles should be aware that the file frontmatter now
  contains the full title verbatim (the slug filename already exposed
  partial title information pre-fix; this aligns the two surfaces).
- **`claude` CLI is now a runtime prereq for playbooks that use
  `delegate_to: plugin` or `delegate_to: agent`** (ADR-011, Phase B).
  Replaces the never-shipped `claude-code-plugin` / `task-tool` binaries
  assumed by ADR-010. Plugin and agent steps invoke `claude --print
  --agent <name>` directly via `tokio::process::Command`. Existing
  Claude Code session is reused (no `ANTHROPIC_API_KEY` required for
  interactive runs); CI runs need the env var. Missing binary surfaces
  `DispatchError::DelegateMissing` with install hint pointing to
  https://code.claude.com/docs/en/install. New per-step `Step.budget_usd`
  (default $1.00) and `Step.allowed_tools` (default `[Read, Glob, Grep]`)
  fields control invocation surface; SPEC-003 1.1 → 1.2 (additive).
  Skill, Command, and ForgeplanCore dispatchers are unchanged.

### Added (developer-facing)

- New Cargo feature `forgeplan-core/test-helpers`: exposes
  `*_for_test` escape hatches on `LanceStore` for downstream test
  fixtures. **Gated on `debug_assertions`** so release builds with this
  feature accidentally enabled still get the lockdown. Production
  binaries MUST NOT enable this feature; release builds with both
  feature on AND debug_assertions off compile-error out.

### Fixed

- Path-traversal CVE class on import: `id` field validation in every
  projection helper that composes a filesystem path.
- Multi-line ratchet test scanner: was missing 21 multi-line
  `store\n.method(` invocations under the previous literal matcher.
- `update --depth --title` orphan-file recreation: metadata mutation
  now runs FIRST so subsequent depth/body renders see the new title.
- `mem-foo` vs `mem-foo-bar` prefix collision: exact-path delete via
  `remove_projection_at`.
- 4-process concurrent `forgeplan update` race: workspace lock plus
  lock-then-open ordering (LanceStore connections snapshot at open).
- `add_link / delete_link` warn-and-continue semantics restored
  (target sync + post-render are best-effort, source side fatal).
- `update_body_with_projection` ordering inverted to file-first.
- `forgeplan_import` no longer leaves DB-only state.
- `forgeplan new` non-tty similar-title prompt: explicit `Error: ...
  Fix: --allow-duplicate` instead of silent cancel.

## [0.27.0] — 2026-04-28 — Real subprocess dispatchers + init recommendation hints + greenfield playbook (EPIC-007 Phase 6)

Phase 6 переводит engine layer из v0.26.0 в **user-facing activation**.
PRD-072 / RFC-007 / ADR-010 закрывают Phase 5 deferral: 5 production
`Dispatcher` impls (real subprocess через `tokio::process` + ForgeplanCore
direct call), `forgeplan init` теперь эмитит recommendation hints, и
канонический `greenfield-kickoff.yaml` доступен в marketplace.

### Added — Real subprocess dispatchers (PRD-072 / RFC-007 / ADR-010)

- **`forgeplan-core::playbook::dispatch::{plugin,agent,skill,command,forgeplan_core}_dispatcher`** —
  5 production реализаций trait `Dispatcher`. Замена `MockDispatcher::AlwaysOk`
  в `playbook run --yes` и MCP `forgeplan_playbook_run`.
- **`PluginDispatcher` (FR-1)** — claude-code-plugin subprocess invocation,
  default 600s timeout, fallback_hint surfacing на missing-install.
- **`AgentDispatcher` (FR-2)** — task-tool agent-invoke, default 300s timeout,
  symmetric к plugin path.
- **`SkillDispatcher` (FR-3)** — in-process v1 stub (trace-only). Real registry
  resolution отложена в Wave 5.
- **`CommandDispatcher` (FR-4)** — security-hardened: `env_clear` + allow-list,
  no shell expansion, `--yes` gate trust upstream. Default 180s.
- **`ForgeplanCoreDispatcher` (FR-5)** — direct internal call (no subprocess)
  для `ingest`/`new`/`validate`/`activate`/`search`. Замена Phase 5 CLI
  shell-out — теперь делегация выполняется в том же процессе.
- **`dispatch::helpers::run_subprocess`** — общая обёртка `tokio::process::Command`
  с `kill_on_drop(true)`, `Stdio::piped` для stdout/stderr, `Stdio::null` для stdin,
  concurrent drain через `tokio::join!`, 10 MiB cap, timeout с child kill.
- **Pre-Wave 0 split**: `dispatch.rs` (single 466 LOC) → `dispatch/` directory
  с per-delegate modules. `mod.rs` сохраняет trait + Mock/Recording stubs +
  DispatchError + SecurityError без изменения публичного API.

### Added — Init recommendation wiring (PRD-067 AC-3/4/5/7 closed)

- **`commands::init::run` extension (FR-6)** — после workspace creation
  собирает project signals (`detect_signals`) + installed plugins
  (`detect_plugins(extended_registry)`) + `build_recommendations` +
  `format_recommendations` → emit на stderr.
- **3 bundled `KnownPlaybook` descriptors** — `greenfield-kickoff`,
  `brownfield-docs`, `brownfield-code` — для recommendation engine
  до момента когда полные marketplace YAML файлы land.
- **Backward compat**: `FORGEPLAN_HINTS=0` или non-TTY stderr → no
  recommendation emission (PRD-067 AC-7).
- **Non-fatal degradation**: signal/plugin detection failure → warning
  на stderr + продолжение init (no abort).

### Added — Canonical greenfield playbook (PRD-072 FR-7)

- **`marketplace/playbooks/greenfield-kickoff.yaml`** — 7 шагов через
  `ForgeplanCore` + 1 optional `Skill` step. Все мандатные шаги без
  внешних плагинов: `capture-vision` (note) → `stack-decision` (adr) →
  `kickoff-epic` (epic) → 3× `prd-feature` (parallel after epic) →
  `scaffold-docs` (skill, `on_error: continue`).
- **`forgeplan playbook validate`** проходит: `OK: greenfield-kickoff
  (7 steps)` + `Done.` hint.
- **Documentation footer в YAML** — purpose, expected duration, fit
  в methodology.

### Changed — Schema 1.0 → 1.1 (additive)

- **`Step.timeout_seconds: Option<u32>`** (FR-8) — backward compat:
  старые playbook'и без поля грузятся OK с дефолтом per-delegate
  type (300s general / 600s plugin / 180s command/skill).
- **`SPEC-003 schema_version`** bumped 1.0 → 1.1. Loader принимает
  оба значения (semver-range minor bump).

### Fixed — Phase 6 real-world bugs (PR #220, commit 69ea571)

После merge'а Phase 6 в dev manual smoke testing на release binary
обнаружил 4 production bugs, которые 1834 automated тестов пропустили:

- **HIGH `playbook show <name>`** — name lookup НЕ находил
  `marketplace/playbooks/`. Discovery roots расширены до workspace
  marketplace, не только `.forgeplan/playbooks/` + `~/.claude/plugins/*/playbooks/`.
  Теперь shipping playbooks доступны через name lookup, не только absolute path.
- **HIGH `plugins doctor`** — exit 0 при missing plugins (документировано
  exit 1). Fixed: `if !missing.is_empty() || !outdated.is_empty() { exit(1) }`.
  CI gate теперь работает.
- **HIGH `marketplace/playbooks/brownfield-code.yaml`** — `detect-c4-need`
  step missing `input.id`, validate fails на step 1. Removed broken step,
  playbook reduced 5 → 4 steps, validate clean.
- **CRITICAL systemic** — все error paths возвращали `exit 0`
  (`eprintln!("Error:..."); return Ok(())`). Fixed ~10 sites в
  `commands/playbook.rs` + `ingest.rs`: explicit `std::process::exit(1)`.
  Real CI integration теперь catches all CLI failures.
- **BONUS dev profile fix (commit 0acf884)** — `[profile.dev] debug =
  "line-tables-only"` снижает linker memory ~50%. Закрыт recurring
  `collect2: ld signal 7 [Bus error]` OOM на ubuntu-latest 16GB
  который преследовал PR #217+ Phase 5/6 PRs. Universal CI speedup.

### Fixed — PROB-047 mitigation 1 (PR #221, commit 80f458c)

`scan-import` classifier (`crates/forgeplan-core/src/scan/detect.rs`)
ошибочно классифицировал product guides и instruction files как
PRD-артефакты через **Tier 3 content heuristics** (`## Goals`,
`## Problem`, `## Decision` headings). PR #218 был symptom-only
cleanup — false-positives recurred при следующем scan-import.

- **`is_doc_path(relative_path: &Path) -> bool`** — blacklist для:
  recursive `docs/`, `marketplace/`, plus root-level meta-files
  (`CLAUDE.md`, `AGENTS.md`, `README.md`, `CHANGELOG.md`,
  `CONTRIBUTING.md`, `TODO.md`, `ROADMAP.md`, `LICENSE.md`,
  `SECURITY.md`, plus `.ru.md` localized variants).
- **`detect_kind_with_path(filename, relative_path, content)`** —
  path-aware variant suppresses Tier 3 ONLY. Tier 1 (frontmatter
  `kind:`) и Tier 2 (filename pattern PRD-XXX/RFC-XXX) остаются
  authoritative — explicit signals always win.
- **`detect_kind`** retained as wrapper passing `None` for path
  — backward compat with all 15 existing tests.
- **+11 unit tests**: `is_doc_path` matrix coverage + path-aware
  Tier 3 suppression + Tier 1/Tier 2 precedence under docs.
- **EVID-092** (verdict: supports, congruence_level: 3, evidence_type:
  test) — same-context measurement linked to PROB-047. R_eff: 0.0 → 0.71 (B).
- **Mitigations 2-5 deferred to Phase 7+** sprint (frontmatter precedence
  formalization, scan-import default `--dry-run` + opt-in `--apply`,
  content_hash idempotency, brownfield test fixtures).

### Workspace hygiene (PR #221)

- `.forgeplan/journal/` (PRD-065 playbook runtime per-run JSONL) → gitignore.
- PROB-046 deprecated — resolved via PRD-071 hint contract (shipped v0.25.0).
- EPIC-007 advisory phase advanced to evidence (children 4/5 shipped).
- 9 untracked scan-import false-positives removed via `forgeplan reindex`.
- `forgeplan health`: "Project looks healthy" — 0 blind spots, 0 orphans,
  0 phase mismatches, 0 duplicate pairs.

### Stats

- **+5000 LOC** across `forgeplan-core::playbook::dispatch` (5 dispatchers
  + helpers) + `commands::init::run` extension + canonical YAML.
- **+60 unit tests** (Wave 1: 44 unit tests распределены по dispatchers + helpers).
- **+5 integration tests** в `integration_phase6_init.rs` (empty repo,
  `.obsidian` vault, legacy code with >100 commits, `FORGEPLAN_HINTS=0`,
  signal failure path).
- **Workspace test count**: 1384+ lib + 372+ integration, all PASS.
- **Code quality**: 0 fmt diffs, 0 check warnings, 0 clippy warnings
  (rust 1.91 strict).
- **3 waves × 8 unique agents** через TeamCreate Mode A:
  - Pre-Wave 0: dispatch.rs split + Spike-2 manual c4-architecture run + EVID-090 (CL3)
  - Wave 1: 6 parallel agents (helpers + 5 dispatchers, strict file ownership)
  - Wave 2: 1 agent (init wiring + integration tests)
  - Wave 3: 1 agent (greenfield-kickoff.yaml + validate)
  - Wave 4: 1 agent (this — docs + EVID-091 + CHANGELOG + TODO)

### Deferred to follow-up sprint

- **`Step.timeout_seconds` per-step override (FR-8 wiring)** — schema field
  landed, executor wiring partial; full per-step override через
  `dispatch::helpers::run_subprocess` parameter — Wave 5.
- **Real `SkillDispatcher` registry** — текущий impl = trace-only stub
  (loggable invariants + fallback_hint). Wave 5 = real skill resolution
  через agent-skills capability registry.
- **Per-step env allow-list extension** — сейчас allow-list захардкожен
  в helpers (`PATH`, `HOME`, `FORGEPLAN_WORKSPACE`). PRD-076 (TBD) —
  декларативный `step.env:` override с whitelist через mapping.
- **MCP `forgeplan_ingest`** wrapper — pure CLI command в v0.27.0
  (still); MCP wrapper remains deferred (CLI cover via `forgeplan serve`).
- **3 canonical playbooks** — `brownfield-docs.yaml`, `audit.yaml`,
  `release.yaml` — backlog (greenfield + brownfield-code published).
- **Parallel step execution** — sequential в v1 per PRD-065 Non-Goals.

### References

- ADR-010 `.forgeplan/adrs/ADR-010-*.md` — subprocess invocation strategy
- RFC-007 `.forgeplan/rfcs/RFC-007-*.md` — Phase 6 dispatcher architecture
- PRD-072 `.forgeplan/prds/PRD-072-*.md` — Phase 6 PRD (FR-1..FR-10)
- EVID-090 — Spike-2 tokio::process measurement (CL3 same-context)
- EVID-091 — Phase 6 closure evidence pack (this release)
- EPIC-007 — Playbook Runtime + Pack Marketplace (parent)

## [0.26.0] — 2026-04-28 — Playbook runtime + Ingest engine + Plugin detection (EPIC-007 Phase 2)

Forgeplan становится **оркестратором**. Три новых core capabilities (PRD-065 / PRD-066 / PRD-067) воплощают ADR-009: сам forgeplan-core не генерирует документы — он **знает когда какой playbook запускать**, **кому делегировать каждый шаг**, и **как ингестить output в forge-граф** с обязательной `## Sources` секцией (hallucination-proof invariant). Реализация — четырёхволновой sprint, 9 параллельных агентов, ~9000 LOC, +168 unit tests, plus integration E2E из Wave 4.

### Added — Playbook runtime (PRD-065 / SPEC-003)

- **`forgeplan-core::playbook::{types,loader,executor,dispatch,journal}`** — декларативная YAML-схема + runtime executor.
- **5 типов делегации** (strict typed, no arbitrary shell): `plugin` (Claude Code plugin via Task tool), `agent` (subagent via Task tool), `skill` (agent-skills capability), `command` (opt-in shell), `forgeplan_core` (internal op: `ingest`/`new`/`validate`/`activate`/`search`).
- **DAG-ordering** через `requires:` (step IDs), цикл-detection, unknown-ref detection в loader.
- **`fallback_hint`** — точная install-команда, эмитится если plugin/skill не установлен (AC-4 PRD-065).
- **Journal** в `.forgeplan/journal/playbook-runs.jsonl` — resumable partial failures.
- **JSON Schema** опубликована в `docs/schemas/playbook.schema.yaml` (FR-2).

### Added — Ingest engine (PRD-066 / SPEC-004)

- **`forgeplan-core::ingest::{types,sources,template,engine,idempotency}`** — declarative mapping engine.
- **Tera-style шаблоны** с **whitelist filters** (10): `trim`, `lower`, `upper`, `bullet_list`, `comma_list`, `slugify`, `truncate`, `default(value=...)`, `replace`, `table`. Любой не-whitelisted filter → load error (security boundary, ADR-009).
- **`## Sources` invariant** — `sources_section.include: false` отвергается deserialization, артефакт без Sources не создаётся.
- **`compat_spec_version`** per mapping — semver-pinning upstream plugin output, fail-fast при upstream breaking change.
- **5 source kinds**: `c4-documentation`, `autoresearch`, `git-log`, `ddd-model`, `sparc-spec`.
- **6 target artifact kinds**: `prd`, `adr`, `epic`, `note`, `spec`, `problem`.
- **Idempotency** через `source_hash` — re-run = update existing, не дубликаты (AC-3 PRD-066).
- **JSON Schema** опубликована в `docs/schemas/mapping.schema.yaml` (FR-2).

### Added — Plugin detection + self-describing hints (PRD-067)

- **`forgeplan-core::plugins::{detection,registry,hints}`** — детектит installed plugins.
- **Detection paths**: `~/.claude/plugins/cache/`, `.claude/plugins/`, `.agentskills/`, `.cursor/skills/`.
- **Project signals**: `empty_repo`, `legacy_code_no_docs`, `docs_vault_present`, `has_package_json`, `has_cargo_toml`, `git_commit_count`.
- **Recommendation engine** — signals × installed_plugins → applicable playbooks; emitted в init hint.
- **CLI**: `forgeplan plugins {list|doctor|info <name>}`.

### Added — CLI / MCP surface

- **5 new CLI commands**: `forgeplan playbook {list|show|run|validate}`, `forgeplan ingest`, `forgeplan plugins {list|doctor|info}`.
- **8 new MCP tools** wrapping the same surface for agent integration.
- All новые команды эмитят PRD-071 hint markers (`Next:` / `Or:` / `Wait:` / `Done.` / `Fix:`) — coverage 100% по drift-prevention audit script.

### Added — Canonical marketplace assets

- **`marketplace/mappings/c4-to-forge.yaml`** — production-ready mapping для c4-architecture plugin output. Per EVID-088 (Spike-1 measurement): target=`note` по умолчанию (не `prd`/`spec`) — code-derived артефакты не имеют product-context для PRD/SPEC validation gate.
- **`marketplace/playbooks/brownfield-code.yaml`** — 5-step canonical playbook: `detect-c4-need` → `run-c4-architecture` (Plugin) → `ingest-c4` (ForgeplanCore + mapping) → `run-history-miner` (Skill) → `summary-note` (ForgeplanCore). `triggered_by: { has_git: true, commit_count_min: 50, has_docs: false }`.

### Added — Documentation

- **`docs/operations/PLAYBOOK-AUTHORING.ru.md`** — guide для авторов playbook'ов: 5 типов делегации, DAG, fallback hints, conventions.
- **`docs/operations/INGEST-MAPPINGS.ru.md`** — guide для авторов mapping'ов: Tera caveat (`default(value="...")`), whitelist, hallucination-proof invariant, target=note default per EVID-088.
- **`docs/README.md` + `docs/README.ru.md`** — index updates.

### Stats

- **+9000 LOC** across 3 new modules + CLI + MCP.
- **+168 unit tests** (W1: 39 / W2: 110 / W3: 58, including 16 dogfood E2E from Wave 3) + Wave 4 integration E2E.
- **0 fmt diffs / 0 clippy warnings** on default and `--features semantic-search`.
- 4 waves × 9 unique agents (1 architect + 3 W1 + 5 W2 + 4 W3 + 2 W4) с gate checks per wave.

### Deferred to follow-up sprint

- **Real Plugin / Agent / Skill dispatchers** — Wave 3 экзекутор делегирует через mocked Task tool subprocess в этом релизе. Production wiring (через runtime Task tool API) — следующий sprint.
- **MCP `forgeplan_ingest`** — pure CLI command в v0.26.0; MCP wrapper отложен (CLI cover тех же scenarios через `forgeplan serve`).
- **`brownfield-docs-pack` / `greenfield-pack`** — only `brownfield-code.yaml` published canonical в этом релизе.
- **Parallel step execution** — sequential в v1 per PRD-065 Non-Goals; parallelizable DAG planner — v2.

### References

- ADR-009 `.forgeplan/adrs/ADR-009-*.md` — orchestrator pivot decision
- EPIC-007 — Playbook runtime + Pack marketplace (parent)
- PRD-065 / SPEC-003 — Playbook runtime + schema
- PRD-066 / SPEC-004 — Ingest engine + mapping schema
- PRD-067 — Plugin detection + hints
- EVID-088 — Spike-1 c4-to-forge concept validation (CL3)
- EVID-089 — Phase 5 implementation evidence pack

## [0.25.0] — 2026-04-27 — Unified hint contract across CLI + MCP (PRD-071 complete)

Forgeplan теперь говорит агентам что делать дальше. Каждый CLI и MCP вывод эмитит **один** контрактный маркер (`Next:` / `Or:` / `Wait:` / `Done.` / `Fix:`) — никаких больше «agent reads no-hint output → re-reads CLAUDE.md → guesses → loops». 5-rule контракт (PRESENCE / ACTIONABILITY / DETERMINISM / CONDITIONALITY / CONSISTENCY) реализован за 5 циклов multi-agent sprint, audit coverage 0% → **100% (70/70)**.

### Added — 5-rule hint contract (PRD-071)

- **`Next: <full command>`** — primary action with real IDs (no `<placeholder>`)
- **`Or: <command>`** — alternate when primary blocks
- **`Wait: <condition>`** — async/TTL retry signal
- **`Done.`** — terminal success (workflow complete)
- **`Fix: <command>`** — error remediation (paired with `Error:` line)
- JSON output: `_next_action` field (string or null)
- MCP responses: `_next_action` in success + error data

### Added — Drift prevention infrastructure

- **`crates/forgeplan-cli/tests/hint_contract.rs`** — 36 integration tests asserting every covered command emits contract marker AND no forbidden placeholders. New CLI/MCP command without hint **fails CI**.
- **`scripts/audit-hints.sh`** — coverage metric (CI-ready), recognizes all 5 markers.
- **`docs/methodology/agent-protocol.md`** — full contract spec with good/bad hint examples and agent reading protocol.
- **`CLAUDE.md`** — Hint protocol section (5-line agent reference).

### Changed — backward-compat preserved

- `forgeplan list --json` and `forgeplan tree --json` retain bare-array stdout (`jq '.[]'` and existing scripts not broken). Hint moves to **stderr** in JSON mode.
- All existing CLI text outputs preserved — hints are additive new lines at end.
- MCP `_next_action` field was already present (just normalizing values).

### Fixed — edge cases

- 9 commands (get/delete/update/score/estimate/progress/calibrate/validate/link) now emit `Fix: forgeplan list` on "Artifact not found" errors. Previously only `Error:` line — failed PRESENCE rule for not-found path.
- Audit script now recognizes `Fix:`/`Or:`/`Wait:`/`Done.` markers (was only `Next:` — produced false negatives).

### Sprint metrics

- 5 cycles × 3 parallel agents = 9 unique agents
- 90 files changed (+3994, -539)
- 1140 lib + 36 hint_contract + 104 cli_integration_test = **1280 tests passing**
- 0 fmt diffs, 0 clippy warnings
- EVID-086 linked to PRD-071, R_eff 0.70 (overall 0.80 A grade)

## [0.24.0] — 2026-04-19 — Orchestrator dispatcher for 2-5 sub-agents (PRD-057 complete)

Forgeplan now dispatches work. One MCP call — `forgeplan_dispatch
--agents N` — hands the orchestrator a parallel-safe plan: which
artifacts each sub-agent should work on, which defer to a serial queue,
and human-readable reasoning for every decision. Ends the manual
"read graph + blocked + list + mental overlap calc" loop that was the
original PRD-057 problem statement.

Four increments (Inc 2, 3, 4) + two adversarial audit rounds (R2 3-agent
mid-sprint, R3 4-agent final) + 94 net new tests (1391 total). Builds
on the Inc 1 workspace lock shipped in v0.23.1.

### Added — Agent identity (Inc 2, FR-009 + AC-5)

- **`AgentIdentity`** captures which MCP client last mutated an artifact
  via `clientInfo` and stamps `last_modified_by: name/version` into
  frontmatter on every write.
- **Unknown-frontmatter preservation** — `projection` now keeps
  agent-owned fm fields (`last_modified_by`, `domain`,
  `affected_files`) across re-renders triggered by unrelated tools.
- **Unicode / control-char rejection** in `AgentIdentity::new` — blocks
  bidi override, ZWJ, RTL, newlines, path separators.
- **Activity log** carries the captured `clientInfo` — previous `None`
  TODO closed.

### Added — Claim protocol (Inc 3, FR-004..006, FR-014, AC-2..3)

- **`ClaimStore`** — soft-coordination signal "agent X works on ID
  until T". YAML files at `.forgeplan/claims/<ID>.yaml` (gitignored).
  TTL 1 min..24 h, default 30 min. Same-agent calls renew; expired
  claims transparently overwritten.
- **Three new MCP tools**: `forgeplan_claim`, `forgeplan_release`
  (`force: true` orchestrator escape hatch), `forgeplan_claims`.
- **Atomic writes** via tempfile + rename.
- **64 KB YAML cap** + path-traversal guard (R2 security HIGH fix).

### Added — Orchestrator dispatcher (Inc 4, FR-001..003, FR-010..011, AC-1)

- **`forgeplan_dispatch`** returns `{buckets, serial_queue, reasoning,
  candidate_count, claimed_count, blocked_count, skipped_parse_errors}`.
- **Jaccard file-overlap detection** (0.3 default threshold).
  Empty `affected_files` biases to serial (R-2 safety).
- **Least-loaded-first greedy packing** — distributes, deterministic.
- **Graph-aware** — blocked artifacts excluded via `kahn_sort`.
- **Claim-aware** — claimed artifacts skipped with reasoning.
- **Skill matching** via `agent_skills` vs artifact `domain`.
- **Markdown-section fallback** — legacy artifacts with only
  `## Affected Files` body section are hydrated via
  `extract_affected_files(body)`.
- **Input clamps**: `MAX_AGENTS=64`, `MAX_SKILLS_PER_AGENT=32`,
  `MAX_AFFECTED_FILES=512`, 512-byte path cap (R3 CWE-770 fix).

### Added — Integration surface (FR-012, FR-013)

- **`forgeplan_health`** body includes `active_claims`,
  `active_claim_count`, `skipped_claim_files`.
- **`forgeplan_get`** `_next_action` appends claim holder + expiry
  when a live claim exists.

### Security

- Path traversal refusal in `ClaimStore` (CWE-22).
- Unicode homograph rejection in `domain` (CWE-176).
- Resource caps on `agents`, skills, file lists, YAML size (CWE-400/770).
- Control chars rejected in agent identity.

### Performance

- Read-only tools (`dispatch`, `claims`, `health`, `get`) don't acquire
  the workspace lock — orchestrator 1 Hz polling doesn't serialize
  writers (R2 architect MED).
- `ClaimStore::list_active_map` for O(1) dispatcher joins.

### Testing

- **+94 tests** (1297 → 1391). 13 dispatch algorithm, 26 claim store
  (inc. hardening), 14 MCP wiring + validation, 10 dogfood E2E, 4
  workflow variations, 1 AC-4 concurrent-forgeplan_new unique-ID E2E.
- **Two adversarial audit rounds** (R2 3-agent, R3 4-agent with
  production-validator for FR/AC task-completion) — 30 findings
  closed with regression tests.

### Deferred to v0.25+

- Shared `kv_yaml` abstraction across `phase::store` + `claim` + future
  dispatch-cache.
- Per-request identity for HTTP/SSE transports.
- `load_frontmatter_full` primitive to dedupe 10 read→parse sites.
- `ListFilter::parent_epic` push-down.
- `DispatchDecision` structured enum for `reasoning` (i18n).
- `list_active_map → HashMap<String, Claim>` for holder-based routing.
- ADR separating claim (ephemeral) from phase (durable) state.
- Agent profiles at `.forgeplan/agents/<agent_id>.yaml` (v0.27 roadmap).

### References

- PRD-057 `.forgeplan/prds/PRD-057-*.md`
- EVID-077 `.forgeplan/evidence/EVID-077-*.md` — R_eff=1.00, CL3

## [0.23.1] — 2026-04-19 — Multi-agent workspace lock foundation (PRD-057 Inc 1)

First safety primitive for multi-agent workflow — workspace-level file
lock that serializes LanceDB write operations across 2-5 concurrent
sub-agents sharing a `.forgeplan/` directory. Patch bump, no breaking
changes, no new user-facing tools.

### Added

- **`forgeplan-core::workspace::lock`** module with `WorkspaceLock`
  RAII guard and `acquire_workspace_lock` async helper. Uses `fs2`
  flock primitive (Unix) / LockFileEx (Windows). Released automatically
  on drop including process crash.
- **30-second timeout** with exponential backoff (10ms → 1000ms) —
  no indefinite hang if a sibling agent is stuck.
- **Symlink guards** on both workspace directory and lock file
  (parity with PRD-055 R3 + PRD-056 hardening).
- **`#[must_use]`** on guard — compiler catches accidental immediate
  drop via `let _ =`.

### Wrapped with lock (all MCP write entry points)

- `forgeplan_new` — prevents duplicate ID collision under concurrent
  `next_id` allocation.
- `forgeplan_update`
- `forgeplan_delete`
- `forgeplan_supersede`
- `forgeplan_deprecate`

### Hygiene

- `.gitignore`: `.forgeplan/.lock` and `.forgeplan/claims/` (prep for
  PRD-057 Inc 2-4).

### Verification

- **1297 tests pass / 0 fail** (+6 new regression tests:
  - `acquire_creates_lock_file`
  - `lock_releases_on_drop`
  - `concurrent_acquirers_serialize_and_total_time` (strengthened
    with wall-time lower bound)
  - `timeout_surfaces_when_lock_held`
  - `symlinked_workspace_dir_is_refused` (unix)
  - `symlinked_lock_file_is_refused` (unix)
- `cargo clippy --workspace --all-targets -D warnings`: clean.
- `cargo fmt --check`: 0 diffs.
- Rust 1.95 toolchain pinned via `rust-toolchain.toml`.

### Audit

5-agent audit Round 1 (security + logic + arch + rust + task-completion)
found 1 CRITICAL + 2 HIGH + 4 MEDIUM — **all fixed** in the same PR
before merge. Net verdict: APPROVE_WITH_FIXES from all 5 agents post-
hotfix.

### Not included — planned for v0.24.0

- `Claim` module + `forgeplan_claim` / `_release` / `_claims` MCP
  tools (PRD-057 Inc 3).
- Agent identity capture (`client_info` → `last_modified_by`
  frontmatter field) (PRD-057 Inc 2).
- `forgeplan_dispatch --agents N` tool (PRD-057 Inc 4) — the dispatcher
  that suggests parallel-safe buckets based on dep graph, file-overlap
  Jaccard, and domain-skill matching.

Refs: EPIC-005, PRD-057 Inc 1, PR #192.

---

## [0.23.0] — 2026-04-18 — Advisory phase state machine (PRD-056, EPIC-005)

First shipped child of **EPIC-005 "Phase state machine & workflow-aware
methodology umbrella"**. Every artifact in the greenfield workflow now
has a visible `current_phase` that auto-advances through the methodology
cycle (`shape → validate → adi → code → test → audit → evidence → done`)
with full transition history on disk.

**Advisory-only** — no existing tool is blocked. Full enforcement lands
in a later PRD under EPIC-005.

### Added — phase state module (`forgeplan-core::phase`)

- Per-artifact state file at `.forgeplan/state/<ID>.yaml` (gitignored)
  with `current_phase`, `workflow_type`, `advanced_at`, append-only
  `history: Vec<PhaseTransition>`, `schema_version`.
- `Phase` enum (Unknown/Shape/Validate/Adi/Code/Test/Audit/Evidence/Done)
  with `as_str()` and `suggested_next()` helpers.
- `WorkflowType` enum (currently Greenfield — brownfield/hotfix/research/
  review-fix/refactor are follow-up child PRDs under EPIC-005).
- Atomic writes: tmp+rename with pid+nanos+AtomicU64-counter filename,
  `create_new(true)` against symlink planting, fsync(file) + fsync(dir).
- Symlink guards on both state directory AND target file, read + write.
- Path traversal defense via `validate_artifact_id` at every entry point.
- Size caps: `MAX_HISTORY_ENTRIES=1024` (FIFO drop preserving index 0),
  `MAX_REASON_LEN=512`, `MAX_STATE_FILE_BYTES=1 MiB`, `MAX_ARTIFACT_ID_LEN=128`.
- Forward-compat: `schema_version > CURRENT` → refused (no silent data loss).
- Corrupt YAML quarantined to `<id>.yaml.corrupt.<timestamp>` rather
  than clobbered — preserves audit-trail forensics.

### Added — auto-advancement hooks (MCP server)

- `forgeplan_new` → `phase=shape` on successful artifact creation.
- `forgeplan_validate` PASS → `phase=validate`.
- `forgeplan_activate` / `_supersede` / `_deprecate` → `phase=done`.
- All hooks fire-and-forget: failures logged via `tracing::warn`,
  never break the calling tool (advisory invariant).

### Added — MCP tools

- **`forgeplan_phase <id>`** — read current phase + workflow_type +
  timestamps + full append-only history. Missing state returns
  `{current_phase: "unknown"}`, never an error.
- **`forgeplan_phase_advance <id> --to <phase> [--reason]`** — manual
  override, appends to history, does NOT validate ordering (advisory
  layer allows out-of-order jumps). `reason` capped at 4096 bytes at
  MCP boundary + 512 bytes on persist.
- `PhaseArg` JSON-Schema enum so LLM clients constrain-sample exact
  values — no paraphrases.

### Added — integration

- `forgeplan_get` response now appends current phase to `_next_action`
  (`"… Phase: \`shape\` → next \`validate\`."`) when tracking is active.
- `forgeplan_health` response includes `advisory_phase_mismatches[]` —
  artifacts with `status=active` but `current_phase` still early-cycle
  (shape/validate/adi). Strictly advisory — no health failure.

### Added — config

- New optional `phase.enabled: bool` block in `.forgeplan/config.yaml`
  (default `true`). Flip to `false` for exact pre-v0.23.0 semantics
  without recompile.

### Fixed — hygiene

- `.gitignore`: added `.forgeplan/logs/` (forgotten in v0.21.0 — activity
  log was leaking into git) and `.forgeplan/state/` (new in this release).

### Verification

- **1291 tests pass / 0 fail** (+30 new vs v0.22.1 baseline):
  - 12 phase module unit tests
  - 14 regression tests (10 from Round 1 + 4 from Round 2 audits)
  - 4 incidental matches
- `cargo clippy --workspace --all-targets -D warnings`: clean.
- `cargo fmt --check`: 0 diffs.
- **2 audit rounds** by multi-agent panel (security + logic + rust +
  architect): 2 CRITICAL + 7 HIGH + 3 MEDIUM findings, **all fixed**
  before ship. R_eff(PRD-056) = 1.00, Grade A.

### Not included — deferred to follow-up PRDs

- `forgeplan_phase_backfill` command (FR-009, COULD) — populate
  phase state for existing ~100 artifacts.
- Full phase enforcement ("замки") — tools refuse to work not in their
  phase. Separate PRD under EPIC-005.
- Brownfield, audit-hotfix, research, review-fix, refactor workflow
  phase enums — each own child PRD under EPIC-005.
- Read-side `O_NOFOLLOW` TOCTOU closure (platform module needed).
- `thiserror`-typed `PhaseError` (advisory module, anyhow is fine here).

Refs: EPIC-005, PRD-056, EVID-076.

---

## [0.22.1] — 2026-04-18 — Undo hardening (post-ship audit Round 3)

Security + correctness hotfix for the undo subsystem shipped in v0.22.0.
A 4-agent multi-lens audit of the PRD-055 code found 2 CRITICAL + 5 HIGH
real issues. All fixed here with regression tests.

### Fixed — Security

- **Path traversal via tampered `projection_path`** (C-1 sec). Restore no
  longer trusts `receipt.snapshot.projection_path` verbatim. Destination
  is recomputed from `workspace + kind + id + slug(title)` and verified
  with `canonicalize().starts_with(workspace)`. An attacker-crafted
  receipt pointing at `/etc/passwd` is refused.
- **Unsanitized strings from receipts reached the agent** (H-1 sec).
  `report.warnings`, `relations_skipped`, and `receipt_id` in
  `forgeplan_restore` / `forgeplan_undo_last` responses now go through
  the same `sanitize_for_hint()` pipeline used elsewhere. Prompt-injection
  content planted in a receipt can no longer ride into agent context.
- **Symlinked trash directory or source projection** (H-2 sec). Both
  `write_receipt` and `trash_projection` now `symlink_metadata`-check
  their inputs and refuse if either is a symlink — prevents an attacker
  who can write the `.forgeplan/` tree from redirecting rename targets
  outside the workspace.

### Fixed — Correctness

- **`mark_consumed` failure silently left receipt unconsumed** (C-1
  logic, FR-011). A subsequent `undo_last` re-applied the same receipt
  (harmless for delete, misleading `Ok` for supersede/deprecate).
  `apply_restore` now propagates the error with clear manual-recovery
  instructions.
- **Receipt ID collision at 1/65 536 under concurrent deletes** (H-1
  logic). Replaced the 16-bit nanos-mask suffix with a 32-bit PRNG
  (`rand::random::<u32>()`) → effective collision probability
  ~1/4 294 967 296.
- **Title edits after creation broke projection resolution** (H-2
  logic). `soft_delete_capture` now scans `<kind>/<ID>-*.md` on the
  filesystem and uses the real filename, falling back to current-title
  slugify only if scan fails. Delete no longer silently leaves an
  orphan markdown that `scan-import` would resurrect.
- **Supersede/deprecate restore on collision branch overwrote a
  different artifact** (H-4 logic). Now refuses if `existing.kind !=
  snapshot.kind` with an explicit error suggesting manual resolution.

### Hardened

- Parent-directory fsync after `write_receipt` file sync (ext4/xfs
  durability — `fsync(file)` alone can lose the directory entry on a
  hard crash).
- `is_cross_device` now handles Windows `ERROR_NOT_SAME_DEVICE` (17) in
  addition to Unix `EXDEV` (18).

### Verification

- **1261 tests pass / 0 fail** (+6 new regression tests covering each
  finding: traversal-projection refusal, `mark_consumed` propagation,
  kind-mismatch refusal, 32-bit PRNG uniqueness, symlinked-trash
  refusal, symlinked-source refusal).
- `cargo clippy --workspace --all-targets -D warnings`: clean.
- `cargo fmt --check`: 0 diffs.

Refs: PRD-055 post-ship audit (4-agent panel: code-reviewer,
security-auditor, rust-pro, architect-reviewer).

---

## [0.22.0] — 2026-04-18 — Reversible destructive ops (PRD-055 complete)

Completes the undo story started in v0.21.0. Every destructive operation —
`delete`, `supersede`, `deprecate` — is now recoverable via a single tool
call within a 30-day TTL window.

### Added — wrapping of destructive ops (PRD-055 increment 2)

All three destructive tool handlers now go through `soft_delete_capture`
before mutating the store:

- `forgeplan_delete`: writes a receipt with full snapshot (body, metadata,
  outgoing + incoming relations), moves the markdown projection into
  `.forgeplan/trash/`, then removes the store row.
- `forgeplan_supersede`: writes a receipt capturing the original status,
  then applies the lifecycle transition. Projection stays in place.
- `forgeplan_deprecate`: same pattern.

Crash invariant (PRD-055 ADR #4): receipt is written BEFORE the store
mutation. A crash in between leaves a harmless orphan receipt which TTL
purge later collects.

Every destructive-op response now includes a `receipt_id` field and a
`_next_action` hint pointing at `forgeplan_undo_last` or
`forgeplan_restore <id>`.

### Added — restore and undo-last tools (PRD-055 increment 3)

- **`forgeplan_restore id=<ID>`** — finds the newest non-consumed receipt
  for that ID, applies restore. For delete: recreates the store row,
  moves the projection back, re-links all captured relations. For
  supersede/deprecate: resets status to pre-op value and drops the new
  link. Orphaned relation targets are tracked in `relations_skipped`.
- **`forgeplan_undo_last within_hours=<N>`** — finds the newest
  non-consumed receipt across all artifacts within the window (default
  24h, max 720h), applies the same restore logic. Never guesses: returns
  an explicit error if the window is empty.

Transactional semantics (FR-011): receipt is marked consumed LAST.
Collision handling (R-3): restore refuses if an artifact with the same
ID already exists in the store.

### Verification

- **1255 tests pass / 0 fail** (+19 undo tests across receipt and restore
  modules, +4 integration tests).
- `cargo clippy --workspace --all-targets -D warnings`: clean.
- `cargo fmt --check`: 0 diffs.

### User-visible workflow

Before: `forgeplan_delete PRD-048` → artifact permanently gone.

After:
```
forgeplan_delete PRD-048
  → receipt written, projection moved to trash, store row removed
  → response: receipt_id + hint "reversible via forgeplan_undo_last"

forgeplan_undo_last
  → PRD-048 restored with identical body, metadata, relations
```

Refs: PRD-055 (now functionally complete), PRD-054.

---

## [0.21.0] — 2026-04-18 — Activity log + soft-delete receipt infrastructure

Builds on the v0.20.0 tool-quality work. Adds two pieces of observability
and recovery infrastructure that make agent-driven use of forgeplan
materially safer.

### Added — Activity log (PRD-054)

Every MCP tool invocation is now recorded in an append-only JSONL file at
`.forgeplan/logs/tools-YYYY-MM-DD.jsonl`. One file per UTC day, daily
rotation happens automatically on first write. Each entry captures
timestamp, tool name, SHA-256-prefix hash of args (args content is
NOT logged by default — prevents secrets in titles / descriptions from
leaking into the log), duration, status (`ok` / `tool_err` / `rpc_err`),
workspace path, and optional client info.

Two new MCP tools surface the log:

- `forgeplan_activity` — query with `since_hours` (default 24, max 720),
  `tool` (comma-separated filter), `status`, `limit` (max 5000). Returns
  entries, warnings about corrupted lines, and a `_next_action` hint.
- `forgeplan_activity_stats` — per-tool aggregates (count, err_count,
  p50/p95/total ms), sorted by total time descending.

Dispatch wrapper sits on top of rmcp's `ToolRouter.call` — any existing
or future tool is logged automatically without per-handler changes. Log
writes fire-and-forget via `tokio::spawn` so the tool response path adds
zero latency. Log-write failures are observed via `tracing::warn` and
never fail the parent tool call.

CLI parity is planned for a future release.

### Added — Soft-delete receipt infrastructure (PRD-055, increment 1 of 3)

Foundation for reversible destructive operations. New module
`forgeplan-core::undo` provides the receipt data model, JSON
serialization, trash directory layout, TTL-based lazy purge, and
cross-platform filesystem rename with fallback to copy+remove for
cross-device moves.

Does NOT yet wire into `forgeplan_delete` / `forgeplan_supersede` /
`forgeplan_deprecate` — those still do hard-delete. Wiring is
planned for v0.22.0. This release ships the underlying primitives so
integration tests and tooling can exercise the receipt format now.

Key design decisions documented inline in PRD-055:
1. Move-to-trash plus receipt, not store tombstone column
2. JSON format, not binary
3. One receipt per operation (inspectable history)
4. Write receipt BEFORE mutation (crash invariant — orphan receipts are
   harmless, but the reverse order would cause data loss)
5. Lazy TTL purge on invocation, no background daemon
6. Relations captured in receipt, not re-derived on restore

Default TTL: 30 days. Configurable per-workspace once the wiring lands
in v0.22.0.

### Changed — Developer experience

- Pinned Rust toolchain to 1.95 via `rust-toolchain.toml` — prevents
  the class of bug where `cargo clippy` passes locally but fails on CI
  due to a version skew between developer and runner (hit PR #178 on
  first push with `clippy::unnecessary_sort_by`).

### Verification

- **1245 tests pass / 0 fail** (+31 new across activity + undo modules,
  of which 18 in activity and 13 in undo).
- `cargo clippy --workspace --all-targets -D warnings`: clean.
- `cargo fmt --check`: 0 diffs.
- E2E smoke on fresh tempdir: activity log writes 3 JSONL lines across
  3 tool calls, no secret content leaks into log body.

### Scope trade-offs

`forgeplan_restore` and `forgeplan_undo_last` MCP tools are deferred to
v0.22.0 along with the wrapping of destructive ops. Shipping the
primitives now exercises the receipt format under real CI and lets the
wiring increment land as a cleaner, smaller PR.

Refs: PRD-054, PRD-055.

---

## [0.20.0] — 2026-04-18 — MCP silent-failure hotfix + tool quality (3-round audit)

Originally a v0.19.1 hotfix for two independent silent failures blocking
MCP adoption in v0.19.0 — users who ran `brew install forgeplan &&
forgeplan mcp install --client claude && restart Claude Code` got
**zero tools visible**. Grew via three full audit rounds into a feature
release: every tool now carries workflow guidance and is hardened
against invisible prompt-injection.

### Fixed — the original hotfix

- **`ServerCapabilities::default()` returned empty `{}`** — per MCP spec,
  clients skip `tools/list` when `tools` capability is absent. All 45
  tools invisible after `forgeplan mcp install`. Fix:
  `ServerCapabilities::builder().enable_tools().build()`.
- **`.mcp.json` carried `transport: "stdio"` field** — not MCP spec,
  silently ignored by Claude Code, compounded the capability miss. Fix:
  drop `transport`; `smart_merge` narrowly removes legacy entries.

### Added — tool discoverability

- **ToolAnnotations on all 45 tools** — `title`, `readOnlyHint`,
  `destructiveHint`, `idempotentHint`, `openWorldHint`. Claude Code now
  auto-approves safe reads and warns before destructive ops.
- **Schema enums × 6** — `relation`, `kind`, `status`, `journal.kind`,
  `phase`, `grade` switched from prose strings to JSON-Schema enums.
  LLMs constrain-sample against these so `"informs"` is verbatim, not
  paraphrased as `"inform"`.
- **`_next_action` on 42/42 tools** — 34 as structured JSON field on
  success, 8 as `_next_action:` prose in error text (via `err_hinted` /
  `artifact_not_found` / `llm_err`). Every response — success or
  error — tells the agent what to do next.

### Security — invisible prompt-injection hardening

- **`sanitize_for_hint()`** strips structural punctuation plus invisible
  Unicode classes: zero-width joiners, bidi overrides/isolates, BOM,
  soft-hyphen, variation selectors, tag characters. Applied at every
  `format!` splice of user-controlled values. 15 new unit tests.
- **`llm_err` no longer echoes upstream error bodies** — provider errors
  sometimes include request IDs and header fragments; now logged only.

### Fixed — silent-failure class

- `unwrap_or(Value::Null)` replaced with `hinted_result<T>()` helper —
  serialization failure surfaces as `McpError::internal_error` instead
  of a `Null` response.
- `forgeplan_blocked.blocked_count` was reporting `cycles.len()` instead
  of `blocked.len()`; fixed.
- `forgeplan_fpf_check` dead match arms (`"deny"/"block"/"warn"`) —
  core emits `EXPLORE`/`INVESTIGATE`/`EXPLOIT`; match rewritten.
- Race-condition panic in `forgeplan_link` when artifact deleted
  concurrently — fixed.

Refs: PROB-039, PRD-048, three audit rounds evidence.

---

## [0.19.0] — 2026-04-16 — One-command MCP install + Clippy 1.95 + website i18n RU

Feature release: `forgeplan mcp install` for frictionless AI agent setup,
website i18n with 144 Russian pages, Mermaid diagrams, and Rust 1.95 clippy compliance.

### Added

- **`forgeplan mcp install --client claude|cursor|windsurf`** — one-command
  MCP server configuration. Smart-merge replaces `command`/`args`/`transport`
  while preserving user `env` (API keys, custom paths). Idempotent, safe to
  re-run. Cross-platform: macOS / Linux / Windows.
- **`forgeplan mcp serve`** — alias for `forgeplan serve` (MCP convention naming).
- **`--use-name [forgeplan|fpl]`** — write the short binary name instead of
  absolute path. For terminal-based clients where `$PATH` is set up.
- **`--scope user|project`** — install to user-global (`~/.claude.json`)
  or project-local (`./.mcp.json`).
- **`--dry-run`** — preview proposed changes without writing.
- **`--binary-path`** — override binary path with validation (absolute, exists,
  executable, no control chars / bidi overrides).
- **Binary detection** prefers PATH-resolved symlink over `current_exe()`.
  Fixes Homebrew upgrade breakage where versioned Cellar path goes stale.
- **Symlink rejection** in atomic write — prevents `.mcp.json -> /etc/passwd`
  type attacks via pre-planted symlinks.
- **Website i18n** — 144 Russian pages via Starlight native i18n + Gemini 2.5
  Flash batch translation. Language switcher EN↔RU. (PRD-047)
- **6 Mermaid diagrams** in EN+RU docs (pipeline, ADI, R_eff, tutorial,
  lifecycle, graph).
- **MCP setup guide** — `docs/guides/mcp-setup` (EN + RU). Covers quick install,
  smart-merge, troubleshooting.
- **Website UI polish** — wider search bar, compact theme toggle + language
  switcher, Cloudflare `/ru/` redirects.

### Fixed

- **Clippy 1.95 compliance** — `collapsible_match` (8 occurrences in
  `forgeplan-core`) and `unnecessary_sort_by` (3 occurrences) converted to
  match guards and `sort_by_key(Reverse(...))`.
- **PROB-026** tag canonicalization + **PROB-027** reindex without `lance/`.
- **PROB-035** + **PROB-036** deprecated (resolved by PRD-046 + PRD-047).

### Stats

- 1194 tests (+44 from v0.18.0 baseline 1150)
- 294 website pages (+2 from v0.18.0 baseline 292)
- 0 clippy warnings on Rust 1.95 (stricter than 1.91 / 1.94)
- PRD-048 R_eff: 0.80 (Adequate), EVID-075 active
- 2 adversarial audit rounds (4 agents), 10 CRITICAL/HIGH/MEDIUM findings, all fixed

---

## [0.18.0] — 2026-04-11 — Production BM25 + Russian morphology + quality gates

Feature release upgrading the search engine and codifying quality rules.

### Added

- **Production BM25 engine** (`bm25` crate v2.3.2). Replaces 140 LOC
  hand-written BM25 with production-quality implementation including
  stemming, stop-word removal, and unicode segmentation.
- **Russian morphology support**. `LanguageMode::Detect` with `whichlang`
  auto-selects Snowball stemmer per document/query. "аутентификация"
  now matches "аутентификации" via shared stem. 17 languages supported.
- **Template noise stripping**. `strip_indexing_noise()` removes YAML
  frontmatter, template placeholder lines `{...}`, markdown table rows
  `|...|`, and HTML comments from BM25 index. Fixes false positives
  where `forgeplan search "auth"` matched unrelated PRDs via `author:`
  in frontmatter.
- **O(N) batch search**. Single-pass `search_scores()` replaces O(N²)
  per-record `.score()` calls. 193-artifact corpus: 0.23s.
- **8-point verification checklist** in CLAUDE.md — mandatory before
  every commit/PR. Covers: unit tests, edge cases, E2E on fresh
  workspace, verbatim template test, dogfood stress test, regression
  guard (A/B), negative tests, cross-language verification.

### Changed

- Health debt resolved: 8 active stubs deprecated/superseded, 5
  duplicate EVID pairs deprecated, 3 orphan NOTEs linked. Health
  dashboard reports "Project looks healthy!" with zero warnings.

### Tests

- 1150 tests pass (+19 from v0.17.2 baseline 1131).
- New regression tests: Russian morphology (2), English stemming (1),
  plural forms (1), stop-word resilience (1), noise stripping (2),
  frontmatter false-positive guard (1).

## [0.17.2] — 2026-04-09 — Quality hotfix: scoring & search integrity

Fixes **five** real bugs found during a dedicated /forge E2E verification
sprint on a fresh workspace (separate from the dogfood audit that produced
v0.17.1). Each bug was reproduced on the v0.17.1 release binary before
fixing, and the fix verified A/B on an identical workspace.

The headline fix is **PROB-034 (CRITICAL)** — a silent trust-calculus
regression present since v0.17.0 that inflated R_eff scores across every
workspace using the default evidence template.

### Fixed

- **PROB-034 (CRITICAL) — Multi-line HTML comments shadowed real
  structured fields in `extract_field`.**
  `crates/forgeplan-core/src/scoring/evidence.rs::extract_field` skipped
  only lines *literally starting* with `<!--`, not lines *inside* a
  multi-line comment block. The evidence template ships with a help
  comment:
  ```markdown
  <!--
       verdict: supports | weakens | refutes
       congruence_level: 0 | 1 | 2 | 3 (CL3=same context, CL0=opposed)
  -->
  ```
  The placeholder line `congruence_level: 0 | 1 | 2 | 3 (CL3=...)` does
  not start with `<!--`, so the parser matched it, `parse::<u8>()` failed
  on the non-numeric string, `explicit_cl` became `None`, and the
  **real** `congruence_level: X` in the Structured Fields section below
  was never inspected. Every evidence artifact ever created via the
  default template silently reset to CL3 (no penalty), artificially
  inflating R_eff across every workspace since v0.17.0.
  - Fix: `extract_field` now implements a proper multi-line comment
    state machine — tracks an `in_multiline_comment` boolean, skipping
    all lines between `<!--` and `-->` when they span multiple lines.
  - Affects all fields parsed via `extract_field`: `verdict`,
    `congruence_level`, `evidence_type`, `source_tier` — all were
    silently defaulted. The fix is transitive.
  - A/B verification on `/tmp/fp-prob034-repro` with identical workspace:
    v0.17.1 binary → `r_eff=1.0000, CL=3`; v0.17.2 binary →
    `r_eff=0.1000, CL=0` (correct for explicit CL0 evidence).
  - Regression tests: `extract_field_ignores_multiline_html_comments`,
    `extract_field_multiline_comment_nested_fields_all_ignored`.

- **PROB-030 — BM25 prefix queries returned 0 results.**
  `crates/forgeplan-core/src/search/smart.rs` computed `keyword_score`
  (substring match) for diagnostics but passed only `bm25_norm` to
  `combined_score`. BM25 is token-based, so `auth` did not match the
  token `authentication`, and prefix queries silently returned nothing.
  - Fix: `let keyword_channel = bm25_norm.max(kw);` — BM25 still wins
    on exact-token matches (richer signal), but substring fallback kicks
    in when BM25 returns 0 for prefix queries.
  - Regression tests: `smart_search_prefix_query_falls_back_to_substring`,
    `smart_search_exact_token_still_wins_over_prefix`.

- **PROB-031 — CLI `score` command had its own divergent evidence
  parser.** The CLI `parse_evidence_from_record` in `score.rs`
  duplicated core's function but with a different default CL (CL0 vs
  CL3), creating a visible contradiction: display said
  `CL0 = 0.1` while the `r_eff_recursive` rollup computed `1.00` via
  core's parser. The local CLI parser also did NOT implement the
  PRD-035 Sprint 13.3 H2 security precedence
  (`min(tier_cl, explicit_cl)`), opening a trust-amplification attack
  surface on the display path.
  - Fix: deleted the local duplicate and `extract_field` helper;
    imported `forgeplan_core::scoring::evidence::parse_evidence_from_record`.
    Display and rollup now read identical values by construction.
  - Regression test:
    `score_uses_core_parser_with_cl3_default_when_no_structured_fields`.

- **PROB-032 — `forgeplan search` breakdown line lied about
  components.** Display showed `kw=0.0 sem=0.0 r=0.0 g=0.0` while total
  was 0.57. Caused by PROB-030: `kw` was computed but never flowed into
  `combined_score`.
  - Auto-fixed as side effect of PROB-030. Breakdown now shows real
    component values.

- **PROB-033 — `forgeplan new evidence` printed confusing session
  warning after `forgeplan route`.** The session state machine
  attempted a `Routing → Evidence` transition, which is disallowed.
  The file WAS created, but stderr showed
  `Session: Cannot go from 'routing' to 'evidence'` — blocking
  legitimate backfill, audit, brownfield, and evidence-import flows
  in perception if not in fact.
  - Fix: `forgeplan new evidence` is now phase-agnostic — it never
    drives the session state machine. Only decision artifacts
    (prd/rfc/adr/epic/spec) advance to Shaping phase. Methodology
    guardrail still enforces at `activate` time via PRD-043 stub
    detection + validation gates.
  - Regression test:
    `new_evidence_works_in_routing_phase_without_session_warning`.

### Tests

- 1137 tests pass (+6 from v0.17.1 baseline 1131).
- 6 new regression tests cover PROB-030 (2), PROB-031 (1), PROB-033 (1),
  PROB-034 (2).
- `cargo fmt --check` clean, `cargo clippy --workspace --all-targets --
  -D warnings` clean on both default and `semantic-search` feature.

### Impact

If you are upgrading from v0.17.0 or v0.17.1 and you have evidence
artifacts in your workspace, your R_eff scores were potentially
inflated by the CL3 default (PROB-034). Re-run `forgeplan score` on
critical PRDs after upgrade — any evidence that explicitly set
`congruence_level` in Structured Fields will now be honored, and weak
CL values may cause R_eff to drop. This is correct behavior; the
previous values were silently wrong.

## [0.17.1] — 2026-04-09 — Post-v0.17.0 dogfood hotfix

Fixes two bugs found during the v0.17.0 final dogfood audit when running
`forgeplan tree` and `forgeplan health` on the dogfood workspace itself.
PRD-043 detection (Sprint 13.1) correctly flagged the issues but two
upstream bugs prevented them from being auto-resolved.

### Fixed

- **PROB-028 — Phantom rows in `forgeplan tree`** (PRD-044).
  `reindex` Phase 2 (orphan cleanup) previously skipped rows whose
  `kind` field failed to parse via `continue`, letting corrupt/empty
  kind rows escape trim forever. Additionally, orphan relations whose
  source or target artifact had been deleted accumulated in the
  relations table and surfaced as `?` phantoms in tree rendering.
  - Fix 1: `Err(_) => continue` changed to treat unparseable kind as
    a definite orphan (no valid kind means no valid directory means
    no possible file). Rows with corrupt kind now get trimmed along
    with normal orphans.
  - Fix 2: new Phase 3 in `reindex` trims orphan relations where
    source or target no longer exists in artifacts.
  - Output now reports removal reason: `corrupt kind field` vs
    `no .md file found` vs `orphan relation (source|target|both missing)`.
  - `reindex` output gains a new counter: "K removed, N orphan relations"

- **PROB-029 — `forgeplan health` verdict contradicted its own warnings**
  (PRD-045). Sprint 13.1 added `active_stubs` and `possible_duplicates`
  detection (PRD-043) and wired them into the warning display, but the
  `generate_next_actions` summary function was never updated to read
  those signals. Result: workspace with 8 stubs + 5 duplicate pairs
  printed "Project looks healthy" at the bottom.
  - Fix: `generate_next_actions` now takes `possible_duplicates` and
    `active_stubs` as parameters; compute order reshuffled so signals
    are available before the summary runs.
  - Next actions for stubs suggest `forgeplan supersede ID --by NEW`
    or `forgeplan deprecate ID --reason "abandoned"` with the concrete
    offending ID.
  - Next actions for duplicates suggest
    `forgeplan deprecate B --reason "duplicate of A"` with the concrete
    pair IDs.
  - "Project looks healthy" message only appears when genuinely no
    warnings of any category exist.

### Methodology (NOTE-044 checklist addition)

- Phase 1 Implementation gains new rule: "Every new CLI flag / command
  / config option ships with ALL of these docs (no feature lands
  without): clap `--help` text, CHANGELOG entry, CLAUDE.md workflow
  section if user-facing, `docs/methodology/` subsection if
  command-level." Red flag: a PR adding a flag/command without
  touching clap help + CHANGELOG is incomplete — block merge.

### Stats

- 1131 tests pass (+3 from v0.17.0 — PRD-045 verdict aggregator tests)
- 0 warnings on both default and `--features semantic-search` builds
- Clippy strict (`-D warnings`) clean on Rust 1.94
- Dogfood verification: `forgeplan tree` on dogfood workspace no
  longer shows `?` phantoms; `forgeplan health` reports 3 concrete
  next actions instead of "looks healthy"

### Refs

- PROB-028 (phantom rows reindex bug)
- PROB-029 (health verdict logic bug)
- PRD-044 (reindex trim orphans — closes PROB-028)
- PRD-045 (health verdict aggregator — closes PROB-029)
- NOTE-044 (sprint checklist framework, docs completeness rule added)
- NOTE-046 (dogfood cleanup task — duplicate EVID pairs, deferred)
- NOTE-047 (dogfood cleanup task — false-active stubs, deferred)

## [0.17.0] — 2026-04-08 — EPIC-003: Search, Discovery, Intelligence

First release of EPIC-003. Adds keyword + semantic search, brownfield
discovery, scoring/routing intelligence, FPF rule surface, methodology
integrity gates, and reusable sprint checklist framework.

### Highlights

- **1109 tests passing** (+280 from v0.16.0), zero failures, zero warnings on
  both default and `--features semantic-search` builds
- **7 PRDs shipped** across 8 sprints (13.0 → 13.7 + post-closeout hotfix)
- **FPF Knowledge Base gains semantic vector search** via BGE-M3 embeddings
- **Methodology integrity gates** catch stub artifacts, duplicates, orphans
- **Sprint checklist framework** (NOTE-044) to prevent regression in future
  releases

### Added

**Smart Search v2** — PRD-039, Sprint 13.2
- BM25 ranking replaces substring scoring in `forgeplan search`
- Composable filter DSL (`--status`, `--depth`, `--since`, `--with-evidence`)
- 1-hop graph neighbor expansion (opt-out via `--no-expand`)
- Extended MCP `search` tool parameters

**Brownfield Discovery** — PRD-035, Sprints 13.3 + 13.4
- Tags system in frontmatter + LanceDB schema (v3→v4 migration)
- `forgeplan tag` / `untag` commands + `list --tag key=value` filter
- SourceTier → Congruence Level mapping (T1→CL3, T2→CL2, T3→CL1)
- `forgeplan discover` CLI command (session state machine)
- MCP tools: `forgeplan_discover_start`, `_scan`, `_next`, `_status`

**Scoring & Routing Intelligence** — PRD-040, Sprint 13.5
- Routing Skills Memory with exponential decay (90-day half-life)
- R_eff confidence intervals heuristic (widens with sparse/stale evidence)
- `forgeplan score` displays `[low — high]` interval alongside point estimate

**FPF Rules Surface** — PRD-041, Sprint 13.6
- `forgeplan fpf rules` — action-grouped tree (EXPLORE/INVESTIGATE/EXPLOIT)
  with `--flat` and `--json` modes
- `forgeplan fpf check <id>` — per-artifact rule match introspection
  with `--verbose` (unmatched list) and `--json` (canonical shape)
- MCP tools: `forgeplan_fpf_rules` (with `action`/`name`/`summary`/`source`
  filters) and `forgeplan_fpf_check`

**FPF KB Vector Search** — PRD-042, Sprint 13.7 (supersedes PRD-018)
- `embedding` column (`FixedSizeList<Float32, 1024>`) added to `fpf_spec`
  table, backward-compatible migration via `NewColumnTransform::AllNulls`
- `LanceStore::search_fpf_by_vector(query_vec, limit)` using LanceDB native
  `vector_search` with `DistanceType::Cosine`
- `forgeplan fpf search <query> --semantic` CLI flag
- MCP `forgeplan_fpf_search` gains `semantic: Option<bool>` param
- **Two-layer graceful fallback** — compile-time (feature off) + runtime
  (Embedder init fail / encode fail / vector search fail) → warning +
  keyword fallback
- NaN/Inf rejection at `insert_fpf_chunks` boundary
- Runtime `Embedder::dim() == EMBEDDING_DIM` assertion

**Methodology Integrity** — PRD-043, Sprint 13.1
- Duplicate guard (`forgeplan new` detects existing similar artifacts)
- Stub detection (blocks `activate` on unfilled templates)
- Health detection (`forgeplan health --ci` exits non-zero on blind spots)
- MCP warning envelope for methodology violations
- State machine: `Phase` enum with `validate_transition` enforcing
  Idle → Routing → Shaping → Coding → Evidence → PR for Standard+ depth

**Sprint Checklist Framework** — NOTE-044 (post-closeout deliverable)
- Reusable quality gate for every future sprint, 7 phases with red flags
- Encodes lessons from Sprint 13.7 retrospective
- Explicit "what not to skip" checklist for planning / implementation /
  audit / fixer / re-audit / manual UX / closeout / meta phases

### Changed

- **FPF KB schema**: backward-compatible migration adds `embedding` column
  (nullable). Existing workspaces work unchanged; re-ingest to populate
  embeddings.
- **MCP tool registry expanded** from ~37 to ~47 tools
- **CI linter**: `forgeplan health --ci` + `validate --ci` land (Sprint 11.3)
- **FpfStorage trait extended** — `insert_fpf_chunks` now accepts optional
  embeddings; `search_fpf_by_vector` added to trait (no default impl,
  forcing explicit backend choice per Sprint 13.7 hotfix re-audit)
- **CLI `fpf search` input validation** — empty / oversized (>8192 chars)
  queries rejected before store access
- **MCP param length bounds** on `forgeplan_fpf_search` and
  `forgeplan_fpf_rules` (id ≤128, name ≤128, action ≤64, source ≤16)
- **ANSI strip** on user-supplied query echoed in error messages
  (`No FPF sections match '{}'` sanitized against control chars)

### Deprecated / Superseded

- **PRD-018 "FPF Knowledge Base — semantic search"** — superseded by PRD-042.
  PRD-018 was a false-active stub with R_eff=1.0 but no real implementation,
  flagged by Sprint 13.1 methodology integrity work. PRD-042 closes the gap
  with actual BGE-M3 integration + supersedes PRD-018 to terminal state.

### Fixed

- **Sprint 13.1.5 hardening**: LazyLock<Regex> for `check_stub`, typed
  `StubReport` return, `forgeplan import` gate for active stubs (security
  bypass closed), configurable `IntegrityConfig` MCP limits
- **Sprint 13.1.7 integrity config wiring**: `IntegrityConfig::validate()`
  now called by CLI command path; `forgeplan health` no longer crashes on
  minimal configs via `#[serde(default)]` on top-level `Config` fields
- **Sprint 13.6 FPF Rules canonical JSON**: CLI and MCP now emit identical
  `{artifact_id, kind, status, matched, unmatched, winning, summary}` shape
  via typed `RuleCheckResult`, replacing hand-rolled `serde_json::json!`
- **Sprint 13.7 post-closeout hotfix** (PR #156):
  - `FpfStorage::search_fpf_by_vector` added to trait (closes asymmetry)
  - MCP handler integration harness at `crates/forgeplan-mcp/tests/`
  - Real BGE-M3 end-to-end test (`#[ignore]`, feature-gated)
  - Real v3 workspace migration test
  - Runtime dim assert + `fpf_spec_schema` rustdoc tying 1024 → BGE-M3
  - `InMemoryStore::search_fpf_by_vector` returns `Err` (not silent empty)
  - Wave 2 completer work re-audited (was originally skipped)

### Stats

- 1109 tests passing (+280 from v0.16.0)
- Core crate: 897 tests; CLI: 99 + 40 integration; MCP: 15 unit + 7 handler
- 42 MB release binary (strip + lto + opt-level=z)
- ~56 CLI commands, ~47 MCP tools
- 7 new PRDs activated, 1 superseded (PRD-018 → PRD-042)
- Sprint retrospective: 19 debts found, 11 fixed in hotfix, 8 backlog
  (NOTE-045), 6 process lessons (NOTE-044)

### Methodology lessons captured

- **Dependent sprint branch base verification** — new CLAUDE.md section
  covering the Sprint 13.1.5 rebase hell that taught us to verify parent
  branches contain expected commits before spawning teammates
- **Sprint Checklist Framework (NOTE-044)** — reusable 7-phase gate to
  prevent planning gaps (was: "user had to ask 'what did we miss'")
- **Sprint 13.7 Deferred Debts (NOTE-045)** — backlog tracking for the
  8 non-blocking items that rolled forward from the retrospective

### Related PRs
PRs #141 → #156. See `git log main..release/v0.17.0` for full list.

[0.17.0]: https://github.com/ForgePlan/forgeplan/releases/tag/v0.17.0
