---
title: forgeplan capture
description: "Capture a decision from conversation into a Note or ADR artifact"
---

Records a decision that emerged mid-conversation into a **Note** artifact (90-day auto-expiring micro-decision). `capture` is designed for the moment when you and a teammate (or you and an AI agent) just agreed on something small and you don't want the friction of a full `new → fill MUST sections → validate` loop, but you also don't want the decision to evaporate.

For permanent architectural decisions, use [`forgeplan new adr`](/docs/cli/new/) — `capture` targets Notes only.

## When to use

- A short discussion just landed on a choice ("we'll use BGE-M3 instead of all-MiniLM"); you want it logged before the context is lost.
- Mid-sprint course-correction: a small architectural call that doesn't deserve a full RFC but should be traceable later.
- You want to convert an AI reasoning session's conclusion into an artifact without leaving the terminal.

## When NOT to use

- The decision needs Problem / Goals / FRs — use [`forgeplan new prd`](/docs/cli/new/) instead.
- The topic spans multiple files and stakeholders — use an RFC with Implementation Phases.
- The decision is durable and architectural — use [`forgeplan new adr`](/docs/cli/new/) for the permanent decision log.

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

`capture` always produces a Note — the cheapest persistence option. If the decision turns out to be load-bearing, create a separate ADR with `forgeplan new adr` and link the original Note.

## Examples

### Example 1: Quick note from a sprint conversation

```bash
forgeplan capture "skip retry for 4xx from embedding API; retry only 5xx" \
  --context "discussed in sprint 13 sync — 4xx means malformed input, retry is wasted"
```

Creates a NOTE-NNN with the decision as title and the context in the body. Notes auto-expire after 90 days — appropriate for reversible, scoped calls.

### Example 2: Minimal note with no extra context

```bash
forgeplan capture "log R_eff to stderr on every validate run"
```

The fastest path from "we decided" to "it's tracked". No template prompts, no required sections — useful when the context is already obvious from recent commits.

### Example 3: Capture then promote to ADR when it matters

```bash
forgeplan capture "use LanceDB as derived index, markdown as source of truth" \
  --context "ADR-003 principle: files are authoritative, lance/ is gitignored"
# Later, if the decision proves load-bearing:
forgeplan new adr "LanceDB as derived index"
forgeplan link NOTE-NNN ADR-005 --relation informs
```

Start with a Note for minimal friction; create an ADR only once the decision clearly deserves the permanent decision log.

## How it fits the workflow

This command belongs in the [full artifact lifecycle](/docs/guides/first-artifact/) — see the tutorial for the end-to-end flow.

## See also

- [`forgeplan new note`](/docs/cli/new/) — manual Note creation with template
- [`forgeplan new adr`](/docs/cli/new/) — manual ADR creation for full workflow
- [`forgeplan promote`](/docs/cli/promote/) — turn a saved memory into an artifact
- [`forgeplan link`](/docs/cli/link/) — attach the captured decision to a parent PRD / RFC
- [Methodology: depth calibration](/docs/methodology/overview/)
