---
title: agents-github
description: "GitHub operations agents: pull-request lifecycle, issue triage, release automation, multi-repository coordination, project boards, workflow engineering and repository scaffolding."
---

**Catalog description:** GitHub operations agents: pull-request lifecycle, issue triage, release automation, multi-repository coordination, project boards, workflow engineering and repository scaffolding. Deliberately not forgeplan-aware - these operate on GitHub, not on the artifact graph. Use when the work is GitHub itself: opening a PR with a real checklist, finding what is blocked on CI, cutting a release, or propagating one change across many repositories. Full history in CHANGELOG.md.

**Cost**: light | **Stability**: stable | **Version**: 1.3.1 | **Category**: developer-tools

**Targets**: claude-code

**Requires**: CLI: gh

**Packaged surface**: 0 command files, 7 agent files, 0 skills

## Agents

Claude Code scoped names are shown below. Its [agent loader](https://code.claude.com/docs/en/plugins/components#agents) scans `agents/` recursively, keeps subfolder segments and applies frontmatter `name` to the final segment. Other runtimes may use different names. Descriptions are packaged metadata except for curated corrections to documented runtime behavior; runtime boundaries, where provided, take precedence over optimistic metadata wording.

| Agent | Source path | What it does |
|---|---|---|
| `agents-github:issue-manager` | `agents/issue-manager.md` | GitHub issue lifecycle specialist. Creates, triages, and decomposes issues into subtasks; tracks progress via label and milestone workflows; automates stale-issue cleanup with configurable thresholds. Use when you need to manage a backlog at scale, bulk-label or bulk-close issues, or set up automated staleness enforcement. Produces `gh` command sequences, label taxonomy suggestions, and triage checklists. Pairs with `pr-manager` for end-to-end issue-to-PR traceability. |
| `agents-github:multi-repo-manager` | `agents/multi-repo-manager.md` | Cross-repository orchestration specialist. Discovers all repos in a GitHub org, applies synchronized file updates or dependency bumps across multiple repos in a single loop, and opens batch PRs with a tracking issue. Use when you need to propagate a change (e.g. Node version, CODEOWNERS, shared config) to 5+ repos at once, or audit org-wide open-PR counts. Produces clone-update-PR shell loops and GitHub API `PUT` calls for no-clone file edits. Pairs with `workflow-engineer` for pipeline-level rollouts. |
| `agents-github:pr-manager` | `agents/pr-manager.md` | Pull request lifecycle specialist. Creates PRs with structured body templates, reviews diff and CI status, selects the right merge strategy (squash/merge/rebase), and coordinates multi-PR batches with label-based filtering. Use when you need to open a PR with a proper checklist, check which PRs are blocked on CI, batch-approve a set of PRs, or enable auto-merge with approval gating. Produces `gh pr create/review/merge` command sequences and merge-strategy recommendations. Pairs with `issue-manager` for issue-to-PR traceability. |
| `agents-github:project-board-manager` | `agents/project-board-manager.md` | GitHub Projects V2 board specialist. Creates projects, configures custom fields (single-select, iteration, number), adds issues and PRs as items, queries item status, updates field values, and archives completed work. Use when you need to set up a sprint board from scratch, bulk-add issues from a milestone, track velocity (Done/Total), or automate end-of-sprint archival. Produces `gh project` command sequences and GraphQL item-edit patterns. Pairs with `issue-manager` for full backlog-to-board flow. |
| `agents-github:release-manager` | `agents/release-manager.md` | Software release coordination specialist. Generates changelogs from merged PRs categorized by label, bumps version files, creates draft GitHub releases with inline or file-based notes, uploads build assets, and publishes. Also handles release branch pattern and hotfix cuts from the latest tag. Use when you need to ship a version end-to-end: plan → changelog → tag → publish → announce. Produces `gh release create/upload/edit` sequences and release note templates. Pairs with `pr-manager` for pre-release PR coordination. |
| `agents-github:repo-architect` | `agents/repo-architect.md` | Repository scaffolding and governance specialist. Creates new GitHub repos from scratch or from templates, configures settings (topics, features, description), scaffolds the full `.github/` directory (PR templates, issue templates, CODEOWNERS, workflows), enforces branch protection rules via API, and manages secrets. Use at project inception to establish a production-grade repo structure, or when onboarding an existing repo to org standards. Produces `gh repo create/edit`, `gh api` branch-protection calls, and ready-to-commit `.github/` file trees. Pairs with `workflow-engineer` for CI/CD layer on top of the scaffold. |
| `agents-github:workflow-engineer` | `agents/workflow-engineer.md` | GitHub Actions CI/CD specialist. Inspects existing workflow runs and logs, diagnoses flaky or failing jobs, creates new workflow YAML files (matrix builds, path-filtered triggers, concurrency groups), and optimizes pipelines with dependency caching, conditional skips, and artifact uploads. Use when CI is broken, you need a new workflow from scratch, or want to reduce pipeline runtime. Produces ready-to-commit `.github/workflows/*.yml` files and `gh run/workflow` command sequences. Pairs with `repo-architect` for initial scaffold and `multi-repo-manager` for org-wide rollouts. |

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-github@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
