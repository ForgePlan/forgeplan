---
title: "forgeplan ci-assign-id"
description: "PROB-060 Phase 0b — atomic CI assigner of `assigned_number`. Walks `.forgeplan/**/*.md` in `--head`, finds candidates with `assigned_number: null`, computes `next = max(assigned_number)+1` per kind from `--base` git ref, rewrites frontmatter (no file rename — Phase 2.1). LanceDB-free per ADR-003. Wrapped in production by `.github/workflows/assign-id.yml` `concurrency: forgeplan-id-assign`"
---

## Usage

The exact signature and options of the selected build appear below. Registration does not prove support for every workspace layout.

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan ci-assign-id --help`.

```text
PROB-060 Phase 0b — atomic CI assigner of `assigned_number`. Walks `.forgeplan/**/*.md` in `--head`, finds candidates with `assigned_number: null`, computes `next = max(assigned_number)+1` per kind from `--base` git ref, rewrites frontmatter (no file rename — Phase 2.1). LanceDB-free per ADR-003. Wrapped in production by `.github/workflows/assign-id.yml` `concurrency: forgeplan-id-assign`

Usage: forgeplan ci-assign-id [OPTIONS]

Options:
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
<!-- forgeplan-runtime:end -->

## Examples

- [Reproducible Constellation walkthrough](/docs/guides/constellation/)
- [Artifact quick start](/docs/getting-started/quick-start/)

## See also

- [CLI reference](/docs/cli/)
- [Constellation reference](/docs/reference/constellation/)
