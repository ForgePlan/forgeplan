---
title: Quick Start
description: "A worked example of creating, validating, evidencing and activating a decision."
---

This is a worked example. For a fully annotated tutorial with recovery, see
[First Artifact Tutorial](/docs/guides/first-artifact/). IDs and output below
are illustrative, not a captured runtime transcript; use the ID returned by
your own command.

:::tip[Shortcut]
In Claude Code, install the bundled `/forge` skill with `forgeplan setup-skill` and run `/forge "your task description"`. Other clients need their own supported skill installation; this command only writes the Claude Code path.
See [Marketplace -- Forgeplan Workflow](/docs/marketplace/forgeplan-workflow/).
:::

By the end of this page you will have:

1. An initialized Forgeplan workspace
2. A routed and validated PRD
3. ADI reasoning captured
4. Evidence linked to the PRD
5. An `R_eff > 0` score
6. An artifact in `active` state

## 0. Install

If you have not installed Forgeplan yet, see [Installation](/docs/getting-started/installation/).

Verify it works:

```bash
forgeplan --version
```

:::caution[Prepare the capabilities you use]
The semantic-search build feature, model files, and workspace vectors are
separate prerequisites. FPF and agent plugins are additional workflow
dependencies, not required for plain keyword retrieval. See
[Installation](/docs/getting-started/installation/).
:::

## 1. Initialize a workspace

```bash
cd /path/to/your/project
forgeplan init -y
```

In a new project this creates artifact directories and `config.yaml`; `-y`
skips prompts. If `.forgeplan/` already exists, including in a parent,
`init -y` returns without repairs. On a fresh clone, use `reindex` for the
derived index; if config is missing and defaults are wanted, deliberately
use `init -y --force`, then restore custom settings from its backup.

This CLI lifecycle walkthrough is not a claim that every command supports
a migrated workspace in local `0.39.0+constellation.s1.1`. Use the
[Constellation guide](/docs/guides/constellation/) for that layout.

```bash
forgeplan health
# -> 0 artifacts, 0 blind spots. Workspace ready.
```

## 2. Route your task

Before writing anything, let Forgeplan pick the right depth:

```bash
forgeplan route "add user authentication"
```

```
Depth: Standard
Pipeline: PRD → RFC
Confidence: 90%
Next: forgeplan new prd "User Authentication"
```

`Standard` means you create a PRD for the "what" and an RFC for the "how".
For more context see [Routing & Depth](/docs/methodology/routing/).

## 3. Shape - create the PRD

```bash
forgeplan new prd "User Authentication"
# -> Created: PRD-001 at .forgeplan/prds/PRD-001-user-authentication.md
```

Prepare the completed body in `prd-body.md` outside `.forgeplan/`, then apply
it with `forgeplan update PRD-001 --body @prd-body.md`. Fill the MUST sections:

- **Problem** - why does this exist?
- **Goals** - what does success look like? (SMART criteria)
- **Non-Goals** - what is explicitly out of scope?
- **Target Users** - who benefits?
- **Functional Requirements (FR)** - `[Actor] can [capability]`
- **Related Artifacts** - parent decisions or an explicit statement that none apply

