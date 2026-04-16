---
title: forgeplan remember
description: "Save a memory (fact, convention, procedure, observation, constraint) for later recall"
---

Capture a short-form memory — a fact, convention, procedure, observation, or
constraint — into the Forgeplan decision journal. Memories are lightweight,
local notes that you can later surface with
[`recall`](/docs/cli/recall/) or promote into a full artifact (Note, Problem,
ADR) with [`promote`](/docs/cli/promote/).

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
  -c, --category <CATEGORY>  Memory kind: fact | convention | procedure | observation | constraint
      --list                 List all memories
      --forget <FORGET>      Forget (delete) a memory by ID
  -h, --help                 Print help
  -V, --version              Print version
```

## The five kinds

Each memory has one kind, which determines how you search for it and how
`promote` will later shape it into a full artifact.

| Kind           | When to use                                                  | Promotes to     |
|----------------|--------------------------------------------------------------|-----------------|
| `fact`         | Ground truth about the project ("we use BGE-M3 for embeds")  | Note            |
| `convention`   | Team agreement ("PRs always merge into dev, not main")       | Note / RFC      |
| `procedure`    | Repeatable how-to ("always `git pull` before new branch")    | Note            |
| `observation`  | Something you noticed, not yet explained                     | ProblemCard     |
| `constraint`   | Hard limit or requirement ("binary must stay under 50MB")    | ADR / RFC       |

## Examples

Record a procedure during a sprint:

```bash
forgeplan remember \
  "always pull dev before creating feature branch" \
  --kind procedure
```

Record an observation that might need investigation:

```bash
forgeplan remember \
  "watch daemon drops events on nfs mounts" \
  --kind observation
```

Record a team convention:

```bash
forgeplan remember \
  "release branches merge with merge-commit, not squash" \
  --kind convention
```

List everything you've captured:

```bash
forgeplan remember --list
```

Forget a memory by its ID:

```bash
forgeplan remember --forget MEM-012
```

## Typical workflow

```text
observe something → remember → recall later → promote to artifact
```

1. **During work** — notice a fact, convention, or surprise. Capture it
   immediately with `remember` so the context isn't lost.
2. **Later** — [`forgeplan recall "keyword"`](/docs/cli/recall/) surfaces
   relevant memories when you start related work.
3. **When it matures** — [`forgeplan promote MEM-XXX`](/docs/cli/promote/)
   upgrades the memory into a proper Note, ProblemCard, or ADR, preserving
   the original capture as evidence.

This is the lightweight end of the Forgeplan artifact spectrum: you don't
need to validate, score, or activate a memory — just capture it and move on.

## Storage

Memories live in `.forgeplan/memory/` as markdown files (git-tracked) and
are indexed in LanceDB like any other artifact. They're subject to the same
source-of-truth rules: edit via CLI, or edit the markdown and run
[`reindex`](/docs/cli/reindex/).

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan recall`](/docs/cli/recall/) — search and retrieve memories
- [`forgeplan promote`](/docs/cli/promote/) — upgrade a memory into an artifact
- [`forgeplan journal`](/docs/cli/journal/) — full decision journal with R_eff
- [`forgeplan new note`](/docs/cli/new/) — go straight to a Note if the memory is already mature
