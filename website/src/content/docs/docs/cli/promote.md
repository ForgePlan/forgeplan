---
title: forgeplan promote
description: "Promote a saved memory into a full artifact (PRD, RFC, ADR, etc.)"
---

Turns a **saved memory** (created earlier via `forgeplan remember`) into a first-class artifact — PRD, RFC, ADR, ProblemCard, or Note. Use `promote` when an observation you jotted down days or weeks ago turns out to be important enough to deserve structured reasoning, evidence, and a lifecycle. It's the bridge between "I noticed something" and "this is a decision we're tracking".

## When to use

- You ran `forgeplan remember` during a session and the observation is still relevant later.
- A memory item keeps showing up in `forgeplan recall` searches — a signal that it's load-bearing.
- You want to upgrade an informal observation into a ProblemCard or PRD with proper links and scoring.
- Post-mortem: several memories from a completed sprint deserve to become ADRs for the decision log.
- Hindsight notification: a memory that was tactical at the time has now accumulated enough context to be formalised.

## When NOT to use

- The memory is ephemeral and will be stale in a week — leave it as a memory, it'll auto-fade.
- You want to create an artifact from scratch — use [`forgeplan new`](/docs/cli/new/) or [`forgeplan generate`](/docs/cli/generate/).
- You're capturing a live decision right now — use [`forgeplan capture`](/docs/cli/capture/).
- The memory doesn't have enough content to fill MUST sections — expand it with more context first, or start from [`forgeplan generate`](/docs/cli/generate/).

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

Run `forgeplan recall` first to list existing memories and copy the correct ID. Promotion creates a new artifact with the memory's body as the starting content — you still need to fill any missing MUST sections and run `validate` before `activate`.

## Examples

### Example 1: Promote an observation into a ProblemCard

```bash
forgeplan recall "search stale"
# → mem-042  "Users report search returns stale results after rename"
forgeplan promote mem-042 --kind problem
```

Creates PROB-NNN seeded with the memory's content as Signal and Context. Open the resulting ProblemCard, add Goals and Anti-Goodhart indicators, link any related Evidence, and activate.

### Example 2: Promote a memory to a PRD

```bash
forgeplan promote mem-auth-decisions --kind prd
```

Turns a running memory about an authentication discussion into a proper PRD. The command fills what it can (title, rough Problem statement) and leaves the rest as template placeholders — treat it like the output of [`forgeplan new`](/docs/cli/new/): fill the remaining MUST sections, then `validate`.

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

`promote` is the **delayed path into the Shape phase**: the observation was cheap to record (`remember`), but now deserves the full lifecycle. After promotion, the new artifact enters the standard `validate → reason → code → evidence → review → activate` flow — nothing special, just with a head start on the body content.

## See also

- [`forgeplan new`](/docs/cli/new/) — create an artifact from scratch without a memory
- [`forgeplan generate`](/docs/cli/generate/) — LLM-drafted artifact from a natural language prompt
- [`forgeplan capture`](/docs/cli/capture/) — log a live decision without a memory round-trip
- [`forgeplan validate`](/docs/cli/validate/) — required before activating the promoted artifact
- [Methodology: artifact model](/docs/methodology/overview/)
