---
title: forgeplan drift
description: "Flag active ADR/RFC affected-file paths with commits since artifact creation. This requests review; it does not prove the code contradicts the decision."
---

`forgeplan drift` checks active ADR and RFC artifacts with an `Affected Files` section. It asks Git whether each path or pattern has a commit since the artifact's stored `created_at` timestamp, not its activation or decision date. Any match marks the report stale; the command does not judge whether the change contradicts the decision.

This differs from evidence decay (`valid_until`). Drift examines committed file history, not uncommitted edits. Git execution failures are treated as no changes, so a clean report alone is not proof that the decision is current.

## When to use

- Monthly architecture review - which ADRs need review after code changes?
- After a big refactor - see which decisions need to be re-validated against the new shape of the code.
- Before citing an ADR in a PR review - is it still describing reality?
- Release prep - refresh drifted decisions before cutting the release.

## When NOT to use

- On artifacts without an `Affected Files` section - use `coverage --backfill` first.
- On a fresh workspace - drift needs git history to compute against.

## Usage

```text
forgeplan drift [OPTIONS]
```

## Options

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Find drifted decisions

```bash
forgeplan drift
```

The text view prints each checked artifact as `STALE` or `up to date`. A stale row includes `Created:` and `Changed: path (last commit date)` lines. The summary gives decisions checked and stale count. It does not count commits or distinguish heavy and minor drift.

### Machine-readable for CI

```bash
forgeplan drift --json | jq '.reports[] | select(.is_stale) | .id'
```

Feed that list into a Slack reminder or a backlog task generator.

### Remediate drift

For each drifted ADR you have three moves:

1. **Re-verify**: read the diffs and document whether the decision still holds. Renewal changes validity and status, not the creation timestamp used here, so it does not clear this drift signal.
2. **Supersede**: create a replacement ADR, then run `forgeplan supersede ADR-OLD --by ADR-NEW`. The old artifact must be active and the replacement must exist.
3. **Deprecate**: if the decision no longer applies, use `forgeplan deprecate ADR-OLD --reason "No longer applicable"`.

## Output interpretation

JSON contains `reports`, `_next_action`, and `hints`. Each report has `id`, `title`, `is_stale`, `created_at`, and `changed_files` (objects with `path` and `last_modified`). Both stale and unchanged reports are returned. Findings do not cause a nonzero exit by themselves; apply your own policy to `.reports[].is_stale`.

## How it fits the workflow

```
scan → coverage → drift → remediate (renew | supersede | deprecate)
```

`drift` is one leg of the codebase ⟷ artifacts bridge. `coverage` asks "do modules
have ADRs?" and `drift` asks "do those ADRs still match the code?"

## See also

- [`forgeplan coverage`](/docs/cli/coverage/) - module-level decision coverage
- [`forgeplan scan`](/docs/cli/scan/) - populate the module list coverage uses
- [`forgeplan supersede`](/docs/cli/supersede/) - retire a drifted decision in favour of a new one
- [`forgeplan renew`](/docs/cli/renew/) - keep a still-valid decision alive
- [CLI overview](/docs/cli/)
