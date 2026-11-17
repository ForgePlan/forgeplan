---
title: forgeplan promote
description: "Convert a local memory into a new artifact of the explicitly selected kind. The command creates a Tactical draft with the memory text under Content and a provenance note, then deletes the original memory and its incident relations."
---

Convert a local memory into a new artifact of the explicitly selected kind. The command creates a Tactical draft with the memory text under Content and a provenance note, then deletes the original memory and its incident relations.

## Usage

```text
forgeplan promote --kind <KIND> <MEMORY_ID>
```

## Arguments

```text
  <MEMORY_ID>  Memory ID to promote (e.g., mem-auth-decisions)
```

## Options

```text
      --kind <KIND>  Target artifact kind: prd, rfc, adr, note, problem, etc
  -h, --help         Print help
  -V, --version      Print version
```

## Example

```bash
forgeplan remember --list
# Replace MEMORY_ID with an actual mem-... identifier:
forgeplan promote MEMORY_ID --kind note
# Read the returned artifact ID and fill its required sections:
forgeplan list --type note
```

## What is preserved

Title, text, author, and tags are carried into the new artifact. The command does not populate a full PRD/RFC/ADR template or automatically retain the old memory as linked evidence. Review the new draft, add the required sections and meaningful relations, then validate before activation. Promotion to kind memory is refused.

## Choosing the operation

Use `new` for an artifact without an existing memory, `capture` for LLM-assisted decision capture, or keep a memory when it need not become an artifact. Memories created by `remember` have no automatic expiry; do not assume they fade away. These commands operate on core memory, not Hindsight banks.

## See also

- [`forgeplan new`](/docs/cli/new/) - create an artifact from scratch without a memory
- [`forgeplan generate`](/docs/cli/generate/) - LLM-drafted artifact from a natural language prompt
- [`forgeplan capture`](/docs/cli/capture/) - log a live decision without a memory round-trip
- [`forgeplan validate`](/docs/cli/validate/) - required before activating the promoted artifact
- [Methodology: artifact model](/docs/methodology/overview/)
