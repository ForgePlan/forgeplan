---
title: forgeplan reconcile-ids
description: "PROB-060 Phase 2.4 (W2.C) - manual cleanup tool for post-merge identity drift. Scans `.forgeplan/<kind>/*.md`, detects four drift categories (filename mismatch, missing `predicted_number`, body-links drift, duplicate `assigned_number`), and either reports (`--check-only`) or auto-fixes the safe categories. LanceDB is never touched; run `forgeplan reindex` afterwards if the index needs to be rebuilt"
---

Clean up artifact identity drift after a merge. Drift appears whenever an
artifact is touched outside the canonical MCP/CLI path, or when a Phase 1.x
migration left a half-finished state: the filename no longer agrees with the
display id, `predicted_number` was never carried over, body links and
frontmatter links disagree, or two artifacts claim the same number.

This is the manual counterpart to [`forgeplan ci-assign-id`](/docs/cli/ci-assign-id/):
the assigner mints numbers in CI, `reconcile-ids` repairs a workspace that
got its identities out of step.

## Forgeplan 0.38.0

`--report-cross-pr` only adds cross_pr_deferred; it does not inspect sibling PRs. Once a report is produced, exits 0/1 depend on unresolved actions, not the scan_errors array. Per-file errors can therefore coexist with summary.unresolved=false and exit 0. Use --check-only in CI and inspect scan_errors as well; a successful exit alone does not prove a complete scan.

## Usage

```text
forgeplan reconcile-ids [OPTIONS]
```

## Options

```text
      --workspace <WORKSPACE>  Workspace root containing `.forgeplan/`. Default: walk-up search from cwd. May also be a `.forgeplan/` directory directly
      --check-only             Report inconsistencies without modifying files. Default is apply mode (auto-rename + auto-fill `predicted_number`)
      --report-cross-pr        Forward-compat flag for RFC-009 §Phase 2.4 cross-PR ref-drift detection. The current implementation surfaces a no-op marker in the JSON output; cross-PR walking is deferred (see module docs)
      --json                   Emit JSON report to stdout (schema_version 1). Default is a scannable human-readable summary
  -h, --help                   Print help
  -V, --version                Print version
```

## What it does

1. Resolves the workspace - `--workspace` if given (either the repo root or
   the `.forgeplan/` directory itself), otherwise a walk-up search from the
   current directory.
2. Scans `.forgeplan/<kind_dir>/*.md` across the ten identity-bearing kinds
   (PRD, RFC, ADR, Epic, Spec, Problem, Solution, Evidence, Note, Refresh).
3. Parses each file's frontmatter and filename, and runs the four drift
   detectors.
4. In `--check-only` mode: reports every finding and writes nothing.
5. In apply mode (the default): renames the files and fills
   `predicted_number`, through the atomic frontmatter helpers and
   `git mv` / `fs::rename`.
6. Emits the summary and sets the exit code.

LanceDB is never touched (ADR-003). If the store needs to follow along,
run [`forgeplan reindex`](/docs/cli/reindex/) afterwards.

## The four drift categories

| Category | Detection | Apply mode |
|---|---|---|
| `filename_mismatch` | Artifact has `assigned_number: N` but the filename is not `<KIND>-<NNN>-<slug-suffix>.md` | Renames to the canonical pattern (`git mv` when tracked, `fs::rename` otherwise) |
| `missing_predicted` | Artifact has a `slug` but no `predicted_number` field | Fills `predicted_number = assigned_number`, falling back to `1` when no number is set |
| `body_links_drift` | A `## Related` or `## Related Artifacts` table in the body names artifact ids that are absent from frontmatter `links:` | **Report only** - never auto-edited |
| `duplicate_assigned` | Two or more artifacts of the same kind share one `assigned_number` | **Manual review required** - auto-fixing risks silent data loss |

`body_links_drift` is deliberately not fixed here. Mutating `links:` behind
the relation graph's back would violate red-line #11, so the report tells you
to use [`forgeplan link`](/docs/cli/link/) instead.

For a duplicate group, one action is emitted per group (not per member); the
other members are listed under `current_state.members` in the JSON payload.

## `--check-only` vs apply

Without `--check-only` the command mutates the workspace. The safe two
categories are fixed; the two unsafe ones are reported and left alone, so a
run can legitimately end with **both** `[applied]` and `[not applied]` lines.

Because apply mode renames files, expect the reported path of a
`filename_mismatch` action to be the **post-rename** name while a
`body_links_drift` action on the same artifact still shows the pre-rename
name it was scanned under.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Workspace coherent, or apply mode fixed everything auto-fixable |
| `1` | Drift found in `--check-only` mode, or unfixable categories (`duplicate_assigned`, `body_links_drift`) were reported |
| `2` | Workspace error - no `.forgeplan/` found, or the scan failed |

## Examples

### Example 1: a coherent workspace

```bash
forgeplan reconcile-ids --check-only
```

Typical output:

```text
Forgeplan reconcile-ids (PROB-060 / RFC-009 §Phase 2.4)
Workspace: .forgeplan
Mode: check-only
Per-kind counts: 0 kinds, 0 artifacts total

No drift detected. Workspace is coherent.
```

Exit code `0`.

### Example 2: report-only mode on a drifted workspace

```bash
forgeplan reconcile-ids --check-only
```

Typical output:

