# Integration: Autoresearch Hooks

> How each of the 12 skills plugs into existing `/autoresearch:*` commands.

## Autoresearch baseline

From `sources/autoresearch/`, the existing command family:

| Command | What it does | Validate/Fix loop? |
|---|---|---|
| `/autoresearch:learn` | Inventory-style documentation with coverage metric | Yes (guard) |
| `/autoresearch:reason` | Adversarial refinement: generate → critique → synthesize → judge | Yes (critique round) |
| `/autoresearch:predict` | Multi-persona prediction with debate | Yes (debate) |
| `/autoresearch:scenario` | Edge-case generation | No |
| `/autoresearch:ship` | Delivery / export | No |

Autoresearch's core primitives we reuse:
- Knowledge graph as markdown files.
- Git hash stamping per artifact.
- Anti-herd constraints (diversity penalty).
- Composite metrics (`R_eff` family).

## Mapping: skill → command

| Skill | Autoresearch command | New mode | Reuses |
|---|---|---|---|
| C1 ubiquitous-language | `/autoresearch:learn` | `--mode=glossary` | coverage metric, validate/fix loop |
| C2 use-case-miner | `/autoresearch:learn` | `--mode=use-case` | coverage metric, anti-herd |
| C3 intent-inferrer | `/autoresearch:reason` | `--mode=intent` | adversarial refinement, generate/critique/synthesize |
| C4 invariant-detector | `/autoresearch:learn` | `--mode=invariant` | coverage metric |
| C5 causal-linker | — | new command `/extract-business-logic:causal` | KG pattern |
| C6 hypothesis-triangulator | `/autoresearch:predict` | `--mode=triangulate` | persona debate (git-persona, docs-persona, naming-persona) |
| C7 interview-packager | — | new command `/interview:draft` | KG pattern |
| C8 scenario-writer | `/autoresearch:scenario` | `--template=gherkin` | edge-case generation |
| C9 kg-curator | — | continuous process, not a command | KG pattern |
| C10 canonical-reproducer | `/autoresearch:learn` | `--mode=canonical` | validate/fix loop (C11 is validator) |
| C11 reproducibility-validator | — | validator plug-in for C10 | validate/fix loop |
| C12 rag-packager | `/autoresearch:ship` | `--target=rag` | delivery pipeline |

## New autoresearch personas (for /predict mode triangulate)

C6 uses `/autoresearch:predict --mode=triangulate`:

```yaml
personas:
  - name: causality-analyst
    prompt_hint: "Focus on temporal ordering and dependencies. Judge hypotheses by causal plausibility."
  - name: reproducibility-judge
    prompt_hint: "Judge hypotheses by whether they would produce the observed code if acted on."
  - name: git-historian
    prompt_hint: "Focus on git log. Favor hypotheses consistent with the commit history."
  - name: naming-linguist
    prompt_hint: "Focus on identifier naming patterns. Favor hypotheses consistent with the lexical signals."
  - name: contrarian
    prompt_hint: "Explicitly argue against the currently leading hypothesis. Force weakness surfacing."
```

The debate produces a consensus confidence score plus minority reports.

## New validate/fix loop: canonical

For C10 + C11, the standard autoresearch validate/fix loop is configured as:

```yaml
command: /autoresearch:learn --mode=canonical
validator:
  tool: C11-reproducibility-validator
  checks:
    - ddl-compile
    - sdl-parse
    - gherkin-parse
    - pseudo-code-coherence
fix_strategy:
  max_iterations: 3
  fallback: flag_as_partial_reproducible
```

## Emit format (for skill outputs)

Every skill emits in autoresearch's journal format:

```json
{
  "command": "autoresearch:learn",
  "mode": "glossary",
  "timestamp": "...",
  "git_hash": "...",
  "artifacts_created": ["TERM-012", "TERM-013"],
  "artifacts_updated": [],
  "validation_results": { ... },
  "metric_snapshot": {
    "glossary_count": 42,
    "verified_rate": 0.62
  }
}
```

## Anti-herd enforcement

Some skills (C3 intent-inferrer) can drift toward generating similar hypotheses. Autoresearch's anti-herd constraint applies:

```yaml
anti_herd:
  window: last_10_hypotheses
  max_similarity: 0.7
  action_on_violation: regenerate_with_explicit_diversity_prompt
```

## Coverage metric adaptation

Autoresearch's coverage metric is adapted for each mode:

| Mode | Coverage denominator | Coverage numerator |
|---|---|---|
| glossary | business terms found in code | business terms with a glossary entry |
| use-case | entry points (graphql + rest + queue + schedule) | entry points with a use-case |
| invariant | code guards | code guards with an invariant |
| canonical | total artifacts in domain | artifacts rendered canonically |

## Composite metric: `extract_score`

See the canonical formula in `05-AUTORESEARCH-INTEGRATION.md`. This file presents the same metric with the same weights.

```
extract_score = w1 * coverage_glossary
           + w2 * coverage_use_case
           + w3 * coverage_invariant
           + w4 * triangulation_rate
           + w5 * reproducibility_rate

w1..w5 = [0.2, 0.25, 0.2, 0.2, 0.15]
```

Tracked in `metric_snapshot` per session.

## Decay policy

Autoresearch supports decay on freshness. Extended for this package:

- `glossary`: decay after 60 days or code mention drift.
- `use-case`: decay after 30 days or entry point code change.
- `invariant`: decay after 30 days or guard code change.
- `scenario`: decay when its use-case or invariant decays.
- `hypothesis` parked: half-life 30 days for priority re-surfacing.
- `domain-model`: decay when any of its refs decay.

## Version history

- v1.0.0 — initial design.
