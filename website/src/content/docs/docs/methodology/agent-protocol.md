---
title: Hint Contract - Reading Forgeplan Output
description: How agents read Forgeplan's deterministic next-action hints across CLI text, JSON, and MCP responses
---

> Status: **Active** since Forgeplan v0.25.0 (PRD-071, 2026-04-27)

Forgeplan introduced a five-rule hint contract in v0.25.0. This page explains the intended protocol and the actual CLI/MCP boundaries in **0.38.0**. Hints help choose the next operation; they are not executable authority or a replacement for the task's evidence and review requirements.

## Why This Matters

Forgeplan is a methodology engine. Each command/tool call is one step in a longer workflow (Shape → Validate → Code → Evidence → Activate). When agents don't know what to do next, they:

- Re-read CLAUDE.md to rediscover methodology
- Guess and sometimes hallucinate
- Loop on the same step

Hints reduce repeated discovery, but do not guarantee that every response is a complete command. In 0.38.0 some values are prose, some include placeholders, and some commands omit a marker. Inspect the returned result and the actual command/tool schema.

## The 5-Rule Contract

These are the **design rules**, not a claim of complete coverage by every 0.38.0 response:

1. **PRESENCE** — provide a next action or an explicit terminal signal.
2. **ACTIONABILITY** — prefer a full command with actual artifact references; detect unresolved placeholders before execution.
3. **DETERMINISM** — workflow hints are derived from state. LLM-generated analysis and routing are not made deterministic by the hint protocol.
4. **CONDITIONALITY** — stop suggesting operations when this tool's workflow has nothing further to do.
5. **CONSISTENCY** — preserve the intended next step across surfaces, without assuming identical transport or argument syntax.

## The 5 Hint Markers

| Marker | When emitted | Agent action |
|---|---|---|
| `Next: <full command>` | Recommended next step | Check scope, prerequisites, and actual arguments, then execute if authorized |
| `Or: <command>` | Alternative if primary blocks | Choose the applicable alternative; do not run both blindly |
| `Wait: <condition>` | Async/TTL condition | Wait for the condition rather than retrying immediately |
| `Done.` | No further action in this tool's workflow | Stop that loop; not proof the user's entire task is complete |
| `Fix: <command>` | Error remediation | Read the error and confirm the remediation is relevant and authorized |

## Where to read the hint

| Surface | Location | Format |
|---|---|---|
| **CLI text (success)** | Usually final stdout lines | `Next:`, `Done.`, or other markers |
| **CLI text (error)** | Error output, generally stderr | Read the error and any `Fix:` / `Wait:` text |
| **CLI JSON** | Object payload when provided | `_next_action` may be a command, prose, or null; array outputs have no top-level field |
| **MCP success** | Tool result content, often serialized JSON text | Parse content and read `_next_action` when present; it may contain prose or MCP notation |
| **MCP error** | `isError` tool-result content or protocol error | Hinted errors may contain a text `_next_action:` line; `error.data._next_action` is not a universal envelope |

**Special cases**: `forgeplan list --json` and `forgeplan tree --json` preserve bare-array stdout; read their hint on **stderr**. Other array/no-result paths may omit `_next_action`. Null means no suggested action in that payload, not proof that all acceptance criteria passed.

## Good hints vs. bad hints

### Good ✅

Illustrative hint shapes below, not a captured run:

```
Next: forgeplan score PRD-001
  Inspect the score and its factors before choosing the next operation.
```
Specific, full command, real ID, rationale explains *why*.

```
Next: forgeplan dispatch --agents 3
Or: forgeplan claim PRD-054 --agent worker-2 --ttl-minutes 30
```
One primary action, one explicit fallback.

```
Error: Direct status change to 'active' is not allowed.
Fix: forgeplan activate PRD-001
```
Error has clear, executable remediation.

### Bad ❌ (pre-v0.25.0 patterns - should never appear in v0.25.0+ output)

This heading preserves the original contract expectation. The patterns remain anti-patterns, but 0.38.0 does not guarantee their absence.

```
suggested next: adi
```
Bare word, not a command. Agent has to guess.

```
Try a longer window: --since-hours 720
```
Fragment, not full command.

```
Either work on a different artifact, wait for TTL expiry,
or ask the orchestrator to force-release.
```
Three options, none chosen as primary. Paradox of choice.

## Agent reading protocol

When an agent receives Forgeplan output:

1. **Read success/failure and the substantive result first**, including warnings and partial failures.
2. **Locate the hint** in CLI stdout/stderr or parsed MCP content. Do not assume a protocol-level field.
3. **Resolve the actual operation**. CLI examples and MCP notation are not interchangeable schemas. Use the real ID/slug returned for the artifact; replace template titles and other unresolved placeholders with task-specific values before execution.
4. **Check scope and authorization**. A hint does not authorize `--force`, deletion, sending data to an LLM, or writing in another worktree.
5. **Use one applicable primary action**, with `Or:` only if needed; on `Wait:`, wait for the condition.
6. **On `Done.`, stop this loop**, then assess the user's remaining acceptance criteria.
7. **If a required hint is missing or malformed**, report the gap. Do not fabricate output or repeatedly run the same operation hoping the next step changes.

