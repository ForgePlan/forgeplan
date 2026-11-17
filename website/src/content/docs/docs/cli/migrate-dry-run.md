---
title: forgeplan migrate-dry-run
description: "Report candidate slug collisions directly from selected workspace artifact directories without reading LanceDB. The scan leaves projections unchanged; --output writes a caller-selected JSON report."
---

`migrate-dry-run` is the collision-reporting pre-flight check for the PROB-060 slug migration. It walks per-kind artifact directories under `.forgeplan/`, computes candidate SPEC-005 slugs, and groups collisions. It does not establish that the entire workspace is safe to migrate: review `scan_errors` as well as collisions.

The scan reads Markdown directly and does not use `.forgeplan/lance/**`, so its collision report is independent of the derived index. It does not rewrite artifact projections as part of scanning. The optional `--output` flag does write and overwrite the chosen report path; keep that path outside projections and other files you need to preserve.

## When to use

- Before running the Phase 4 slug migration: a greenlight here is the precondition.
- After importing artifacts from another workspace or an older binary.
- In CI, as a gate that runs before any migration job.
- When you suspect two artifacts were created with the same title.

## When NOT to use

- As the migration itself. `migrate-dry-run` reports; it never renames anything.
- On an empty workspace. It succeeds trivially and tells you nothing.
- When you want one artifact's slug. Use `forgeplan get` on the artifact and read its filename.

## What it does

1. Resolves the workspace: `--workspace` may point at the project root that contains `.forgeplan/` or at the `.forgeplan/` directory itself. Without the flag it walks up from the current working directory.
2. Scans every artifact directory under the workspace for the ten kinds that carry slugs. The Memory kind is excluded - it is outside health, scoring, and lifecycle, so it is not part of slug migration.
3. Reads each file's frontmatter for `title` and `assigned_number`. Legacy artifacts written before SPEC-005 have no `assigned_number` in frontmatter; in that case the number is recovered from the canonical `<KIND>-<NNN>-<slug>.md` filename.
4. Computes the candidate slug for each artifact from its kind prefix and title.
5. Groups records by `kind` and candidate slug, and emits a collision for every group with two or more members. Collisions are sorted by kind then slug, so output ordering is deterministic.
6. With `--auto-suffix`, proposes a resolution per collision: the first member (lexicographic by path) keeps the slug, the rest get `<slug>-<assigned_number>` (or `<slug>-<index>` when the number is missing). Each proposal is validated; a proposal that fails validation surfaces a `validation_error` instead of a `new_slug`.
7. Prints a human summary, optionally a JSON report on stdout and/or to a path, and exits.

A file that fails to parse is recorded in `scan_errors` and the scan continues - one malformed artifact does not hide the rest of the workspace.

## Usage

```text
forgeplan migrate-dry-run [OPTIONS]
```

## Options

```text
      --workspace <WORKSPACE>
          Workspace root containing `.forgeplan/`. Default: current working dir (walk-up search like other commands)

      --auto-suffix
          Include `suggested_resolution` per collision in the JSON output. Without this flag the run defaults to fail-and-list (exit 1)

      --output <OUTPUT>
          Also write the JSON report to this path (the same JSON as `--json`). Optional — caller chooses where to persist evidence

      --json
          Emit JSON to stdout. Default is a human-readable summary table

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

## Output modes

- Default: human-readable summary table. Lists every scanned kind with its artifact count, every scan error (zero if all files parsed), and every collision with its members.
- `--json`: prints the same data as a structured CD-3 JSON document on stdout. Schema version is `1`. Kinds with zero artifacts are omitted. Scan errors use workspace-relative paths.
- `--output <PATH>`: writes the JSON report to a caller-selected path, creating parent directories if necessary. With `--json`, stdout contains the same JSON data; the “Wrote JSON report” message goes to stderr. Without `--json`, stdout remains the human summary.
- `--auto-suffix`: only meaningful with `--json` (or `--output`). Adds a `suggested_resolution` array on each collision, holding one object per member with `path` and either `new_slug` or `validation_error`.

## Exit codes

- `0` - no collisions detected, even if `scan_errors` is nonempty. Do not use the exit code alone as migration approval.
- `1` - collisions found. The JSON `summary.exit_code` mirrors this.
- `2` - workspace resolution or top-level discovery failure, such as a missing `.forgeplan/`. These are plain stderr errors, not a structured JSON error response.

## Examples

All output values below are illustrative fixtures, not results from an executed verification.

### Example 1: Greenlight with one PRD

A workspace with a single, cleanly-titled PRD passes on the first call.

```bash
forgeplan migrate-dry-run
```

```
Forgeplan migration dry-run (PROB-060 / EVID-C)
Workspace: /private/tmp/acme-api/.forgeplan
Scanned: 1 artifacts across 1 kinds

Per-kind counts:
  prd    1

Scan errors: 0

No collisions detected. Greenlight Phase 4.
```

Exit code `0` with no collisions or scan errors in this illustration. Review the actual report and migration prerequisites before proceeding.

### Example 2: Collision when two PRDs share a title

Two PRD files with title `"Payments API"` and different assigned numbers collide on the candidate slug, even though their filenames differ. This example assumes those files already exist; the dry-run does not create them.

```bash
forgeplan migrate-dry-run
```

```
Forgeplan migration dry-run (PROB-060 / EVID-C)
Workspace: /private/tmp/acme-api/.forgeplan
Scanned: 2 artifacts across 1 kinds

Per-kind counts:
  prd    2

Scan errors: 0

Collisions: 1 (across kinds: prd)
  [prd] prd-payments-api (2 members)
    - #1 /private/tmp/acme-api/.forgeplan/prds/PRD-001-payments-api.md "Payments API"
    - #2 /private/tmp/acme-api/.forgeplan/prds/PRD-002-payments-api.md "Payments API"

