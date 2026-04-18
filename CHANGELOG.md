# Changelog

All notable changes to Forgeplan are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/). Semver: `MAJOR.MINOR.PATCH`
with pre-1.0 minor bumps for breaking changes.

This file starts at v0.17.0. For prior releases, see git tags and the
corresponding sprint evidence under `.forgeplan/evidence/`.

## [0.20.0] — 2026-04-18 — MCP silent-failure hotfix + tool quality (3-round audit)

Originally a v0.19.1 hotfix for two independent silent failures that blocked
MCP adoption in v0.19.0. Grew via three full audit rounds into a feature
release: every tool now carries workflow guidance and is hardened against
invisible prompt-injection.

### Fixed — the hotfix reason

- **`ServerCapabilities::default()` returned empty `{}`** — per MCP spec,
  clients skip `tools/list` when `tools` capability is absent. All 45 tools
  invisible to Claude Code / Cursor / Windsurf after `forgeplan mcp install`.
  Fix: `ServerCapabilities::builder().enable_tools().build()`.
- **`.mcp.json` carried `transport: "stdio"` field** — not MCP spec; Claude
  Code silently ignores unknown fields, compounding the capability miss.
  Fix: drop `transport`; `smart_merge` narrowly removes legacy `transport:
  "stdio"` and `type: "stdio"` while preserving `type: "http"` configs.

### Added — tool discoverability (agents work better)

- **ToolAnnotations on all 45 tools** — `title`, `readOnlyHint`,
  `destructiveHint`, `idempotentHint`, `openWorldHint`. Claude Code
  auto-approves safe reads and warns before destructive ops.
- **Schema enums × 6** — `relation`, `kind`, `status`, `journal.kind`,
  `phase`, `grade` switched from prose-listed strings to typed JSON-Schema
  enums. LLMs constrain-sample against these so `"informs"` is verbatim,
  not paraphrased as `"inform"`.
- **`_next_action` on 42/42 tools** — 34 as structured JSON field on
  success, 8 as `_next_action:` prose in error text via `err_hinted` /
  `artifact_not_found` / `llm_err`. Every response — success or error —
  tells the agent what to do next.

### Security — invisible prompt-injection hardening (audit Rounds 2-3)

- **`sanitize_for_hint()`** strips structural punctuation (`` ` ``, `{`,
  `}`, quotes, backslashes, control chars) **and** invisible Unicode
  classes: zero-width joiners (U+200B..U+200F), bidi overrides/isolates
  (U+202A..U+202E, U+2066..U+2069), BOM (U+FEFF), soft-hyphen, Arabic
  letter mark, Mongolian separators, variation selectors (U+FE00..U+FE0F,
  U+E0100..U+E01EF), tag characters (U+E0000..U+E007F). Truncation to
  80 chars happens AFTER filtering so hidden chars can't consume budget.
  Applied at every `format!` splice of user-controlled values in
  `_next_action` and error messages. +15 unit tests covering each class.
- **`llm_err` no longer echoes upstream error bodies** — Anthropic /
  OpenAI / Gemini sometimes include request IDs and header fragments in
  errors. Now logged via `tracing::warn` only; user-visible text is
  generic remediation.

### Fixed — silent-failure class (audit R2 H-1)

- **`unwrap_or(Value::Null)` replaced with `hinted_result<T>()`** —
  serialization failure now surfaces as `McpError::internal_error`
  instead of a `Null` response (same bug class as the v0.19.0
  capability regression).
- **`forgeplan_blocked.blocked_count` fixed** — was reporting
  `cycles.len()` instead of `blocked.len()` (audit R2 H-3). Shipping
  tool with wrong numbers.
- **`forgeplan_fpf_check` dead match arms** — referenced `"deny"` /
  `"block"` / `"warn"` but core only emits `EXPLORE` / `INVESTIGATE` /
  `EXPLOIT`. All agents fell through to generic default. Rewritten
  against the actual `ActionType::Display` taxonomy.
- **Race-condition panic in `forgeplan_link`** —
  `.unwrap_or(Some(record)).unwrap()` panicked on `Ok(None)` when
  another MCP client deleted the artifact concurrently. Fixed to
  `.ok().flatten().unwrap_or(record)` (R3 deep-QA finding).

### Added — integration test for the regression

- **`tests/server_capabilities.rs`** — asserts `get_info()` declares
  `tools` capability both in the Rust struct and in the serialized
  JSON (wire-format). Would have caught v0.19.0 bug pre-release.

### Verification

- 1214 tests pass / 0 fail (+63 since v0.19.0, of which 15 are new
  `sanitize_for_hint` tests covering every Unicode injection class).
- `cargo clippy --workspace --all-targets -D warnings`: clean.
- `cargo fmt --check`: 0 diffs.
- Full E2E smoke on fresh tempdir + real workspace (212 artifacts):
  42/42 tools return workflow hints (34 success + 8 graceful error).
- Real Claude Code dogfood: all 45 tools visible after session restart;
  `_next_action` populated; injection payload via crafted artifact ID
  stripped and hint surfaced.

Refs: PROB-039, PRD-048, audit rounds 1-3 evidence.

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
