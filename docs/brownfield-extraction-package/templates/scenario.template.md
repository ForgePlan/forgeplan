---
kind: scenario
id: SC-{{auto}}
feature: "{{feature name — matches use case}}"
use_case_ref: "{{UC-XXX}}"
invariants_verified: []
scenario_type: "{{happy_path | failure | edge_case | exploratory}}"
gherkin_feature: |
  Feature: {{feature name}}
    As a {{actor}}
    I want {{goal}}
    So that {{business value}}

    Background:
      Given {{common setup step}}
      And {{...}}

    Scenario: {{happy path name}}
      When {{trigger action}}
      Then {{expected outcome}}
      And {{side effect}}

    Scenario: {{failure name}}
      Given {{condition that triggers failure}}
      When {{trigger action}}
      Then an error {{ERROR_CODE}} is raised
      And {{no state change}}

visualizations:
  - type: "mermaid-sequence"
    content: |
      sequenceDiagram
          participant A as {{actor}}
          participant S as {{system}}
          A->>S: {{action}}
          S-->>A: {{response}}
verification:
  automated: "{{path to test file OR 'pending'}}"
  manual: "{{manual run notes OR null}}"
  source: "{{inferred_from_code | domain_owner}}"
  confidence: "{{verified | strong-inferred | inferred | speculation}}"
  evidence_refs: []
  last_verified: "{{YYYY-MM-DD}}"
bounded_context: "{{domain}}"
lifecycle_state: "draft"
---

# {{SC-ID}} — {{scenario name}}

## Feature overview
{{2-3 sentences about what this feature is validating, in business terms.}}

## Gherkin feature

```gherkin
{{embedded from frontmatter}}
```

## Mermaid sequence

```mermaid
{{sequence diagram}}
```

## Verification
- **Automated**: {{status}}
- **Manual**: {{last manual run}}
- **Confidence**: {{level}} — {{rationale}}

## Traceability
- Use case: {{UC-XXX}}
- Invariants: {{INV list}}
- Terms: {{TERM list}}
- Related scenarios: {{SC list}}
