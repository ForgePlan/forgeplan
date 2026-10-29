---
name: forgeplan-methodology
description: "Forgeplan structured engineering workflow — plan features, create PRD/RFC/ADR, check project health, route tasks to correct depth, review artifacts, think before coding. Triggers: forge, forgeplan, route task, create prd, check health, review artifact, activate, lifecycle, R_eff, evidence. Not for: simple bug fixes, formatting, trivial changes."
argument-hint: "[task description, artifact ID, or 'health'/'status']"
---

# Forgeplan — Think Before You Build

You are a **forgeplan methodology expert**. Forgeplan is an MCP-first structured engineering
tool — always prefer `forgeplan_*` MCP tools over manual markdown edits.

**When to use**: any non-trivial engineering task. New features, architecture changes, new
modules, cross-team work. When user says "plan", "think about", "create prd", "what should
I document", "check project health".

**When NOT to use**: obvious bug fixes, typo fixes, formatting, trivial refactors.

---

## MCP tools — canonical operations

Every artifact mutation flows through MCP tools, not direct file edits. Tools return
`_next_action` hints — follow them to stay on the methodology path (Shape → Validate →
Code → Evidence → Activate).

| Tool | Purpose | CLI equivalent |
|---|---|---|
| `forgeplan_health` | Project health: gaps, risks, blind spots, next actions | `forgeplan health` |
| `forgeplan_route` | Depth + pipeline; MCP may use configured LLM classification with keyword fallback | `forgeplan route` (`--level 0` forces keywords; `--level 1` requests LLM) |
| `forgeplan_new` | Create artifact from template with auto-ID | `forgeplan new` |
| `forgeplan_validate` | Check completeness against schema rules | `forgeplan validate` |
| `forgeplan_review` | Lifecycle checklist: ready to activate? | `forgeplan review` |
| `forgeplan_activate` | Draft → Active (with validation gate) | `forgeplan activate` |
| `forgeplan_get` | Read full artifact by ID | `forgeplan get` |
| `forgeplan_update` | Modify metadata or body | `forgeplan update` |
| `forgeplan_search` | Find related decisions by keyword (semantic) | `forgeplan search` |
| `forgeplan_link` | Connect artifacts with typed relationships | `forgeplan link` |
| `forgeplan_list` | Browse artifacts with filters | `forgeplan list` |
| `forgeplan_score` | R_eff quality score (weakest-link, evidence-based) | `forgeplan score` |
| `forgeplan_reason` | ADI cycle — 3+ hypotheses → deduction → induction | `forgeplan reason` |
| `forgeplan_supersede` | Active → Superseded with replacement link | `forgeplan supersede` |
| `forgeplan_deprecate` | → Deprecated with reason | `forgeplan deprecate` |

---
The table describes the full MCP profile; the read-only profile exposes no
artifact mutations. The call notation below is pseudocode: send JSON named
arguments matching the connected tool schema, including the real title/IDs.
CLI help describes keywords as the default, but the matching handler can
auto-select configured LLM routing and escalate Deep results. Use explicit
`--level 0` when deterministic keyword routing is required.


## Core loop — every non-trivial task

```
forgeplan_health()              ← session start, orient
  ↓
forgeplan_route("task")         ← always route before coding
  ↓ (if Standard+)
forgeplan_new(kind, title)      ← Shape
  ↓
[fill MUST sections immediately — never leave stubs]
  ↓
forgeplan_validate(id)          ← PASS (0 MUST errors)
  ↓ (if Deep+)
forgeplan_reason(id)            ← ADI: 3+ hypotheses
  ↓
[Code → Audit → Fix → Test → Fmt → Lint → Verify]
  ↓
forgeplan_new(kind: "evidence", title: "Observed result")
[fill the evidence body with actual results and structured fields]
forgeplan_link(source: EVID, target: artifact, relation: "informs")
  ↓
forgeplan_activate(id)          ← draft → active
```

