---
kind: use-case
id: UC-{{auto}}
name: "{{imperative_phrase — e.g. 'Confirm an order'}}"
actor: "{{role name, not username}}"
trigger:
  type: "{{user_action | event | schedule}}"
  interface: "{{graphql:mutation | rest:endpoint | queue:name | cron:schedule}}"
  identifier: "{{exact entry point name}}"
preconditions:
  - "{{plain-English precondition 1 — reference invariants}}"
  - "{{precondition 2}}"
outcome:
  primary: "{{primary result in business terms}}"
  alternatives:
    - "{{alternative outcome 1}}"
steps:
  - step: 1
    actor: "{{role}}"
    action: "{{what they do}}"
    system_response: "{{what the system does}}"
invariants_invoked: []
domain_events_emitted: []
related_use_cases: []
bounded_context: "{{domain}}"
code_refs: []
verification:
  source: "{{inferred_from_code | domain_owner | external_doc}}"
  confidence: "{{verified | strong-inferred | inferred | speculation}}"
  evidence_refs: []
  last_verified: "{{YYYY-MM-DD}}"
lifecycle_state: "draft"
---

# {{UC-ID}} — {{use case name}}

## Overview
{{2-3 sentences: what this use case is for, at business level. No code references.}}

## Actors
- **Primary**: {{role}}
- **Observer(s)**: {{role(s)}}

## Trigger
{{Entry point in business framing: "X clicks Confirm button on order detail screen."}}

## Preconditions
1. {{precondition, linking to invariant IDs when applicable}}
2. {{…}}

## Main flow
1. {{step 1 actor/action/response}}
2. {{step 2}}
3. {{…}}

## Alternative flows

### {{alternative name}}
- **Trigger**: {{what branches here}}
- **Outcome**: {{result}}

### {{failure name}}
- **Trigger**: {{error condition}}
- **Outcome**: {{error, rollback, etc.}}

## Outcome
- {{observable business effect 1}}
- {{side effect to other domains / events emitted}}

## Domain events
- {{EventName — payload summary}}

## Business rules applied
- {{INV-XXX — short description}}

## Related use cases
- {{UC-XXX — precursor / variation / successor}}

## Open questions
- {{question for Domain Owner — parked in interview packet ID if applicable}}
