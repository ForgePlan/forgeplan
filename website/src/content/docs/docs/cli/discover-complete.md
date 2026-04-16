---
title: forgeplan discover complete
description: "Finalize a discovery session and emit recommended artifacts (PROBs, PRDs, Notes) to create"
---

`forgeplan discover complete <SESSION_ID>` closes a brownfield discovery session. It marks the session row as completed, runs the recommendation pass over all accumulated findings, and prints a list of **proposed artifacts** — typically Problem cards, PRDs, Notes, and sometimes ADRs — for the user to create with `forgeplan new`.

This is where the brownfield pipeline turns raw findings into real methodology state.

## When to use

- **After the agent has finished walking the source tiers** and `discover show` reports adequate coverage.
- **At the end of an onboarding sprint** — crystalize everything learned into artifacts.
- **When you want to stop a session and capture whatever was found so far** — partial sessions still produce recommendations.

## When NOT to use

- While the agent is still actively submitting findings — wait for it to settle.
- To abort without saving — there's no rollback; once completed, proposals are emitted.
- On an already-completed session — idempotent but pointless.

## Usage

```text
forgeplan discover complete <SESSION_ID> [OPTIONS]
```

## Arguments

```text
  <SESSION_ID>   Discovery session identifier (from discover list / start)
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

```bash
forgeplan discover complete disc-001

# Full onboarding loop
forgeplan init -y
forgeplan discover start
# ... agent runs ...
forgeplan discover show disc-001
forgeplan discover complete disc-001
# → proposals printed; create the ones you accept
forgeplan new prob "Auth drift between docs and code"
forgeplan new prd  "Unified config loader"
```

## What the recommendation pass does

At completion, the engine:

1. **Groups findings** by category (decisions, invariants, drift, debt, risks, ownership).
2. **Deduplicates** — merges findings that reference the same module/concept.
3. **Maps categories to artifact kinds** — drift → Problem, major decisions → ADR, gaps → PRD, observations → Note.
4. **Suggests depth** — Tactical / Standard / Deep based on blast radius and reversibility heuristics.
5. **Emits proposals** as a list of `forgeplan new` commands the user can accept, edit, or skip.

The user stays in control: proposals are printed, not auto-created.

## How it fits

`discover complete` is the **hand-off point** from discovery to the normal Forgeplan lifecycle:

```
discover start → (findings) → discover complete → proposals
                                                    ↓
                          forgeplan new prob/prd/adr/note ...
                                                    ↓
                          validate → reason → activate
```

From here, the project flows through the standard Shape → Validate → ADI → Code → Evidence → Activate cycle like any greenfield work.

## See also

- [`forgeplan discover`](/docs/cli/discover/) — parent command + pipeline overview
- [`forgeplan discover show`](/docs/cli/discover-show/) — coverage check before completing
- [`forgeplan discover list`](/docs/cli/discover-list/) — session index
- [`forgeplan new`](/docs/cli/new/) — create the proposed artifacts
- [`forgeplan validate`](/docs/cli/validate/) — next step after new
- [Methodology guide](/docs/methodology/overview/)
