---
title: forgeplan activate
description: "Transition a draft artifact to active state through a validation gate - the moment a decision goes live."
---

`forgeplan activate` moves a draft artifact to `active`. For PRD, RFC, ADR, Epic and Spec, it checks MUST validation rules and methodology gates for body length, linked evidence and unfilled template content. `Note` and `Problem` skip those methodology gates, but still obey lifecycle transition rules. A positive R_eff remains a workflow requirement; the activation evidence-link gate does not itself prove evidence quality.

## When to use

- You just finished implementing a PRD: all FRs are checked off, an EvidencePack is linked, and `forgeplan score PRD-001` returns `R_eff > 0`.
- An ADR has been reviewed with the team and the decision is final - time to make it authoritative.
- An RFC has passed a verification gate and its implementation phases have started producing evidence.

## When NOT to use

- The artifact still has MUST validation errors - fix them first with `forgeplan validate <id>` instead of reaching for `--force`.
- There is no code, test, or measurement behind the decision yet. Activating without evidence creates a blind spot in `forgeplan health`.

## Usage

```text
forgeplan activate [OPTIONS] <ID>
```

## Arguments

```text
  <ID>  Artifact ID
```

## Options

```text
      --force    Force activation even if validation has MUST errors
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Example 1: Activate a fully-validated PRD

```bash
forgeplan validate PRD-001
forgeplan score PRD-001
forgeplan activate PRD-001
```

Run `validate` and `score` first to confirm the MUST gate passes and R_eff is non-zero, then activate.

### Example 2: Activate an ADR after review

```bash
forgeplan review ADR-005
forgeplan activate ADR-005
```

`review` runs the shared validation and methodology gates without changing state. Activation also checks the status transition and, on its legacy-store path, configured Git-delta provenance; a passed review is not a guarantee that activation will succeed.

### Example 3: Force activation despite validation errors

```bash
forgeplan activate RFC-012 --force
```

Use only in exceptional cases (e.g. migrating brownfield artifacts). Prefer fixing the gaps.

## How it fits the workflow

This command belongs in the [full artifact lifecycle](/docs/guides/first-artifact/) - see the tutorial for the end-to-end flow. Activation is how Forgeplan distinguishes "in progress" work from "live" decisions that other artifacts are allowed to depend on.

## Common errors

| Error | Cause | Fix |
|---|---|---|
| MUST validation finding | Required frontmatter or section is empty | Update the artifact with `forgeplan update <id> --body @body.md`, then validate again |
| Invalid transition from `active` | Already activated | Inspect with `forgeplan get <id>`; activation is not an idempotent no-op |
| `R_eff = 0 after activation` | Linked evidence may be absent or refuting | Inspect the packs and their fields before adding evidence or rescoring |
| `Cannot activate from terminal state` | Artifact is `superseded` or `deprecated` | Direct activation is refused. Create a new draft, or use receipt-based `restore`/`undo-last` for recovery. |

## See also

- [`forgeplan validate`](/docs/cli/validate/) - run the validation gate without changing state
- [`forgeplan score`](/docs/cli/score/) - verify R_eff before and after activation
- [`forgeplan review`](/docs/cli/review/) - readiness dry run
- [`forgeplan supersede`](/docs/cli/supersede/) - replace one active artifact with another
- [`forgeplan deprecate`](/docs/cli/deprecate/) - retire an active artifact without replacement
- [Lifecycle v2 guide](/docs/guides/lifecycle-v2/)
- [Methodology: Artifact Lifecycle](/docs/methodology/lifecycle/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan activate --help`.

```text
Activate an artifact (draft → active) with validation gate

Usage: forgeplan activate [OPTIONS] <ID>

Arguments:
  <ID>  Artifact ID

Options:
      --force    Force activation even if validation has MUST errors
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
