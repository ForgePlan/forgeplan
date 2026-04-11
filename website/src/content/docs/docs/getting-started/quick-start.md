---
title: Quick Start
description: "From zero to your first activated decision in 10 minutes."
---

This is the 10-minute walkthrough. If you want a slower, fully annotated
tutorial with error recovery, see [First Artifact Tutorial](/docs/guides/first-artifact/).

:::tip[Shortcut]
In Claude Code, Cursor, Windsurf and similar AI agents the entire workflow below is packaged as the `/forge` skill. Install with `forgeplan setup-skill` and run `/forge "your task description"`.
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
# forgeplan 0.18.0
```

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

## 3. Shape — create the PRD

```bash
forgeplan new prd "User Authentication"
# -> Created: PRD-001 at .forgeplan/prds/PRD-001-user-authentication.md
```

Open the file and fill the MUST sections:

- **Problem** — why does this exist?
- **Goals** — what does success look like? (SMART criteria)
- **Non-Goals** — what is explicitly out of scope?
- **Target Users** — who benefits?
- **Functional Requirements (FR)** — `[Actor] can [capability]`

:::caution[Don't leave stubs]
A stub PRD with no Problem/Goals is technical debt. Fill it now or
`forgeplan delete PRD-001`. Half-filled PRDs pollute `forgeplan health`.
:::

## 4. Validate

```bash
forgeplan validate PRD-001
```

Expected:
```
PRD-001: PASS ✓ (0 MUST errors, 2 SHOULD warnings)
```

If you see MUST errors, the sections are missing or empty. Fix them and
re-run. The validator also catches implementation leakage: writing
"Use JWT" in a requirement fails the gate — requirements describe WHAT,
not HOW.

## 5. Reason (ADI)

For Standard depth, ADI is recommended. For Deep and Critical, it is
mandatory:

```bash
forgeplan reason PRD-001
```

The router generates **3+ hypotheses**, derives predictions for each, and
scores them against any evidence already in the workspace. The output is
stored with the artifact so future readers can see how you arrived at the
decision.

Want FPF knowledge-base context injected into the prompt?

```bash
forgeplan reason PRD-001 --fpf
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

## 7. Prove — create Evidence

```bash
forgeplan new evidence "Auth tests — 12 pass, JWT validate 0.3ms"
# -> Created: EVID-001
```

Open the evidence file and add the **structured fields** block to the body:

```markdown
## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test
```

Without these fields, the R_eff parser assigns CL0 (0.9 penalty) and your
score collapses to near zero. This is the number one beginner mistake.

Link the evidence to the PRD:

```bash
forgeplan link EVID-001 PRD-001 --relation informs
```

## 8. Check the score

```bash
forgeplan score PRD-001
```

```
PRD-001: R_eff = 1.00 — Adequate
  EVID-001: supports, CL3, test -> score 1.0
```

If R_eff is 0.0, check: (a) is evidence linked? (b) does evidence body
contain the structured fields? See [Evidence & R_eff](/docs/methodology/evidence/).

## 9. Activate

```bash
forgeplan review PRD-001
# -> Review PASSED — ready to activate

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
Artifacts: 1 (1 active, 0 draft, 0 stale)
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

- [First Artifact Tutorial](/docs/guides/first-artifact/) — 20-minute guided version with error recovery
- [Methodology Overview](/docs/methodology/overview/) — the 10 rules
- [CLI Reference](/docs/cli/) — every command documented
- [Configuration](/docs/getting-started/configuration/) — LLM providers, embeddings, git sync
- [Marketplace Overview](/docs/marketplace/overview/) — plugins for `/audit`, `/sprint`, `/fpf` and more
