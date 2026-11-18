---
title: forgeplan decompose
description: "Print AI-generated RFC task suggestions from a PRD without creating artifacts."
---

`forgeplan decompose` reads a PRD and sends its title and body to the configured
LLM to suggest RFC tasks and dependencies. It prints the model's response.
It does **not** create an RFC, set checkboxes, or add links.

Review the suggestions, create the necessary RFCs with `forgeplan new rfc`,
then link each to the PRD with `--relation refines`. Validation and approval
are workflow prerequisites you check yourself, not enforced by this command.

## When to use

- PRD is validated (`forgeplan validate PRD-XXX` = PASS) and reasoned (`forgeplan reason`)
- Depth is **Standard** or higher - Tactical tasks do not need a separate RFC
- You are transitioning from Shape to Code and want a checklist-ready plan
- The PRD has 5+ functional requirements and phasing is non-obvious

## When NOT to use

- Depth is **Tactical** - go straight to code
- PRD is still a stub - incomplete requirements can produce unsupported suggestions
- An RFC for this PRD already exists - use `forgeplan update` or supersede flow instead
- You disagree with the PRD's goals - fix the PRD first, don't paper over it with an RFC

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

Reads `PRD-019` and prints LLM-generated task suggestions. Create and fill the
RFC artifacts separately; replace example IDs with the IDs actually returned.

### Example 2: Full pipeline from idea to sprint plan

```bash
forgeplan route "add OAuth2 login flow"           # -> Standard, PRD -> RFC
forgeplan new prd "OAuth2 login flow"             # -> PRD-042
# ... fill MUST sections ...
forgeplan validate PRD-042                        # PASS
forgeplan reason PRD-042 --fpf                    # ADI cycle
forgeplan decompose PRD-042                       # prints suggestions only
forgeplan new rfc "OAuth2 implementation"         # record the returned RFC ID
# Fill the RFC body, link it to PRD-042 with --relation refines, then validate it.
```

After reviewing the output, create and fill the RFCs before implementation.
Check off phase items only when the corresponding work is complete.

## Output interpretation

The model's output varies. This is an illustrative plan, not a creation receipt:

```
Suggested tasks for PRD-042:
  Phase 1: Authentication provider abstraction (3 tasks)
  Phase 2: OAuth2 flow implementation (5 tasks)
  Phase 3: Session persistence + refresh (4 tasks)
  Phase 4: E2E tests + rollout gate (2 tasks)
```

Red flags:

- Single phase with 20 sub-tasks - the PRD is too broad, split into multiple PRDs
- Phases reference FRs that are not in the PRD - LLM hallucination, re-run or edit
- No rollback/evidence phase - add it manually before activation

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

- [`forgeplan reason`](/docs/cli/reason/) - run ADI before decomposing
- [`forgeplan new`](/docs/cli/new/) - create the source PRD
- [`forgeplan validate`](/docs/cli/validate/) - gate for decomposition input
- [`forgeplan generate`](/docs/cli/generate/) - generate content for any artifact kind
- [Methodology: PRD → RFC flow](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan decompose --help`.

```text
Decompose a PRD into RFC tasks using AI

Usage: forgeplan decompose <ID>

Arguments:
  <ID>  PRD artifact ID to decompose

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
