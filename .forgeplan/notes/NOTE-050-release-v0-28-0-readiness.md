---
depth: standard
id: NOTE-050
kind: note
last_modified_at: 2026-05-03T09:42:05.259897+00:00
last_modified_by: claude-code/2.1.126
links:
- target: ADR-003
  relation: informs
- target: ADR-011
  relation: informs
- target: PRD-073
  relation: informs
- target: EVID-097
  relation: informs
- target: NOTE-049
  relation: informs
status: draft
title: Release v0.28.0 readiness
---

# NOTE-050: Release v0.28.0 readiness

| Field | Value |
|-------|-------|
| Status | Draft (until activate after EVID-098) |
| Created | 2026-05-03 |
| Valid Until | 2026-08-01 |
| Context | release, v0.28.0, file-first, claude-print, playbooks |

## Note

Cut release **v0.28.0** на main, accumulating 14 merge-PR (#224..#237) с
момента v0.27.0 (2026-04-28). Bundle theme:

1. **PRD-073 file-first invariant** (Phase 3a/3b/3c/4) — ADR-003 compile-enforced:
   `LanceStore::*` mutating methods stали `pub(crate)`, file-first projection
   wrapper helpers единственная mutation surface. 4 audit rounds, 7 CRITICAL +
   13 HIGH closed. EVID-094 R_eff=0.80 grade A.
2. **ADR-011 Phase B Wave 1** — claude --print dispatchers replace fictional
   task-tool. PluginDispatcher + AgentDispatcher через real `claude --print`,
   8 R1 audit findings closed. EVID-093 + EVID-096 + EVID-097 (последний из
   PR 1, real-E2E на production binary) R_eff=0.70 grade B.
3. **Track 4-A8 canonical playbooks** — `release.yaml` + `brownfield-docs.yaml`
   shipped as REFERENCE pattern для marketplace.
4. **Step.budget_usd + Step.allowed_tools + Step.timeout_seconds** на runtime
   (PRD-072 FR-8).
5. **Real-E2E closure** — Phase B Wave 1 verified end-to-end на production
   `claude` 2.1.126 (PR 1, 2026-05-03), 5 successful invocations + argv
   recording wrapper, EVID-097 supports CL3.

Dependabot status: 18 alerts open на main, 16 auto-close на этом release
merge (lockfile в dev уже at patch versions). 2 carry-forward
(lru transitive via tantivy, uuid transitive via mermaid) с обоснованием
из round 2 triage doc.

Pre-release pre-conditions (verified в этом sprint):
- `cargo fmt --check` clean
- `cargo clippy --workspace --all-targets --features test-helpers -- -D warnings` clean
- `cargo test --workspace --features test-helpers` ← в progress (см. EVID-098)
- `forgeplan health` clean (270 artifacts при создании NOTE-050, +1 для
  EVID-098, 0 blind / orphan / stale)

**Post-merge follow-up** (CLAUDE.md red line #9): после merge
`release/v0.28.0 → main` обязательно открыть sync PR
`chore/sync-main-to-dev-after-v0.28.0` (canonical example: PR #223 после
v0.27.0). Без этого dev forever lags `Cargo.toml` version, и следующий
release создаёт merge conflict на bumped version.

Auto-expires 2026-08-01.

## Hypotheses (ADI seed)

| ID | Hypothesis | Risk | Test |
|----|-----------|------|------|
| HR1 | Все 14 merge-PR's accumulated work ready для production roll-out (compile clean, test green, no breaking surface change для CLI/MCP consumers) | Medium | Full `cargo test --workspace --features test-helpers` PASS |
| HR2 | PRD-073 `pub(crate)` lockdown не breaking для external library consumers потому что blanket `From` impl preserves `?` ergonomics | Medium | grep external usages of `LanceStore::*` mutating methods (нет за границей forgeplan-core) |
| HR3 | claude --print dispatchers ready for production users — argv shape, validation guard, JSON envelope decode work end-to-end | Low | Already verified в EVID-097 PR 1 (real-binary) |
| HR4 | Cargo.toml workspace bump 0.27.0 → 0.28.0 не порушит ABI/API consumers (semver-minor, additive features) | Low | Все public API changes в [Unreleased] CHANGELOG marked **BREAKING** только на library crate scope, не CLI/MCP wire |
| HR5 | brew formula publish flow (cargo-dist + publish-homebrew-formula GitHub Action) сработает на tag v0.28.0 push | Low | Same flow как v0.27.0 (последний successful release); CI workflow files не менялись с того момента |

## Related

| Artifact | Relation |
|----------|----------|
| ADR-003 | informs (file-first invariant — bundle theme) |
| ADR-011 | informs (claude --print dispatchers — bundle theme) |
| PRD-073 | informs (file-first invariant compile-enforced — bundle theme) |
| EVID-094 | informs (PRD-073 closure measurement) |
| EVID-097 | informs (Phase B real-E2E closure measurement) |
| NOTE-049 | informs (PR 1 — closes Phase B verification debt before this release) |
| PROB-049 | informs (Phase 3d follow-ups deferred to PR 3) |
| PROB-050 | informs (Phase B follow-ups deferred to PR 4) |
