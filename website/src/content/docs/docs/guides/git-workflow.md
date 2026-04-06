---
title: Git Workflow
description: Branching strategy and commit conventions for Forgeplan projects
---

## Branching Strategy

```
main                    ← production (tagged releases)
  │
dev                     ← integration branch
  ├── feat/prd-018-dag  ← feature branch
  ├── fix/search-bug    ← bugfix branch
  └── docs/rfc-002      ← docs-only branch
```

## Branch Rules

| Branch | Created from | Merges into | Strategy |
|--------|-------------|-------------|----------|
| `feat/*`, `fix/*` | **dev** | **dev** | Merge commit via PR |
| `release/v0.x.0` | **dev** | **main** + **dev** | Merge commit |
| `hotfix/*` | **main** | **main** + **dev** | Cherry-pick |

## Commit Format

```
<type>(<scope>): <description>

[body — what and why]

Refs: RFC-001, FR-001..004
```

### Types

| Type | When |
|------|------|
| `feat` | New functionality |
| `fix` | Bug fix |
| `docs` | Documentation / artifacts |
| `refactor` | No behavior change |
| `test` | Tests only |
| `chore` | Build, deps, CI |

## PR Pipeline

```
Code → Audit → Fix → Test → Fmt → Lint → PR
```

**Never create PR immediately after code.** PR means: "I tested, audited, formatted, and everything works."

## Before Every Commit

```bash
cargo fmt           # format
cargo fmt -- --check  # verify: 0 diffs
cargo check         # compile: 0 warnings
cargo test          # all pass
```
