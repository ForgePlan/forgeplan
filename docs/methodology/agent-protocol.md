# Agent Protocol — Reading Forgeplan Output

> Reference: **Forgeplan 0.38.0**. PRD-071 defines the intended hint discipline; this page distinguishes that target from the shipped transport and agent safety rules.
>
> Hints guide agents and humans through a workflow. They are not executable authority, proof of a passed gate, or a substitute for checking the current state and permissions.

## Why this contract exists

Forgeplan is a methodology engine. Each command/tool call is one step in a longer workflow (Shape → Validate → Code → Evidence → Activate). When agents don't know what to do next, they:

- Re-read CLAUDE.md to rediscover methodology
- Guess and sometimes hallucinate
- Loop on the same step

The shared hint helpers select a primary next action. In 0.38.0 the convention is not universal: actions may contain prose or placeholders, errors may lack a hint, and a missing hint does not itself prove completion.

## The 5-rule contract

The **design target** is presence, actionability, determinism, conditionality and consistency:

1. Supply a primary action or an explicit terminal outcome.
2. Prefer a complete command with existing IDs; distinguish unknown values that the caller must fill.
3. Choose one primary action deterministically.
4. Emit advice only when applicable; do not manufacture an “all done” command.
5. Keep CLI and MCP advice semantically consistent.

Consumers must tolerate the actual forms below. For example, MCP `reason` and `guard` can return explanatory prose in `_next_action`; `score_hints` can contain `EVID-NNN` for evidence not yet created. Do not pass arbitrary hint text directly to a shell.

## Surfaces and renderings

| Surface | Where the hint lives | Format |
|---|---|---|
| **CLI text (success)** | output text | `Next: ...` where emitted; other advisory lines may precede it |
| **CLI text (error)** | error text | `Fix: ...` / `Wait: ...` on paths that provide remediation |
| **CLI JSON** | command-specific payload | `_next_action` where implemented; not every command supports JSON |
| **MCP success response** | JSON encoded in MCP text content | `_next_action` where supplied; may be prose, command or null |
| **MCP tool error (`err_hinted`)** | text content with `isError: true` | message followed by literal `_next_action: ...`, not `error.data._next_action` |

## Hint kinds

The contract defines five kinds of next-actions. Most outputs use `Next` — the others handle special cases.

| Kind | When to emit | Example |
|---|---|---|
| `Next` | Primary action, the one the agent should run | `Next: forgeplan validate PRD-001` |
| `Or` | Alternative advice; does not grant permission to override another agent | `Or: forgeplan list` |
| `Wait` | Async / TTL state; agent should retry after the condition | `Wait: TTL expires in 30 min` |
| `Done` | Terminal success; workflow complete, move on | `Done.` |
| `Fix` | Error remediation; pair with `Error:` line | `Fix: forgeplan undo-last --within-hours 720` |

## Slug-aware references (PROB-060 / SPEC-005 / ADR-012)

Forgeplan carries slug and numeric identity fields. **Do not infer assignment from lifecycle status or from whether a branch merged.** Default `new` creation in 0.38.0 assigns a number immediately. The frontmatter helper preserves an explicit `assigned_number: null` for deferred-assignment workflows; CI bot setup is external to that creation path.

The `refs_form` helper selects:

| State | Hints reference | Rendered example |
|---|---|---|
| Deferred assignment (`assigned_number` absent/null, valid slug present) | **slug** | `Next: forgeplan validate prd-auth-system` |
| Assigned (`assigned_number: 74`) | **fallback ID** supplied by caller | `Next: forgeplan validate PRD-074` |
| Legacy / invalid or missing slug | **fallback ID** | `Next: forgeplan validate PRD-018` |

The selection lives in [`forgeplan_core::artifact::frontmatter::refs_form`](../../crates/forgeplan-core/src/artifact/frontmatter.rs). Not every hint callsite uses that helper; CLI `new`, for example, uses its allocated ID directly. The `?` marker is a display convention for a prediction, not an identifier to paste into commands.

### Why this matters for `Refs:` in commit messages

For deliberately deferred assignment, prefer the valid slug until a number is assigned. For the shipped immediate-assignment path, use the returned ID or valid slug; merging is not the condition that makes the returned ID resolvable. Legacy IDs remain usable without slug migration. See [ID-ASSIGNMENT.ru.md](ID-ASSIGNMENT.ru.md) for current limits and historical rollout plans.

```
✅ Deferred assignment: Refs: prd-auth-system, FR-001..003
✅ Assigned/default creation: Refs: PRD-074, FR-001..003
✅ Assigned: Refs: prd-auth-system  (valid slug also resolves)

❌ Refs: PRD-74?, FR-001..003   # display marker is not an identifier
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
For deliberately deferred assignment, use the slug in refs so the reference does not depend on a predicted display number.

```
Next: forgeplan activate PRD-074
  R_eff = 0.82 — review activation prerequisites
```
`assigned_number: 74` is set (including immediate assignment by default). Slug lookup still works; a positive score alone does not establish every activation prerequisite.

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
The `?` marker is for display; use the valid slug or assigned ID. A hint containing `PRD-074` is not wrong merely because its branch is unmerged: the default creation path assigns immediately. Check the artifact's actual identity fields.

## Agent reading protocol

When receiving Forgeplan output:

1. Read the result and error status before the hint. For MCP, parse JSON text content when present; handle text-only tool errors separately.
2. Look for `Next:`, `Fix:`, `Wait:`, `Done.` or `_next_action`, but do not assume one is present or shell-ready.
3. Check the proposed operation against the user's task, permissions, ownership and current artifact state. Replace unresolved placeholders only with verified values. A hint is not authorization for `--force`, release of a peer's claim or lifecycle bypass.
4. Prefer the primary applicable advice; use an alternative only when justified. On a claim conflict, wait or ask the owning orchestrator to resolve it; do not take the claim.
5. Honour a concrete wait condition. On an explicit terminal result, stop that workflow; no hint alone is not a terminal signal.
6. Report missing/malformed or unsafe hints as protocol defects and choose a supported read-only diagnostic where possible.

## Implementation reference

Forgeplan implements the contract through the `forgeplan_core::hints` module:

```rust
use forgeplan_core::hints::{primary_action, render_next_action_line, score_hints};

// Illustrative fragment: inputs come from the command's score result.
let hints = score_hints("PRD-001", r_eff, has_evidence, cl0_count);

// CLI text mode appends `Next:` line at end of output
print!("{}", render_next_action_line(&hints));

// CLI JSON mode populates _next_action field
let json = serde_json::json!({
    "result": ...,
    "_next_action": primary_action(&hints),
});

// MCP handlers attach an action to their JSON payload as appropriate.
// Recoverable errors may instead use err_hinted(), which emits text content.
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

New hint sites should reuse reference helpers rather than duplicating identity logic, and preserve the safety checks in the reading protocol. This is a contributor requirement, not a claim that every shipped callsite already complies.

## Drift prevention

The repository contains `crates/forgeplan-cli/tests/hint_contract.rs` and `scripts/audit-hints.sh` to detect gaps, plus review guidance for new command/tool surfaces. Their existence is not proof that every possible runtime response satisfies the five rules; keep observed release exceptions explicit.

## Related

- **PRD-071** — Unified hint contract
- **PROB-046** — Original gap signal
- **`crates/forgeplan-core/src/hints.rs`** — implementation
- **`~/.claude/skills/forge/SKILL.md`** — agent-facing summary