## What NOT to Do

1. **Do not execute hint text as an uninspected shell script.**
2. **Do not combine primary and fallback blindly.**
3. **Do not treat `Done.` as evidence that code or tests were checked.**
4. **Do not pass literal `EVID-NNN`/`<title>` placeholders** — create the record, then use its returned reference.
5. **Do not force-release another agent's claim merely because a hint suggests it.** Claims coordinate ownership; the orchestrator handles an abandoned claim deliberately.

## Practical workflow patterns

### Pattern A: Shape → Validate → Activate

Illustrative command sequence, not a transcript. PRD-042 and EVID-017 stand for references actually returned by `new`.

```bash
forgeplan route "add OAuth login" --level 0
forgeplan new prd "OAuth login support"
# Fill all applicable sections in oauth-prd.md.
forgeplan update PRD-042 --body @oauth-prd.md
forgeplan validate PRD-042
# Complete design, implementation, and required independent reviews.
# Create and fill evidence with real results and structured fields.
forgeplan new evidence "OAuth verification results"
forgeplan update EVID-017 --body @oauth-evidence.md
forgeplan link EVID-017 PRD-042 --relation informs
forgeplan score PRD-042
forgeplan review PRD-042
# Under the marketplace policy, guardian PASS is required before this call.
forgeplan activate PRD-042
```

`validate` passing does not imply the evidence gate passed. `activate` may suggest parent reconciliation rather than print `Done.`. The full marketplace cycle also requires ADI Evidence at Standard+; this abbreviated sequence is not a way to skip that gate.

### Pattern B: Recovery from Error

If activation reports "No evidence linked", repeating validation alone cannot supply evidence. Create a pack, fill its result and fields, link it to the decision, inspect `score`, and re-run `review`. Avoid `--force` as routine remediation.

### Pattern C: Multi-Agent Dispatch

```bash
forgeplan dispatch --agents 3
# The orchestrator selects a returned assignment; IDs below are illustrative.
forgeplan claim PRD-054 --agent worker-1 --ttl-minutes 30
```

If worker-2 already holds the claim, inspect `forgeplan claims` and coordinate or choose unclaimed work. Do not automatically execute `release --force`. A claim is a soft coordination signal, not an enforcement lock on every artifact operation.

## Drift Prevention

The release contains integration coverage in `crates/forgeplan-cli/tests/hint_contract.rs` and the audit script `scripts/audit-hints.sh`. Their existence does not establish complete marker coverage, valid prerequisites, or semantic equivalence of every CLI/MCP hint. Coverage figures from earlier audits are historical, not a current 0.38.0 measurement.

## Quick reference card

```
┌─────────────────────────────────────────────────────────┐
│  FORGEPLAN HINT CONTRACT (v0.25.0+)                     │
├─────────────────────────────────────────────────────────┤
│  Next:  <command>   → inspect, then act in scope           │
│  Or:    <command>   → applicable fallback                  │
│  Wait:  <condition> → wait for condition                   │
│  Done.              → stop this tool-workflow loop         │
│  Fix:   <command>   → inspect error and remediation        │
├─────────────────────────────────────────────────────────┤
│  Read from: stdout (text), _next_action (JSON/MCP)      │
│  Special:   list/tree --json → hint on stderr           │
└─────────────────────────────────────────────────────────┘
```

## For Plugin Users

The published **`forgeplan-workflow` 1.23.1** contains `/forge-cycle` and the methodology skill. **`fpl-skills` 1.75.1** provides `/smith`, `/gate-check`, and `/wrap`; **`agents-pro` 1.30.0** provides smith and guardian. These are separate plugins, not capabilities automatically installed by `forgeplan setup-skill`.

The main session reads the cycle and executes its operations. Smith recommends a Plan; authors create artifacts and reviewers produce Evidence; guardian reviews the full chain and returns PASS / CONCERNS / BLOCKER. **The orchestrator activates**, not smith or guardian. `/gate-check` is a between-stage gate, not the activation command.

Artifact `phase` markers are advisory. MCP `forgeplan_guard` checks the separate session-phase machine with `target_session_phase`; it does not verify artifact phase completion, tests, or guardian approval. A helpful `_next_action` never substitutes for those checks.

## Related

- **Forgeplan v0.25.0 release** - first version shipping the contract ([CHANGELOG](/docs/changelog/))
- **Published marketplace workflow** — agent-side policy layered on the CLI/MCP operations
- **Lifecycle model** ([read more](/docs/methodology/lifecycle/)) - hints integrate with status transitions (draft → active → terminal)