---

## Depth calibration — route first, always

| Situation | Depth | Artifacts | ADI |
|---|---|---|:---:|
| Fix typo, update config, trivial refactor | **Tactical** | Nothing or Note | — |
| Feature 1-3 days, has a choice | **Standard** | PRD → RFC | recommended |
| Irreversible, 1-2 weeks | **Deep** | PRD → Spec → RFC → ADR | **required** |
| Cross-team, strategic | **Critical** | Epic → PRD[] → Spec[] → RFC[] → ADR[] | **required + review** |

Critical is a methodology label here; the current core parses it as Deep.
The durations are sizing examples, not a runtime timing guarantee.

Automatic escalation triggers (in `forgeplan_route`):
- `security`, `auth`, `compliance` → Deep+
- `breaking change`, `migration` → Deep+
- `cross-team`, `multi-service` → Standard+
- `irreversible`, `cannot undo` → Deep+

If user disagrees with routing, offer alternatives — don't force the depth.

---

## Artifact MUST sections

Validator blocks activation on MUST failures. The table summarizes current
section checks, not the entire validation contract; depth and domain add rules.
Aliases are accepted.

| Kind | MUST sections | Aliases |
|---|---|---|
| **PRD** | Problem, Goals, Non-Goals, Functional Requirements, Related Artifacts; Target Users at Standard/Deep; additional Deep/domain rules | Motivation = Problem; Out of Scope = Non-Goals; Target Audience = Target Users; Success Criteria = Goals |
| **RFC** | Summary, Motivation, Proposed Direction; Risks at Deep | Options Considered and Implementation Phases are SHOULD checks |
| **ADR** | Context, Decision, Consequences; Invariants and Rollback Plan at Deep | — |
| **Epic** | Vision, Outcomes, Children, Phases, Progress | Artifacts = Children |
| **Spec** | Summary, API/Data Model/Contract **or** Requirements/Behavioral Contract, Related | Structural and behavioral forms are alternatives, not cumulative requirements |

Notes and Problems have base metadata checks, not the PRD/RFC/ADR/Epic/Spec
section rule sets. Brownfield kinds have their own required sections and
source/verification checks; this table is not a complete kind list.

---

## Critical gotcha — Evidence structured fields

Evidence **body MUST** contain three structured fields. Missing or unrecognized
`verdict`, or missing/invalid congruence information, fails closed to CL0
(penalty 0.9). A supports pack then scores 0.1; the aggregate also depends
on the other current evidence.

```markdown
## Structured Fields

verdict: supports            # supports / weakens / refutes
congruence_level: 3          # CL3 = same context (best), CL0 = opposed (worst)
evidence_type: measurement   # measurement / test / benchmark / audit
```

Then link to the artifact being supported:

```
forgeplan_link(source: "EVID-001", target: "PRD-001", relation: "informs")
```

Link relations: `informs`, `based_on`, `supersedes`, `contradicts`, `refines`.

---

## R_eff scoring — weakest link, never average

- `R_eff = min(evidence_scores)` over current evidence; superseded/deprecated packs are excluded, drafts still count, no eligible packs → 0
- CL penalties: CL3=0.0, CL2=0.1, CL1=0.4, CL0=0.9
- Evidence decay: past `valid_until` → score drops to 0.1
- Derived display categories: Stub, Shaped, Validated, Evidenced, Activated; these are not persisted lifecycle statuses

One CL0 or expired evidence tanks the whole score. This is intentional — forces honesty.

The formula above summarizes the flat evidence population. The explicit score
report also evaluates dependencies and evidence-type modifiers (benchmark 0.1,
audit 0.2); measurement/test have no type modifier. Record scores and the
recursive report can therefore differ. These are heuristics, not probabilities.

---

## Lifecycle — state machine

