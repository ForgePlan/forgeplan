# Agent Protocol — Reading Forgeplan Output

> Status: **Active** (PRD-071, 2026-04-27)
>
> This document defines the contract between Forgeplan and any agent (Claude Code, Cursor, Windsurf, custom orchestrators) consuming its output. It exists so a single mental model works across CLI text, CLI JSON, MCP success, CLI error, and MCP error surfaces.

## Why this contract exists

Forgeplan is a methodology engine. Each command/tool call is one step in a longer workflow (Shape → Validate → Code → Evidence → Activate). When agents don't know what to do next, they:

- Re-read CLAUDE.md to rediscover methodology
- Guess and sometimes hallucinate
- Loop on the same step

Each of these costs tokens and risks correctness. The contract eliminates ambiguity by guaranteeing every output carries an explicit, deterministic next-action.

## The 5-rule contract

Every Forgeplan output, regardless of surface, satisfies these:

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

Forgeplan uses **two-layer artifact identity**: a canonical `slug`
(`prd-auth-system`) that never changes, and a derived `display id`
(`PRD-074`) that is finalised by a CI bot when the artifact's branch
merges to `dev`. While the bot has not yet flipped `assigned_number`
from `null` to a number, the artifact is **pre-merge** and the
display id carries a `?` marker (`PRD-74?`) signalling "predicted, not
final".

The hint protocol mirrors this contract:

| State | Hints reference | Rendered example |
|---|---|---|
| **Pre-merge** (`assigned_number: null`) | the **slug** | `Next: forgeplan validate prd-auth-system` |
| **Post-merge** (`assigned_number: 74`) | the **display id** | `Next: forgeplan validate PRD-074` |

The selection happens in [`forgeplan_core::artifact::frontmatter::refs_form`](../../crates/forgeplan-core/src/artifact/frontmatter.rs);
all CLI / MCP hint sites consult that helper so the choice stays in
exactly one place. Agents do not need to re-implement the rule —
`Next:` lines and `_next_action` JSON fields already carry the right
form. Just **paste the command verbatim**.

### Why this matters for `Refs:` in commit messages

Because `assigned_number` is write-once and only flips at merge, a
commit body that pins `Refs: PRD-074` **before** the CI bot has
assigned it points to nothing — the number is still predicted and may
shift if a sibling branch merges first. Pre-merge commits MUST use
the slug; post-merge they MAY use either form (the resolver maps both
to the same artifact).

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
Bare word, not a command. Agent has to guess `forgeplan phase-advance --to adi`.

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
2. **Execute primary if present**. If `Next:` or `Fix:` — execute the command exactly as written. Do not paraphrase, do not substitute placeholders (there shouldn't be any), do not split into multiple commands.
3. **Use `Or:` only if primary blocks**. The primary `Next:` is the recommended path. Fall back to `Or:` only when primary fails or doesn't apply (e.g. claim held by another agent → `Or: --force`).
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

The contract is enforced by:

1. **Integration test** `tests/hint_contract.rs` — runs every CLI command + asserts every response has `Next:` line or explicit terminal status.
2. **Audit script** `scripts/audit-hints.sh` — produces coverage metric, runs in CI.
3. **Code review checklist** — any new CLI command or MCP tool without a hint fails review.

## Related

- **PRD-071** — Unified hint contract
- **PROB-046** — Original gap signal
- **`crates/forgeplan-core/src/hints.rs`** — implementation
- **`~/.claude/skills/forge/SKILL.md`** — agent-facing summary
