---
title: forgeplan promote
description: "Promote a saved memory into a full artifact (PRD, RFC, ADR, etc.)"
---

Turns a project-local memory created by `forgeplan remember` into a draft artifact. This command does not promote a Hindsight bank memory. It copies the memory body and provenance into a new artifact, preserves author/tags, then deletes the original local memory after creation. Keep a separate copy first if you need the original record.

## When to use

- You ran `forgeplan remember` during a session and the observation is still relevant later.
- A memory item keeps showing up in `forgeplan recall` searches - a signal that it's load-bearing.
- You want to upgrade an informal observation into a ProblemCard or PRD with proper links and scoring.
- Post-mortem: several memories from a completed sprint deserve to become ADRs for the decision log.
- A project-local memory has enough context to enter the formal artifact lifecycle.

## When NOT to use

- The observation is temporary: review or explicitly remove it when obsolete. Local memories have no automatic validity deadline or auto-fade.
- You want to create an artifact from scratch - use [`forgeplan new`](/docs/cli/new/) or [`forgeplan generate`](/docs/cli/generate/).
- You're capturing a live decision right now - use [`forgeplan capture`](/docs/cli/capture/).
- The memory doesn't have enough content to fill MUST sections - expand it with more context first, or start from [`forgeplan generate`](/docs/cli/generate/).

## Usage

```text
forgeplan promote --kind <KIND> <MEMORY_ID>
```

## Arguments

```text
  <MEMORY_ID>  Memory ID to promote (e.g., mem-042, mem-auth-decisions)
```

## Options

```text
      --kind <KIND>  Target artifact kind: prd, rfc, adr, note, problem, epic, spec
  -h, --help         Print help
  -V, --version      Print version
```

Use `forgeplan recall` to copy an actual local memory ID. Promotion creates a draft with a title, `## Content`, the original body and a promotion provenance line; it does not infer required kind-specific sections. Fill missing MUST sections, validate and satisfy applicable gates before activation.

## Examples

### Example 1: Promote an observation into a ProblemCard

```bash
forgeplan recall "search stale"
# → mem-042  "Users report search returns stale results after rename"
forgeplan promote mem-042 --kind problem
```

Creates a Problem draft using that Content wrapper, not generated Signal/Context/Goals sections. Add the required Problem sections, constraints and real evidence; validate and complete the applicable lifecycle before activation.

### Example 2: Promote a memory to a PRD

```bash
forgeplan promote mem-auth-decisions --kind prd
```

Creates a PRD draft with the memory content. It does not synthesize a Problem statement or generate template placeholders. Add the required PRD sections yourself and run `validate`.

### Example 3: Promote a decision memory to an ADR

```bash
forgeplan promote mem-017 --kind adr
```

Useful when a memory captures a decision rationale that deserves to enter the permanent decision log. The resulting ADR draft inherits the memory body; review, add Consequences / Alternatives, and run the validate → activate flow.

## How it fits the workflow

```
(earlier session)  forgeplan remember  →  mem-NNN
                                              │
(later session)  forgeplan recall  →  promote --kind <kind>  →  fill MUST  →  validate  →  activate
```

`promote` is the **delayed path into the Shape phase**: the observation was cheap to record (`remember`), but now deserves the full lifecycle. After promotion, the new artifact enters the standard `validate → reason → code → evidence → review → activate` flow - nothing special, just with a head start on the body content.

## See also

- [`forgeplan new`](/docs/cli/new/) - create an artifact from scratch without a memory
- [`forgeplan generate`](/docs/cli/generate/) - LLM-drafted artifact from a natural language prompt
- [`forgeplan capture`](/docs/cli/capture/) - log a live decision without a memory round-trip
- [`forgeplan validate`](/docs/cli/validate/) - required before activating the promoted artifact
- [Methodology: artifact model](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan promote --help`.

```text
Promote a memory to a full artifact (e.g., forgeplan promote mem-xxx --kind prd)

Usage: forgeplan promote --kind <KIND> <MEMORY_ID>

Arguments:
  <MEMORY_ID>  Memory ID to promote (e.g., mem-auth-decisions)

Options:
      --kind <KIND>  Target artifact kind: prd, rfc, adr, note, problem, etc
  -h, --help         Print help
  -V, --version      Print version
```
<!-- forgeplan-runtime:end -->
