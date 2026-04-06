---
depth: standard
id: PRD-034
kind: prd
links:
- target: EPIC-002
  relation: based_on
- target: NOTE-026
  relation: refines
status: draft
title: CI/CD Architecture Linter — forgeplan health and validate as pipeline gates
---

# PRD-034: CI/CD Architecture Linter — forgeplan health and validate as pipeline gates

## Progress

```
Phase 1  ░░░░░░░░░░░░░░░░░░░░░░░░  0/5  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/5  (  0%)
```

## Problem

`forgeplan health` и `forgeplan validate` показывают проблемы (orphans, blind spots, MUST errors), но **только когда разработчик сам запускает**. Если забыл — мусор попадает в dev. Нет автоматического enforcement в CI pipeline.

**Impact**: architectural debt копится незаметно. Orphans, blind spots, невалидные артефакты merge-ятся без проверки.

## Goals

- [CI pipeline] can block PR merge when architecture health degrades beyond configurable thresholds
- [Developer] can run `forgeplan health --ci` and get exit code 1 if issues exceed thresholds
- [Developer] can run `forgeplan validate --ci` and get exit code 1 if any MUST rules fail
- [Maintainer] can configure thresholds via `--fail-on` flags (orphans, blind_spots, stale)

## Non-Goals

- GitHub Action marketplace package (`uses: forgeplan/action@v1`) — future work
- Integration with other CI systems (GitLab CI, Jenkins) — just shell script
- Auto-fix of found issues — only detection and reporting

## Target Users

| Persona | Description | Key pain |
|---------|------------|----------|
| Solo developer | Uses forgeplan for personal project | Forgets to run health before PR |
| AI agent (MCP) | Runs forgeplan in automated pipeline | Needs exit codes, not pretty output |

## Functional Requirements

| ID | Priority | Requirement | Journey |
|----|----------|-------------|---------|
| FR-001 | Must | [CI pipeline] can run `forgeplan health --ci` and receive exit code 0 (pass) or 1 (fail) | CI gate |
| FR-002 | Must | [CI pipeline] can run `forgeplan validate --ci` and receive exit code 0 (pass) or 1 (fail) for MUST rules | CI gate |
| FR-003 | Must | [Developer] can configure thresholds via `--fail-on orphans=N,blind_spots=M` | CI gate |
| FR-004 | Should | [CI pipeline] can output machine-readable summary (JSON) with --ci --json | CI gate |
| FR-005 | Should | [Maintainer] can add GitHub Actions workflow that runs health+validate on PR | CI setup |

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-002 | Parent epic | active |
| NOTE-026 | Original idea | draft |


