---
title: forgeplan decompose
description: "Break an approved PRD into an RFC with implementation phases and sub-tasks using AI"
---

`forgeplan decompose` takes an approved PRD and asks the LLM to produce a
matching RFC: implementation phases, sub-tasks per phase, and a dependency
ordering. It bridges the gap between "we know what we want" (PRD) and "here is
the sprint plan" (RFC) without forcing the author to restructure requirements
by hand.

The output is a draft RFC artifact with filled **Implementation Phases**
checkboxes, linked back to the source PRD via an `implements` relation. You
still review and edit it — decompose is a first draft, not the final word.

## When to use

- PRD is validated (`forgeplan validate PRD-XXX` = PASS) and reasoned (`forgeplan reason`)
- Depth is **Standard** or higher — Tactical tasks do not need a separate RFC
- You are transitioning from Shape to Code and want a checklist-ready plan
- The PRD has 5+ functional requirements and phasing is non-obvious

## When NOT to use

- Depth is **Tactical** — go straight to code
- PRD is still a stub (missing Problem, Goals, FR) — decompose will hallucinate phases
- An RFC for this PRD already exists — use `forgeplan update` or supersede flow instead
- You disagree with the PRD's goals — fix the PRD first, don't paper over it with an RFC

## Usage

```text
forgeplan decompose <ID>
```

## Arguments

```text
  <ID>  PRD artifact ID to decompose
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Example 1: Decompose a validated PRD

```bash
forgeplan validate PRD-019
forgeplan decompose PRD-019
```

Reads `PRD-019`, sends its Problem/Goals/FR/Non-Goals to the LLM, and creates
`RFC-0XX` linked with `implements: PRD-019`. The generated RFC contains an
**Implementation Phases** section with unchecked boxes ready for progress
tracking.

### Example 2: Full pipeline from idea to sprint plan

```bash
forgeplan route "add OAuth2 login flow"           # -> Standard, PRD -> RFC
forgeplan new prd "OAuth2 login flow"             # -> PRD-042
# ... fill MUST sections ...
forgeplan validate PRD-042                        # PASS
forgeplan reason PRD-042 --fpf                    # ADI cycle
forgeplan decompose PRD-042                       # -> RFC-018 draft
forgeplan validate RFC-018                        # sanity check
```

After decompose, open `RFC-018` in your editor, tighten phase descriptions,
and start checking boxes as phases complete.

## Output interpretation

Decompose prints the created RFC ID and a summary of generated phases:

```
Created RFC-018 (draft) linked to PRD-042
  Phase 1: Authentication provider abstraction (3 tasks)
  Phase 2: OAuth2 flow implementation (5 tasks)
  Phase 3: Session persistence + refresh (4 tasks)
  Phase 4: E2E tests + rollout gate (2 tasks)
```

Red flags:

- Single phase with 20 sub-tasks — the PRD is too broad, split into multiple PRDs
- Phases reference FRs that are not in the PRD — LLM hallucination, re-run or edit
- No rollback/evidence phase — add it manually before activation

## How it fits the workflow

```
Shape → Validate → Reason → [decompose] → Code → Evidence → Activate
                                 ^
                             you are here
```

- **Before**: `forgeplan validate PRD-XXX` (PASS), `forgeplan reason PRD-XXX`
- **After**: edit the RFC, create a feature branch, start implementing phase 1
- Progress tracking: tick phase checkboxes as you merge PRs; `forgeplan progress RFC-018` prints the bar

## See also

- [`forgeplan reason`](/docs/cli/reason/) — run ADI before decomposing
- [`forgeplan new`](/docs/cli/new/) — create the source PRD
- [`forgeplan validate`](/docs/cli/validate/) — gate for decomposition input
- [`forgeplan generate`](/docs/cli/generate/) — generate content for any artifact kind
- [Methodology: PRD → RFC flow](/docs/methodology/overview/)
