# Integration: Forgeplan MCP Additions

> New MCP tools and extensions to existing tools. Spec for the forgeplan maintainer agent.

## Scope

This document defines the forgeplan MCP server changes needed to support the 6 new artifact kinds and the extraction workflow.

## 1. Extended `forgeplan_new`

Existing tool. Needs to accept new `kind` values:

| kind | new? |
|---|---|
| glossary | new |
| use-case | new |
| invariant | new |
| scenario | new |
| hypothesis | new |
| domain-model | new |

Validation per-kind follows schemas in `artifact-kinds/*.md`.

## 2. Extended `forgeplan_validate`

New validators per kind:

```typescript
validators.glossary = (artifact) => {
  assert unique term within bounded_context;
  assert aliases are disjoint globally;
  assert related_terms reference existing glossary;
  assert contradictions reference existing glossary;
  assert lifecycle_state in ['draft','active','deprecated','superseded'];
};

validators.use-case = (artifact) => {
  assert trigger.identifier exists (parsed or verified);
  assert steps non-empty;
  assert invariants_invoked reference existing invariants;
  assert verification.confidence != 'speculation' OR body wraps speculative parts;
};

validators.invariant = (artifact) => {
  assert statement is single sentence;
  assert category in known_categories;
  assert affected_use_cases reference existing use-cases;
  assert no contradicts relation with confidence=verified unless resolved;
};

validators.scenario = (artifact) => {
  run gherkin_parser(gherkin_feature);
  assert invariants_verified non-empty;
  assert use_case_ref exists;
};

validators.hypothesis = (artifact) => {
  assert candidates.length >= 3 OR lifecycle in ['verified','refuted'];
  assert selected_candidate in candidates;
  assert lifecycle_state in valid transitions from previous state;
};

validators.domain-model = (artifact) => {
  run psql_check(canonical_ddl);
  run graphql_parse(canonical_sdl);
  assert use_cases_ref, invariants_ref, glossary_ref resolve;
};
```

## 3. Extended `forgeplan_graph`

New relations (edge types):

| Relation | From → To | Purpose |
|---|---|---|
| `defines` | glossary → use-case/invariant/scenario | term usage |
| `triggers` | use-case → event | event emission |
| `verifies` | scenario → use-case/invariant | test anchor |
| `infers_from` | hypothesis → observation | inference source |
| `resolved_by` | hypothesis → interview-answer | resolution path |
| `parked_in` | hypothesis → interview-packet | parking trail |
| `catalogs` | domain-model → aggregate | composition |
| `emitted_by` | event → use-case | origin |
| `causes` | event → use-case | downstream reaction |
| `mutates` | use-case → entity | side-effect |
| `listens_to` | use-case → queue-topic | subscription |
| `loop` | event → event | causal cycle warning |

Existing relations still work: `refines`, `informs`, `based_on`, `supersedes`, `contradicts`.

## 4. New MCP tools (9)

### `forgeplan_hypothesis_status`

```
INPUT: { hypothesis_id?: string, domain?: string, lifecycle_state?: string }
OUTPUT: [{id, subject, lifecycle_state, days_since_creation, blocks_count}]
```

### `forgeplan_hypothesis_promote`

```
INPUT: { hypothesis_id: string, new_state: 'verified'|'strong-inferred'|'inferred'|'refuted'|'parked', evidence_refs: string[], rationale: string }
OUTPUT: { ok: true, cascade: [list of downstream artifacts re-queued for update] }
```

Enforces state-machine transitions.

### `forgeplan_coverage_business`

```
INPUT: { domain: string }
OUTPUT: {
  glossary: { count, expected, rate },
  use_cases: { count, expected, rate },
  invariants: { count, expected, rate },
  scenarios: { per_use_case_avg, coverage_rate },
  hypotheses: { by_state: { verified, inferred, parked, refuted } },
  canonical: { ddl_compile, sdl_parse, pseudo_code_coherence, reproducibility },
  extract_score: 0.0-1.0
}
```

### `forgeplan_contradictions`

```
INPUT: { domain?: string, kind?: string }
OUTPUT: [{
  left_artifact: string, right_artifact: string,
  kind_of_contradiction: 'invariant_conflict'|'hypothesis_duplicate'|'glossary_divergence'|'scenario_vs_invariant',
  severity: 'critical'|'high'|'medium',
  suggested_resolution: string
}]
```

### `forgeplan_orphans`

```
INPUT: { domain?: string }
OUTPUT: [{
  id: string, kind: string,
  reason: 'uncovered_use_case'|'unverified_invariant'|'orphan_term'|'un_triangulated_hypothesis'|'incomplete_model'
}]
```

### `forgeplan_interview_packet_draft`

```
INPUT: { domain?: string, max_questions?: number }
OUTPUT: { packet_id: string, path: string, question_count: number, priority_breakdown: { P1, P2, P3 } }
```

Wraps C7 draft mode.

### `forgeplan_interview_packet_ingest`

```
INPUT: { packet_id: string, answered_markdown_path: string }
OUTPUT: { hypotheses_updated: number, promoted: [], refuted: [], still_parked: [], cascade: [] }
```

Wraps C7 ingest mode.

### `forgeplan_render_canonical`

```
INPUT: { domain: string, mode: 'render-all'|'render-domain'|'diff' }
OUTPUT: { written_files: [], validation_summary: {...} }
```

Wraps C10.

### `forgeplan_export_rag`

```
INPUT: { domain?: string, output_dir: string, chunk_size_hint?: 'small'|'medium'|'large', embed?: bool }
OUTPUT: { chunks_count, manifest_path, embedding_model?: string }
```

Wraps C12.

## 5. Storage schema additions

Each new artifact kind stored as markdown + frontmatter like existing kinds. No database schema change.

Confidence-per-assertion: HTML-comment wrapper (see `04-FORGEPLAN-EXTENSIONS.md`):

```markdown
<!-- confidence:speculation -->
This interpretation assumes that forwarder_confirmed implies resource commitment. Unverified.
<!-- /confidence -->
```

`forgeplan_validate` scans these wrappers and includes them in the `speculation_count` of `forgeplan_health`.

## 6. CLI commands (optional)

If the MCP server also exposes a CLI:

```
forgeplan hypothesis list --domain orders --state parked
forgeplan hypothesis promote HYP-042 --state verified --evidence EVID-101
forgeplan coverage business --domain orders
forgeplan interview draft --domain orders --max 15
forgeplan interview ingest packet-2026-04-21.md
forgeplan render canonical --domain orders
forgeplan export rag --output ./rag-pkg/
```

## 7. Backward compatibility

- All existing artifact kinds continue to work unchanged.
- All existing tools (`forgeplan_new`, `forgeplan_validate`, etc.) accept new kinds via extended dispatchers.
- Confidence-per-assertion is opt-in (empty wrappers → no effect).

## 8. Testing strategy for MCP additions

- Unit tests per kind validator (fixture: valid and invalid example each).
- Integration test: create glossary → use-case → invariant → scenario → domain-model in sequence; verify graph edges.
- State-machine test: hypothesis transitions from each state.
- Performance test: render 1000-artifact workspace in < 30s for `forgeplan_coverage_business`.

## 9. Migration path

Existing workspaces with old artifact kinds:
1. Ship new MCP version.
2. Existing artifacts continue unchanged.
3. User opts in to extraction by running `/extract-business-logic <domain>`.
4. New artifacts created alongside old.
5. Nothing migrates automatically — old inventory-style docs stay as legacy reference.

## Version history

- v1.0.0 — initial design.
