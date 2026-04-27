# Skill: interview-packager (C7)

> Clusters parked hypotheses and open questions into structured Domain Owner interview packets.

## Why this skill exists

Some hypotheses can only be verified by a human Domain Owner. Without a structured interview process, questions accumulate and never get answered. This skill turns the "parking lot" into actionable conversations.

## Input

- Hypotheses in state `parked` (from C6).
- `problem` artifacts marked `needs_domain_owner`.
- Open questions from use-cases, invariants, scenarios.
- Domain Owner profile (optional): who to address, language preference, preferred format.

## Output

Interview packets — standalone markdown files with:
- Cover page (context, instructions).
- Clustered questions by domain + entity + priority.
- Response template (Q&A blocks for filling in).
- Supporting context per question.

Filename: `interview-packet-{domain}-{date}.md`.

Also:
- Forgeplan artifact `interview-packet` (if the kind is added) OR `note` with reference.
- Links from each parked hypothesis to the packet (`parked_in` relation).

## Packet structure

```markdown
# Interview Packet: Orders Domain (2026-04-21)

## Instructions for Domain Owner

This packet contains {N} questions about how the Orders domain is intended to work.
Please answer each question. If you're unsure, say "I don't know" — that's also useful.

Estimated time: {30} minutes.
Language: Russian / English.
Submission: edit this file in place, save as `answered-<packet-id>.md`, send back.

## Priority 1 — Blocks major docs

### Q1 — Order.status.removed value

**Context**: The `removed` status is defined in the ENUM but our analysis found NO code that ever sets it. We have three candidate interpretations:

1. Historical reserved value (intended for soft-delete, never implemented).
2. Currently used via an external system we haven't seen.
3. Reserved for future feature.

**Question**: Is `removed` actually used today? If yes, by whom / how?

**Your answer**:
[ ] Not used.
[ ] Used by __________________.
[ ] Reserved for __________________.
[ ] Other: ____________________________________________.

**Impact**: Affects documentation of order lifecycle and what statuses to include in the public API.

---

### Q2 — _cancel cascade only for forwarder_confirmed

**Context**: When an order is canceled, cascade cleanup (shoulders, points, cargo, invoices) only happens if the order was in `forwarder_confirmed`. For other statuses (accepted, fulfillment, paid), cancel does not cascade. Our hypotheses:

1. Intentional: earlier statuses have nothing allocated; later statuses shouldn't cancel.
2. Bug: cascade should cover more statuses.
3. Handled elsewhere: e.g., `v4.sales.cancel.order` does additional cleanup.

**Question**: What is the intended behavior of cancel for orders in `accepted`, `fulfillment`, or `paid` status?

**Your answer**: _____________________________________________

**Impact**: PROB-018.027 cannot be categorized as bug vs intentional without this.

## Priority 2 — Blocks specific use cases

...

## Priority 3 — Nice to verify (can leave unanswered)

...
```

## Modes

### Mode 1: `draft`
Generate packet from current parked hypotheses.

### Mode 2: `prioritize`
Sort questions by:
- Number of artifacts blocked.
- Domain importance.
- Severity of related problem.

### Mode 3: `cluster`
Group related questions so Domain Owner answers them in context.

### Mode 4: `ingest`
Parse answered packet → update hypothesis confidence to `verified` (or `refuted`).

## Algorithm

### Draft

```
parked = forgeplan.query(kind=hypothesis, state=parked)
problems = forgeplan.query(kind=problem, tag='needs_domain_owner')
open_qs = collect_open_questions_from_artifacts()

questions = parked + problems + open_qs
for q in questions:
  q.priority = compute_priority(q)
  q.context = build_context(q)
  q.options = generate_plausible_options(q)

clusters = cluster_by_domain_entity(questions)
packet = render_packet(clusters, cover_page, instructions)
save_packet(packet)
```

### Prioritize

Priority = f(
  artifacts_blocked_count,     # how many docs depend on this
  domain_importance,           # is it a core vs peripheral domain
  problem_severity_if_any,     # CRITICAL vs MEDIUM
  age_of_parking               # older questions get priority
)

### Cluster

Group by (domain, entity). Max 5 questions per cluster.

### Ingest

Parse answered markdown:
- Detect `**Your answer**` blocks.
- Detect inline answers ("X is because Y").
- For each answered question, locate corresponding hypothesis/problem.
- Update confidence: `verified` if clear answer, `refuted` if answer contradicts hypothesis, `still-unclear` if DO says "I don't know".

## Metric

- `questions_per_packet`: 5-20 (too few → wasteful; too many → overwhelm).
- `answered_rate_per_packet`: tracks DO engagement.
- `questions_age`: median age of pending questions (should decrease over time).

## Dependencies

- C6 output (parked hypotheses).
- Other skills' artifacts (use-cases, invariants, scenarios) with open questions.
- Access to forgeplan MCP.

## Integration with autoresearch

No direct autoresearch integration — this is a human-in-the-loop skill.

Optional: `/autoresearch:ship --type interview` to deliver packet via email/chat.

## Prompt template

See `references/draft-packet-prompt.md`.

## Failure modes

| Failure | Detection | Mitigation |
|---|---|---|
| Questions too technical | DO can't understand | Translate to business-level; run through a "non-technical reviewer" LLM persona |
| Too many low-priority questions | Packet > 30 questions | Split by domain; defer non-blockers |
| Answers ambiguous | "maybe" answers | Follow-up clarification packet |
| Answers contradict multiple hypotheses | Many refutes at once | Generate re-inference triggers for C3 |

## Example

See a sample packet in `examples/interview-packet-orders-sample.md` (to be produced by the implementation).

## Testing

Fixture: 10 parked hypotheses → expect well-formed packet with appropriate clustering + prioritization.

## Version history

- v1.0.0 — initial design.
