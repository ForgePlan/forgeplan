---
title: forgeplan remember
description: "Save a memory (fact, convention, procedure, observation, constraint) for later recall"
---

Saves a short project-local memory for [`recall`](/docs/cli/recall/) or [`promote`](/docs/cli/promote/). These Markdown records are distinct from a Hindsight server bank. Memory creation does not set a validity deadline or automatically expire the record.

Think of it as a working memory for the project: stuff you don't want to lose
but that isn't ready to be a PRD or ADR yet.

## Usage

```text
forgeplan remember [OPTIONS] [TEXT]
```

## Arguments

```text
  [TEXT]  Text to remember (omit for --list or --forget)
```

## Options

```text
  -c, --category <CATEGORY>  Category string (default fact; examples: convention, procedure, insight)
      --list                 List all memories
      --forget <FORGET>      Forget (delete) a memory by ID
  -h, --help                 Print help
  -V, --version              Print version
```

## Categories, not promotion rules

`--category` stores a string tag, defaulting to `fact`. Help suggests `fact`, `convention`, `procedure`, and `insight`; the handler does not enforce a closed enumeration. A category does not select the promoted artifact kind: `promote` requires its own explicit `--kind`. Treat a remembered observation as an observation, not independently verified evidence.

## Examples

Record a procedure during a sprint:

```bash
forgeplan remember \
  "always pull dev before creating feature branch" \
  --category procedure
```

Record an observation that might need investigation:

```bash
forgeplan remember \
  "watch daemon drops events on nfs mounts" \
  --category insight
```

Record a team convention:

```bash
forgeplan remember \
  "release branches merge with merge-commit, not squash" \
  --category convention
```

List everything you've captured:

```bash
forgeplan remember --list
```

Forget a memory by its ID:

```bash
forgeplan remember --forget mem-example   # illustrative ID; copy the actual ID from --list
```

## Typical workflow

```text
observe something → remember → recall later → promote to artifact
```

1. **During work** - notice a fact, convention, or surprise. Capture it
   immediately with `remember` so the context isn't lost.
2. **Later** - [`forgeplan recall "keyword"`](/docs/cli/recall/) surfaces
   relevant memories when you start related work.
3. **When it matures** - [`forgeplan promote`](/docs/cli/promote/) with the real memory ID and explicit `--kind` creates a draft carrying the content/provenance, then deletes the original local memory. It does not create an EvidencePack or preserve the original record as separate evidence.

This is the lightweight end of the Forgeplan artifact spectrum: you don't
need to validate, score, or activate a memory - just capture it and move on.

## Storage

Canonical memories live in `.forgeplan/memory/` as Markdown. Whether Git tracks them depends on the repository. Mutation is file-first and reconciles the owning index; use `index` for Constellation maintenance, not legacy table recreation. A Hindsight bank is not the local memory source.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan recall`](/docs/cli/recall/) - search and retrieve memories
- [`forgeplan promote`](/docs/cli/promote/) - upgrade a memory into an artifact
- [`forgeplan journal`](/docs/cli/journal/) - full decision journal with R_eff
- [`forgeplan new note`](/docs/cli/new/) - go straight to a Note if the memory is already mature

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan remember --help`.

```text
Save a memory (fact, convention, procedure) for later recall

Usage: forgeplan remember [OPTIONS] [TEXT]

Arguments:
  [TEXT]  Text to remember (omit for --list or --forget)

Options:
  -c, --category <CATEGORY>  Memory category: fact, convention, procedure, insight
      --list                 List all memories
      --forget <FORGET>      Forget (delete) a memory by ID
  -h, --help                 Print help
  -V, --version              Print version
```
<!-- forgeplan-runtime:end -->
