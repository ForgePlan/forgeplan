---
depth: tactical
id: EVID-052
kind: evidence
links:
- target: PRD-026
  relation: informs
status: draft
title: PR 114 docs reorganization — 138 files tracked, CI green, ADR-003 fulfilled
---

# EVID-052: PR #114 docs reorganization

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3

## Congruence Level Justification

CL3 — same context (this very repository), internal dogfood measurement. Changes were made to this repo, CI ran on this repo, counts verified via git commands on this repo.

## Measurement

Direct measurement of repository state after PR #114 commit 0a55be6:

| Metric | Value | Command |
|---|---|---|
| Markdown files tracked in .forgeplan/ | 138 | `git ls-files .forgeplan \| grep -c '\.md$'` |
| Legacy files in docs/{epics,prds,rfcs,adrs,specs}/ | 0 | `find docs/{epics,prds,rfcs,adrs,specs} -name '*.md' 2>/dev/null \| wc -l` |
| docs/guides/ broken references in CLAUDE.md | 0 | `grep -c 'docs/guides/' CLAUDE.md` |
| docs/README.md exists | yes | `test -f docs/README.md` |
| AGENTS.md exists | yes | `test -f AGENTS.md` |

## CI Results

PR #114 checks (GitHub Actions run 24023218770):

| Check | Status | Duration |
|---|---|---|
| Check, Lint & Format | pass | 55s |
| Tests | pass | 1m30s |
| plan (cargo-dist) | pass | 17s |

## Result

All functional requirements from PRD-026 are met. Workspace now follows ADR-003 in practice: markdown files in .forgeplan/ are tracked (source of truth), lance/cache/config stay gitignored (derived/local).

Industry pattern applied identically to node_modules/, target/, .venv/, .astro/ — derived cache inside project, not committed.

## Interpretation

PRD-026 acceptance criteria AC-1, AC-2, AC-3 verified. Success criteria SC-1..SC-6 achieved (SC-5 will be re-verified post-merge on a fresh clone).

The reorganization removes sync drift risk (there was evidence of 4 orphan files before scan-import), establishes single source of truth, and provides proper GitHub discoverability for 138 artifacts that were previously invisible to external contributors.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-026 | informs |
| ADR-003 | informs |