:::caution[Don't leave stubs]
Fill missing sections before activation. Do not delete a draft merely
because validation reports incomplete content.
:::

## 4. Validate

```bash
forgeplan validate PRD-001
```

For recorded compatibility coverage and remaining limits, see
[Constellation](/docs/reference/constellation/). Use an explicit `workspace`
when calling full MCP `forgeplan_validate`. Registration alone does not
establish runtime coverage for every lifecycle tool.

Illustrative validator output for a supported workspace:
```
PRD-001: PASS ✓ (0 MUST errors, 2 SHOULD warnings)
```

MUST findings can cover more than missing sections. Fix each reported rule
and re-run. Technology choices such as JWT belong in design, not requirements:
the implementation-leakage rule is SHOULD at Standard and MUST at Deep.

## 5. Reason (ADI)

For Standard depth, ADI is recommended. For Deep and Critical, it is
mandatory:

```bash
forgeplan reason PRD-001 --save
```

The configured LLM is asked to generate hypotheses, predictions and an
evidence-oriented comparison; the router selects depth, not ADI content.
Review the output against actual evidence. `--save` creates a linked Note; without it the CLI
only prints the analysis.

Alternatively, request FPF knowledge-base context (do not run both examples
unless you want two saved Notes). This CLI path currently proceeds without
retired-table FPF context on migrated Constellation workspaces:

```bash
forgeplan reason PRD-001 --fpf --save
```

See [ADI Reasoning](/docs/methodology/adi/) for the full cycle.

## 6. Build

Code your feature. The critical rule: **write a test for every `pub fn`
before moving to the next function**. Tests become Evidence in step 7.

When the feature is done:

```bash
cargo test       # or your equivalent
cargo fmt
cargo check
```

All must be green before you activate.

## 7. Prove - create Evidence

```bash
forgeplan new evidence "Auth tests - 12 pass, JWT validate 0.3ms"
# -> Created: EVID-001
```

Prepare `evidence-body.md` outside `.forgeplan/` with actual results and this
**structured fields** block. The CL3 example requires matching test context:

```markdown
## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test
```

An absent/unrecognised verdict or absent/invalid CL source fails closed to
CL0 (0.9 penalty). Declare `evidence_type` too; it has its own scoring role.
Do not copy the illustrative test count or timings as measurements. For
code-claiming evidence, record real Git provenance as required by your gate.

Apply the completed evidence body, then link it to the PRD:

```bash
forgeplan update EVID-001 --body @evidence-body.md
forgeplan link EVID-001 PRD-001 --relation informs
```

## 8. Check the score

```bash
forgeplan score PRD-001
```

```
PRD-001: R_eff = 1.00 - Adequate
  EVID-001: supports, CL3, test -> score 1.0
```

If R_eff is 0.0, inspect the score breakdown: missing current evidence,
refutation, CL penalties or dependencies can explain it. Missing structured
claims commonly produce 0.1, not necessarily zero.
See [Evidence & R_eff](/docs/methodology/evidence/).

## 9. Activate

```bash
forgeplan review PRD-001
# -> Review PASSED - ready to activate

forgeplan activate PRD-001
# -> draft → active
```

The validation gate runs one more time on activation. If it fails, the
transition is rejected.

## 10. Verify health

```bash
forgeplan health
```

```
Artifacts: 3 (1 active, 2 draft: EVID-001 and the saved ADI Note)
Blind spots: 0
Orphans: 0
Evidence coverage: 100%
```

You now have a fully traced decision with measurable trust. Future-you (or
your teammate) can open PRD-001 in six months and see: the problem, the
goals, the ADI hypotheses, the chosen approach, and the evidence that
justified it.

## The full cycle (cheat sheet)

```
Route → Shape → Validate → Reason → Build → Prove → Activate
  1        2        3         4       5       6         7

1. forgeplan route "task"
2. forgeplan new prd "Title"     # fill MUST sections
3. forgeplan validate PRD-XXX
4. forgeplan reason PRD-XXX      # ADI, mandatory on Deep+
5. code + test
6. forgeplan new evidence "..." + link + score
7. forgeplan review + activate
```

:::tip[Work isn't done until]
PRD filled + validated + ADI captured (Standard+) + evidence created + R_eff > 0 + activated.
Anything less is an open tab, not a finished task.
:::

## Next steps

- [First Artifact Tutorial](/docs/guides/first-artifact/) - 20-minute guided version with error recovery
- [Methodology Overview](/docs/methodology/overview/) - the 10 rules
- [CLI Reference](/docs/cli/) - every command documented
- [Configuration](/docs/getting-started/configuration/) - LLM providers, embeddings, git sync
- [Marketplace Overview](/docs/marketplace/overview/) - plugins for `/audit`, `/sprint`, `/fpf` and more