```text
Forgeplan reconcile-ids (PROB-060 / RFC-009 §Phase 2.4)
Workspace: .forgeplan
Mode: check-only
Per-kind counts: 1 kinds, 3 artifacts total

Actions: 6
  [not applied] duplicate_assigned PRD-007 prds/PRD-007-alpha.md
  [pending] filename_mismatch PRD-005 prds/prd-widget.md
  [pending] missing_predicted PRD-005 prds/prd-widget.md
  [pending] missing_predicted PRD-007 prds/PRD-007-alpha.md
  [pending] missing_predicted PRD-007 prds/PRD-007-beta.md
  [not applied] body_links_drift PRD-005 prds/prd-widget.md

Unresolved drift remains. Manual review needed for duplicate_assigned / body_links_drift; rerun without --check-only to apply auto-fixable categories.
```

Nothing on disk changed; the exit code is `1` because drift was found.

### Example 3: apply mode fixes the safe categories

```bash
forgeplan reconcile-ids
```

Typical output:

```text
Forgeplan reconcile-ids (PROB-060 / RFC-009 §Phase 2.4)
Workspace: .forgeplan
Mode: apply
Per-kind counts: 1 kinds, 3 artifacts total

Actions: 6
  [not applied] duplicate_assigned PRD-007 prds/PRD-007-alpha.md
  [applied] filename_mismatch PRD-005 prds/PRD-005-widget.md
  [applied] missing_predicted PRD-005 prds/PRD-005-widget.md
  [applied] missing_predicted PRD-007 prds/PRD-007-alpha.md
  [applied] missing_predicted PRD-007 prds/PRD-007-beta.md
  [not applied] body_links_drift PRD-005 prds/prd-widget.md

Unresolved drift remains. Manual review needed for duplicate_assigned / body_links_drift; rerun without --check-only to apply auto-fixable categories.
```

On disk, `prds/prd-widget.md` became `prds/PRD-005-widget.md` and gained
`predicted_number: 5`. The exit code is still `1`, because the two unfixable
findings remain.

Re-running after the apply drops the four resolved actions and leaves only
the pair that needs a human (header and trailing advisory lines omitted):

```bash
forgeplan reconcile-ids --check-only
```

Typical output:

```text
Actions: 2
  [not applied] duplicate_assigned PRD-007 prds/PRD-007-alpha.md
  [not applied] body_links_drift PRD-005 prds/PRD-005-widget.md
```

### Example 4: JSON report

```bash
forgeplan reconcile-ids --check-only --json --report-cross-pr
```

```json
{
  "actions": [
    {
      "applied": false,
      "artifact_id": "PRD-007",
      "artifact_path": "prds/PRD-007-alpha.md",
      "category": "duplicate_assigned",
      "current_state": {
        "assigned_number": 7,
        "kind": "prd",
        "members": ["prds/PRD-007-alpha.md", "prds/PRD-007-beta.md"]
      },
      "suggested_fix": {
        "action": "manual_review_required",
        "note": "Auto-fix is disabled for duplicate_assigned — human must decide which artifact retains the number and which one is renumbered or deprecated."
      }
    },
    {
      "applied": false,
      "artifact_id": "PRD-005",
      "artifact_path": "prds/PRD-005-widget.md",
      "category": "body_links_drift",
      "current_state": { "body_refs_not_in_links": ["PRD-009"] },
      "suggested_fix": {
        "action": "report_only",
        "note": "Use `forgeplan link <source> <target> --relation <r>` to update frontmatter links — direct edits violate red-line #11."
      }
    },
    {
      "applied": true,
      "artifact_id": "<workspace>",
      "artifact_path": "",
      "category": "cross_pr_deferred",
      "current_state": { "flag": "--report-cross-pr" },
      "suggested_fix": {
        "action": "deferred",
        "note": "Cross-PR Refs: drift detection is deferred — see RFC-009 §Phase 2.4. Workspace-only categories are covered above."
      }
    }
  ],
  "check_only": true,
  "scan_errors": [],
  "schema_version": 1,
  "summary": { "total_actions": 3, "unresolved": true },
  "timestamp": "2026-09-16T13:51:44Z",
  "workspace": ".forgeplan"
}
```

(For brevity, the `members` and `body_refs_not_in_links` arrays are shown on
one line each; the rest of the payload is verbatim.)

`--report-cross-pr` does not walk sibling branches yet. It appends a single
informational `cross_pr_deferred` action so a JSON consumer can tell the
no-op apart from a genuine absence of findings.

### Example 5: no workspace at all

```bash
forgeplan reconcile-ids
```

Typical output:

```text
Error: No .forgeplan/ workspace found from /private/tmp/agent-recon-none
```

Exit code `2`.

## Notes

- **Apply mode is the default.** There is no `--apply` flag; you opt out with
  `--check-only`. Wire the checker form into CI and the bare form into a
  cleanup PR.
- When a report is produced, JSON uses schema_version 1 and summary.unresolved describes unresolved actions. Fatal workspace/discovery failures return 2 on stderr without a JSON report.
- Renames use `git mv` when the file is tracked, so history survives the
  post-merge squash; `fs::rename` is the fallback outside a repository.
- The scanner never rewrites artifact bodies - only frontmatter through the
  canonical helpers.
- A workspace-relative form of the path is used in the report
  (`prds/PRD-007-alpha.md`), so the output is safe to paste into a CI log.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan ci-assign-id`](/docs/cli/ci-assign-id/) - the CI assigner whose output this command repairs
- [`forgeplan reindex`](/docs/cli/reindex/) - rebuild the store after a rename
- [`forgeplan link`](/docs/cli/link/) - the canonical way to fix a body-links drift finding
- [`forgeplan migrate-dry-run`](/docs/cli/migrate-dry-run/) - preview an identity migration before running it
