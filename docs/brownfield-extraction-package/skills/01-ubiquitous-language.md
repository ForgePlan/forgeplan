# Skill: ubiquitous-language (C1)

> Extracts business terms, definitions, aliases, and synonyms from a codebase to build a domain glossary.

## Why this skill exists

Without a shared vocabulary, business documentation is impossible. Every downstream skill depends on knowing what terms mean. This is the foundation.

In brownfield codebases, business terms are scattered across:
- Code symbols (class names, function names, property names).
- GraphQL type/enum names and fields.
- Database column names.
- String literals (status enums, type enums).
- Inline comments (Russian or English).
- URL/route naming.
- Event / queue names.

Many terms have **synonyms** (technical vs business) that need consolidation: `forwarder_confirmed` (code) ↔ `carrier committed capacity` (business).

## Input

- **Scope**: directory or file glob (e.g., `services/**/*.js`, `models/*.js`).
- **Domain hint** (optional): bias extraction toward a specific bounded context (e.g., `orders`).
- **Existing glossary** (optional): start from this; grow incrementally.
- **Stop-list** (optional): technical terms to exclude (e.g., `ctx`, `broker`, `fn`).

## Output

One or more `glossary` artifacts (one per term).

Frontmatter per artifact:
```yaml
kind: glossary
id: GLOS-{auto}
term: "forwarder_confirmed"
domain: "orders"
tier: intent | factum
confidence: drafted | inferred | verified  (from this skill: always starts as inferred)
aliases: ["FC state", "carrier committed"]
related_terms: ["cargo_owner_confirmed", "fulfillment"]
code_refs:
  - file: "models/Order.js"
    line: 82
    context: "ENUM declaration"
  - file: "services/v5.orders.service.js"
    line: 3431
    context: "status assignment in _confirm"
sources:  # where the term came from
  - code_symbol
  - string_literal
  - comment
  - graphql_enum
frequency: 47  # occurrences across scope
created_by: skill:ubiquitous-language v1.0.0
git_sha: <sha>
```

Body sections:
1. **Definition** (≥ 30 words, intent-level, human-readable).
2. **Factum evidence** (code snippets).
3. **Alternatives considered** (other plausible definitions).
4. **Open questions** (for Domain Owner).
5. **Related terms** (cross-refs).

## Modes

### Mode 1: `discover`
Given scope, produce candidate terms with frequency. No definitions yet.

```
input: scope, stop-list
output: <term, frequency, source_files>[]
```

### Mode 2: `define`
For each candidate, generate a draft definition using LLM + code context.

```
input: candidate term, code context (surrounding usage)
output: glossary artifact with tier=inferred, confidence=inferred
```

### Mode 3: `consolidate`
Merge synonyms (e.g., detect `carrier`, `forwarder`, `transporter` are same concept).

```
input: all glossary artifacts in domain
output: updated glossary with aliases populated, duplicates marked as superseded
```

### Mode 4: `update`
Re-scan codebase; for each existing term, update `code_refs`, `frequency`. New terms added; unused terms flagged as stale.

## Algorithm

### Discover phase

1. **Collect candidates**:
   - AST parse each file (Babel for JS/TS).
   - Extract: identifiers (PascalCase → classes; camelCase → methods/vars; snake_case → DB/config).
   - Extract: string literals that look like enum values (all-lowercase, underscore-separated, appears in `==='X'`).
   - Extract: GraphQL type/enum/field names.
   - Extract: comment text (Russian + English words).

2. **Filter**:
   - Remove stop-list (language keywords, framework noise).
   - Threshold frequency (≥ 3 occurrences or known significant patterns).
   - Remove obvious noise (numeric, single-letter).

3. **Group**:
   - By similarity (Levenshtein distance < 0.3 → likely same term variant).
   - By co-occurrence (terms appearing in same function likely related).

### Define phase

For each candidate:

1. Collect up to 5 usage contexts (±10 lines around each occurrence).
2. Prompt LLM with:
   ```
   You are extracting a business term for a {domain_hint} domain.
   Term: {term}
   Usage contexts: {5 snippets}

   Generate:
   1. A business-facing definition (≥ 30 words, not technical).
   2. 2-3 plausible alternative meanings.
   3. Confidence estimate (inferred / speculation).
   4. Open questions for a Domain Owner (1-3).
   ```
3. Write glossary artifact.
4. If LLM says `speculation`, mark as `parked for interview`.

### Consolidate phase

1. For each pair of terms, check:
   - String similarity.
   - Co-occurrence ratio.
   - Domain overlap.
2. If similarity score high, propose merge:
   - Canonical term = most frequent one.
   - Others → `aliases`.
   - Record as `superseded` with reason.
3. Ask for LLM review if ambiguous.

## Metric

`coverage = defined_terms / candidate_terms` (target ≥ 0.8).

Quality sub-metrics:
- `% with ≥ 1 alias` (consolidation health).
- `% verified` (requires DO input).
- `% with open questions` (parking lot).

## Validation rules (for the output artifacts)

See `04-FORGEPLAN-EXTENSIONS.md` validation rules for `glossary`. Plus:
- Definition must not contain the term itself (no circular definitions).
- Aliases must not overlap with other terms' canonical names in the same domain.

## Dependencies

- Forgeplan MCP: `forgeplan_new`, `forgeplan_update`, `forgeplan_link`.
- Extension: new kind `glossary`, new MCP tool `forgeplan_contradictions` (for consolidation mode).

## Integration with autoresearch

Wire via `/autoresearch:learn --mode glossary`:
- Uses learn's scout for file discovery.
- Uses learn's validate-fix loop for consolidation.
- Metric and target fit autoresearch's iteration model.

## Prompt templates

**Template A — Definition generation** (in `references/define-prompt.md`):
```
System:
You extract business terminology from code. You produce definitions suitable for
non-technical readers (product managers, domain experts, auditors).

User:
Term: {term}
Domain: {domain}
Code usage contexts (N samples): {contexts}

Output as JSON:
{
  "definition": "...",
  "tier": "intent" | "factum",
  "confidence": "inferred" | "speculation",
  "alternatives": ["alt1", "alt2"],
  "open_questions": ["q1", "q2"],
  "aliases_suspected": ["a1"]
}

Rules:
- Definition must be ≥ 30 words.
- Do NOT use the term itself in the definition.
- Do NOT introduce new technical terms.
- Prefer "what it means for the business" over "what the code does".
```

**Template B — Consolidation** (in `references/consolidate-prompt.md`):
```
Two terms and their contexts:

Term A: {A.term}
Contexts: {A.contexts}

Term B: {B.term}
Contexts: {B.contexts}

Question: Are these the same business concept?
Response:
{
  "same_concept": true | false,
  "canonical_term": "A" | "B" | "neither",
  "reasoning": "..."
}
```

## Failure modes

| Failure | Detection | Mitigation |
|---|---|---|
| Too many generic terms (e.g., `status`, `type`) | Output > 200 terms per small scope | Require compound keys `orders.status`, `invoices.status` |
| Hallucinated definitions | Definition contradicts code snippets | Cross-check: does definition + code examples make sense? |
| Missed alias | Two terms with same meaning not merged | Run consolidate phase with LLM review |
| Russian/English mix chaos | Same concept named twice | Normalize at discover phase (translate or map both) |

## Example

See `examples/tripsales-glossary-sample.md` for sample output.

## Testing

Fixture: `fixtures/glossary-fixture/` with 5 files containing 20 known terms. Expected: C1 extracts ≥ 16, defines all with confidence ≥ inferred, merges 2 known synonyms.

## Version history

- v1.0.0 — initial design (this document).
