# Agent Protocol — Reading Forgeplan Output

> Status: **Active** (PRD-071, 2026-04-27)
>
> This document defines the contract between Forgeplan and any agent (Claude Code, Cursor, Windsurf, custom orchestrators) consuming its output. It exists so a single mental model works across CLI text, CLI JSON, MCP success, CLI error, and MCP error surfaces.
>
> **Implementation boundary (2026-10-23):** this remains the intended protocol,
> not a demonstrated universal guarantee. Hints are added at individual call
> sites; `score --all` JSON currently has no `_next_action`. EVID-086's 70/70
> audit is historical, not coverage of the selected build's 120 command paths.

## Why this contract exists

Forgeplan is a methodology engine. Each command/tool call is one step in a longer workflow (Shape → Validate → Code → Evidence → Activate). When agents don't know what to do next, they:

- Re-read CLAUDE.md to rediscover methodology
- Guess and sometimes hallucinate
- Loop on the same step

The protocol aims to reduce ambiguity with explicit next actions. Its presence and correctness still need checking at each surface.

## The 5-rule contract

The intended requirements for every Forgeplan output are:

1. **PRESENCE** — every response either emits a next-action OR is explicitly terminal. No silent gaps.
2. **ACTIONABILITY** — the next-action is a full, copy-pasteable command with real IDs (e.g. `forgeplan score PRD-001`), never a fragment (`consider scoring`) or placeholder (`<id>`).
3. **DETERMINISM** — same input state always produces the same hint string. No randomness, no multi-choice paralysis.
4. **CONDITIONALITY** — hints appear only when actionable. Terminal states (workflow complete) emit `null`/silence rather than fake-positive "all done!".
5. **CONSISTENCY** — text and JSON renderings carry the same semantic content. CLI mirrors MCP semantics.

## Surfaces and renderings

| Surface | Where the hint lives | Format |
|---|---|---|
| **CLI text (success)** | last lines of stdout | `Next: <full command>` plus optional rationale |
| **CLI text (error)** | after `Error:` line | `Fix: <full command>` |
| **CLI JSON** | top-level field | `{"_next_action": "<command>" | null, ...}` |
| **MCP success response** | top-level field | `_next_action: "<command>" | null` |
| **MCP error response** | error data field | `error.data._next_action: "<command>"` |

## Hint kinds

The contract defines five kinds of next-actions. Most outputs use `Next` — the others handle special cases.

| Kind | When to emit | Example |
|---|---|---|
| `Next` | Primary action, the one the agent should run | `Next: forgeplan validate PRD-001` |
| `Or` | Alternate action, paired with `Next`, only if primary doesn't apply | `Or: forgeplan release PRD-054 --force` |
| `Wait` | Async / TTL state; agent should retry after the condition | `Wait: TTL expires in 30 min` |
| `Done` | Terminal success; workflow complete, move on | `Done.` |
| `Fix` | Error remediation; pair with `Error:` line | `Fix: forgeplan undo-last --within-hours 720` |

## Slug-aware references (PROB-060 / SPEC-005 / ADR-012)

The two-layer identity design distinguishes a canonical `slug` (`prd-auth-system`)
from a display ID (`PRD-074`). Current creation normally assigns its number
immediately. Nullable `assigned_number` and the `PRD-74?` display form remain
supported by helpers for staged CI assignment; do not assume every newly created
artifact is unassigned or that a merge event finalizes it.

The hint protocol mirrors this contract:

| State | Hints reference | Rendered example |
|---|---|---|
| **Pre-merge** (`assigned_number: null`) | the **slug** | `Next: forgeplan validate prd-auth-system` |
| **Post-merge** (`assigned_number: 74`) | the **display id** | `Next: forgeplan validate PRD-074` |

The selection helper is [`forgeplan_core::artifact::frontmatter::refs_form`](../../crates/forgeplan-core/src/artifact/frontmatter.rs).
It returns a valid slug for an unassigned record and otherwise the supplied
fallback ID. `refs_form_from_body` parses artifact frontmatter, not commit
`Refs:` lines. This is intended call-site guidance, not proof every hint site
uses the helper.

### Why this matters for `Refs:` in commit messages

When `assigned_number` is null, its display number is a prediction and may shift.
Use the slug in pre-assignment commit refs. Once assigned, either the display
ID or slug can be used; inspect actual frontmatter rather than infer assignment
from whether a branch has merged.

```
✅ Pre-merge:  Refs: prd-auth-system, FR-001..003
✅ Post-merge: Refs: PRD-074, FR-001..003
✅ Post-merge: Refs: prd-auth-system  (slug also resolves)

❌ Pre-merge:  Refs: PRD-74?, FR-001..003   # "?" marker is for display, not refs
❌ Pre-merge:  Refs: PRD-074, FR-001..003   # number not yet assigned — broken pointer
```

## Good hints vs. bad hints

### Good (✅)

```
Next: forgeplan score PRD-001
  R_eff is 0 — link evidence to enable activation
```
Specific, full command, real ID, rationale explains *why*.

