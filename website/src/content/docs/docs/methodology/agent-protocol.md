---
title: Hint Contract — Reading Forgeplan Output
description: How agents read Forgeplan's deterministic next-action hints across CLI text, JSON, and MCP responses
---

> Status: **Active** since Forgeplan v0.25.0 (PRD-071, 2026-04-27)

Forgeplan v0.25.0 introduced a **unified 5-rule hint contract** so any agent (Claude Code, Cursor, Windsurf, custom orchestrators) can read its output without re-discovering the methodology each time. This page is the canonical agent-facing reference.

## Why This Matters

Forgeplan is a methodology engine. Each command/tool call is one step in a longer workflow (Shape → Validate → Code → Evidence → Activate). When agents don't know what to do next, they:

- Re-read CLAUDE.md to rediscover methodology
- Guess and sometimes hallucinate
- Loop on the same step

Each costs tokens and risks correctness. The contract eliminates ambiguity by guaranteeing every output carries an explicit, deterministic next-action.

## The 5-Rule Contract

Every Forgeplan output, regardless of surface, satisfies these:

1. **PRESENCE** — every response either emits a next-action OR is explicitly terminal. No silent gaps.
2. **ACTIONABILITY** — the next-action is a full, copy-pasteable command with real IDs, never a fragment or placeholder.
3. **DETERMINISM** — same input state always produces the same hint string. No randomness.
4. **CONDITIONALITY** — hints appear only when actionable. Terminal states emit `Done.` rather than fake "all done!".
5. **CONSISTENCY** — text and JSON renderings carry the same semantic content. CLI mirrors MCP semantics.

## The 5 Hint Markers

| Marker | When emitted | Agent action |
|---|---|---|
| `Next: <full command>` | Primary action — recommended next step | Execute exactly as written |
| `Or: <command>` | Alternate when primary blocks (e.g. claim held) | Use only if primary fails |
| `Wait: <condition>` | Async/TTL state | Retry after the condition |
| `Done.` | Workflow complete (terminal) | Move on, do not loop |
| `Fix: <command>` | Error remediation (paired with `Error:`) | Run the fix command immediately |

## Where to Read the Hint

| Surface | Location | Format |
|---|---|---|
| **CLI text (success)** | last lines of stdout | `Next: <full command>` plus optional rationale |
| **CLI text (error)** | after `Error:` line | `Fix: <full command>` |
| **CLI JSON** | top-level field | `{"_next_action": "<command>" \| null, ...}` |
| **MCP success** | top-level field | `_next_action: "<command>" \| null` |
| **MCP error** | error data field | `error.data._next_action: "<command>"` |

**Special case**: `forgeplan list --json` and `forgeplan tree --json` preserve bare-array stdout (backward compat for `jq '.[]'` consumers). The hint is emitted to **stderr** in JSON mode.

## Good Hints vs. Bad Hints

### Good ✅

```
Next: forgeplan score PRD-001
  R_eff is 0 — link evidence to enable activation
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

### Bad ❌ (pre-v0.25.0 patterns — should never appear in v0.25.0+ output)

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

## Agent Reading Protocol

When an agent receives any Forgeplan output:

1. **Look for the next-action**.
   - CLI text: scan for `Next:`, `Fix:`, `Wait:`, or `Done.` line
   - CLI JSON: read `_next_action` field (or stderr `Next:` for list/tree)
   - MCP: read `_next_action` field of response
2. **Execute primary if present**.
   - If `Next:` or `Fix:` — execute the command **exactly as written**
   - Do not paraphrase, do not substitute placeholders
   - Do not split into multiple commands
3. **Use `Or:` only if primary blocks**.
4. **On `Wait:`, retry after condition**.
5. **On `Done.`, the workflow is complete** — move to next task, do not loop.
6. **On no hint and not terminal — report a contract violation**. This is a bug in Forgeplan. Do not improvise.

## What NOT to Do

1. **Don't paraphrase the hint** — full command is given for a reason
2. **Don't combine `Next:` + `Or:`** in same call — pick one
3. **Don't ignore `Done.`** — explicit terminal signal
4. **Don't substitute `EVID-NNN` placeholders** — first run `forgeplan_new evidence`, then use the real ID
5. **Don't panic on `Wait:`** — async/TTL is normal; just retry

## Practical Workflow Patterns

### Pattern A: Shape → Validate → Activate

```
forgeplan_route("add OAuth login")
  → Next: forgeplan new prd "<title>"

forgeplan_new(kind: "prd", title: "OAuth login support")
  → Next: forgeplan validate PRD-042

forgeplan_validate("PRD-042")
  → Next: forgeplan activate PRD-042   (if PASS)
  → Fix: forgeplan validate PRD-042    (if errors)

forgeplan_activate("PRD-042")
  → Done.
```

### Pattern B: Recovery from Error

```
forgeplan_activate("PRD-042")
  → Error: No evidence linked
  → Fix: forgeplan validate PRD-042

forgeplan_validate("PRD-042")
  → Result: PASS
  → Next: forgeplan score PRD-042
```

### Pattern C: Multi-Agent Dispatch

```
forgeplan_dispatch(--agents 3)
  → Next: forgeplan claim PRD-054 --agent worker-1 --ttl 30
  → Or:   forgeplan list --status draft

forgeplan_claim("PRD-054")
  → Error: Already held by worker-2
  → Or: forgeplan release PRD-054 --force
```

## Drift Prevention

The contract is enforced by:

1. **Integration test** `crates/forgeplan-cli/tests/hint_contract.rs` — 36 tests, fails CI if any command lacks contract marker
2. **Audit script** `scripts/audit-hints.sh` — coverage metric, target 100%, currently 100% (70/70 commands)

## Quick Reference Card

```
┌─────────────────────────────────────────────────────────┐
│  FORGEPLAN HINT CONTRACT (v0.25.0+)                     │
├─────────────────────────────────────────────────────────┤
│  Next:  <command>   → execute as-is                     │
│  Or:    <command>   → fallback if primary blocks        │
│  Wait:  <condition> → retry after condition             │
│  Done.              → workflow complete, move on        │
│  Fix:   <command>   → error remediation (with Error:)   │
├─────────────────────────────────────────────────────────┤
│  Read from: stdout (text), _next_action (JSON/MCP)      │
│  Special:   list/tree --json → hint on stderr           │
└─────────────────────────────────────────────────────────┘
```

## For Plugin Users

The marketplace plugin **`forgeplan-workflow` v1.5.0+** teaches Claude Code agents to read these markers automatically. After installing:

```
/plugin marketplace update ForgePlan-marketplace
```

Your agent will read `Next:`/`Fix:` hints across `/forge-cycle`, the `forge-advisor` agent, and the methodology skill — no manual configuration needed.

## Related

- **Forgeplan v0.25.0 release** — first version shipping the contract ([CHANGELOG](/docs/changelog/))
- **Marketplace plugin forgeplan-workflow v1.5.0** — agent-side awareness layer
- **Lifecycle model** ([read more](/docs/methodology/lifecycle/)) — hints integrate with status transitions (draft → active → terminal)
