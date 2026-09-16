---
title: agents-github
description: "GitHub operations agents: pull-request lifecycle, issue triage, release automation, multi-repository coordination, project boards, workflow engineering and repository scaffolding."
---

GitHub operations agents: pull-request lifecycle, issue triage, release automation, multi-repository coordination, project boards, workflow engineering and repository scaffolding. Deliberately not forgeplan-aware - these operate on GitHub, not on the artifact graph. Use when the work is GitHub itself: opening a PR with a real checklist, finding what is blocked on CI, cutting a release, or propagating one change across many repositories. Full history in CHANGELOG.md.

**Cost**: light | **Stability**: stable | **Version**: 1.3.1 | **Category**: developer-tools

**Targets**: claude-code

**Requires**: CLI: gh

## Agents

| Agent | What it does |
|---|---|
| `issue-manager` | GitHub issue lifecycle specialist. |
| `multi-repo-manager` | Cross-repository orchestration specialist. |
| `pr-manager` | Pull request lifecycle specialist. |
| `project-board-manager` | GitHub Projects V2 board specialist. |
| `release-manager` | Software release coordination specialist. |
| `repo-architect` | Repository scaffolding and governance specialist. |
| `workflow-engineer` | GitHub Actions CI/CD specialist. |

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-github@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
