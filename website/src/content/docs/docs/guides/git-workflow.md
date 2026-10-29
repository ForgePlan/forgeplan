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
| `release/v0.x.0` | **dev** | **main**, then a separate main → dev sync PR | Merge commit for release |
| `hotfix/*` | **main** | **main**, then a backport PR to **dev** | Cherry-pick or merge for backport |

## Commit Format

```
<type>(<scope>): <description>

[body - what and why]

Refs: RFC-001, FR-001..004
```

Use the stable artifact slug in `Refs:` before merge-time number assignment;
use an assigned display ID such as `RFC-001` only after assignment.

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
Code → Audit → Fix → Test → Fmt → Lint → Verify → PR
```

**Never create a PR immediately after code.** Record the checks actually run,
including real end-to-end verification of every affected surface. A passing
unit suite is not proof that every workflow works.

## Before Every Commit

```bash
cargo fmt           # format
cargo fmt -- --check  # verify: 0 diffs
cargo check         # compile: 0 warnings
cargo test          # all pass
```

These are Rust contributor checks, not commands required in every project
using Forgeplan. In parallel work, one assigned build owner runs checks after
source changes finish; other agents do not start competing Cargo processes.
