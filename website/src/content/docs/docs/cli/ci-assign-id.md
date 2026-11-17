---
title: forgeplan ci-assign-id
description: "Assign numbers from the working tree and --base; update frontmatter and filenames without LanceDB."
---

Mint the numeric half of artifact identity (`assigned_number`) from candidates in the working tree and the per-kind maximum in `--base`. The binary numbers files; the wrapping workflow commits and pushes. It is not a global reservation service: two runs against the same unchanged base can choose the same next number.

The write path is deliberately narrow. `ci-assign-id` never touches LanceDB
(ADR-003 red line #8), never reads the `change_log` table (PROB-061
isolation), performs no network calls, and never runs `git commit` or
`git push` - the wrapping workflow does that.

## Forgeplan 0.38.0

The 0.38.0 help still says “no file rename”, but runtime renames files after numbering. These are separate filesystem steps, not one atomic batch transaction. Collisions are skipped with a suggestion; --auto-suffix does not apply a suffix even though it changes the final exit to 0. The shipped assign-id.yml dry-run branch uses unsupported --dry-run-only: change it to --dry-run when adapting that workflow. Review the plan, actions, and Git diff before committing; refresh the index with reindex. Fixture captures below are retained historical examples, not results of a current run.

## Usage

```text
forgeplan ci-assign-id [OPTIONS]
```

## Options

```text
      --pr <PR>                PR number (informational, used in commit message). Required in CI [default: 0]
      --repo <REPO>            Repo slug "owner/name" (informational). Default: detect from origin
      --base <BASE>            Git ref for "destination" state for max(assigned_number) lookup [default: origin/dev]
      --head <HEAD>            Git ref for "incoming" PR state [default: HEAD]
      --workspace <WORKSPACE>  Workspace root. Default: cwd
      --dry-run                Do not write frontmatter; print what would change
      --auto-suffix            On slug collision (slug already exists on --base), suggest `<slug>-<assigned_number>` rename. Phase 0b: warning only
      --json                   Emit machine-readable JSON to stdout
  -h, --help                   Print help
  -V, --version                Print version
```

## What it does

1. Walks the working tree under `.forgeplan/<kind_dir>/` for `.md` files that
   carry both a `slug:` and `assigned_number: null`, across the ten
   identity-bearing kinds (PRD, RFC, ADR, Epic, Spec, Problem, Solution,
   Evidence, Note, Refresh); memories are excluded because they do not carry
   an `assigned_number`.
2. Validates every slug. A slug must start with its kind prefix (`prd-`,
   `rfc-`, `adr-`, `evid-`, `prob-`, and so on, followed by `-`).
3. For each kind, looks up `max(assigned_number)` **in the `--base` git ref**
   - git-native, no store reads - and mints sequential numbers from `max+1`
   in a deterministic order.
4. Detects slug collisions against `--base` and records them.
5. Rewrites `assigned_number` in the frontmatter through the atomic
   `set_assigned_number` helper (temp file + POSIX rename).
6. Renames the file so the on-disk name agrees with the freshly minted
   display id, and emits either the human summary or the `--json` payload.

## The concurrency contract

Numbering is only safe if two runs cannot interleave. That guarantee does
**not** come from the binary - it comes from the workflow that wraps it.
`.github/workflows/assign-id.yml` declares:

```yaml
concurrency:
  group: forgeplan-id-assign
  cancel-in-progress: false
```

- The **group** serializes every run in the repository, so a second PR waits
  while the first PR's numbering run is in flight.
- **`cancel-in-progress: false`** prevents cancellation of the running job, but does not guarantee that every pending run is retained. Serialization does not reserve numbers: the next assignment must see a base containing previously accepted assignments. Review merge/queue integration and rerun numbering when needed.
- The job only fires on `pull_request: [labeled]` with the `ready-to-merge`
  label on a non-draft PR targeting `dev`, or on `workflow_dispatch`. The
  label gate is deliberate - the workflow compiles PR-controlled code with
  `GITHUB_TOKEN` active.
- The workflow fetches the base with
  `git fetch origin dev:refs/remotes/origin/dev --no-tags --depth=200`, then
  commits the result as `forgeplan-bot` and pushes it back to the PR head
  branch.

Run the binary **outside** that group (locally, or in a parallel job) and you
own the race yourself.

