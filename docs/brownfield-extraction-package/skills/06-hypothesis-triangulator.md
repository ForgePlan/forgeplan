# Skill: hypothesis-triangulator (C6)

> Validates hypotheses against multiple independent sources to assign confidence scores.

## Why this skill exists

Hypotheses from C3 are initially `drafted`. Without triangulation, agents can't tell a strong hypothesis from a weak one. Docs end up mixing solid claims with guesses.

## Input

- `hypothesis` artifacts in state `drafted` or `inferred`.
- Access to:
  - Git log / blame.
  - Legacy docs (tier-3 markdown files).
  - Code comments (Russian + English).
  - Naming patterns (class, file, action).
  - Domain Owner interview answers (when available).

## Output

Updated `hypothesis` artifacts with new confidence + `triangulation_sources` filled.

Additionally, triangulation reports:
- For each hypothesis: which sources said yes/no/unclear.
- Aggregation rule that produced final confidence.
- Alternatives considered and demoted.

## Triangulation sources

### Source 1: Git history
- `git log --all --follow <file>` — when did feature arrive?
- `git log -p --grep '<term>'` — was feature name ever mentioned in commit messages?
- `git blame <file> <line>` — who added this guard? When?
- Signals:
  - Added all at once → coherent design (supports structural hypothesis).
  - Added incrementally → evolutionary fix (supports incremental / bugfix hypothesis).
  - Reverted multiple times → historical controversy (supports legacy / uncertainty).

### Source 2: Legacy docs
- Scan `docs/`, `README.md`, `*.md` outside `.forgeplan/`.
- Fuzzy match term / hypothesis subject.
- Signals:
  - Mentioned with positive framing → supports hypothesis.
  - Mentioned as "deprecated" or "legacy" → supports "tech debt" hypothesis.
  - Not mentioned → no evidence either way.

### Source 3: Code comments
- Nearby comments (±10 lines of observation).
- Docstrings on containing function.
- TODO / FIXME / HACK markers.
- Signals:
  - "TODO: remove" → legacy hypothesis supported.
  - "хак для старых данных" → migration-compat hypothesis.
  - Substantive intent comment → primary evidence.

### Source 4: Naming patterns
- Function / action / file name analysis.
- Signals:
  - Versioned name (`MarginalV2`) → migration pattern.
  - Typo in name (`CARETD`) → legacy unreliable.
  - Consistent domain naming → coherent design.

### Source 5: Test evidence (if tests exist)
- Tests name hypotheses or describe intent.
- Red-green-refactor history in git indicates stability.

### Source 6: Domain Owner
- Top-tier signal.
- If answered, hypothesis goes to `verified`.

## Confidence aggregation rules

| Evidence | Result |
|---|---|
| Domain Owner confirms | `verified` |
| ≥ 3 independent sources aligned, no contradicting | `strong-inferred` |
| 2 aligned sources, no contradicting | `inferred` |
| 1 source only, no contradicting | `inferred` (weak) |
| Contradicting sources | **park** for interview |
| No sources | remain `drafted`; escalate |
| Source rules out hypothesis | `refuted` |

## Modes

### Mode 1: `query`
For a hypothesis, query each source in sequence.

### Mode 2: `aggregate`
Apply aggregation rules to compute new confidence.

### Mode 3: `update`
Update artifact state and record triangulation log.

### Mode 4: `escalate`
For hypotheses that cannot be resolved (drafted → drafted after full query), add to interview packet via C7.

## Algorithm

```
for each hypothesis in state drafted or inferred:
  signals = []

  # Query git
  git_signal = query_git(hypothesis.observation)
  signals.append(('git', git_signal))

  # Query legacy docs
  docs_signal = query_legacy_docs(hypothesis.subject)
  signals.append(('docs', docs_signal))

  # Query comments
  comments_signal = query_code_comments(hypothesis.code_refs)
  signals.append(('comments', comments_signal))

  # Query naming
  naming_signal = query_naming_patterns(hypothesis.code_refs)
  signals.append(('naming', naming_signal))

  # Aggregate
  confidence = aggregate(signals)

  # Update
  hypothesis.confidence = confidence
  hypothesis.triangulation_sources = signals

  if confidence == 'drafted' and all signals empty:
    park_for_interview(hypothesis)
```

## Metric

- `hypotheses_triangulated / total_hypotheses`: target 100%.
- `hypotheses_verified_by_interview / hypotheses_parked`: tracks DO engagement.
- `confidence_distribution`: healthy shape (e.g., 20% verified, 30% strong-inferred, 30% inferred, 20% parked).

## Dependencies

- C3 output (hypotheses).
- Git access.
- C1 output (glossary — for term alias resolution).

## Prompt template

See `references/triangulate-prompt.md` for source-specific queries.

## Failure modes

| Failure | Detection | Mitigation |
|---|---|---|
| Git repo shallow (squashed history) | Few commits in blame | Use available sources only; note limitation |
| Legacy docs stale | Docs reference removed code | Discount docs as tier-3; use for intent hints only |
| Comment is a lie | Comment says "do X" but code does Y | Favor code; flag discrepancy as a problem |
| Aggregation over-confident | All sources weak but aligned → claims strong-inferred | Require diverse sources (3 different categories), not 3 of same |

## Example

Hypothesis: "The duration=5 hardcoded for LO stevedores (FreightSeaParser.js:483) is a placeholder awaiting domain owner value."

Triangulation:
- Git: blame shows comment `// TODO: Придумать лучшее решение` added with the code → supports placeholder hypothesis.
- Legacy docs: no mention.
- Comment: explicit TODO → strong signal.
- Naming: no signal.
- Result: 2 aligned sources (git + comment) → `inferred`. Park for Domain Owner: "What should duration be for LO stevedores?"

## Testing

Fixture: 10 hypotheses with known triangulation outcomes → expect ≥ 9 correct.

## Version history

- v1.0.0 — initial design.
