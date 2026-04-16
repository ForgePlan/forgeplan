---
title: First Artifact Tutorial
description: "A 20-minute hands-on walkthrough: create, validate, reason, prove, and activate your first PRD — with common errors and how to fix them."
---

This is the long version of the [Quick Start](/docs/getting-started/quick-start/).
It covers the same seven steps but with more explanation, realistic file
contents, and **the errors you will hit at each step with how to fix them**.

Plan for 20 minutes. You will end up with a real activated decision that
trains you on the full Forgeplan cycle.

## What you will build

A PRD for a small, real feature: **"Add a `--dry-run` flag to `forgeplan
new` so users can preview what would be created without writing files."**

It is intentionally small — routing will probably say `Standard`, which
means we hit every step except the Epic/Spec tiers. Perfect training load.

```mermaid
flowchart LR
  S1["1. Init"] --> S2["2. Route"] --> S3["3. Shape"]
  S3 --> S4["4. Validate"] --> S5["5. ADI"]
  S5 --> S6["6. Build"] --> S7["7. Evidence"]
  S7 --> S8["8. Score"] --> S9["9. Activate"]
  S9 --> S10["10. Health ✓"]
```

## Prerequisites

- Forgeplan installed (`forgeplan --version`)
- A directory where you can create a workspace (throwaway is fine)
- Optional: a `GEMINI_API_KEY` or other LLM provider for ADI reasoning

## Step 1 — Initialize the workspace

```bash
mkdir ~/forgeplan-tutorial && cd ~/forgeplan-tutorial
forgeplan init -y
```

You should see:

```
Initialized .forgeplan/ workspace at /Users/you/forgeplan-tutorial/.forgeplan
  Created: prds/, rfcs/, adrs/, evidence/, notes/, problems/, ...
  Created: config.yaml
Ready.
```

Confirm:

```bash
forgeplan health
```

```
Project Health
  Total artifacts: 0
  Blind spots: 0
  Orphans: 0
  Stale: 0
  Status: OK — empty workspace
```

### Error you might hit

**`Error: .forgeplan/ already exists`** — you ran `init` in an existing
workspace. Either `cd` elsewhere or `rm -rf .forgeplan` (only on a
throwaway tutorial directory — never on a real project without exporting
first: `forgeplan export --output backup.json`).

## Step 2 — Route the task

```bash
forgeplan route "add --dry-run flag to forgeplan new for preview"
```

Expected output:

```
Task: add --dry-run flag to forgeplan new for preview
Depth: Standard
Pipeline: PRD → RFC
Confidence: 82%
Signals:
  + new feature (not a fix)
  + CLI UX surface change
  + multiple possible implementations
Recommendation:
  1. forgeplan new prd "CLI dry-run flag"
  2. Fill MUST sections (Problem, Goals, FR)
  3. forgeplan reason PRD-XXX  (recommended at Standard)
```

If the router says `Tactical`, override and treat it as Standard anyway —
we want to practice the full cycle. See
[Routing & Depth](/docs/methodology/routing/) for the decision tree.

## Step 3 — Shape: create the PRD

```bash
forgeplan new prd "CLI dry-run flag"
```

```
Created: PRD-001 at .forgeplan/prds/PRD-001-cli-dry-run-flag.md
```

Open the file in your editor. You will see a template with section
headers. Fill the MUST sections:

```markdown
# PRD-001: CLI dry-run flag

## Problem

Users who run `forgeplan new prd "..."` on a shared workspace cannot
preview what files would be created before committing. Mistakes
(wrong title, wrong ID collision) require manual cleanup.

## Goals

- G1: User can see the full file path and content of what `new` would
  create without the files being written to disk
- G2: `--dry-run` output is suitable for piping into review tools
- G3: No change to behavior when the flag is absent

## Non-Goals

- Not a full "simulation mode" — only covers the `new` command
- Not a rollback mechanism — files that were created without `--dry-run`
  stay created

## Target Users

CLI users creating artifacts in shared or production workspaces, and AI
agents that want to preview their output before committing.

## Functional Requirements

