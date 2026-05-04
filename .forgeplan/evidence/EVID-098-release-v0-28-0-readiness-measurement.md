---
depth: standard
id: EVID-098
kind: evidence
last_modified_at: 2026-05-03T09:55:04.729430+00:00
last_modified_by: claude-code/2.1.126
links:
- target: NOTE-050
  relation: informs
- target: ADR-003
  relation: informs
- target: ADR-011
  relation: informs
- target: PRD-073
  relation: informs
status: draft
title: Release v0.28.0 readiness measurement
---

# EVID-098: Release v0.28.0 readiness measurement

| Field | Value |
|-------|-------|
| Status | Draft (activate after link) |
| Created | 2026-05-03 |
| Valid Until | 2026-08-01 |
| Target | NOTE-050 (release readiness) + bundle artifacts (ADR-003 + ADR-011 + PRD-073) |

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3

## Measurement

Pre-release readiness measurement для cut release **v0.28.0** (release/v0.28.0
→ main). Bundle = 14 merge-PR's (#224..#237) с момента v0.27.0
(2026-04-28). Verification protocol per CLAUDE.md /forge-cycle steps 7-10.

**Pre-flight gates (all run on release/v0.28.0 branch, post-version-bump):**

1. **`cargo fmt --check`** — exit `0`. Zero formatting diffs.
2. **`cargo clippy --workspace --all-targets --features test-helpers -- -D warnings`** —
   exit `0`. Zero warnings под strict deny config (Rust 1.95
   compliance). Verified via `/tmp/clippy-pr2.log` (background run
   bclafhaq7, completion notification 2026-05-03T07:43Z).
3. **`cargo test --workspace --features test-helpers`** — exit `0`.
   **1940 tests PASS across 37 binaries**, 0 failed, 26.84s для
   forgeplan-core lib alone (largest binary). Verified via
   `/tmp/test-pr2.log` (background run bgdsh5az5, completion notification
   2026-05-03T07:46Z). All 4 R1 audit rounds от PRD-073 + Phase B Wave
   1 represented в test corpus.
4. **`cargo check --workspace`** post-version-bump (0.27.0 → 0.28.0
   workspace + 4 internal crate refs) — exit `0`, "Finished `dev`
   profile in 0.56s" (incremental rebuild, no recompilation needed).
5. **`forgeplan health`** — clean: 271 artifacts (post EVID-098 create),
   0 blind / orphan / stale. Pre-EVID-098 count = 270.

**Version bump verification:**
- Workspace `Cargo.toml`: `version = "0.27.0"` → `"0.28.0"` (line 10)
- `crates/forgeplan-cli/Cargo.toml`: 2 internal dep refs bumped
- `crates/forgeplan-mcp/Cargo.toml`: 2 internal dep refs (main + dev-dep)
- `Cargo.lock`: auto-updated by `cargo check` post-bump (`forgeplan
  v0.28.0`, `forgeplan-core v0.28.0`, `forgeplan-mcp v0.28.0`)
- No source code changes — pure version + doc commit

**CHANGELOG promotion:**
- `[Unreleased]` block content (151 lines, PRD-073 file-first +
  ADR-011 Phase B + Track 4-A8 playbooks) moved к
  `## [0.28.0] — 2026-05-03 — file-first invariant compile-enforced
  + claude --print dispatchers + canonical playbooks`
- New empty `[Unreleased]` placeholder с italic `_No changes yet_`
- `### Verification (PR 1 closures, 2026-05-03)` subsection с
  honest 5-invocation breakdown (3 success + 1 budget-error envelope +
  1 retracted env-export attempt)
- BREAKING-tagged section preserved verbatim для downstream library consumers

**Dependabot triage round 3 (2026-05-03):**
- 18 open alerts на main (5 high / 7 medium / 6 low) — verified via
  `gh api repos/ForgePlan/forgeplan/dependabot/alerts --jq ...`
- Same 18 alert IDs as round 2 (no churn, no new vulnerabilities since 2026-05-02)
- Cargo.lock unchanged since round 2 commit `5c5a182` —
  `git diff 5c5a182..HEAD -- Cargo.lock website/package-lock.json | wc -l` = `0`
- 16 of 18 alerts auto-close on `release/v0.28.0 → main` merge
  (lockfile already at patch versions in dev)
- 2 carry-forward (lru transitive via tantivy, uuid transitive via
  mermaid) — accepted-with-justification per round 2 doc

**Audit gate (release-readiness, 1 round, 2 lenses, adversarial):**

architect-reviewer (CONDITIONAL pass): 1 HIGH + 3 MEDIUM + 2 LOW. All
addressed in this branch:
- A-1 HIGH: stale `task-tool 1.x` references в `audit.yaml` +
  `brownfield-docs.yaml` headers + CHANGELOG `### Added` line — fixed
  to ADR-011 reality. Discovered deeper finding: `validate_agent_name`
  regex rejects colon-namespaced slugs — added as PROB-050 A-28.
- A-2 MEDIUM: NOTE-050 `269 → 270 artifacts` count fixed.
- A-3 MEDIUM: post-merge sync PR reminder added к NOTE-050.
- A-4 MEDIUM: dep triage 2026-05-03 verification command corrected
  (compares to round 2 commit, not v0.27.0 tag).
