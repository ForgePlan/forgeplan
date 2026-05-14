---
depth: standard
id: PRD-077
kind: prd
last_modified_at: 2026-05-13T22:33:10.027239+00:00
last_modified_by: claude-code/2.1.139
status: draft
title: 'v0.32.0 — Loop closure: onboarding + skills evidence autopublish + quality polish'
---

# PRD-077: v0.32.0 — Loop closure + complete polish (everything works consistently)

## Sprint scope change (2026-05-13)

User direction: "Нужно чтобы всё работало идеально — каждый пункт и тем более со скиллами чтобы было чётко и синхронизировано. Всё должно работать консистентно. И вот эти отложенные пункты тоже делаем в самом конце."

This is no longer a "minimal v0.32.0" — this is the **comprehensive consistency release**. Every previously-deferred item moves into Block 4. We do not ship until each FR has been verified to actually work end-to-end (not just compile + pass unit tests).

## Naming note (Round-2 audit fix, 2026-05-13)

Original draft of this PRD spoke of `secrets.yaml`. During Wave 1.5 (audit-fix round) the file was renamed to `secrets.env` because the body is shell-syntax `export VAR=...`, not YAML — YAML linters errored, `serde_yaml::from_str` rejected, users were instructed to `source` a `.yaml` file. The new name matches dotenv/direnv convention. All FRs below now reference `secrets.env`; the code in Wave 1.5 commit `838f8c5b` shipped under that name.

## Problem

ForgePlan v0.31.0 shipped with strong artifact infrastructure and security gates, but the work loop is not closed end-to-end. 5-agent research (2026-05-13) found:

1. **Onboarding silent failure** — `forgeplan init` creates 12 empty artifact directories; git does not track empty dirs. Real bug: explosivebit and Илья both ran `forgeplan init && git commit && git push`, neither saw each other's artifacts because all dirs disappeared in transit. Plus there is no canonical place to store API keys during local dev — `config.yaml::llm.api_key_env` references an env var name, not a file.

2. **Reason + dispatch tools are functional but undiscoverable** — `forgeplan reason <id>` works (verified live on PROB-021 returning structured 3-hypothesis JSON), requires Gemini LLM via `GEMINI_API_KEY`, but `--help` does not mention it. `forgeplan dispatch` correctly serialises artifacts without `affected_files` frontmatter for safety, but produces no per-artifact reason field, so a 50-item serial queue reads as "broken".

3. **Skills × forgeplan loop is broken** — `/audit`, `/build`, `/do` recommend `forgeplan new evidence` but never invoke it. `/sprint` in fpl-skills 1.9.0 has integration at wave-close step 4b-bis but it is opt-in suggestion. Our dogfood repo has 4 blind-spot PROBs (062, 067, 069, 070); none had evidence written automatically. The graph is blind to the work we did. `forgeplan health` honestly reports `unhealthy`.

4. **Quality polish residual** — PROB-069 stress test timing breach, PATH-race flake in `commands/mcp.rs`, multiple PROB-070 deferred subitems, release protocol undocumented (v0.31.0 CHANGELOG was hand-written despite `forgeplan release-notes` existing).

5. **Previously-deferred items**: full PROB-062 config-split + migration, Windows path masking (SEC-003), CI continue-on-error policy (SEC-004), multi-point benchmark (TST-003), recursive audit of TIER 1 fixes from v0.31.0. User decision (2026-05-13): all of these now in v0.32 Block 4 instead of v0.33+.

## Goals

1. **Onboarding integrity** — `forgeplan init` produces a workspace that round-trips cleanly through `git commit && git push && git clone && forgeplan list` on a fresh machine.
2. **Tool discoverability** — `forgeplan reason` and `forgeplan dispatch` explain themselves; users know preconditions without reading source.
3. **Skills evidence loop closure** — completing `/audit`, `/sprint`, `/build` writes EVID automatically. PR creation hard-blocked when target lacks evidence (with documented bypass).
4. **Quality polish** — flakes resolved, deferred audit findings closed, release protocol documented and dogfooded.
5. **Complete consistency** — every "deferred" item from v0.31 closure that was kicked to v0.33 now lands here. The release ships only when every FR is verified end-to-end working.

## Non-Goals

Nothing is non-goal anymore. Per user direction the previously-deferred bucket moves into this PRD as Block 4. Anything that would normally be deferred to v0.33+ must be explicitly accept-with-justification'd at audit-close, not pre-scoped out.

