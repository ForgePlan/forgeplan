---
title: forgeplan_hypothesis_promote
description: "Promote a hypothesis along the verification state machine (parked → inferred → strong-inferred → verified; or any → refuted; or any → parked). Mutates the artifact body via file-first projection: updates the `hypothesis_state:` frontmatter field and appends an audit entry to the `## Lifecycle State` section. Returns a list of artifacts that informs / based_on the hypothesis (cascade) - informational only, not auto-bumped. Rejects illegal transitions with a structured error enumerating the allowed next states."
---

The one supported writer for the `kind: hypothesis` lifecycle. A hypothesis carries a
`hypothesis_state:` frontmatter field and this tool is the only thing that moves it.
[`forgeplan_activate`](/docs/mcp/forgeplan_activate/) deliberately refuses the kind and redirects
here, because a plain activate would flip `status` without touching `hypothesis_state` and leave
the two fields disagreeing.

Each call rewrites the artifact in three linked places:

1. `hypothesis_state:` in the frontmatter is overwritten with the target state.
2. `status` is derived from that target state in the same write, so the pair can never drift:
   `verified` becomes `active`, `refuted` becomes `deprecated`, and `parked` / `inferred` /
   `strong-inferred` all stay `draft`.
3. One audit line is appended to the `## Lifecycle State` body section - created on the spot if a
   legacy artifact predates it.

The mutation goes through the file-first projection helper (markdown first, then the store row),
and the whole handler runs under the workspace write lock so two agents promoting the same
hypothesis cannot lose an audit line.

**Category**: Other

## When an agent calls this

- **Evidence arrived.** A brownfield extraction pass parked a claim, then corroborating evidence
  turned up: walk one step forward, `parked` → `inferred` → `strong-inferred` → `verified`.
- **Counter-evidence landed.** `any` → `refuted` at any point; this also flips `status` to
  `deprecated`, and the response's cascade list names the downstream artifacts to revisit.
- **Signal was insufficient.** `any` → `parked` is the reversible hold, and `parked` → `inferred`
  later reopens the work.
- **Activation.** Promoting to `verified` is the only path that makes a hypothesis `active`.
  Calling [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) instead returns an error telling
  you to use this tool with `new_state=verified` (activate) or `new_state=refuted` (deprecate).
- **After the call.** Follow the returned `_next_action` and re-score what the hypothesis feeds via
  [`forgeplan_score`](/docs/mcp/forgeplan_score/).

## The verification state machine

The forward chain is strictly one step at a time. `parked`, `inferred` and `strong-inferred` are
working states; `verified` and `refuted` are terminals that absorb forward moves.

| State | Meaning | Legal next states |
|---|---|---|
| `parked` | On hold - nothing gathered yet, or deliberately paused. Reversible. | `inferred`, `parked`, `refuted` |
| `inferred` | Claim articulated with some signal, no corroborating evidence yet. The template default. | `parked`, `inferred`, `strong-inferred`, `refuted` |
| `strong-inferred` | One or more independent evidence items agree, but the bar for `verified` is not met. | `parked`, `strong-inferred`, `verified`, `refuted` |
| `verified` | Terminal. Downstream artifacts may treat the claim as load-bearing. | `parked`, `verified`, `refuted` |
| `refuted` | Terminal. Counter-evidence disproved the claim. | `parked`, `refuted` |

The rules behind that table:

- **Forward, one step.** `parked` → `inferred`, `inferred` → `strong-inferred`,
  `strong-inferred` → `verified`. Skipping a step (`parked` → `verified`) is rejected.
- **No downgrade.** `verified` → `inferred` and `verified` → `strong-inferred` are rejected - a
  verified claim is not walked back, a new hypothesis is created instead.
- **Two universal sinks.** `any` → `refuted` (counter-evidence can land at any time) and
  `any` → `parked` (hold the work) are always allowed.
