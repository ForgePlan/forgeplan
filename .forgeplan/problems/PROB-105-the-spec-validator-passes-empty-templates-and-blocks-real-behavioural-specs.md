---
depth: tactical
id: PROB-105
kind: problem
links:
- target: PRD-086
  relation: informs
status: draft
title: The SPEC validator passes empty templates and blocks real behavioural specs
---

---
assigned_number: 105
context: '{grouping tag}'
created: 2026-09-07
predicted_number: 105
slug: prob-the-spec-validator-passes-empty-templates-and-blocks-real-behavioural-specs
---

# PROB-105: the SPEC validator is inverted

## Signal

Measured on 0.36.0 with the shipped binary, in a fresh workspace.

**An untouched SPEC template validates clean and activates with perfect trust.**

```
$ forgeplan new spec "Widget sync API"      # 110-line template, nothing edited
$ forgeplan validate SPEC-001
  Result: PASS -- 0 error(s), 0 warning(s)
Next: forgeplan activate SPEC-001
```

Link any evidence pack and it activates: `Status: active`, `R_eff: 1.00`. The
body is still `{METHOD} /v1/{resource}` and "Что специфицируется. Одно
предложение."

**A complete behavioural spec fails a MUST and cannot activate.**

```
$ forgeplan validate SPEC-003    # 2 × ### Requirement, 2 × #### Scenario, GIVEN/WHEN/THEN
  x [MUST] spec-contracts: Missing '## API Contracts' or '## Data Models' section
  Result: FAIL -- 1 error(s)
```

So the validator waves through a document that says nothing and blocks one that
carries a full test oracle.

## Why each half happens

**The block.** `check_spec_contracts` looks for the literal headings
`API Contracts` or `Data Models` (rules.rs, `section_exists`). `## Requirements`
and `## Behavioral Contract` are not in `expand_aliases`, so a behavioural spec
has no route through a MUST rule. The kernel already encodes one methodology's
shape as mandatory — which is the opposite of what issue #450 alleges.

**The pass.** `check_stub` is the gate that exists to catch unfilled templates.
Its twelve `PHRASE_MARKERS` are all PRD prose — "Что мы строим и почему это
важно", "What we are building and why", `[Actor] can [capability]`. None appears
in the SPEC template. It also counts placeholders, but caps that signal at +1
while the threshold is 3, so fifteen unfilled placeholders count the same as one.

The stub gate is structurally incapable of seeing a SPEC.

## The measurement that gives a shape-agnostic fix

Single-brace placeholders, counted across the corpus:

| Document | Placeholders |
|---|---|
| SPEC template, untouched | **15** |
| SPEC-001 … SPEC-006 (real) | 0–3 |
| PRD template (which the gate *does* catch) | 5 |

Clean separation with a wide margin, and it asks nothing about methodology: not
"is this Gherkin or API", but "is this still a form to fill in".

A line-count test would NOT work — the untouched template has 25 non-empty lines
under `## API Contracts`, because placeholder JSON is still lines.

## What this explains

The marketplace TDD flow grew its own gate (`tdd-planner` HARD RULE 1 refusing to
plan against a spec with no `#### Scenario`) because core actively rejects the
shape it needs. Issue #450 read that as "core has no rule"; the truth is core has
a MUST pointing the other way.

## Fix

Three changes, none of which teaches the kernel a methodology:

1. **`check_stub` learns to see a SPEC** — scale the placeholder signal instead of
   capping it at +1. Catches the template, silent on all six real specs.
2. **`spec-contracts` accepts a behavioural route** — `Requirements`,
   `Contract`, `Behavioral Contract` alongside the API-shaped headings. The rule
   still demands *a* contract; it stops demanding one particular form of it.
3. **Conditional consistency rule (#450, literally)** — if a spec opens
   `### Requirement`, each must carry a `#### Scenario`. Vacuous on all six of
   ours (none uses those headings); fires on a half-authored behavioural spec,
   which is the real defect. Should, not Must.

## Related

| Artifact | Relation |
|---|---|
| PRD-086 | informs |

GitHub: #450 (its premise is inverted — see above), #449 (separate).


