# Tasks (for the forgeplan maintainer agent)

> Concrete checklist. Each task is independently trackable.

## How to use

The forgeplan agent should:
1. Read `00-CONTEXT.md` → `03-ARCHITECTURE.md` to understand the goal.
2. Use `ROADMAP.md` to plan waves.
3. Pick one task from this file at a time.
4. Mark as done when the exit criteria in `VERIFICATION.md` pass.

## Wave 1 tasks

### T1.1 — Add 6 new artifact kinds to forgeplan
- [ ] Register kinds: `glossary, use-case, invariant, scenario, hypothesis, domain-model`.
- [ ] Implement kind-specific validation rules from `04-FORGEPLAN-EXTENSIONS.md` section 6.
- [ ] Copy templates from `templates/*.template.md` to forgeplan's templates directory.
- [ ] Extend `forgeplan new <kind>` command parser.
- [ ] Extend `forgeplan_validate` MCP tool.
- [ ] Update `forgeplan --help` and documentation.
- **Exit**: `forgeplan new glossary "test"` creates a valid artifact, `forgeplan validate` passes.

### T1.2 — Add new relation types
- [ ] Register relations: `defines, triggers, verifies, infers_from, resolved_by, parked_in, catalogs, emitted_by, causes`.
- [ ] Extend `forgeplan_link` MCP tool.
- [ ] Update graph rendering in `forgeplan_graph`.
- **Exit**: `forgeplan link A B --relation causes` works.

### T1.3 — Confidence scoring per-assertion
- [ ] Choose implementation: HTML-comment wrapper OR YAML sub-frontmatter.
- [ ] Parser that extracts confidence levels per section.
- [ ] Aggregator that computes weakest-link artifact-level confidence.
- [ ] Display confidence badges in markdown projection.
- [ ] Flag demotion events.
- **Exit**: an artifact with `<!-- confidence: inferred -->` wrappers shows per-section confidence in the projection and in `forgeplan_score`.

### T1.4 — New MCP tools (see `integration/forgeplan-mcp-additions.md` for schemas)
- [ ] `forgeplan_hypothesis_status`
- [ ] `forgeplan_hypothesis_promote`
- [ ] `forgeplan_coverage_business`
- [ ] `forgeplan_contradictions`
- [ ] `forgeplan_orphans`
- [ ] `forgeplan_interview_packet_draft`
- [ ] `forgeplan_interview_packet_ingest`
- [ ] `forgeplan_render_canonical`
- [ ] `forgeplan_export_rag`
- [ ] `forgeplan_reproducibility_check`
- **Exit**: each tool has JSON schema, is callable via the MCP server, produces expected output.

### T1.5 — New CLI commands
- [ ] `forgeplan hypothesis list/promote`
- [ ] `forgeplan coverage business`
- [ ] `forgeplan interview draft/ingest`
- [ ] `forgeplan render canonical`
- [ ] `forgeplan export rag`
- [ ] `forgeplan reproducibility check`
- [ ] `forgeplan contradictions`, `forgeplan orphans`
- **Exit**: commands work end-to-end on a sample workspace.