## `--base` and `--head`

- `--base` (default `origin/dev`) is the **destination** state. It is the
  only place `max(assigned_number)` is read from, and it is also what
  collision detection compares against. In CI it is always passed
  explicitly as `origin/dev`.
- `--head` (default `HEAD`) is validated and echoed into the report as the
  "incoming" side. Candidates themselves are discovered from the working
  tree, so a modified-but-uncommitted artifact is fully visible to a local
  run.
- Both refs go through `validate_git_ref`, which rejects a leading `-`, a
  `..` range operator, a non-alphanumeric first character, and any character
  outside `[A-Za-z0-9_/.-]`.

## Frontmatter and filenames

A real (non-dry) run rewrites `assigned_number` **and** renames the file
from the placeholder shape to the display-id shape:

```text
.forgeplan/prds/prd-widget-auth.md
  -> .forgeplan/prds/PRD-001-widget-auth.md
```

The rename uses `git mv` when the file is tracked (preserving blame through
the post-merge squash) and falls back to `fs::rename` outside a repository.
It is idempotent: if the name already matches, no rename is attempted.

The generated help text still carries the Phase 0b phrasing "no file rename -
Phase 2.1", but the Phase 2.2 rename is what the released 0.38.0 implementation does. The JSON
`action` field distinguishes the cases:

| `action` | Meaning |
|---|---|
| `would_assign` | `--dry-run` preview; nothing was written |
| `assigned` | Frontmatter rewritten, no rename needed (filename was already canonical) |
| `renamed` | File renamed only - recovers a run that rewrote frontmatter then failed before renaming |
| `renamed_and_assigned` | Both happened this run; the common fresh-assignment path |
| `skipped_already_assigned` | Idempotent no-op on a fully numbered, correctly named artifact |

`--auto-suffix` does **not** rename anything. On a slug collision it only
downgrades the outcome: the collision is still listed, but the process exits
`0` instead of `1`, so a workflow does not fail on a known collision.

## Examples

### Example 1: preview without writing

```bash
forgeplan ci-assign-id --base origin/dev --head HEAD --dry-run --pr 1234
```

Typical output:

```text
ci-assign-id (PR #1234, base=origin/dev, head=HEAD, dry-run)
  [would_assign] PRD-001 (prd-widget-auth): /private/tmp/agent-ciid2/.forgeplan/prds/prd-widget-auth.md
Summary: 1 candidates, 1 assigned, 0 skipped, 0 collisions (exit 0)
```

### Example 2: the real run, which also renames

```bash
forgeplan ci-assign-id --base origin/dev --head HEAD --pr 1234
```

Typical output:

```text
ci-assign-id (PR #1234, base=origin/dev, head=HEAD)
  [renamed_and_assigned] PRD-001 (prd-widget-auth): /private/tmp/agent-ciid2/.forgeplan/prds/PRD-001-widget-auth.md
Summary: 1 candidates, 0 assigned, 0 skipped, 0 collisions (exit 0)
```

The frontmatter now reads `assigned_number: 1`, and the old filename is gone.

### Example 3: idempotent re-run

```bash
forgeplan ci-assign-id --base origin/dev --head HEAD --pr 1234
```

Typical output:

```text
ci-assign-id (PR #1234, base=origin/dev, head=HEAD)
  [skipped_already_assigned] PRD-001 (prd-widget-auth): /private/tmp/agent-ciid2/.forgeplan/prds/PRD-001-widget-auth.md
Summary: 1 candidates, 0 assigned, 0 skipped, 0 collisions (exit 0)
```

### Example 4: the JSON payload

```bash
forgeplan ci-assign-id --base origin/dev --head HEAD --pr 77 --json
```