## Target Users

- New ForgePlan adopters running `forgeplan init` in a team setting (gitkeep + secrets)
- Multi-agent operators running `/audit`, `/sprint`, `/build` (loop closure)
- CI maintainers (flake resolution, SHA-pin hygiene)
- Windows users (SEC-003) — even though we don't ship Windows binaries yet, the sanitiser stops leaking platform paths
- ForgePlan dogfood team (us) — closing our own blind spots demonstrates the methodology

## Functional Requirements

### Block 1 — Onboarding Integrity

- [x] **FR-001**: `forgeplan init` writes `.gitkeep` (empty file) into all 12 trackable artifact subdirs (`prds`, `epics`, `specs`, `rfcs`, `adrs`, `problems`, `solutions`, `evidence`, `notes`, `refresh`, `memory`, `discovery`)
- [x] **FR-002**: `forgeplan init` creates `.forgeplan/secrets.env` with a commented-out template referencing the same env var keys as `config.yaml::llm.api_key_env`. Template only — never loaded by forgeplan code
- [x] **FR-003**: Workspace `.gitignore` managed block gains `.forgeplan/secrets.env` line
- [x] **FR-004**: Integration test `cli_init_git_roundtrip` — init in tempdir → `git init && git add .` → assert all 12 artifact dirs visible in `git ls-files`
- [x] **FR-005**: Integration test asserts `secrets.env` present after init but NOT in `git ls-files`. Plus negative regression: legacy `secrets.yaml` is NOT created
- [x] **FR-006**: `forgeplan health` gitignore-drift detector includes `secrets.env` (warns if user accidentally git-adds it)
- [x] **FR-007**: `forgeplan reason --help` text mentions LLM requirement + the `api_key_env` env var pattern
- [x] **FR-008**: Missing-LLM error message includes single `Fix:` hint pointing at `.forgeplan/secrets.env`. SEC-C3: all `llm.provider` / `api_key_env` / error chain interpolations routed through `sanitize_for_hint` to block control-byte injection (round-2 audit)
- [x] **FR-009**: `forgeplan new prd/rfc/adr/epic/spec` Hint protocol emits `Next: forgeplan reason <id>` for Standard+ depth
- [x] **FR-010**: `forgeplan dispatch` adds `serial_queue[].reason` field naming why each artifact was serialised. Multi-parent dependency now lists ALL blockers (round-2 audit CR-H4 fix)

### Block 2 — Loop Closure

- [x] **FR-011**: `/audit` skill autopublishes EVID after 4-agent panel — calls MCP `forgeplan_new(kind=evidence)` + structured fields (verdict/CL/evidence_type=audit) + `forgeplan_link` to audited artifact + optional activate prompt. **Lives in marketplace repo** branch `feat/v032-loop-closure`
- [x] **FR-012**: `/sprint` skill step 4b-bis upgraded from "recommendation" to "obligation". Per-wave EVID emission; if zero EVIDs emitted at sprint-close, warn with batch-emit suggestion. **Marketplace repo**
- [x] **FR-013**: `/build` skill autopublishes EVID after build, same pattern. **Marketplace repo**
- [x] **FR-014**: New hook `.claude/hooks/pre-pr-evidence-check.sh` — blocks `gh pr create` when artifact in branch/commit `Refs:` lacks linked evidence. Override via `--no-evidence-check` flag or `FORGEPLAN_SKIP_EVIDENCE=1`. Exit 2 = block. **Round-2 fix**: hook now reads Claude Code PreToolUse JSON payload from stdin, narrows to only `gh pr create` invocations (previously fired on every Bash). Registered in `.claude/settings.json`. Graph JSON parsed via `jq` (no greedy regex)
- [x] **FR-015**: Hook documentation in CLAUDE.md + `docs/methodology/EVIDENCE-PROTOCOL.md` (EN + RU) explaining when bypass is appropriate (docs-only PRs, sync PRs, etc.)
- [ ] **FR-016**: PROB-067 EVID closure — file EVID describing cross-worktree lock + collision detection (work already in v0.31.0), link to PROB-067, activate. Done via skill autopublish after this PRD activates (dogfooding)

### Block 3 — Quality Polish