- FR1: User can pass `--dry-run` to `forgeplan new <kind> "title"`
- FR2: User can see the exact file path that would be created
- FR3: User can see the templated file content on stdout
- FR4: User can rely on exit code 0 when `--dry-run` would succeed
  and non-zero when validation would fail
```

Notice: no mention of "use clap" or "emit JSON" or any specific
implementation. That is rule 3 — FRs describe capabilities, not
implementations. See [Methodology Overview](/docs/methodology/overview/).

## Step 4 — Validate

```bash
forgeplan validate PRD-001
```

Best case:

```
PRD-001: PASS ✓
  MUST: 0 errors
  SHOULD: 1 warning (density: Problem section is terse)
```

### Errors you will probably hit

**`MUST error: Problem section missing`** — you forgot a required header.
Aliases work: `## Motivation`, `## Background`, `## Problem Statement` all
count as Problem. Add one.

**`MUST error: implementation leakage in FR2`** — you wrote something like
"Use JSON output with serde". Rewrite as "User can see the exact file path
and content". The validator flags library names and technology choices in
requirements.

**`MUST error: no functional requirements`** — the `## Functional
Requirements` section exists but has no bullets. Add at least one FR with
the `[Actor] can [capability]` pattern.

**`MUST error: vague goal "system should be fast"`** — validator caught an
unmeasurable claim. Rewrite with numbers or remove.

Re-run `forgeplan validate PRD-001` after each fix.

## Step 5 — Reason (ADI)

```bash
forgeplan reason PRD-001
```

If you have an LLM configured (`.forgeplan/config.yaml` has a provider
and key), you will see something like:

```
ADI cycle for PRD-001
─────────────────────
Abduction — 3 hypotheses:
  H1: Single --dry-run flag that short-circuits file write
  H2: Separate `forgeplan preview new` command
  H3: Interactive confirmation prompt (y/n before write)

Deduction — predictions per hypothesis:
  H1:
    - Minimal code change, 1 branch in `new` command
    - Reusable: same flag can extend to other write commands
    - No new command surface to document
  H2:
    - Discoverable via `forgeplan --help`
    - Duplicates template rendering logic (or extracts it)
    - Doubles the surface area users must learn
  H3:
    - Forces interactivity, breaks AI-agent usage
    - No way to see output before committing
    - Violates `-y` non-interactive contract

Induction — evidence check:
  H1: supports — aligns with existing flag patterns (e.g. `init -y`)
  H2: weakens — duplication + discoverability win does not offset cost
  H3: refutes — breaks AI agent workflow (see MUST in CLAUDE.md)

Recommendation: H1
Confidence: 0.87
```

No LLM configured? You will get a template asking you to fill in the
hypotheses manually. That still counts — the value is in thinking through
alternatives, not in LLM output.

See [ADI Reasoning](/docs/methodology/adi/) for why this step exists.

### Error

**`Error: no LLM provider configured`** — open `.forgeplan/config.yaml`
and add a provider block. For the tutorial you can skip this and write
the 3 hypotheses directly into the PRD body under a `## Reasoning`
section.

## Step 6 — Build

Write the code. For this tutorial, pretend you implemented H1 and
wrote tests. In real Forgeplan work you would:

```bash
cargo test      # or npm test, pytest, go test, ...
cargo fmt
cargo check     # 0 warnings, 0 errors
```

All three must pass before you create evidence claiming they do. If
`cargo check` has warnings, fix them — Forgeplan's own CLAUDE.md rule
is "0 warnings, 0 errors" on every commit.

## Step 7 — Prove: create Evidence

```bash
forgeplan new evidence "CLI dry-run — 8 unit tests pass, flag works end-to-end"
```

```
Created: EVID-001 at .forgeplan/evidence/EVID-001-cli-dry-run....md
```

Open the file and add the **structured fields** block to the body. This
is the single most important part of the tutorial:

```markdown
## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Details

- 8 unit tests pass in `tests/cli_dry_run.rs`
- Manual smoke test: `forgeplan new prd "Test" --dry-run` prints template
  without creating the file
- Verified exit code 0 on success and 1 on template error
```

