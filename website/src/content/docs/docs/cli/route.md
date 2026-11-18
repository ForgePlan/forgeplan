---
title: forgeplan route
description: "Suggest depth level and artifact pipeline for a task description"
---

`forgeplan route` is the first command you run on any new task. Given a
natural-language description, it returns a **depth** (Tactical / Standard /
Deep / Critical), a **pipeline** (which artifacts to create, e.g. `PRD → RFC`
or `Epic → PRD[] → Spec[] → RFC[] → ADR[]`), a **confidence** score, and a
short list of **alternatives** in case the primary suggestion feels off.

Omitting `--level` tries configured LLM routing and falls back to keyword rules.
An automatic Deep result can escalate to Level 2 reasoning. Use `--level 0`
without `--explain` for rule-only routing; adding `--explain` can invoke a configured LLM.
Captured help's “0 = keywords (default)” text
does not fully describe this handler. Routing also updates methodology session state.

## When to use

- Starting any new task - bug, feature, refactor, doc update
- Deciding whether you need a PRD, RFC, ADR, or just a Note
- Second-guessing yourself: "Is this really Tactical? Could it be Standard?"
- Onboarding - helps new contributors calibrate against the methodology
- Pre-flight check before a sprint - route each task, sum depths, verify capacity

## When NOT to use

- The task already has an artifact - use `forgeplan calibrate <ID>` to re-route instead
- You are mid-implementation - route is a planning tool, not an in-flight check
- For an approved emergency procedure - follow it; routing does not authorize bypassing project gates or committing directly to `main`.
- For housekeeping already covered by a project workflow - follow that workflow.

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

### Example 1: Explicit rule-based routing

```bash
forgeplan route --level 0 "add rate limiting to API"
```

Illustrative explanation, not a measured accuracy or runtime transcript:

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

Level 1 tries the configured LLM with rule-based fallback. Latency and quality
depend on the provider; no fixed duration or accuracy improvement is guaranteed.

### Example 3: Trivial task - no artifact needed

```bash
forgeplan route "fix typo in README"
```

Illustrative outcome; actual routing depends on matched signals:

```
Depth:      Tactical
Pipeline:   (none - commit directly)
Confidence: 99%
```

The recommendation does not override the project's artifact, review or commit requirements.

### Example 4: Critical task - full pipeline

```bash
forgeplan route "redesign artifact storage to use content-addressable hashing"
```

Illustrative planning scenario, not guaranteed command output:

```
Depth:      Critical
Pipeline:   Epic → PRD[] → Spec[] → RFC[] → ADR[]
Confidence: 92%
Rationale:  irreversible, cross-cutting, affects all existing artifacts
```

Do **not** shortcut this. Create the Epic first.

## Output interpretation

- **Depth** - a recommendation. Core modes are Note, Tactical, Standard and Deep;
  `critical` is a Deep alias. Project gate requirements remain separate authority.
- **Pipeline** - ordered list of artifacts to create. Guideline, not contract -
  you may collapse steps if the project phase allows
- **Confidence** - 0-100%. Below 70% means the description is ambiguous; rerun
  with more detail or use `--level 1`
- **Alternatives** - two or three other plausible routes. Pick one if the
  primary does not match your intuition

Red flags:

- Confidence < 50% - description is too vague, add context ("component X", "user-facing", "affects DB schema")
- Route says Tactical for what feels like a week of work - add detail about blast radius
- Route says Critical for a one-liner - simplify the description

## Routing Skills Memory (v0.17+, PRD-040)

The core supports `RoutingSkill` values explicitly supplied by a caller.
Confidence combines success rate, usage weight and exponential age decay,
with a 90-day half-life and a 0.1 floor; it does not forget entries at 90 days.

The selected CLI's rule-only path calls `route(description)` with an empty
skill list. This handler does not load persistent routing history, learn it
on activation, or escalate at a historical-confidence threshold.
Do not rely on automatic personalization from ordinary `route`/`activate` calls.

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

- [`forgeplan new`](/docs/cli/new/) - create the artifact suggested by route
- [`forgeplan reason`](/docs/cli/reason/) - required for Deep/Critical depth
- [`forgeplan calibrate`](/docs/cli/calibrate/) - re-route an existing artifact
- [`forgeplan health`](/docs/cli/health/) - session start check
- [Methodology: depth calibration](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan route --help`.

```text
Suggest depth level and artifact pipeline for a task description

Usage: forgeplan route [OPTIONS] <DESCRIPTION>

Arguments:
  <DESCRIPTION>  Task description in natural language

Options:
      --explain        Optional: use LLM to explain the routing decision (deprecated, use --level 1)
      --level <LEVEL>  Routing level: 0 = keywords (default), 1 = LLM-classified
  -h, --help           Print help
  -V, --version        Print version
```
<!-- forgeplan-runtime:end -->
