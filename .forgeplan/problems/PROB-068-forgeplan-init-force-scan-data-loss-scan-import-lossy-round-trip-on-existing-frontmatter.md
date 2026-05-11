---
depth: tactical
id: PROB-068
kind: problem
links:
- target: PROB-061
  relation: informs
status: active
title: forgeplan init --force --scan data loss + scan-import lossy round-trip on existing frontmatter
---

# PROB-068: forgeplan init --force --scan data loss + scan-import lossy round-trip

## Signal

Discovered during v0.31.0 cleanup sprint (2026-05-11) by `w5-prob-065-fix` worker. Two data-loss surfaces observed:

1. `forgeplan init --force --scan` invocation в a populated workspace **blew away markdown artifact bodies** (kept frontmatter, dropped narrative content). Recovered только via `git restore .forgeplan/`.

2. Subsequent `forgeplan reindex` invocation **reverse-mutated 83 artifact frontmatters** — dropped `links:` sections + injected `author: scan-import`. Lossy round-trip from LanceDB index back к markdown files.

## Context

- **Workspace**: dev branch checkout, 309 active artifacts (PRDs/RFCs/ADRs/Epics/Specs/Problems/Evidence/Notes), pre-existing `links:` relationships across most.
- **Trigger**: `forgeplan init --force --scan` (force re-init + scan-import existing files).
- **Recovery**: `git restore .forgeplan/` reverted both data-loss vectors. Lost work would require manual reconstruction or git history excavation if no backup taken.
- **Reproducibility**: 100% on any workspace с existing artifacts and populated links.

## Root cause (hypothesized)

Two independent issues:

1. **`init --force` нестабилен в populated workspace** — `--force` semantics unclear: does it preserve content or restart? Current behavior suggests partial wipe но keeps frontmatter — worst of both worlds.

2. **`scan-import` lossy round-trip on existing frontmatter** — scan-import builds artifact from filesystem state but doesn't preserve metadata not visible to scanner: `links:` (relations are stored в LanceDB, scan-import may not reconstruct from markdown alone), `author:` (overwritten к "scan-import" — original author lost).

## Why now

PROB-060 Phase 2.5 work + W5A worker's experimentation с `init --force --scan` для consistency check. Pre-PROB-060 this combination wasn't common.

## Decision / Proposed fix

**Option A**: make `init --force` strictly additive — only init missing directories/files, never overwrite existing artifact .md bodies. Doc: `--force` reinitializes config + indices только.

**Option B**: scan-import should preserve existing frontmatter fields it doesn't own (`links:`, `author:`, custom fields). Implement union-merge instead of overwrite.

**Option C**: pre-flight backup mandatory — `forgeplan init --force` requires `--no-backup` to skip, otherwise auto-exports artifact bodies к `.forgeplan-backup-$DATE/`.

Combine A + B + C для defense-in-depth. C is cheapest mitigation if A/B fixes are scoped to multi-PR.

## Acceptance criteria

1. `forgeplan init --force` в populated workspace preserves all artifact .md body content
2. `forgeplan scan-import` (after manual file edit OR git pull) preserves `links:` section + `author:` field if present
3. Regression test: populate workspace с linked artifacts, run init --force --scan, verify links + bodies unchanged
4. CLI help text explicitly describes destructive vs non-destructive `--force` semantics

## Linked artifacts

- informs PROB-061 (change_log table reset on reindex — similar class)

## References

- v0.31.0 sprint, 2026-05-11
- w5-prob-065-fix worker recovery via `git restore`
- Severity: HIGH (data loss on common operation)
- CWE-664 (Improper Control of Resource Through Lifetime)
- CWE-693 (Protection Mechanism Failure)


