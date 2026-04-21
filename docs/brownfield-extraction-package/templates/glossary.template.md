---
kind: glossary
id: TERM-{{auto}}
term: "{{term_name}}"
aliases: []
domain: "{{bounded_context}}"
definition: "{{one_sentence_plain_language}}"
code_usage:
  - path: "{{file}}"
    role: "{{e.g. ENUM value, column name, class name, function identifier}}"
related_terms: []
contradictions: []
verification:
  source: "{{inferred_from_code | domain_owner | external_doc}}"
  confidence: "{{verified | strong-inferred | inferred | speculation}}"
  evidence_refs: []
  last_verified: "{{YYYY-MM-DD}}"
bounded_context: "{{bounded_context}}"
lifecycle_state: "draft"
---

# {{term_name}}

## Definition
{{Plain-English one-paragraph definition. No code references in the body.}}

## Context
{{When and where is this term used in the business? What parties care about it?}}

## Aliases
- {{alias 1 with source/domain}}
- {{alias 2}}

## Examples
- {{concrete instance}}

## Contradictions / ambiguities
{{If the term means different things in different contexts, list them.}}

## Evidence
- {{link to evidence artifact or comment}}

{{For speculation: wrap uncertain portions in <!-- confidence:speculation -->...<!-- /confidence --> blocks.}}

## Related
- Related terms: {{list}}
- Used by use-cases: {{list}}
- Referenced in invariants: {{list}}