Run with --auto-suffix --json to emit suggested resolution slugs.
```

Exit code `1`. Resolve by merging, deleting, or renaming one of the artifacts before retrying the migration.

### Example 3: JSON report for a CI gate

Pipe the JSON report into `jq` to count collisions per kind:

```bash
forgeplan migrate-dry-run --json
```

```
{
  "kinds": {
    "prd": {
      "collisions": [
        {
          "artifacts": [
            {
              "assigned_number": 1,
              "path": "/private/tmp/acme-api/.forgeplan/prds/PRD-001-payments-api.md",
              "title": "Payments API"
            },
            {
              "assigned_number": 2,
              "path": "/private/tmp/acme-api/.forgeplan/prds/PRD-002-payments-api.md",
              "title": "Payments API"
            }
          ],
          "count": 2,
          "slug": "prd-payments-api"
        }
      ],
      "count": 2
    }
  },
  "scan_errors": [],
  "scanned_at": "2026-09-16T13:52:57Z",
  "schema_version": 1,
  "summary": {
    "exit_code": 1,
    "kinds_with_collisions": [
      "prd"
    ],
    "total_collisions": 1
  },
  "total_artifacts": 2,
  "workspace": "/private/tmp/acme-api/.forgeplan"
}
```

The exit code matches `summary.exit_code`. A CI gate should require both `summary.total_collisions == 0` and `scan_errors` to be empty; parse/slug errors alone do not make this command exit nonzero.

### Example 4: Auto-suffix resolution proposal

When `--auto-suffix` is set, each collision gains a `suggested_resolution` array. The first member (lex-sorted by path) keeps the slug; the rest get `<slug>-<assigned_number>`. Pipe through Python for a compact summary:

```bash
forgeplan migrate-dry-run --json --auto-suffix \
  | python3 -c 'import json,sys; d=json.load(sys.stdin); \
      print(json.dumps(d["kinds"]["prd"]["collisions"][0]["suggested_resolution"], indent=2))'
```

```
[
  {
    "new_slug": "prd-payments-api",
    "path": "/private/tmp/acme-api/.forgeplan/prds/PRD-001-payments-api.md"
  },
  {
    "new_slug": "prd-payments-api-2",
    "path": "/private/tmp/acme-api/.forgeplan/prds/PRD-002-payments-api.md"
  }
]
```

Proposals are validated by the slug validator; a failure shows up as `validation_error` rather than `new_slug`, with the rejected candidate kept under `candidate`.

### Example 5: Persisting the report to disk

Use `--output` to drop the same JSON into a file alongside the stdout stream, so a CI job can attach the evidence.

```bash
forgeplan migrate-dry-run --json --output .forgeplan-ci-report.json
```

Illustrative stdout for an empty workspace:

```json
{
  "schema_version": 1,
  "scanned_at": "2026-10-23T12:00:00Z",
  "workspace": "/private/tmp/acme-api/.forgeplan",
  "total_artifacts": 0,
  "kinds": {},
  "scan_errors": [],
  "summary": {
    "total_collisions": 0,
    "kinds_with_collisions": [],
    "exit_code": 0
  }
}
```

Stderr separately prints `Wrote JSON report to .forgeplan-ci-report.json`.

The returned summary's exit code reflects collisions, not per-file scan errors. A report write failure is an ordinary command error. An existing output file is overwritten.

### Example 6: Bad workspace path

A missing path or a directory without `.forgeplan/` exits with code `2` and a plain stderr error (also with `--json`).

```bash
forgeplan migrate-dry-run --workspace /tmp/empty
```

```
Error: migrate-dry-run: no .forgeplan/ directory found at /tmp/empty
```

Exit code `2`. Fix the path or run `forgeplan init` to create the workspace.

## Notes

- The scan does not write artifact files or read LanceDB. `--output` does write the chosen report path, so choose a path outside artifact projections. This page's review did not execute the command or compare file hashes.
- The collision key is `kind` plus the computed slug, not the filename. Two files with different filenames but identical kind + title still collide.
- Memory artifacts are excluded by design (they are not part of lifecycle or scoring, so they do not participate in slug migration).
- Per-file parse failures land in `scan_errors` and never abort the scan. The rest of the workspace is still reported.
- A title that cannot produce a valid slug is excluded from collision grouping and recorded in `scan_errors` by this command; do not wait for migration to discover it.
- `scan_errors[*].path` is workspace-relative in JSON, with a basename fallback outside the workspace. The `reason` string can still contain an absolute path, as can workspace and collision-member fields; inspect reports before publishing them.
- Determinism: collisions sort by kind then slug; members within a collision sort lexicographically by path. Two identical runs produce identical JSON modulo `scanned_at`.

## Related commands

- [`forgeplan migrate`](/docs/cli/migrate/) - schema migration command; a dry-run collision report is not proof that a specific migration is applicable or that all schema prerequisites passed.
- [`forgeplan reconcile-ids`](/docs/cli/reconcile-ids/) - re-numbers artifacts to close ID gaps; a different, more invasive rewriter.
- [`forgeplan reindex`](/docs/cli/reindex/) - scan projections after a rewrite: import new rows, sync existing bodies, add missing links, and clean up orphans. It is not a complete metadata or removed-link refresh for existing records.
- [`forgeplan health`](/docs/cli/health/) - session-start dashboard that surfaces stale and at-risk artifacts.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan migrate`](/docs/cli/migrate/) - schema migration reference
- [`forgeplan reconcile-ids`](/docs/cli/reconcile-ids/) - re-numbering companion
- [`forgeplan health`](/docs/cli/health/) - session-start health check
- [Methodology guide](/docs/methodology/overview/)
