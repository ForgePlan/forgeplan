# Forgeplan Extensions

> What needs to change in forgeplan itself to support the extraction methodology.

## Scope of changes

Six new artifact kinds + new relations + confidence scoring per-assertion + MCP tools additions. **No breaking changes** — existing artifacts continue to work.

## 1. New artifact kinds (6)

> **Source of truth**: the full canonical schemas, body structure, validation rules, lifecycle, and link types for each kind are in `artifact-kinds/<kind>.md`. The sketches below are just a taste — if they disagree with `artifact-kinds/`, `artifact-kinds/` wins.

### 1.1 `glossary` — see `artifact-kinds/glossary.md`
A single business term with definition, aliases, code usage, related terms, and confidence. Foundation layer — everything else depends on the glossary.

### 1.2 `use-case` — see `artifact-kinds/use-case.md`
A user journey from an actor's trigger through system steps to a business outcome. Contains `trigger`, `preconditions`, `steps`, `outcome`, `alternatives`, `invariants_invoked`, `domain_events_emitted`.

### 1.3 `invariant` — see `artifact-kinds/invariant.md`
A business rule that must hold. Contains `statement` (one sentence), `category`, `scope`, `violation_consequence`, `rationale`. Scenarios verify invariants.

### 1.4 `scenario` — see `artifact-kinds/scenario.md`
An executable specification of a use case in Gherkin Given/When/Then form, plus Mermaid sequence diagrams. Anchored to a `use_case_ref` and verifies one or more invariants.

### 1.5 `hypothesis` — see `artifact-kinds/hypothesis.md`
A first-class, lifecycle-tracked claim about code intent. Has ≥ 3 candidate explanations and evolves through states:

```
drafted → triangulated → verified | strong-inferred | inferred | refuted | parked
                                                                    ↓
                                                             interview answer
                                                                    ↓
                                                          verified | refuted
```

Two distinct fields:
- `lifecycle_state` — where in the workflow (drafted / triangulated / parked / etc.).
- `verification.confidence` — how strong the evidence is (verified / strong-inferred / inferred / speculation).

### 1.6 `domain-model` — see `artifact-kinds/domain-model.md`
A DDD-style canonical description of a bounded context: aggregate roots, entities, value objects, standalone DDL, canonical pseudo-code for actions, canonical GraphQL SDL, domain events, state machines, and refs to all its use-cases / invariants / glossary terms.

## 2. New relations

In addition to existing `informs, based_on, supersedes, contradicts, refines`:

| New relation | Semantic |
|---|---|
| `defines` | `glossary → domain-model` or `invariant` — the term is formally defined in X |
| `triggers` | `use-case → scenario` — this journey triggers this scenario |
| `verifies` | `scenario → invariant` — running this scenario validates this invariant |
| `infers_from` | `hypothesis → (evidence \| code-ref)` — this hypothesis was generated from that observation |
| `resolved_by` | `hypothesis → (interview-packet \| adr)` — confidence was lifted here |
| `parked_in` | `hypothesis → interview-packet` — waiting for Domain Owner |
| `catalogs` | `domain-model → (entities/invariants)` — aggregate root catalogs these |
| `emitted_by` | `event → service/action` — domain events |
| `causes` | `action → outcome` — causal chain |

## 3. Confidence scoring per-assertion

Currently forgeplan has `r_eff_score` at the artifact level. We need **section-level** confidence for fine-grained tracking.

**Implementation**: a `<!-- confidence: verified --><!-- /confidence -->` HTML-comment wrapper inside markdown bodies. Forgeplan tooling parses these and:
- Renders badges in projections.
- Aggregates to artifact-level confidence (weakest-link by default).
- Flags demotion events when a wrapped assertion's confidence drops.

Alternative implementation: YAML sub-frontmatter blocks. Pick what's simpler to integrate.

## 4. New MCP tools

Add to the forgeplan MCP server. Full I/O schemas in `integration/forgeplan-mcp-additions.md` — that file is the source of truth.

