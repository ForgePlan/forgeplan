---
title: forgeplan route
description: "Suggest depth level and artifact pipeline for a task description"
---

`forgeplan route` is the first command you run on any new task. Given a
natural-language description, it returns a **depth** (Tactical / Standard /
Deep / Critical), a **pipeline** (which artifacts to create, e.g. `PRD → RFC`
or `Epic → PRD[] → Spec[] → RFC[] → ADR[]`), a **confidence** score, and a
short list of **alternatives** in case the primary suggestion feels off.

Routing is rule-based by default (Level 0 — fast, deterministic, no LLM
required). Pass `--level 1` to get LLM-classified routing with explanations.
The goal is the same: stop you from over-engineering a one-hour fix, and
stop you from skipping the PRD on a week-long refactor.

## When to use

- Starting any new task — bug, feature, refactor, doc update
- Deciding whether you need a PRD, RFC, ADR, or just a Note
- Second-guessing yourself: "Is this really Tactical? Could it be Standard?"
- Onboarding — helps new contributors calibrate against the methodology
- Pre-flight check before a sprint — route each task, sum depths, verify capacity

## When NOT to use

- The task already has an artifact — use `forgeplan calibrate <ID>` to re-route instead
- You are mid-implementation — route is a planning tool, not an in-flight check
- For pure hotfix on `main` — you skip methodology anyway
- For housekeeping commits (dependency bump, lint fix) — just commit, no routing

## Usage

```text
forgeplan route [OPTIONS] <DESCRIPTION>
```

## Arguments

```text
  <DESCRIPTION>  Task description in natural language
```

## Options

```text
      --explain        Use LLM to explain the routing decision (deprecated, use --level 1)
      --level <LEVEL>  Routing level: 0 = keywords (default), 1 = LLM-classified
  -h, --help           Print help
  -V, --version        Print version
```

## Examples

### Example 1: Quick rule-based routing (default)

```bash
forgeplan route "add rate limiting to API"
```

Output:

```
Depth:      Standard
Pipeline:   PRD → RFC
Confidence: 88%
Rationale:  new capability, multiple components, reversible within a sprint
Alternatives:
  - Deep (if rate limiter is cross-service)
  - Tactical (if single middleware with existing library)
```

### Example 2: LLM-classified routing for nuanced tasks

```bash
forgeplan route --level 1 "refactor embedding pipeline to support BGE-M3"
```

Level 1 invokes the configured LLM (Gemini, GPT, Claude) to classify the
task against FPF heuristics. Slower (~2s) but handles ambiguous phrasing
and cross-cutting concerns better than the keyword matcher.

### Example 3: Trivial task — no artifact needed

```bash
forgeplan route "fix typo in README"
```

Output:

```
Depth:      Tactical
Pipeline:   (none — commit directly)
Confidence: 99%
```

You are allowed to skip the artifact. Just commit.

### Example 4: Critical task — full pipeline

```bash
forgeplan route "redesign artifact storage to use content-addressable hashing"
```

Output:

```
Depth:      Critical
Pipeline:   Epic → PRD[] → Spec[] → RFC[] → ADR[]
Confidence: 92%
Rationale:  irreversible, cross-cutting, affects all existing artifacts
```

Do **not** shortcut this. Create the Epic first.

## Output interpretation

- **Depth** — one of four levels. Drives quality gates: Tactical = no gates,
  Standard = Verification Gate, Deep = Adversarial Review, Critical = review + ADR
- **Pipeline** — ordered list of artifacts to create. Guideline, not contract —
  you may collapse steps if the project phase allows
- **Confidence** — 0-100%. Below 70% means the description is ambiguous; rerun
  with more detail or use `--level 1`
- **Alternatives** — two or three other plausible routes. Pick one if the
  primary does not match your intuition

Red flags:

- Confidence < 50% — description is too vague, add context ("component X", "user-facing", "affects DB schema")
- Route says Tactical for what feels like a week of work — add detail about blast radius
- Route says Critical for a one-liner — simplify the description

## Routing Skills Memory (v0.17+, PRD-040)

As of v0.17.0 (PRD-040, Scoring Intelligence) the router keeps a **routing
skills memory** — a rolling log of past depth predictions plus their
subsequently-observed accuracy. This memory decays on a **90-day exponential
window with a 30-day half-life**: a correct prediction you made yesterday
carries almost its full weight, a correct prediction from two months ago
carries roughly a quarter of its weight, and anything older than 90 days is
effectively forgotten.

The router uses this memory in two ways:

1. **Self-calibration.** Each time you route a task and later activate the
   resulting artifact, the router learns how accurate the predicted depth
   turned out to be for your workspace. Over time the rule engine biases
   toward depths that have been correct historically.
2. **Automatic Level 1 escalation.** If Level 0 (keyword rules) produces a
   depth where historical confidence has dropped **below 60%** for similar
   task shapes, the router automatically escalates to Level 1 (LLM
   classifier) instead of returning a low-confidence guess. You get a better
   answer without having to remember `--level 1` yourself.

### Why it matters

Routing is opinionated, and the opinions are wrong for some workspaces. A
team doing mostly infra work will get different depth distributions than a
team doing mostly product features. Routing Skills Memory lets the router
**adapt to your team's actual decision patterns** instead of shipping a
fixed ruleset that is right on average and wrong for you.

No configuration is required — the memory starts empty on a fresh
workspace and builds up automatically every time you `forgeplan activate`
an artifact that was previously routed. As of v0.18.0 there is no dedicated
`route --stats` flag yet; the memory is consulted internally by `route` and
only surfaces through the confidence score and alternatives list. If you
want to see whether the memory is influencing a decision, run
`forgeplan route --level 1` and read the LLM rationale — it references the
historical signal when relevant.

## How it fits the workflow

```
[route] → Shape → Validate → Reason → Code → Evidence → Activate
  ^
you are here
```

- **Before**: looking at a task in TODO.md or an issue tracker
- **After**: `forgeplan new prd|rfc|note` based on the suggested pipeline
- For Deep/Critical, follow up with `forgeplan reason` (ADI mandatory)

## See also

- [`forgeplan new`](/docs/cli/new/) — create the artifact suggested by route
- [`forgeplan reason`](/docs/cli/reason/) — required for Deep/Critical depth
- [`forgeplan calibrate`](/docs/cli/calibrate/) — re-route an existing artifact
- [`forgeplan health`](/docs/cli/health/) — session start check
- [Methodology: depth calibration](/docs/methodology/overview/)