- **Same to same** is legal in the state machine but short-circuited by this tool - see
  [Idempotent re-application](#idempotent-re-application).

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `hypothesis_id` | `string` | yes | Hypothesis artifact id or slug (`HYP-001`, `hyp-some-claim`). Must resolve to an artifact whose `kind` is `hypothesis`. |
| `new_state` | `string` | yes | Target state in kebab-case: `parked`, `inferred`, `strong-inferred`, `verified`, or `refuted`. Parsed case-insensitively after trimming, and `strong_inferred` / `stronginferred` are accepted aliases. |
| `evidence_refs` | `string[]` | no | Evidence artifact ids backing the move, normally `EVID-NNN`. Defaults to an empty list, which is legal for holds but not for `verified`. Each entry must match the id shape: one leading ASCII letter, then letters, digits, `-` or `_`. |
| `rationale` | `string` | yes | One-line reason for the audit trail, stored verbatim in the body. Embedded line breaks and control characters are rejected. |
| `workspace` | `string` | no | Explicit workspace directory for this call, so an agent in a git worktree can target its own `.forgeplan/` instead of the server's launch directory. Absolute or tilde-expanded path. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::HypothesisPromoteParams`_

## What it does

1. Acquires the workspace write lock, so concurrent promotes serialise instead of racing on the
   audit-line append. A held lock returns an error asking you to retry.
2. Parses `new_state` before touching the store. An unknown state fails fast with the valid set.
3. Resolves `hypothesis_id` (id or slug) to a canonical id and loads the record.
4. Enforces the kind gate: anything that is not `kind: hypothesis` is rejected, with a hint to
   inspect it via [`forgeplan_get`](/docs/mcp/forgeplan_get/) or find a valid target via
   [`forgeplan_list`](/docs/mcp/forgeplan_list/).
5. Reads the current state from the `hypothesis_state:` frontmatter field. A missing, null, non-string
   or unparseable value defaults to `parked` - the safe choice, since every forward move is legal
   from there.
6. Checks the transition against the matrix above. An illegal move returns an error that lists the
   allowed next states for the current one.
7. Short-circuits a no-op (`old == new`) into an `unchanged: true` envelope without writing an
   audit line - the state machine permits same-to-same for idempotent re-application, but a
   duplicate audit entry would be misleading.
8. Validates `rationale` (no control characters, no Unicode line or paragraph separators) and every
   `evidence_refs` entry (id shape only).
9. For `verified` only: requires at least one evidence ref and resolves every one of them against
   the store. This is the gate that stops a stub hypothesis from reaching `status: active` with
   fabricated evidence.
10. Rewrites the frontmatter and appends the audit line, then persists through the projection
    helper.
11. Enumerates the cascade: artifacts pointing at this hypothesis with an `informs` or `based_on`
    relation. Informational only - nothing downstream is re-scored or re-activated for you.

## Returns

A successful forward promotion returns the old and new state, the refs and rationale you supplied,
and the cascade:

```json
{
  "_next_action": "Next: re-score downstream artifacts — `forgeplan_score HYP-001`.",
  "cascade": [
    "EVID-001",
    "PRD-001"
  ],
  "evidence_refs": [],
  "id": "HYP-001",
  "new_state": "inferred",
  "old_state": "parked",
  "rationale": "initial signal from the refund policy grep"
}
```

The audit line written into `## Lifecycle State` for that call reads:

```text
- 2026-09-16: parked → inferred (refs: none; rationale: initial signal from the refund policy grep)
```

Empty `evidence_refs` render as `none`; a blank `rationale` renders as `(no rationale provided)`.

## Idempotent re-application

Promoting to the state a hypothesis is already in writes nothing and reports it:

```json
{
  "_next_action": "Done. — `HYP-001` already in `inferred`.",
  "cascade": [],
  "id": "HYP-001",
  "new_state": "inferred",
  "old_state": "inferred",
  "unchanged": true
}
```

Note the empty cascade in that envelope: the no-op path returns before cascade enumeration runs,
so an empty list there means "nothing happened", not "nothing depends on this".

## Rejected calls

Every rejection is a proper MCP error whose text ends with a `_next_action` fix line.

| Rejection | Error text |
|---|---|
| Unknown state | `unknown hypothesis state:` followed by the value you passed |
| Unknown artifact | `Artifact 'HYP-999' not found.` |
| Wrong kind | `` artifact `PRD-001` is kind=`prd`, expected `hypothesis` `` |
| Illegal transition | `` illegal hypothesis transition for `HYP-001`: `inferred` → `verified` `` |
| Multi-line rationale | `rationale contains control characters or Unicode line separators` |
| Malformed evidence id | `` evidence_ref `not an id` is not a valid artifact id `` |
| No evidence for `verified` | `Cannot promote to verified with no evidence_refs` |
| Missing evidence for `verified` | `` evidence_ref `EVID-999` does not exist in workspace `` |

The illegal-transition error carries the allowed set in its hint, for example from `inferred`:

```text
Allowed next states from `inferred`: parked, inferred, strong-inferred, refuted.
```

## Safety notes

- **This is a body mutation.** It is not read-only, and it is not marked idempotent in the MCP
  annotations - a successful promote changes the artifact on disk.
- **`status` follows `hypothesis_state`.** Do not fight it with
  [`forgeplan_update`](/docs/mcp/forgeplan_update/); the `hypothesis-state-status-consistent`
  validation rule (severity `should`) flags a hypothesis whose two fields disagree.
- **`refuted` deprecates.** Promoting to `refuted` derives `status: deprecated`, which is how the
  rest of the workspace learns the claim is dead.
- **The cascade is advice.** The tool never auto-bumps downstream artifacts, by design - the agent
  decides what to re-score.

## Example invocation

```json
{
  "hypothesis_id": "HYP-001",
  "new_state": "strong-inferred",
  "evidence_refs": ["EVID-001"],
  "rationale": "grep evidence corroborates the gate"
}
```

Refuting a claim, which needs no forward step:

```json
{
  "hypothesis_id": "HYP-001",
  "new_state": "refuted",
  "evidence_refs": ["EVID-007"],
  "rationale": "integration test shows the gate is bypassed for refunds under 100"
}
```

Holding a claim that lacks signal, with no evidence and the rationale still mandatory:

```json
{
  "hypothesis_id": "HYP-004",
  "new_state": "parked",
  "rationale": "no second source found; revisit after the billing interview"
}
```

## Typical sequence

```text
forgeplan_hypothesis_status                    ← where is everything right now
forgeplan_hypothesis_promote (parked → inferred)
forgeplan_hypothesis_promote (inferred → strong-inferred)
forgeplan_hypothesis_promote (strong-inferred → verified)
forgeplan_score HYP-001                        ← the returned `_next_action`
```

## Related commands

- [`forgeplan graph --brownfield-only`](/docs/cli/graph/) - renders hypothesis nodes coloured by
  their current state.
- [`forgeplan anomalies`](/docs/cli/anomalies/) - flags `untriangulated_hypothesis` when a claim has
  fewer than two distinct evidence references.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_hypothesis_status`](/docs/mcp/forgeplan_hypothesis_status/) - the read side of this state machine
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) - re-score the cascade after a promotion
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) - the generic lifecycle gate that rejects hypotheses
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) - attach the evidence a promotion cites