- [x] **FR-017**: PROB-069 stress test fix — split fast (3 seeds @ 15s default) + slow (12 seeds @ 60s `#[ignore]`'d). No CI flake on default `cargo test --workspace`
- [x] **FR-018**: `which_on_path_finds_fake_binary` + all PATH-mutating tests in `commands/mcp.rs` (and `playbook/dispatch/helpers.rs`, `plugin_dispatcher.rs`, `plugins/types.rs`, `mcp/server.rs`) wrap in `serial_test` crate with `env_path` / `env_home` named keys
- [x] **FR-019**: SHA-pin all third-party + GitHub-official `uses:` in `.github/workflows/*.yml`. Dependabot config gains `cargo` ecosystem group. Round-2 fix: two missed `dtolnay/rust-toolchain@stable` in ci.yml pinned to SHA `29eef336...`. Comments now reflect real version (`# stable (pinned)`)
- [x] **FR-020**: LOG-003 — `health/mod.rs::health_report_with_phase` parallel reader replaces `.ok().flatten()` with explicit match + `tracing::warn!`. New `HealthReport.phase_read_errors: usize` field surfaced in CLI dashboard + JSON. Round-2 fix: `e` routed through `sanitize_error_chain`, `path` through `sanitize_path_for_display`. E2E test uses `chmod 000` to trigger real EACCES (corrupt-YAML hits quarantine `Ok(None)` path, NOT counter)
- [x] **FR-021**: DOC-003 — `strict_exit_code` rustdoc explicitly notes duplicates + stale contribute via verdict promotion, not direct count
- [x] **FR-022**: New doc `docs/operations/RELEASE-PROTOCOL.md` (EN + RU) describing canonical release flow. Round-2 fix: PR numbers corrected (#283 was v0.32 wave, not v0.31 integration; replaced with actual chain #282 → #284 → #285)

### Block 4 — Previously-Deferred (still upcoming as of 2026-05-14)

- [ ] **FR-023 (PROB-062 full)**: Split `config.yaml` capability — introduce `forgeplan_core::config::secrets` module that reads `secrets.env` if present, merges API keys at LLM-call time, never persists to disk. `config.yaml` stays as primary config. Migration command `forgeplan migrate-secrets` extracts any inline API keys from `config.yaml`, writes them to `secrets.env`, replaces with `api_key_env` reference. Dry-run mode default
- [ ] **FR-024 (SEC-003 Windows)**: `sanitize_text_with` gains `#[cfg(windows)]` branch reading `USERPROFILE` + `TEMP` env vars; applies same anchored mask logic as Unix HOME. Plus byte-level handling for backslash separators
- [ ] **FR-025 (SEC-004 CI policy)**: Decision: either remove `continue-on-error: true` from `security.yml` (hard gate) OR remove `pull_request:` trigger entirely (push + cron only). Whatever lands gets a comment block explaining the choice. Tracked in ADR
- [ ] **FR-026 (TST-003 multi-point bench)**: `health_bench.rs` parametrized over `[1000, 5000, 10000]` fixture sizes. Assertion: latency grows sub-quadratically. Stays `#[ignore]` for default CI, optional opt-in nightly job in `.github/workflows/perf.yml`
- [ ] **FR-027 (recursive audit on v0.31 TIER 1)**: Spawn one focused audit on commits `b5a21bf` + `cef1695` + `36c720d` + `897377b` + `095bc4b` (the v0.31 audit-fix bundle). Sanity-check the fixes themselves did not introduce regressions. Findings either inline-fixed or filed PROB. ≥1 finding required (escalate if zero — superficial review signal)

## Acceptance Criteria

- All 27 FRs landed
- Every FR verified end-to-end working (not just compile/unit-test) — written into closure EVID body with concrete reproduction commands
- `cargo test --workspace` passes 0 failures (PROB-069 closed via FR-017)
- `cargo clippy --workspace --all-targets -- -D warnings` clean
- `bash scripts/smoke-test.sh` passes 18+ ops
- `scripts/check-mcp-tool-count.sh` zero drift (72 tools)
- `forgeplan health` on this repo verdict: `healthy` OR `needs_attention` with explicit accept-with-justification for any remaining blind spot (was `unhealthy` pre-sprint)
- EVID for PRD-077 written via skills autopublish (dogfooding FR-011/12/13)
- v0.32.0 release notes generated via `forgeplan release-notes` first, then human narrative wrap (FR-022 protocol verified end-to-end)
- 2-agent adversarial audit on integration branch before release PR — findings either fixed or accept-with-justification

## Status (Wave 1 + 1.5 + Wave 2 partial — 2026-05-14)

**Done**:
- Block 1 (FR-001..010) — Wave 1 W1+W2, audit-fixed in Wave 1.5 F1-F3 + manual round-2 fixes
- Block 2 (FR-011..015) — Wave 1 W7 (hook + docs in main repo) + Wave 2 W5+W6 (skills in marketplace repo). Round-2 fix wired the hook to `.claude/settings.json`
- Block 3 (FR-017..022) — Wave 1 W3+W4, audit-fixed in Wave 1.5 F1+F2+F3

**In progress**:
- FR-016 — depends on FR-011 (use the new auto-emit skill)
- Block 4 (FR-023..027) — Wave 3 not yet started

**Round-2 audit closure** (2026-05-13/14):
- HIGH SEC-H-R2-1 ✓ — two missed `dtolnay/rust-toolchain@stable` SHA-pinned
- CRITICAL CR2 ✓ — pre-pr-evidence-check.sh wired to `.claude/settings.json`, narrowed to `gh pr create`
- MED R2-M2 ✓ — three remaining `secrets.yaml` strings in `reason.rs` replaced
- MED R2-M3, R2-M4, R2-M5 ✓ — addressed in this PRD body update + follow-up commits
- LOW R2-L1, R2-L2, R2-L3 — accept-with-justification (dead_code variant, doc-doc consistency, phrase unification — non-blockers, addressed in CHANGELOG)

## Wave Structure

**Wave 1** ✓ — Block 1 + Block 3 (parallel, file-disjoint, 4 workers W1-W4)
**Wave 1.5** ✓ — audit-fix round (F1 CI, F2 security, F3 data/arch). Round-2 audit caught residual issues, fixed in main thread
**Wave 2** ✓ partial — Block 2 (W5 + W6 in marketplace repo, W7 in main repo)
**Wave 3** — Block 4 (FR-023 secrets module + migration, FR-024 Windows, FR-025 CI policy ADR, FR-026 multi-point bench, FR-027 recursive audit)
**Wave 4** — Integration + 2-agent final audit + Cargo bump + release-notes dogfood + release PR → main → tag → sync PR

## Blast Radius

- **High**: skills changes user-visible immediately; hard pre-PR hook can frustrate without good docs (mitigated via thorough EVIDENCE-PROTOCOL.md)
- **High**: FR-023 `secrets.env` migration touches config loader — needs backward compat test (existing workspaces with inline API keys must keep working)
- **Medium**: FR-024 Windows code is new platform branch; tests must run under `cfg(windows)` somehow OR be unit-tested by passing fake env vars
- **Medium**: FR-025 CI policy change can flake or block other PRs depending on choice
- **Low**: dispatch explain, reason help, LOG-003 logging, DOC-003 doc fix — additive

## Reversibility

All FRs reversible by `git revert`. No on-disk artifact format changes. No LanceDB schema changes. `secrets.env` purely additive (no consumer code until FR-023 lands, even then opt-in via file presence). Pre-PR hook ships with override flag from day 1.

## Related Artifacts

| Artifact | Relation |
|---|---|
| PROB-062 | refines (Phase 1+2 — gitkeep/secrets in Block 1, full config split in Block 4) |
| PROB-067 | refines (EVID closure for v0.31 work) |
| PROB-069 | refines (stress test budget fix lands here) |
| PROB-070 | refines (ALL 8 subitems land in this sprint per user direction) |
| EVID-122 | based_on (v0.31.0 closure evidence — informs what's "done" vs "deferred") |

## References

- 5-agent research panel 2026-05-13 — explorers analysed init/gitignore, reason/dispatch, health/release-notes/session, skills/hooks integration
- Wave 1 audit Round-1 (2026-05-13) — 12 findings, addressed in Wave 1.5
- Wave 1 audit Round-2 (2026-05-13/14) — 12 findings, addressed in main-thread inline fixes
- User direction 2026-05-13: scope expansion to include all previously-deferred items
- User direction 2026-05-13: "оставим config.yaml как есть и просто добавим secrets.yaml + в ignore" (file renamed to `secrets.env` during CR-C4 fix — body is shell syntax not YAML)
- User direction 2026-05-13: "Нужно чтобы всё работало идеально — каждый пункт и тем более со скиллами чтобы было чётко и синхронизировано"