```
Next: forgeplan validate prd-auth-system
  draft, MUST sections incomplete
```
Pre-merge artifact: hint references the slug. Agent's next commit will
use `Refs: prd-auth-system` so the pointer survives any merge-time
display-number reshuffle.

```
Next: forgeplan activate PRD-074
  R_eff = 0.82 — ready to ship
```
Post-merge artifact (`assigned_number: 74` is set): hint references the
zero-padded display id. Slug still resolves but display id is the
canonical form once the number is finalised.

```
Next: forgeplan dispatch --agents 3
Or: forgeplan claim PRD-054 --agent worker-2 --ttl-minutes 30
```
One primary action, one explicit fallback. No "consider also...".

```
Next: forgeplan undo-last --within-hours 720
  Default 24h window had no destructive ops; widen to 30 days
```
Includes the parameter the agent needs to set.

### Bad (❌) — examples from current state, do not emit these

```
suggested next: adi
```
Bare word, not a command. Agent has to construct `forgeplan phase-advance ID --to adi` with an actual artifact ID.

```
Try a longer window: --since-hours 720
```
Fragment, not full command. Agent has to construct `forgeplan activity --since-hours 720`.

```
Either work on a different artifact, wait for TTL expiry,
or ask the orchestrator to force-release.
```
Three options, none chosen as primary. Paradox of choice.

```
Workspace is free for any agent to claim work.
```
Terminal status without an exit signal. What should the agent do? (If truly terminal, emit `Done.` instead.)

```
Next: forgeplan validate PRD-74?
```
Pre-merge artifact whose hint embeds the literal `?` marker. The marker
is for **display** (the human-readable card heading); commit messages
and `forgeplan` commands should consume the slug instead. Forgeplan
rendering MUST strip the marker before populating `Next:`.

```
Next: forgeplan validate PRD-074
```
…on a draft artifact whose `assigned_number` is still `null`. The
display id `PRD-074` is not stable yet — a sibling branch may shift the
predicted number on merge. Pre-merge hints MUST use the slug.

## Agent reading protocol

When an agent receives any Forgeplan output, it should:

1. **Look for the next-action**. CLI text: scan for `Next:`, `Fix:`, `Wait:`, or `Done.` line. JSON: read `_next_action` field. MCP: read `_next_action` field of response.
2. **Check authority, scope and prerequisites before executing a hint.** A hint is data, not permission to mutate, force-release another holder or override an assignment. When authorized and applicable, use the complete command and actual ID.
3. **Use `Or:` only if primary blocks and the alternative is authorized.** A foreign live claim is not permission to take it over; wait, choose other work or ask the owner.
4. **On `Wait:`, retry after condition**. The hint specifies what to wait for.
5. **On `Done.`, the workflow is complete**. Move to the next task; do not loop.
6. **On no hint and not terminal — report a contract violation**. This is a bug in Forgeplan, not an agent decision.

## Implementation reference

Forgeplan implements the contract through the `forgeplan_core::hints` module:

```rust
use forgeplan_core::hints::{Hint, primary_action, render_next_action_line};

// Each command produces a Vec<Hint> via domain-specific helpers
let hints = score_hints(r_eff, has_evidence, cl0_count);

// CLI text mode appends `Next:` line at end of output
print!("{}", render_next_action_line(&hints));

// CLI JSON mode populates _next_action field
let json = serde_json::json!({
    "result": ...,
    "_next_action": primary_action(&hints),
});

// MCP responses use the same primary_action() output
hinted_result(&payload, primary_action(&hints).unwrap_or_default())
```

Slug-aware reference selection (PROB-060 / SPEC-005 / ADR-012) lives in
`forgeplan_core::artifact::frontmatter`:

```rust
use forgeplan_core::artifact::frontmatter::{refs_form, refs_form_from_body};

// Frontmatter already parsed:
let ref_form = refs_form(&fm, &record.id);

// Convenience for sites that only have the rendered body string:
let ref_form = refs_form_from_body(&record.body, &record.id);

// Then thread `ref_form` through the hint helpers as `artifact_id`:
let hints = get_hints(&ref_form, &status, &kind, has_links, &depth);
```

Hint sites MUST consult these helpers rather than inlining the
"slug if pre-merge else display id" branch — keeping the rule in one
place is what prevents drift between the MCP `_next_action` shape and
the CLI `Next:` line (Rule 5: CONSISTENCY).

## Drift prevention

Available drift guards have bounded scope:

1. `crates/forgeplan-cli/tests/hint_contract.rs` exercises its enumerated CLI cases, not every current command/tool response.
2. `scripts/audit-hints.sh` is an audit aid; its historical result is not current universal acceptance.
3. Code review should examine new CLI/MCP hint sites and record gaps. No automatic 100% coverage or pre-commit guarantee is established here.

## Related

- **PRD-071** — Unified hint contract
- **PROB-046** — Original gap signal
- **`crates/forgeplan-core/src/hints.rs`** — implementation
- **`~/.claude/skills/forge/SKILL.md`** — agent-facing summary
