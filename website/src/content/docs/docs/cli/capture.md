---
title: forgeplan capture
description: "Capture a decision from conversation into a Note or ADR artifact"
---

Records a conversational decision as an LLM-drafted **Note or ADR**. The model selects Note for simple observations and ADR for architectural decisions with alternatives; malformed type output falls back to Note. Review the created draft before relying on it.

For manual control over the kind and wording, use [`forgeplan new adr`](/docs/cli/new/). `capture` requires a configured LLM and does not set a 90-day expiry: its `valid_until` is unset.

## When to use

- A short discussion just landed on a choice ("we'll use BGE-M3 instead of all-MiniLM"); you want it logged before the context is lost.
- Mid-sprint course-correction: a small architectural call that doesn't deserve a full RFC but should be traceable later.
- You want to convert an AI reasoning session's conclusion into an artifact without leaving the terminal.

## When NOT to use

- The decision needs Problem / Goals / FRs - use [`forgeplan new prd`](/docs/cli/new/) instead.
- The topic spans multiple files and stakeholders - use an RFC with Implementation Phases.
- You need a manually authored durable decision record - use [`forgeplan new adr`](/docs/cli/new/).

## Usage

```text
forgeplan capture [OPTIONS] <DECISION>
```

## Arguments

```text
  <DECISION>  The decision statement (quoted one-liner)
```

## Options

```text
      --context <CONTEXT>  Additional context (optional)
  -h, --help               Print help
  -V, --version            Print version
```

`capture` reports the auto-detected kind and created ID. Review both; if you later create a separate ADR, link the actual captured artifact rather than assuming it was a Note.

## Examples

### Example 1: Quick note from a sprint conversation

```bash
forgeplan capture "skip retry for 4xx from embedding API; retry only 5xx" \
  --context "discussed in sprint 13 sync - 4xx means malformed input, retry is wasted"
```

The model drafts the body from the decision and context. The title comes from the first line of the decision, truncated to 80 characters. Copy the returned ID; neither Note kind nor automatic expiry is guaranteed.

### Example 2: Minimal note with no extra context

```bash
forgeplan capture "log R_eff to stderr on every validate run"
```

No interactive template prompts are used, but the LLM draft still needs review and the applicable validation and lifecycle checks.

### Example 3: Capture then promote to ADR when it matters

```bash
forgeplan capture "use LanceDB as derived index, markdown as source of truth" \
  --context "ADR-003 principle: files are authoritative, lance/ is gitignored"
# Later, if the decision proves load-bearing:
forgeplan new adr "LanceDB as derived index"
forgeplan link NOTE-NNN ADR-005 --relation informs
```

This illustrates linking a previously captured Note to a later ADR. Use the actual returned IDs; this architectural statement may already be classified as an ADR by `capture`.

## How it fits the workflow

This command belongs in the [full artifact lifecycle](/docs/guides/first-artifact/) - see the tutorial for the end-to-end flow.

## See also

- [`forgeplan new note`](/docs/cli/new/) - manual Note creation with template
- [`forgeplan new adr`](/docs/cli/new/) - manual ADR creation for full workflow
- [`forgeplan promote`](/docs/cli/promote/) - turn a saved memory into an artifact
- [`forgeplan link`](/docs/cli/link/) - attach the captured decision to a parent PRD / RFC
- [Methodology: depth calibration](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan capture --help`.

```text
Capture a decision from conversation into a Note or ADR artifact

Usage: forgeplan capture [OPTIONS] <DECISION>

Arguments:
  <DECISION>  The decision statement

Options:
      --context <CONTEXT>  Additional context (optional)
  -h, --help               Print help
  -V, --version            Print version
```
<!-- forgeplan-runtime:end -->
