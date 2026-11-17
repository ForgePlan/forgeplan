---
title: forgeplan supersede
description: "Retire an active artifact in favor of a replacement while preserving history. The old artifact becomes superseded. Runtime writes an old-ID → replacement-ID relation named supersedes, not superseded_by."
---

Retire an active artifact in favor of a replacement while preserving history. The old artifact becomes superseded. Runtime writes an old-ID → replacement-ID relation named `supersedes`, not `superseded_by`.

## Usage

```text
forgeplan supersede --by <BY> <ID>
```

## Arguments

```text
  <ID>  Artifact ID to supersede
```

## Options

```text
      --by <BY>  Replacement artifact ID
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

```bash
# Recommended: review and activate the successor first.
forgeplan activate ADR-005
forgeplan supersede ADR-003 --by ADR-005
forgeplan get ADR-003
```

## Gates and warnings

The source must be active and the replacement must exist. A draft replacement produces a warning, not a refusal; a superseded or deprecated replacement is refused. Prefer an activated successor so readers do not inherit an unreviewed decision. Missing replacements are rejected even though the CLI wrapper can resolve forward-reference syntax.

## Evidence and recovery

Superseded evidence is excluded from the current weakest-link population, and affected dependents are rescored. Keep the displaced pack for historical inspection. Superseded is terminal for ordinary lifecycle transitions. The 0.38.0 CLI supersede path does not create a recovery receipt: do not promise that `restore` or `undo-last` can reverse this invocation. Those commands require an existing matching receipt; undo-last could instead undo an unrelated operation. Without a replacement, use `deprecate`; for re-evaluation with a new draft use `reopen`.

## See also

- [`forgeplan deprecate`](/docs/cli/deprecate/) - retire without a replacement
- [`forgeplan reopen`](/docs/cli/reopen/) - deprecate and start a new draft for re-evaluation
- [`forgeplan activate`](/docs/cli/activate/) - activate the successor before superseding
- [`forgeplan get`](/docs/cli/get/) - inspect the `supersedes` relationship
- [Lifecycle v2 guide](/docs/guides/lifecycle-v2/)
- [Methodology: Artifact Lifecycle](/docs/methodology/lifecycle/)
