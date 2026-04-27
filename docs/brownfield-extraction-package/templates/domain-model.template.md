---
kind: domain-model
id: DM-{{auto}}
domain: "{{bounded_context}}"
aggregate_roots:
  - name: "{{AggregateName}}"
    description: "{{one-sentence purpose}}"
entities:
  - name: "{{EntityName}}"
    parent_aggregate: "{{AggregateName}}"
value_objects:
  - name: "{{ValueObjectName}}"
external_dependencies:
  - domain: "{{other_bounded_context}}"
    relationship: "{{how it depends}}"
canonical_ddl: |
  -- full standalone DDL goes here; no file references
canonical_pseudo_code:
  actions: []
canonical_sdl: |
  # full standalone GraphQL SDL goes here
domain_events: []
state_machine:
  initial: []
  states: []
  transitions: []
use_cases_ref: []
invariants_ref: []
scenarios_ref: []
glossary_ref: []
verification:
  source: "{{inferred_from_code | domain_owner | external_doc}}"
  confidence: "{{verified | strong-inferred | inferred | speculation}}"
  evidence_refs: []
  last_verified: "{{YYYY-MM-DD}}"
  validation_passes:
    ddl_compile: false
    sdl_parse: false
    reproducibility_simulation: "pending"
bounded_context: "{{domain}}"
lifecycle_state: "draft"
---

# {{DM-ID}} — {{domain name}} Domain

## Overview
{{The domain's purpose in the business. 2-4 sentences, plain language.}}

## Aggregate roots
### {{AggregateName}}
{{Purpose, responsibilities, boundaries.}}

## Entities (non-root)
- {{EntityName}} — {{role within aggregate}}

## Value objects
- {{ValueObjectName}} — {{what it represents}}

## Canonical DDL
```sql
{{full DDL}}
```

## State machine
```mermaid
stateDiagram-v2
    [*] --> {{initial state}}
    {{state transitions}}
```

## Canonical pseudo-code
{{one sub-section per action, or reference external files in canonical/}}

### Action: {{action name}}
{{pseudo-code}}

## Canonical SDL
```graphql
{{full SDL}}
```

## Domain events
- {{EventName}} — emitted by {{UC-XXX}}

## External dependencies
- {{other_bounded_context}}: {{how it's used}}

## Invariants (local to this domain)
- {{INV-XXX}} — {{short}}

## Key use-cases
- {{UC-XXX}} — {{what}}

## Glossary (local)
- {{TERM-XXX}} — {{term}}

## Open questions / parked hypotheses
- {{HYP-XXX}} — {{question}}