Link it to the PRD:

```bash
forgeplan link EVID-001 PRD-001 --relation informs
```

### The #1 mistake

**Forgetting the structured fields.** If you just write prose in the
evidence body and skip the `verdict: supports / congruence_level: 3 /
evidence_type: test` lines, the R_eff parser falls back to CL0 with a 0.9
penalty. Your score will be near zero even though the evidence is strong.
Always include the three fields.

## Step 8 — Check the score

```bash
forgeplan score PRD-001
```

Expected:

```
PRD-001: CLI dry-run flag
  R_eff = 1.00 — Adequate
  Evidence:
    EVID-001: supports, CL3, test → score 1.0
```

### What if R_eff = 0.0?

1. `forgeplan list evidence` — is EVID-001 in the workspace?
2. `cat .forgeplan/evidence/EVID-001-*.md | grep -E "verdict|congruence_level|evidence_type"`
   — all three fields present?
3. `forgeplan link EVID-001 PRD-001 --relation informs` — was the link created?

See [Evidence & R_eff](/docs/methodology/evidence/) for the full formula.

## Step 9 — Activate

```bash
forgeplan review PRD-001
```

```
Reviewing PRD-001...
  Validation: PASS ✓
  Evidence: 1 linked (R_eff = 1.00)
  Status: ready to activate
```

```bash
forgeplan activate PRD-001
```

```
PRD-001: draft → active
  Validation gate: PASS
  R_eff preserved: 1.00
```

If the validation gate fails here, the transition is rejected. The most
common cause is that you edited the PRD after step 4 and introduced a
MUST violation. Re-run `forgeplan validate PRD-001` and fix it.

## Step 10 — Verify

```bash
forgeplan health
```

```
Project Health
  Total artifacts: 2 (PRD-001, EVID-001)
  Active: 1
  Draft: 1 (EVID-001 — evidence packs stay draft)
  Blind spots: 0
  Orphans: 0
  Stale: 0
  Status: HEALTHY
```

Congratulations — you have a fully traced decision with measurable trust.

## What you learned

- **Route first.** You did not start coding immediately; you asked
  Forgeplan what depth the task needed.
- **Shape before code.** The PRD captured the `why` and `what` before a
  single line of implementation.
- **Validate early.** The validator caught implementation leakage and
  missing sections before they became habits.
- **Reason with ADI.** You generated alternatives before committing to
  one, which is how you avoid "I wish we had thought of that" moments.
- **Prove with evidence.** R_eff is not a decoration — it is the number
  that tells you whether you should trust your own decision.
- **Activate only when ready.** The gate prevents "active PRDs with no
  code" — a false promise to future readers.

## Next steps

- Run the full cycle on a real task in your main project
- Read [Methodology Overview](/docs/methodology/overview/) for the 10 rules
- Explore [CLI Reference](/docs/cli/) — every command is documented
- Dive into [Artifact Lifecycle](/docs/methodology/lifecycle/) to learn
  `supersede`, `deprecate`, `renew`, and `reopen`
- Check [Configuration](/docs/getting-started/configuration/) to set up
  an LLM provider for smarter routing and ADI

## Troubleshooting cheat sheet

| Symptom | Cause | Fix |
|---------|-------|-----|
| `MUST error: Problem missing` | No `## Problem` / `## Motivation` section | Add one of the aliases |
| `implementation leakage in FR` | Named a library/tech in a requirement | Rewrite as `[Actor] can [capability]` |
| `R_eff = 0.0` on scored artifact | Evidence missing `verdict` / `congruence_level` / `evidence_type` | Add the three fields to the evidence body |
| `activate` rejected | Validation gate fails after edits | Re-run `validate`, fix MUST errors |
| `Error: no LLM provider` on `reason` | No key in `.forgeplan/config.yaml` | Add provider block or write hypotheses manually |
| `health` shows blind spot | Active artifact with no linked evidence | Create evidence and `forgeplan link` |
| `stale` warning on fresh artifact | `valid_until` too short | Set realistic 90-180 day expiry |