```
draft ──review──► draft (if MUST failures)
draft ──activate──► active (validation + applicable activation gates)
active ──supersede──► superseded (+ link to replacement)  [TERMINAL]
active ──deprecate──► deprecated (+ reason)               [TERMINAL]
active ──(overdue valid_until)──► listed as stale (no automatic status rewrite)
stale ──renew──► active (extend valid_until)
stale ──reopen──► deprecated + NEW draft
```

The stored transition guard permits stale→active renewal, but an overdue
record can still be stored as active; detection alone does not make
active→active renewal legal. Reopen accepts active or stale and creates the
new draft before deprecating the original. Renew/reopen are CLI operations,
not tool names in the captured full MCP catalog.

Rule: **supersede, don't delete.** History is the source of why-decisions-changed.

---

## Red lines — never do

1. **Never activate without evidence** — R_eff must be > 0
2. **Never leave PRD stubs** — fill MUST sections immediately after `forgeplan_new`
3. **Never skip routing** — always `forgeplan_route` before coding non-trivial tasks
4. **Never commit directly to main/dev** — always feature branch → PR → merge
5. **Never create PR before** Code → Audit → Fix → Test → Fmt → Lint → Verify pipeline

---

## Section router — deeper dives

The installed bundle is a single `SKILL.md`; `setup-skill` does not install a
`sections/` tree. In a Forgeplan source checkout, read the relevant file:

| User asks about... | Source-checkout reference |
|---|---|
| Workflow, process, steps, "how do I start" | `docs/methodology/HOW-TO-USE.md` |
| PRD, RFC, ADR, Evidence (specific artifact) | `docs/methodology/ARTIFACT-MODEL.md` |
| Depth, routing, tactical vs deep | `docs/methodology/DEPTH-CALIBRATION.md` |
| Scoring, R_eff, congruence, decay, CL levels | `docs/methodology/EVIDENCE-PROTOCOL.md` |
| Quality gates, adversarial review, audits | `docs/methodology/QUALITY-GATES.md` |

Outside that checkout, use https://forgeplan.dev/ or the connected tool schema
and matching `forgeplan <command> --help`; do not assume those local files exist.

---

## Cross-session memory

Forgeplan persists context between sessions. It is **not** a key-value store —
you save a sentence, and you find it by searching its text:

- **Store**: `forgeplan remember "<text>" --category <fact|convention|procedure|insight>`
- **Search**: `forgeplan recall "<query>"` — substring match over title and body
- **List**: `forgeplan recall` — no query lists up to 10 memories by default; use `--limit <N>` for a larger result set
- **Promote**: `forgeplan promote <mem-id> --kind note` — turn a memory into an artifact

The id is derived from the text (`mem-we-picked-tract-over-onnx`), so write the
sentence you would want to find later, not a short label.

Use for: architecture decisions spanning sessions, team conventions, known gotchas,
progress checkpoints on long tasks.

---

## Proactive behavior

### Always do

1. **Session start** → `forgeplan_health()` to orient
2. **Before coding** → `forgeplan_route()` on the task description
3. **After `forgeplan_new`** → fill ALL MUST sections immediately (no stubs)
4. **After implementation** → create evidence (with structured fields!) + link + activate
5. **After `forgeplan_*` tool call** → check `_next_action` hint in response

### When to escalate

- `forgeplan_route` says Standard+ but user wants to skip → explain tradeoff, don't force
- `forgeplan_health` shows "ALL DRAFT" → suggest reviewing mature artifacts for activation
- `forgeplan_health` shows blind spots (active without evidence) → prioritize evidence
- 0 findings after audit → suspicious, re-review with different agent

---

## Key principle

**Pipeline = guideline, NOT bureaucracy.**

Don't create every artifact type for every task. Tactical = just do it.
Standard = PRD + RFC. Use the routed depth and applicable project policy for
Deep/Critical work rather than treating this summary as an exemption.

The goal is to **think before coding**, not to generate documents nobody reads.
