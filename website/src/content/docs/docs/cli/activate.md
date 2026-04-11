---
title: forgeplan activate
description: "Transition a draft artifact to active state through a validation gate — the moment a decision goes live."
---

`forgeplan activate` moves an artifact from `draft` to `active`. This is the most frequent lifecycle transition: it marks the point at which a PRD, RFC, ADR, Epic or Spec is considered live, ready to be referenced by other artifacts, and expected to be backed by evidence. For structured artifacts, activation runs the validation gate — all MUST rules must pass. For lightweight artifacts (`Note`, `Problem`), activation is unguarded and happens immediately.

## When to use

- You just finished implementing a PRD: all FRs are checked off, an EvidencePack is linked, and `forgeplan score PRD-001` returns `R_eff > 0`.
- An ADR has been reviewed with the team and the decision is final — time to make it authoritative.
- An RFC has passed a verification gate and its implementation phases have started producing evidence.

## When NOT to use

- The artifact still has MUST validation errors — fix them first with `forgeplan validate <id>` instead of reaching for `--force`.
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

`review` runs the same checks `activate` uses but without changing state — useful as a dry run before flipping the switch.

### Example 3: Force activation despite validation errors

```bash
forgeplan activate RFC-012 --force
```

Use only in exceptional cases (e.g. migrating brownfield artifacts). Prefer fixing the gaps.

## How it fits the workflow

This command belongs in the [full artifact lifecycle](/docs/guides/first-artifact/) — see the tutorial for the end-to-end flow. Activation is how Forgeplan distinguishes "in progress" work from "live" decisions that other artifacts are allowed to depend on.

## Common errors

| Error | Cause | Fix |
|---|---|---|
| `MUST rule failed: Problem section missing` | Required frontmatter or section is empty | Edit the artifact file, then re-run `forgeplan validate <id>` |
| `Artifact already in state: active` | Already activated in a previous session | No action needed; inspect with `forgeplan show <id>` |
| `R_eff = 0 after activation` | No EvidencePack linked | Create evidence with `forgeplan new evidence`, then `forgeplan link` + `forgeplan score` |
| `Cannot activate from terminal state` | Artifact is `superseded` or `deprecated` | Terminal states never re-activate — create a new draft instead |

## See also

- [`forgeplan validate`](/docs/cli/validate/) — run the validation gate without changing state
- [`forgeplan score`](/docs/cli/score/) — verify R_eff before and after activation
- [`forgeplan review`](/docs/cli/review/) — readiness dry run
- [`forgeplan supersede`](/docs/cli/supersede/) — replace one active artifact with another
- [`forgeplan deprecate`](/docs/cli/deprecate/) — retire an active artifact without replacement
- [Lifecycle v2 guide](/docs/guides/lifecycle-v2/)
- [Methodology: Artifact Lifecycle](/docs/methodology/lifecycle/)
