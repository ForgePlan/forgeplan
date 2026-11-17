---
title: forgeplan_hypothesis_promote
description: "Promote a hypothesis along the verification state machine (parked → inferred → strong-inferred → verified; or any → refuted; or any → parked). Mutates the artifact body via file-first projection: updates the `hypothesis_state:` frontmatter field and appends an audit entry to the `## Lifecycle State` section. Returns a list of artifacts that informs / based_on the hypothesis (cascade) - informational only, not auto-bumped. Rejects illegal transitions with a structured error enumerating the allowed next states."
---

The dedicated writer for the `kind: hypothesis` lifecycle updates `hypothesis_state:`
and derives the corresponding artifact status together.
[`forgeplan_activate`](/docs/mcp/forgeplan_activate/) deliberately refuses the kind and redirects
here, because a plain activate would flip `status` without touching `hypothesis_state` and leave
the two fields disagreeing.

A state-changing call rewrites the artifact in three linked places; a same-state call writes nothing:

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
- **After the call.** Inspect `cascade` and decide which affected artifacts need [`forgeplan_score`](/docs/mcp/forgeplan_score/). `_next_action` is advisory; its literal hint names the hypothesis itself, not each cascade member.

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

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `evidence_refs` | `array<string>` | no | IDs of evidence artifacts (typically `EVID-NNN`) backing the promotion. May be empty (e.g. parked-for-hold) but the rationale is still mandatory. Schema default: `[]`. |
| `hypothesis_id` | `string` | yes | Hypothesis artifact id or slug (`HYP-001` / `hyp-some-claim`). Must resolve to an artifact whose `kind == "hypothesis"`. |
| `new_state` | `string` | yes | Target lifecycle state in kebab-case: `parked`, `inferred`, `strong-inferred`, `verified`, or `refuted`. |
| `rationale` | `string` | yes | Free-form one-line rationale for the audit trail in `## Lifecycle State`. Stored verbatim in the body, sanitised for hint emission. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## What it does

1. Acquires the workspace write lock, parses `new_state`, resolves the hypothesis ID/slug and requires `kind: hypothesis`.
2. Reads `hypothesis_state` from frontmatter. A missing, null, non-string or unparseable value defaults to `parked`; the only forward step from there is `inferred`, not a direct jump to `verified`.
3. Checks the transition matrix. Same-state calls return `unchanged: true` with an empty cascade before rationale/ref validation; they do not update an audit line or repair an inconsistent stored status.
4. For a real change, rejects control characters and Unicode line/paragraph separators in `rationale`. Every reference must be an ASCII ID-like token starting with a letter. `rationale` is a required string (an empty string is accepted); only omitted `evidence_refs` defaults to `[]`. The no-op shortcut does not waive schema-required keys.
5. For a change to `verified`, requires at least one reference and checks that every referenced artifact resolves and exists. **0.38.0 limit:** this checks existence, not `kind: evidence`, verdict, congruence or a positive score. It does not create evidence links for you.
6. Updates the frontmatter/status and appends an audit entry through the projection helper. Enumerates incoming `informs` / `based_on` sources as `cascade`; no downstream artifact is auto-promoted or re-scored. Enumeration failure is non-fatal and can return an empty cascade.

## Returns

A state-changing success contains `id`, `old_state`, `new_state`, the supplied `evidence_refs` and `rationale`, `cascade`, and `_next_action`. This handler does not add workspace trace fields. The cascade names sources of incoming `informs` / `based_on` edges; it is not a count of completed follow-up actions.

For example, a `parked` → `inferred` change can append:

```text
- 2026-09-16: parked → inferred (refs: none; rationale: initial signal from the refund policy grep)
```

Empty references render as `none`; a blank rationale renders as `(no rationale provided)`. A no-op has the different payload documented below and omits the supplied rationale/refs.

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

Transition, kind and input refusals are MCP error text, often with an advisory `_next_action` line. Store/projection failures can use other error text; there is no universal structured error object.

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

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "hypothesis_id": "HYP-001",
  "new_state": "parked",
  "rationale": "Awaiting an independent source."
}
```

## Typical sequence

1. Read `forgeplan_hypothesis_status` and the hypothesis.
2. Record the supported one-step transitions: `parked` → `inferred` → `strong-inferred`.
3. For `verified`, supply existing references, for example `evidence_refs: ["EVID-001"]`, and a truthful rationale. Create/link evidence separately where the workflow requires it.
4. Inspect the returned `cascade`, then explicitly score/review the affected artifacts. Do not execute `_next_action` as a shell command.

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
