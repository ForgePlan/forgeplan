---
title: Quick Start
description: "Create, validate and support your first decision with evidence."
---

This walkthrough shows the CLI cycle for a decision backed by implemented
work. It is not a time guarantee: requirements, implementation and real
verification take as long as the task requires. For error recovery, see
[First Artifact Tutorial](/docs/guides/first-artifact/).

:::tip[Shortcut]
In Claude Code, `forgeplan setup-skill` installs the bundled `/forge` guidance.
It does not install MCP or the full marketplace harness. For `/smith`,
`/forge-cycle` and specialist agents, follow
[Installation](/docs/getting-started/installation/#full-harness-bundle-claude-code).
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

:::caution[Choose the prerequisites for this workflow]
Basic artifact operations need no model or LLM. Step 5 uses `reason` and
requires an LLM provider in `.forgeplan/config.yaml`; `--save` preserves its
output. FPF context is optional for the CLI and requires corpus ingestion.
Semantic search separately requires model weights and artifact vectors.
Marketplace agent plugins are not required for this shell walkthrough.
:::

## 1. Initialize a workspace

```bash
cd /path/to/your/project
forgeplan init -y
```

This creates `.forgeplan/` with empty `prds/`, `rfcs/`, `adrs/`, `evidence/`
directories and a `config.yaml`. The `-y` flag skips interactive prompts
(required for AI agents).

```bash
forgeplan health
# -> 0 artifacts, 0 blind spots. Workspace ready.
```

## 2. Route your task

Before creating an artifact, request a depth recommendation. Force keyword
routing here to avoid an unplanned LLM call:

```bash
forgeplan route "add user authentication" --level 0
```

Read the returned depth, matched triggers and pipeline rather than expecting
a fixed confidence or `Standard` result. Security-sensitive work may need a
deeper pipeline. Create the artifacts the recommendation calls for and set
their depth explicitly; routing does not change a newly created PRD's
default depth. This page illustrates a PRD; it does not replace required
RFC, ADR or SPEC work. See [Routing & Depth](/docs/methodology/routing/).

## 3. Shape - create the PRD

```bash
forgeplan new prd "User Authentication"
# Use the ID and path printed by this command.
```

The commands below use `PRD-001`, `EVID-001` and `NOTE-001` as example IDs;
substitute the IDs from your workspace.

Read the generated template, then prepare its body in an editing buffer
outside `.forgeplan/`, preserving the template headings. Fill the sections
required for its depth, including:

- **Executive Summary** — vision, problem and target users
- **Success Criteria** — measurable targets and how to measure them
- **Product Scope** — in scope and out of scope
- **User Journeys** — who does what and what the system returns
- **Functional Requirements** — `[Actor] can [capability]`, linked to journeys
- **Non-Functional Requirements** — metrics, conditions and measurement

Save the body without YAML frontmatter to `/tmp/auth-prd-body.md`, then sync it
through the CLI:

```bash
forgeplan update PRD-001 --body @/tmp/auth-prd-body.md
```

Use `forgeplan update PRD-001 --depth deep` if that is the routed depth.
Do not edit canonical `.forgeplan/` artifacts directly; `update` synchronizes
their file and index.

:::caution[Don't leave stubs]
A stub PRD with no Problem/Goals is technical debt. Fill it now or
`forgeplan delete PRD-001`. Half-filled PRDs pollute `forgeplan health`.
:::

## 4. Validate

```bash
forgeplan validate PRD-001
```

Continue only when the command reports no MUST errors. The number of SHOULD
warnings depends on the content. Fix reported sections or content rules and
re-run. Requirements describe WHAT, not implementation choices; the validator
checks for implementation leakage, but a PASS is not proof that the design
or implementation is correct.

## 5. Reason (ADI)

ADI generates alternatives and predictions for independent assessment.
The published harness calls for ADI on Standard+ work; it is not a substitute
for testing. Configure and authenticate an LLM provider first. HTTP providers
use the configured API-key environment variable; local subscription CLI
providers require an existing login and are explicit personal-development
opt-ins, not shared/CI deployments.

```bash
forgeplan reason PRD-001 --save
```

This is `reason`, not the task router. Without `--save`, the CLI prints the
analysis; with it, it creates a linked Note. Review the generated hypotheses
and verify their claims rather than treating LLM output as observed evidence.

Want FPF knowledge-base context injected into the prompt?

```bash
forgeplan reason PRD-001 --fpf --save
```

See [ADI Reasoning](/docs/methodology/adi/) for the full cycle.

## 6. Build

Implement the agreed feature and verify its acceptance criteria. Select the
methodology before coding: BMAD is for a new product; SPARC is for one feature
in an existing active system; enforced TDD requires an active PRD and an
active scenario-bearing SPEC. A planning artifact can be activated before
implementation; that activation is not evidence that the feature is shipped.

When the feature is done:

```bash
cargo test       # or your equivalent
cargo fmt
cargo check
```

All must be green before you activate.

## 7. Prove - create Evidence

```bash
forgeplan new evidence "Authentication acceptance test results"
# Use the returned evidence ID; do not claim a test result in the title before measuring it.
```

The evidence template already contains a **Structured Fields** section.
Prepare the complete evidence body outside `.forgeplan/`, replace its
measurement/result placeholders with observed commands, results, environment
and limitations, and set the structured fields to match those observations.
For a supporting test run in the same context, the block is:

```markdown
## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test
```

Missing or unrecognized verdicts and missing congruence declarations fail
closed to CL0 in 0.38.0 (a supporting base score of 1.0 becomes 0.1).
Do not choose CL3 just to raise the score: explain why the test context
matches the claim. For code-claiming evidence, include `base_sha`,
`result_sha` and `changed_paths` together so provenance can be checked
against the real git delta. See [Evidence & R_eff](/docs/methodology/evidence/).

Save the completed body to `/tmp/auth-evidence-body.md` and synchronize it:

```bash
forgeplan update EVID-001 --body @/tmp/auth-evidence-body.md
```

Link the evidence to the PRD:

```bash
forgeplan link EVID-001 PRD-001 --relation informs
```

## 8. Check the score

```bash
forgeplan score PRD-001
```

Use the score actually returned, not a target number to copy into Evidence.
R_eff uses the weakest current linked evidence; draft packs still count,
while superseded/deprecated packs do not. A current `refutes` verdict can
zero the result. If the score is low, inspect links, structured fields,
congruence and freshness. A positive score alone does not prove correctness.

## 9. Activate

```bash
forgeplan review PRD-001
# -> Review PASSED - ready to activate

forgeplan activate PRD-001
# -> draft → active
```

Activation reruns validation and can reject the transition. Resolve the
reported findings; do not use `update --status active` to bypass lifecycle
commands. In the agent harness, an independent reviewer/guardian supplies the
verdict and the main orchestrator activates; the author does not self-certify.

## 10. Verify health

```bash
forgeplan health
```

Inspect the actual counts and findings. This example creates a PRD, an
EvidencePack and, with `reason --save`, an ADI Note; other routed artifacts
add to the total. Do not expect a one-artifact workspace or a fixed health
verdict. Follow up on blind spots, unlinked artifacts and stale evidence.

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
4. forgeplan reason PRD-XXX --save  # LLM required; preserve ADI
5. code + test
6. forgeplan new evidence "..." + link + score
7. forgeplan review + activate
```

:::tip[Work isn't done until]
For a completed feature: requirements filled and validated, ADI captured for
Standard+ harness work, implementation independently checked, evidence linked,
R_eff inspected and the appropriate artifacts activated. An active plan is
not the same milestone as a delivered feature.
:::

## Next steps

- [First Artifact Tutorial](/docs/guides/first-artifact/) - 20-minute guided version with error recovery
- [Methodology Overview](/docs/methodology/overview/) - the 10 rules
- [CLI Reference](/docs/cli/) - every command documented
- [Configuration](/docs/getting-started/configuration/) - LLM providers, embeddings, git sync
- [Marketplace Overview](/docs/marketplace/overview/) - plugins for `/audit`, `/sprint`, `/fpf` and more