| Tool name | Purpose |
|---|---|
| `forgeplan_hypothesis_status` | Query hypotheses by state/domain/age |
| `forgeplan_hypothesis_promote` | Transition a hypothesis (drafted → triangulated / verified / refuted / parked); enforces state machine |
| `forgeplan_coverage_business` | Business-specific coverage breakdown (glossary / use-cases / invariants / scenarios / canonical / R_business) |
| `forgeplan_contradictions` | List contradictory artifact pairs with severity |
| `forgeplan_orphans` | List artifacts with missing expected links (uncovered use-cases, unverified invariants, orphan terms) |
| `forgeplan_interview_packet_draft` | Create a Domain Owner interview packet from parked hypotheses (wraps C7 draft) |
| `forgeplan_interview_packet_ingest` | Ingest an answered packet; cascade updates to dependent artifacts (wraps C7 ingest) |
| `forgeplan_render_canonical` | Produce standalone DDL / SDL / pseudo-code / scenarios per domain (wraps C10) |
| `forgeplan_export_rag` | Produce a RAG-ready chunk bundle with manifest (wraps C12) |
| `forgeplan_reproducibility_check` | Run the validator on canonical docs (wraps C11) |

## 5. New commands (CLI)

```bash
forgeplan hypothesis list --domain orders --state parked
forgeplan hypothesis promote HYP-042 --state verified --evidence EVID-101
forgeplan coverage business --domain orders
forgeplan interview draft --domain orders --max 15
forgeplan interview ingest packet-2026-04-21.md
forgeplan render canonical --domain orders
forgeplan export rag --output ./rag-pkg/
forgeplan reproducibility check --domain orders
forgeplan contradictions [--domain <x>]
forgeplan orphans [--domain <x>]
```

## 6. Validation rules (per kind)

> Full rules are enumerated in each `artifact-kinds/<kind>.md`. Summary here for convenience.

### glossary
- `term` unique within `bounded_context`; `aliases` disjoint globally.
- `definition` non-empty plain-English sentence.
- `related_terms`, `contradictions` reference existing glossary IDs.

### use-case
- `actor` is a role (not a username).
- `trigger.identifier` exists.
- `steps[]` ≥ 1.
- `invariants_invoked` reference existing invariants.
- Speculative sections must be wrapped in `<!-- confidence:speculation -->` blocks.

### invariant
- `statement` is a single sentence.
- `category` is from the known list (authorization / state_transition / referential_integrity / temporal / financial / data_validation).
- `affected_use_cases` reference existing use-cases.
- `contradicts` relations with `confidence=verified` must be resolved before lifecycle = `active`.

### scenario
- `gherkin_feature` parses with `@cucumber/gherkin`.
- `invariants_verified` non-empty.
- `use_case_ref` exists.
- Embedded Mermaid (if any) renders without syntax errors.

### hypothesis
- `candidates` length ≥ 3 (unless lifecycle_state is `verified` or `refuted`).
- `selected_candidate` is one of the candidates.
- `lifecycle_state` transitions follow the state machine.
- Refuted hypotheses keep their body for historical reference; never deleted.

### domain-model
- `canonical_ddl` passes `psql --check` (enforced via C11).
- `canonical_sdl` parses with a GraphQL parser (enforced via C11).
- Every `use_cases_ref`, `invariants_ref`, `scenarios_ref`, `glossary_ref` resolves.
- Every state-machine transition has a trigger + guard.

## 7. Graph extensions

Add to `forgeplan_graph`:
- **Typed nodes** — visualize different kinds with different shapes.
- **Confidence overlay** — color edges by weakest confidence.
- **Contradiction detection** — pairs of invariants that conflict.
- **Coverage heatmap** — which domains have hypothesis density, which are thin.

## 8. Migrations (for existing workspaces)

No destructive changes. New kinds and relations are additive. Existing artifacts work unchanged.

Optional one-time migration: scan existing `spec`/`note` artifacts and auto-extract glossary terms for bootstrap.

## 9. Template files

Each new kind needs a template in the forgeplan `templates/` directory. See `templates/` in this package for ready-to-use versions.

## 10. Integration with forgeplan's existing features

| Existing feature | How it interacts with new kinds |
|---|---|
| `forgeplan_validate` | Extended with new kind-specific rules (see section 6) |
| `forgeplan_score` / `r_eff` | Aggregates section-level confidence |
| `forgeplan_drift` | Checks canonical DDL/SDL against current code |
| `forgeplan_blindspots` | Finds hypotheses without evidence, domain-models without invariants |
| `forgeplan_search` | Semantic search now includes glossary terms as search boost |
| `forgeplan_health` | Adds "hypothesis_parked_too_long" and "interview_overdue" signals |
| `forgeplan_coverage` | Per-domain coverage of use-cases vs inferred use-cases |

## 11. Backward compatibility

All changes are additive. Users who don't adopt new kinds see zero change. Users who adopt get:
- New validation rules only on new-kind artifacts.
- New MCP tools available, existing ones unchanged.
- New commands — existing commands unchanged.

## Next document

→ `05-AUTORESEARCH-INTEGRATION.md` (how to wire with autoresearch)
