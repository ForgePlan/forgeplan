---
title: forgeplan gaps
description: "Report PRD depth requirements, missing or weak decision evidence, old drafts, and unlinked artifacts without changing lifecycle state."
---

`forgeplan gaps` reports depth-based PRD requirements, active decisions without
current evidence, old drafts, and unlinked artifacts. Findings carry `MUST`,
`SHOULD`, or `COULD` severity and are printed in that order.

This is a read-only diagnostic, not a lifecycle transition or an activation
guard. A reported `MUST` is the gap's severity; the command still exits
successfully after printing findings.

## When to use

- Mid-sprint: "I said this was Deep, did I actually build out Spec + ADR?"
- After reviewing a depth recommendation from `calibrate` - inspect requirements for the depth actually stored on each PRD.
- Pre-release health pass: "is the project aligned with its own methodology?"
- Brownfield cleanup: imported artifacts often have depth without downstream chain.

## When NOT to use

- To check body/schema completeness alone - use `validate`. Tactical artifacts can still have evidence, old-draft, or orphan findings.
- As a blocker for activation - gaps are advisory, `validate` is the gate.

## Usage

```text
forgeplan gaps
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### See all pipeline gaps

```bash
forgeplan gaps
```

Illustrative finding fragment (invented artifact values, not an executed run):

```text
  MUST:
    ! PRD-004 "Search Intelligence"
      PRD-004 Deep depth but no linked RFC

  SHOULD:
    ~ PRD-004 "Search Intelligence"
      PRD-004 Deep depth but no linked Spec
```

The complete text report also has a severity-count summary and a follow-up hint.

### Fix gaps

For each line:

1. **Missing RFC/Spec/ADR** → create the genuinely needed artifact, fill it, and link it to the PRD. For example, after `new rfc`, use its returned ID in `forgeplan link RFC-018 PRD-004 --relation refines`.
2. **Active decision without evidence** → collect real verification and link a current EvidencePack; a generated analysis is not a test result.
3. **Critical PRD without parent Epic** → check whether the scope belongs under an Epic and set the actual parent metadata.
4. **Old draft or orphan** → review relevance and missing relationships rather than creating meaningless links just to silence the report.

## Output interpretation

| Implemented PRD depth check | Severity when missing |
|----------------------------|-----------------------|
| Tactical                   | no depth-specific requirement |
| Standard                   | RFC: MUST |
| Deep                       | RFC: MUST; Spec and ADR: SHOULD |
| Critical                   | `parent_epic`, RFC, ADR: MUST |

These checks skip deprecated/superseded PRDs. Linked kinds are detected in either relation direction, regardless of relation type; the command does not validate a full ordered chain or require several RFCs under an Epic. Critical does not currently check for a Spec.

Other checks cover active decision kinds with no non-terminal evidence (MUST), weak evidence below 0.1 using the flat evidence helper (SHOULD), drafts at least 90 days old (COULD), and artifacts with no relations at all (COULD; Notes and Problems are exempt). Evidence and Refresh artifacts are skipped. The weak-evidence helper is not the recursive scoring contract of `score`.

## How it fits the workflow

```
review depth → gaps → resolve real omissions → validate → review → activate
```

`calibrate` recommends depth; it does not rewrite it. `gaps` reads stored depth and links. If review shows the depth is genuinely wrong, use `forgeplan update PRD-004 --depth standard` with the actual ID and intended depth. This updates stored depth and its projection; editing frontmatter and running `reindex` does not refresh depth on an existing indexed record in 0.38.0. Do not downgrade depth merely to hide missing work.

## See also

- [`forgeplan calibrate`](/docs/cli/calibrate/) - recalibrate depth when gaps appear
- [`forgeplan health`](/docs/cli/health/) - project-level aggregate
- [`forgeplan blocked`](/docs/cli/blocked/) - dependency graph view
- [Artifact Model](/docs/methodology/overview/) - depth-to-pipeline mapping
- [CLI overview](/docs/cli/)