```json
{
  "schema_version": 1,
  "ran_at": "2026-09-16T13:51:28.043154+00:00",
  "pr": 77,
  "repo": "",
  "base": "origin/dev",
  "head": "HEAD",
  "dry_run": false,
  "assignments": [
    {
      "slug": "prd-widget-auth",
      "kind": "prd",
      "path": "…/PRD-001-widget-auth.md",
      "predicted_number": null,
      "assigned_number": 1,
      "max_in_base": null,
      "action": "skipped_already_assigned"
    },
    {
      "slug": "prd-third",
      "kind": "prd",
      "path": "…/PRD-003-third.md",
      "predicted_number": null,
      "assigned_number": 3,
      "max_in_base": null,
      "action": "skipped_already_assigned"
    },
    {
      "slug": "prd-fourth",
      "kind": "prd",
      "path": "…/PRD-004-fourth.md",
      "predicted_number": null,
      "assigned_number": 4,
      "max_in_base": null,
      "action": "assigned"
    }
  ],
  "collisions": [],
  "summary": {
    "total_candidates": 4,
    "assigned": 1,
    "skipped_already_assigned": 3,
    "collisions": 0,
    "exit_code": 0
  },
  "commit_message_suggested": "chore(ci): assign artifact IDs for PR #77\n\nAssigned: PRD-004 (prd-fourth)\n\nRefs: PROB-060, PRD-076, RFC-009 §Phase 0b"
}
```

These retained fixture captures abbreviate paths to `…`. Current reporting redacts in-workspace paths to workspace-relative form; do not parse the historical absolute paths as a contract.

`commit_message_suggested` is built only from assignments whose action is
`assigned` or `would_assign`, so a run that only renamed or skipped leaves it
empty. The workflow reads this field out of the JSON and feeds it to
`git commit -m`.

### Example 5: a slug collision

When the slug already exists on `--base`, the run reports the collision on
stderr and in the summary, and exits `1`:

```bash
forgeplan ci-assign-id --base origin/dev --head HEAD
```

Typical output:

```text
warning: slug collision: prd-widget-auth (prd) collides with .forgeplan/prds/prd-widget-auth.md; suggested: prd-widget-auth-1
ci-assign-id (PR #0, base=origin/dev, head=HEAD)
Collisions:
  prd-widget-auth (prd) ↔ .forgeplan/prds/prd-widget-auth.md; suggested: prd-widget-auth-1
Summary: 1 candidates, 0 assigned, 0 skipped, 1 collisions (exit 1)
```

Adding `--auto-suffix` keeps the same report but returns `0`. Nothing is
renamed: the file was still `prd-widget-auth.md` with `assigned_number: null`
after the run.

### Example 6: a malformed slug stops the run

```bash
forgeplan ci-assign-id --base origin/dev --head HEAD
```

Typical output:

```text
Error: discovering candidates under /private/tmp/agent-ciid
Caused by:
    ci-assign-id: malformed slug "widget-auth" in .forgeplan/prds/prd-widget-auth.md: invalid slug: must start with one of ["prd", "rfc", "adr", "epic", "spec", "prob", "sol", "evid", "note", "ref", "mem", "uc", "glos", "inv", "scen", "hyp", "dm"] followed by `-`, got "widget-auth"
```

## Notes

- **`--dry-run` is the only dry-run flag.** `--dry-run-only` is not
  accepted: the binary exits with
  `error: unexpected argument '--dry-run-only' found` and
  `tip: a similar argument exists: '--dry-run'`.
- Paths in error messages are redacted to workspace-relative form when the
  file lives inside the workspace (CWE-200: CI layout stays out of logs).
- A frontmatter parse failure prints
  `ci-assign-id: skipping <path>: frontmatter parse failed` and continues
  with the remaining candidates, so inspect warnings rather than treating a completed run as proof that every file parsed.
- Exit status: `0` for a completed plan (also unresolved collisions when `--auto-suffix` is set); `1` for collisions without that flag or ordinary command errors; `2` when no candidates are found; `3` for invalid Git refs; `4` for the assignment invariant violation. Inspect the JSON collision list, not only exit 0.
- `--pr` and `--repo` are informational only. `--pr` defaults to `0`; when
  `--repo` is omitted the slug is detected from `git remote get-url origin`.
- Because the binary never touches the index, run
  [`forgeplan reindex`](/docs/cli/reindex/) afterwards if a long-lived
  workspace needs its store refreshed.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan reconcile-ids`](/docs/cli/reconcile-ids/) - clean up the drift a bypassed assigner leaves behind
- [`forgeplan reindex`](/docs/cli/reindex/) - rebuild the index after an out-of-band rename
- [`forgeplan migrate-dry-run`](/docs/cli/migrate-dry-run/) - the identity-family diagnostic that previews a migration before it runs
- [Methodology guide](/docs/methodology/overview/)