- A-5 LOW: semver bump justified — sign-off, no action.
- A-6 LOW: `[Unreleased]` placeholder style — keep as `_No changes yet_`.

code-analyzer (CONDITIONAL pass): 0 HIGH + 2 MEDIUM + 4 LOW. All addressed:
- C-1 MEDIUM: "5 successful" overstated — reworded to "5 measured (3
  happy + 1 budget-error envelope + 1 retracted)".
- C-2 MEDIUM: CHANGELOG duplicate narrative — `### Originally — PRD-073`
  collapsed to `### Detail — PRD-073 file-first invariant (EVID-094 R_eff=0.80
  grade A)`.
- C-3 LOW: NOTE-050 trailing 5 blank lines trimmed.
- C-4 LOW: typo "readyдля" → "ready для".
- C-5 LOW: dep triage round-1 missing-doc-artifact note added.
- C-6 LOW: dep triage method §3 cross-check wording simplified.

**New PROB-050 acceptance criteria (added in this PR):**
- A-27: sweep marketplace/playbooks/* headers для stale ADR-010 refs
  (audit.yaml + brownfield-docs.yaml partially done, others pending)
- A-28: validate_agent_name regex too restrictive для colon-namespaced
  Claude Code agent slugs — pick (a) YAML rewrite или (b) regex broaden

## Result

**All pre-flight gates ✅ PASS**:

| Gate | Result | Evidence |
|------|--------|----------|
| `cargo fmt --check` | exit 0 | (silent, 0 diff) |
| `cargo clippy --workspace --all-targets --features test-helpers -- -D warnings` | exit 0 | `/tmp/clippy-pr2.log` |
| `cargo test --workspace --features test-helpers` | exit 0 | `/tmp/test-pr2.log` (1940 tests PASS) |
| `cargo check --workspace` post-bump | exit 0 | 0.56s incremental |
| `forgeplan health` | clean | 271 artifacts, 0 blind/orphan/stale |
| Dependabot triage | re-verified | round 3 doc, 18 alerts unchanged |
| Audit (architect + code-analyzer) | CONDITIONAL → all findings closed | в этом diff |

**Bundle composition (14 merge-PRs):**
- PRD-073 file-first invariant compile-enforced (ADR-003) — EVID-094 R_eff=0.80 grade A
- ADR-011 Phase B Wave 1 (claude --print dispatchers) — EVID-093+096+097, R_eff=0.70 grade B
- Track 4-A8 canonical playbooks (release.yaml + brownfield-docs.yaml)
- Step.budget_usd + Step.allowed_tools + Step.timeout_seconds runtime fields
- Real-E2E closure of Phase B (PR 1, EVID-097): 5 measured claude invocations,
  ~$0.98 USD spent, byte-identical argv recording, validation guard pre-spawn reject

**No release blockers identified.** All audit findings (HIGH+MED+LOW)
closed в branch before this evidence pack created. Net-new findings
(A-27, A-28) added к PROB-050 для proper handling в PR 4 — no impact
на v0.28.0 release readiness.

## Interpretation

**Release v0.28.0 ready for cut.** All pre-flight gates green, audit findings
closed, dependabot triage re-verified, no source code changed (pure
version-bump + CHANGELOG promotion + supporting documentation).

The release primarily ships **library-API-breaking-but-CLI-API-stable**
changes (PRD-073 `pub(crate)` lockdown). Pre-1.0 minor bump (0.27 → 0.28)
correctly signals breaking via Cargo's caret-pinning behavior:
downstream library consumers с `^0.27` lock won't auto-upgrade. CLI
+ MCP wire surfaces unchanged.

**Risks (residual, не блокеры):**
1. PROB-050 A-28 (validate_agent_name regex incompatibility с
   `agents-pro:architect-reviewer` slug format) means `audit.yaml`
   currently cannot run end-to-end через dispatcher — это shipped в
   v0.27.0 unchanged, не regression на v0.28.0. Tracked for PR 4.
2. SkillDispatcher v1 stub (PROB-050 A-23) still returns success без
   real invocation — same shipped state как v0.27.0, не regression.
3. Brew formula publish flow (cargo-dist + publish-homebrew-formula
   GitHub Action) — same flow as v0.27.0 successful release, no CI
   workflow file changes since the previous tag.

**Post-merge follow-up (CLAUDE.md red line #9):**
Open `chore/sync-main-to-dev-after-v0.28.0` PR after release/v0.28.0 merges.
Captured в NOTE-050 §"Post-merge follow-up" so cold-context handoff
agent will see it.

## Congruence Level Justification

CL3 — same context, direct measurement on the actual release branch
(`release/v0.28.0`) of the actual workspace before push. All gate
results captured from real command output (cargo fmt/clippy/test/check,
forgeplan health, gh api dependabot, git diff). No proxies, no
synthetic measurements. Cost: $0 (no claude API calls в этом PR —
only doc + version bump).

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| NOTE-050 | informs (release readiness shape) |
| ADR-003 | informs (bundle theme — file-first invariant compile-enforced) |
| ADR-011 | informs (bundle theme — claude --print dispatchers) |
| PRD-073 | informs (closure артефакта в этом релизе) |
| EVID-094 | informs (PRD-073 closure measurement, predecessor) |
| EVID-097 | informs (Phase B real-E2E closure measurement, sibling в этом релизе) |
| PROB-050 | informs (A-27 + A-28 added в этом sprint) |