### T1.6 — Build C1 skill (`ubiquitous-language`)
- [ ] Read `skills/01-ubiquitous-language.md` for full spec.
- [ ] Create `.claude/skills/ubiquitous-language/SKILL.md`.
- [ ] Create `.claude/skills/ubiquitous-language/references/extract-terms.md`.
- [ ] Create `.claude/skills/ubiquitous-language/references/validate-terms.md`.
- [ ] Wire into `/autoresearch:learn --mode glossary` (modify autoresearch's learn command OR create new command `/extract:glossary`).
- [ ] Integration test: run on a 5-service fixture → produce glossary artifacts.
- **Exit**: skill produces ≥ 80% term coverage on the fixture.

### T1.7 — Build C4 skill (`invariant-detector`)
- [ ] Read `skills/04-invariant-detector.md` for full spec.
- [ ] Create `.claude/skills/invariant-detector/SKILL.md`.
- [ ] Create reference workflow docs.
- [ ] Wire into `/autoresearch:learn --mode invariant`.
- [ ] Integration test: run on a file with known guards → detect them.
- **Exit**: detects ≥ 80% of `if/throw` guards + groups them into semantic invariants.

### T1.8 — Update forgeplan documentation
- [ ] Update forgeplan README with new kinds.
- [ ] Add section "Brownfield Business Logic Extraction" linking to skills.
- [ ] Add "Quickstart for brownfield" in README.
- **Exit**: new reader can start extraction in < 15 minutes.

---

## Wave 2 tasks

### T2.1 — Build C2 skill (`use-case-miner`)
- [ ] Read `skills/02-use-case-miner.md`.
- [ ] Create skill package.
- [ ] Entry-point detection (GraphQL mutations, REST routes, queue producers).
- [ ] Flow tracing (action → ctx.calls → DB writes → events).
- [ ] Journey assembly per entry point.
- [ ] Wire into `/autoresearch:learn --mode use-case`.
- **Exit**: maps ≥ 80% of entry points to use-cases.

### T2.2 — Build C5 skill (`causal-linker`)
- [ ] Read `skills/05-causal-linker.md`.
- [ ] Create skill package.
- [ ] Multi-file causal trace.
- [ ] Event emission/listening detection.
- [ ] Cycle detection.
- [ ] Wire into `/autoresearch:predict --persona causality-analyst`.
- **Exit**: produces graph with `causes, emits, listens_to, loop` edges.

---

## Wave 3 tasks

### T3.1 — Build C3 skill (`intent-inferrer`)
- [ ] Read `skills/03-intent-inferrer.md`.
- [ ] Design ADI prompt template (Abduction → Deduction → Induction).
- [ ] Multi-persona judges (Business Analyst, Technical Historian, Domain Lawyer, Anti-Pattern Detective, Devil's Advocate).
- [ ] Require 3+ diverse alternatives per hypothesis.
- [ ] Wire into `/autoresearch:reason --mode intent`.
- [ ] Quality check: penalize near-duplicate alternatives.
- **Exit**: for each code pattern, produces ≥ 3 hypothesis alternatives with distinct reasoning.

### T3.2 — Build C6 skill (`hypothesis-triangulator`)
- [ ] Read `skills/06-hypothesis-triangulator.md`.
- [ ] Triangulation sources: git log, git blame, legacy docs, code comments, naming patterns.
- [ ] Confidence transition rules.
- [ ] State machine enforcement (`drafted → inferred → verified/refuted/parked`).
- **Exit**: confidence distribution settles (not inflating, not collapsing).

---

## Wave 4 tasks

### T4.1 — Build C7 skill (`interview-packager`)
- [ ] Read `skills/07-interview-packager.md`.
- [ ] Cluster parked hypotheses by domain + entity + priority.
- [ ] Generate markdown packet with context + questions + response template.
- [ ] Handle answer ingestion (`forgeplan interview answer`).
- **Exit**: packet is answerable by a non-developer Domain Owner in reasonable time.

### T4.2 — Build C8 skill (`scenario-writer`)
- [ ] Read `skills/08-scenario-writer.md`.
- [ ] Gherkin feature generation from verified use-cases.
- [ ] Mermaid sequence per scenario.
- [ ] Wire into `/autoresearch:scenario --template gherkin`.
- [ ] Gherkin syntax validation.
- **Exit**: scenarios parse with a standard Gherkin parser.

### T4.3 — Build C9 skill (`kg-curator`)
- [ ] Read `skills/09-kg-curator.md`.
- [ ] Build semantic graph from all artifacts.
- [ ] Contradiction detection (conflicting invariants, circular causality, mutually-exclusive hypotheses).
- [ ] Tier-based navigation (high-level overview → drill-down).
- **Exit**: contradictions report with specific artifact IDs.

---

## Wave 5 tasks

### T5.1 — Build C10 skill (`canonical-reproducer`)
- [ ] Read `skills/10-canonical-reproducer.md`.
- [ ] Standalone DDL generation (from data models + verified schema history).
- [ ] Standalone pseudo-code (from verified use-cases + invariants).
- [ ] Standalone SDL (from GraphQL types extracted).
- [ ] Wire into `/autoresearch:learn --mode canonical`.
- **Exit**: per-domain markdown file has zero `file:line` references in final sections.

### T5.2 — Build C11 skill (`reproducibility-validator`)
- [ ] Read `skills/11-reproducibility-validator.md`.
- [ ] DDL lint (syntactic check).
- [ ] Pseudo-code walk-through vs actual code behavior.
- [ ] Scenario replay (run scenarios as tests or manual review list).
- [ ] Wire into `/autoresearch:predict --persona reproducibility-judge`.
- **Exit**: finds discrepancies between docs and code, routes to C3/C7 for resolution.

### T5.3 — Build C12 skill (`rag-packager`)
- [ ] Read `skills/12-rag-packager.md`.
- [ ] Chunk all verified artifacts at logical boundaries.
- [ ] Generate metadata (tags, domain, entity_refs, confidence).
- [ ] Produce JSON bundle ready for ingestion.
- [ ] Optional: converters for popular vector stores (Qdrant, Weaviate, Pinecone).
- **Exit**: RAG bundle importable by target store.

### T5.4 — Build orchestrator (`/extract-business-logic`)
- [ ] Read `orchestration/extract-business-logic.md`.
- [ ] Create `.claude/commands/extract/business-logic.md`.
- [ ] Phase transitions (1 → 5).
- [ ] Parallel sub-agent dispatch within a phase.
- [ ] Failure handling + retries.
- [ ] Final report generation (`extract_score` + per-skill coverage).
- [ ] Chain support: `--chain security,scenario,ship`.
- **Exit**: end-to-end run on sample brownfield produces all 10 goal artifacts from `01-PROBLEM-STATEMENT.md`.

---

## Cross-wave tasks

### TX.1 — Update forgeplan methodology docs
- [ ] Copy `02-METHODOLOGY.md` into forgeplan as methodology reference.
- [ ] Link from ADR template.

### TX.2 — Add examples repository
- [ ] Use `examples/` from this package as starter.
- [ ] Add fresh examples after running the pipeline on real brownfields.

### TX.3 — CI/CD for forgeplan
- [ ] Add tests for new kinds.
- [ ] Add linter for new relations.
- [ ] Add end-to-end extraction test (small fixture).

### TX.4 — Skill versioning
- [ ] Each skill has a version in frontmatter.
- [ ] Backward compatibility guarantees (skill v2 reads v1 artifacts).

---

## Progress tracking

Suggested approach — use forgeplan to track its own extension:

1. Create EPIC-brownfield-extension in the forgeplan-dev workspace.
2. Each wave → RFC.
3. Each task → sub-RFC or problem artifact.
4. Use `forgeplan_progress` to track completion.

## Open questions (for the forgeplan agent to resolve or escalate)

1. **Confidence scoring mechanism** — HTML-comments vs YAML blocks. Decide before T1.3.
2. **Autoresearch modification strategy** — fork or wrap? Decide before T1.6.
3. **Template sync** — one source or duplicate? Decide before T1.1.
4. **RAG target format** — neutral JSON + converters OR pick one store? Decide before T5.3.
5. **Gherkin parser choice** — which library for validation in T4.2?

## Next document

→ `VERIFICATION.md` (acceptance criteria)
