# Forgeplan DDD Analysis — Spike-3 (2026-04-21)

> **Provenance**: Produced by the `agents-pro:ddd-domain-expert` agent on the Forgeplan
> Rust workspace (`crates/forgeplan-core`, `crates/forgeplan-cli`, `crates/forgeplan-mcp`)
> as Spike-3 for EPIC-008 Factum/Intent methodology validation. This is a **CL3
> measurement** — all code references are verified against real file paths and struct
> names in the repository at the time of the spike.
>
> **Purpose**: empirically validate that `ddd-expert` agent output can be
> deterministically mapped to forge artifacts (Epic, PRDs, Specs, Notes, and future
> EPIC-008 kinds: glossary, invariant, hypothesis). Demonstrated mapping ships in
> peer marketplace: `forgeplan-brownfield-pack/mappings/ddd-to-forge.yaml`.

## 1. Bounded Contexts

### 1.1 Artifact Authoring Context
**Responsibility**: Own the creation, identity, parsing, and filesystem projection of structured markdown artifacts (PRD, RFC, ADR, Epic, Spec, Note, Problem, Solution, Evidence, Refresh, Memory).

**Modules**:
- `crates/forgeplan-core/src/artifact/` (types, frontmatter, identity, sections, store, delta)
- `crates/forgeplan-core/src/template/engine.rs`
- `crates/forgeplan-core/src/projection/mod.rs`

**Neighbors**:
- → Lifecycle: **Customer-Supplier** (lifecycle depends on artifact types; artifact is upstream).
- → Validation: **Published Language** (artifact frontmatter + body sections are the shared schema).
- → Persistence: **Anti-Corruption Layer** via `projection::render_projection_record` (translates `ArtifactRecord` ↔ on-disk markdown). Files-first per ADR-003.

### 1.2 Lifecycle & Methodology-Gate Context
**Responsibility**: Enforce the state machine `draft → active → {superseded|deprecated|stale}` with activation gates (length, evidence, stub, MUST validation).

**Modules**:
- `crates/forgeplan-core/src/lifecycle/mod.rs` + `transitions.rs`
- `crates/forgeplan-core/src/status/derived.rs`
- `crates/forgeplan-core/src/phase/mod.rs` (per-artifact phase)
- `crates/forgeplan-core/src/session/mod.rs` (session-wide phase)

**Neighbors**:
- → Scoring: **Customer-Supplier** (gates call `get_relations` + `r_eff_recursive` as inputs).
- → Validation: **Customer-Supplier** (gates call `validation::validate` + `rules::check_stub`).
- → Persistence: **Conformist** (reads/writes `ArtifactRecord.status` strings as-is).

### 1.3 Trust & Scoring Context (R_eff)
**Responsibility**: Compute the weakest-link trust score, decay, FGR formality/granularity/reliability, confidence interval. Never averages — invariant is `min()`.

**Modules**:
- `crates/forgeplan-core/src/scoring/` (reff, evidence, decay, fgr)
- `crates/forgeplan-core/src/fpf/core/trust.rs`

**Neighbors**:
- → Graph: **Customer-Supplier** (recursion walks typed relations for dependency penalties).
- → Lifecycle: **Open Host Service** (exposes `r_eff`, `r_eff_recursive`, `r_eff_with_ci` as stable API).
- → Artifact: **Conformist** (parses `EvidencePack` body fields `verdict/congruence_level/evidence_type`).

### 1.4 Linking & Graph Context
**Responsibility**: Typed relations (`informs`, `based_on`, `supersedes`, `contradicts`, `refines`, `supports`), dependency graph traversal, topological order, cycle detection, bounded-context auto-clustering.

**Modules**:
- `crates/forgeplan-core/src/link/mod.rs`
- `crates/forgeplan-core/src/graph/` (mod, topological, knowledge)
- `crates/forgeplan-core/src/fpf/contexts.rs` (connected-component clustering)

**Neighbors**:
- → Persistence: **Shared Kernel** with `RelationStorage` trait (relations table is shared representation).
- → Scoring: **Customer-Supplier** (supplies edges for recursion).
- → Artifact: **Published Language** (link frontmatter YAML is the wire format).

### 1.5 Validation & Routing Context
**Responsibility**: MUST/SHOULD/COULD rule evaluation per (kind, depth); keyword + LLM + ADI three-level router that picks depth and pipeline.

**Modules**:
- `crates/forgeplan-core/src/validation/` (rules, checks, adversarial)
- `crates/forgeplan-core/src/routing/` (pipeline, rules, signals, skills)
- `crates/forgeplan-core/src/depth/mod.rs`

**Neighbors**:
- → LLM Reasoning: **ACL** (routing calls LLM via `LlmConfig`, falls back gracefully to keyword rules).
- → Lifecycle: supplies `ValidationResult` as published value object.

### 1.6 Persistence & Indexing Context (LanceDB / Files-First)
**Responsibility**: Durable storage for `artifacts`, `evidence`, `relations`, `fpf_spec`, `change_log` tables; vector similarity; keyword/BM25 search; scan-import from markdown.

**Modules**:
- `crates/forgeplan-core/src/db/` (store, schema, migrate, convert)
- `crates/forgeplan-core/src/driver/` (traits, in_memory, lance, factory)
- `crates/forgeplan-core/src/embed/mod.rs`
- `crates/forgeplan-core/src/search/` (smart, bm25, filter)
- `crates/forgeplan-core/src/scan/` (import from markdown)
- `crates/forgeplan-core/src/git/mod.rs`

**Neighbors**:
- → Artifact: **ACL** via `projection` + `scan::import` (bidirectional translation between markdown and Arrow records).
- → All upstream contexts: **Open Host Service** via `StorageDriver` supertrait (`ArtifactStorage + RelationStorage + SearchStorage + VectorStorage + FpfStorage`). Explicitly ISP-segregated in `driver/mod.rs:6-14`.

### 1.7 Multi-Agent Coordination Context
**Responsibility**: Claim protocol (advisory locks on artifacts), dispatch plan (file-overlap Jaccard → parallel buckets + serial queue), workspace lock (OS flock), agent identity stamping.

**Modules**:
- `crates/forgeplan-core/src/claim/mod.rs`
- `crates/forgeplan-core/src/dispatch/mod.rs`
- `crates/forgeplan-core/src/workspace/lock.rs`
- `crates/forgeplan-core/src/artifact/identity.rs` (`AgentIdentity`)

**Neighbors**:
- → Persistence: **Customer-Supplier** (needs `next_id`, workspace path).
- → Session/Phase: **Separate Ways** (deliberately decoupled — claim is about artifacts, session about methodology phases).

### 1.8 Observability & Audit Context
**Responsibility**: Append-only activity log (JSONL per day, args_hash), change_log table, journal timeline, health report, undo, drift detection, coverage, duplicate detection.

**Modules**:
- `crates/forgeplan-core/src/activity/mod.rs`
- `crates/forgeplan-core/src/changelog/mod.rs`
- `crates/forgeplan-core/src/journal/mod.rs`
- `crates/forgeplan-core/src/health/mod.rs`
- `crates/forgeplan-core/src/undo/`, `drift/`, `coverage/`, `duplicate/`, `gaps/`, `stale/`, `discover/`

**Neighbors**:
- → Every write-path context: **Observer** (pure read-only; log-write failures never block a tool call, per `activity/mod.rs:16`).
- → Persistence: **Conformist** (consumes existing records without requiring upstream changes).

### 1.9 Interface Contexts (CLI / MCP)
**Responsibility**: Two adapter surfaces onto the domain core — `clap`-based CLI binary and `rmcp`-based stdio MCP server exposing ~47 tools.

**Modules**:
- `crates/forgeplan-cli/src/` (commands/, main.rs)
- `crates/forgeplan-mcp/src/` (server.rs, convert.rs, types.rs)

**Neighbors**:
- → forgeplan-core: **Conformist** — both adapters bind to core types (`ArtifactKind`, `Mode`, `ArtifactRecord`) directly. No translation layer; when core changes, both adapters recompile.

---

## 2. Aggregates

### Artifact Authoring Context

- **`Artifact` (root)** — `artifact/types.rs:207`. Contains `Meta` value object + `body` + `embedding`. Invariants: id must match prefix from `ArtifactKind::prefix()`, slugified title determines filename, parent_epic references an Epic.
- **`ArtifactKind` (value object)** — `artifact/types.rs:9`. Enum with prefix/dir_name/template_key.
- **`Status` (value object)** — `artifact/types.rs:142`. Five states; `#[serde(rename_all = "snake_case")]` (renamed `RefreshDue` → `Stale` to match string-based lifecycle checks — PROB-040 C1 fix 2026-04-21).
- **`Link` (value object)** — `artifact/types.rs:161`. `{target, relation}`. Immutable.
- **`Mode` / depth (value object)** — `artifact/types.rs:168`. tactical/standard/deep/note.

### Lifecycle Context

- Lifecycle operations are a **domain service**, not an aggregate — `lifecycle::activate/supersede/deprecate/renew/reopen`. Guarded by `transitions::validate_transition`.
- **`GatesReport` (value object)** — `lifecycle/mod.rs:76`. Immutable snapshot: `length_ok`, `evidence_ok`, `stub_ok`. Enforces DRY invariant that review and activate produce the same verdict.
- **`PhaseState` (aggregate root, per-artifact)** — `phase/mod.rs`. History bounded to 1024 entries; reason ≤512 bytes.
- **`SessionState` (aggregate root, workspace-level)** — `session/mod.rs:48`. `Phase` enum + last 20 transitions.

### Trust & Scoring Context

- **`EvidenceItem` (value object)** — `scoring/reff.rs:37`. Immutable: `id`, `evidence_type`, `verdict`, `congruence_level` (0–3), `valid_until`.
- **`Verdict` / `EvidenceType` (value objects)** — `scoring/reff.rs:11,20`. Pure enums with `.score()` method.
- **`ReffCi` (value object)** — `scoring/reff.rs:97`. Confidence interval: point, low, high, evidence_count, stale_count.
- **`AssuranceReport` (value object)** — `scoring/reff.rs:185`. Cycle-aware recursive result.
- **Weakest-link invariant**: `r_eff()` uses `min_by`, never `average` — `scoring/reff.rs:76-85`.

### Linking & Graph Context

- **`Edge` (value object)** — `graph/mod.rs:12`. Directed `{from, to, relation}`.
- **`BoundedContext` (value object, auto-detected)** — `fpf/contexts.rs:12`. Connected-component result; not a persisted aggregate.
- **Relations are a set-aggregate** owned by `RelationStorage` trait. Invariants: no duplicate `(source, target, relation)`; `normalize_relation` accepts snake_case + kebab-case; targets uppercase (`link/mod.rs:10`).

### Persistence Context

- **`ArtifactRecord` (aggregate root)** — `db/store.rs:78`. id, kind, status, title, body, depth, author, parent_epic, cached r_eff_score, valid_until, timestamps, tags, body_hash, embedding. Invariants: body_hash preserved across full-row rewrites (audit C2); embedding preserved across tag mutations.
- **`LanceStore` (repository)** — `db/store.rs`. Implements all five driver traits.
- **`NewArtifact` (factory input DTO)** — `db/store.rs:61`.
- **`VectorSearchHit` (value object)** — `db/store.rs:110`.
- **`FpfChunk` / `ChangeLogEntry`** — smaller aggregates with their own tables.

### Multi-Agent Context

- **`Claim` (aggregate root)** — `claim/mod.rs:40`. One file per claimed artifact. Invariants: TTL within `MIN_TTL=60s`..`MAX_TTL=24h`; `agent_id` non-empty; `id` validated against `../` traversal; file ≤64 KB.
- **`DispatchPlan` (value object, computed)** — `dispatch/mod.rs:124`. Not persisted. Invariants: `agent_count ≤ MAX_AGENTS=64`; skills ≤32; affected_files ≤512.
- **`ArtifactCandidate` (value object, dispatcher input)** — `dispatch/mod.rs:106`.
- **`AgentIdentity` (value object)** — `artifact/identity.rs:53`. Sanitized against bidi/RTL/zero-width chars.

### Observability Context

- **`ActivityEntry` (value object, append-only)** — `activity/mod.rs:29`. Daily-rotated JSONL.
- **`ChangeLogEntry` (value object)** — `changelog/mod.rs:5`.
- **`HealthReport`, `JournalEntry`, `BlindSpot`, `AtRiskArtifact`, `DuplicatePair`, `ActiveStub`** — read-model DTOs in `health/mod.rs`.

---

## 3. Ubiquitous Language Glossary (23 terms)

| Term | Definition | Context | Code reference |
|---|---|---|---|
| **Artifact** | Durable record of a decision, plan, or evidence with lifecycle, links, and optional embedding. | Artifact Authoring | `Artifact` struct, `artifact/types.rs:207` |
| **ArtifactKind** | Typed taxonomy of the 11 artifact flavors. | Authoring | enum in `artifact/types.rs:9` |
| **R_eff** | Effective reliability = min of evidence scores (weakest-link). Never averages. | Trust & Scoring | `fn r_eff`, `scoring/reff.rs:76` |
| **Congruence Level (CL)** | 0–3 alignment rating between evidence and its target context; penalty 0.9/0.4/0.1/0.0. | Trust & Scoring | `fn cl_penalty`, `scoring/reff.rs:47` |
| **EvidenceItem** | Verdict + CL + evidence_type + TTL, parsed from `EvidencePack` body fields. | Trust & Scoring | `struct EvidenceItem`, `scoring/reff.rs:37` |
| **Verdict** | `supports` / `weakens` / `refutes`, scoring 1.0/0.5/0.0. | Trust & Scoring | enum in `scoring/reff.rs:20` |
| **Weakest Link** | Dependency or evidence with lowest effective score; surfaced in `AssuranceReport`. | Trust & Scoring | `AssuranceReport.weakest_link`, `scoring/reff.rs:189` |
| **Lifecycle Status** | Persisted state in `{draft, active, superseded, deprecated, stale}`. | Lifecycle | enum `Status`, `artifact/types.rs:142` |
| **DerivedStatus** | Computed methodology progress: Stub → Shaped → Validated → Evidenced → Activated. | Lifecycle | `status/derived.rs:6` |
| **Phase** | Per-artifact or per-session workflow marker (idle/routing/shaping/coding/evidence/pr). | Lifecycle | `Phase` enum, `session/mod.rs:17` and `phase/mod.rs` |
| **Gates (Activation Gates)** | Length (≥100), evidence linked, not a stub — ALL must pass to go draft→active. | Lifecycle | `GatesReport`, `lifecycle/mod.rs:76` |
| **LinkType (relation)** | Typed edge: `informs`/`based_on`/`supersedes`/`contradicts`/`refines`/`supports`. | Graph | `VALID_RELATIONS`, `link/mod.rs:89` |
| **Depth / Mode** | Task complexity classifier: tactical/standard/deep/note; drives pipeline + validation rules. | Validation & Routing | enum `Mode`, `artifact/types.rs:168` |
| **Pipeline** | Ordered sequence of `ArtifactKind` the router prescribes for a task. | Validation & Routing | `RoutingResult.pipeline`, `routing/mod.rs:22` |
| **Routing Level** | 0 = keyword rules, 1 = LLM-classified, 2 = FPF ADI reasoning. | Validation & Routing | `routing/mod.rs:27` |
| **Validation Finding** | Rule hit with severity `Must/Should/Could`. | Validation | `validation/mod.rs:28` |
| **Claim** | Advisory TTL-bound assertion "agent X works on Y". | Multi-Agent | `struct Claim`, `claim/mod.rs:40` |
| **DispatchPlan** | Parallel buckets + serial queue computed from file-set Jaccard overlap. | Multi-Agent | `struct DispatchPlan`, `dispatch/mod.rs:124` |
| **AgentIdentity** | Sanitized `{name, version}` written into `last_modified_by` frontmatter. | Multi-Agent | `artifact/identity.rs:53` |
| **Workspace** | `.forgeplan/` directory — source of truth (markdown) + derived LanceDB index. | Persistence | `FORGEPLAN_DIR`, `workspace/init.rs:7` |
| **Projection** | Rendering of `ArtifactRecord` to markdown on disk (files-first, ADR-003). | Persistence | `render_projection`, `projection/mod.rs:34` |
| **Scan-import** | Rebuild the LanceDB index from markdown files. | Persistence | `scan/import.rs` |
| **FGR** | Formality-Granularity-Reliability trio feeding the FPF trust score. | Trust & Scoring | `scoring/fgr.rs`, `fpf/core/trust.rs` |

---

## 4. Domain Events (12)

Currently the codebase uses **database side effects** as its event substrate (change_log table + activity log JSONL). No explicit `DomainEvent` type exists yet — this is a key gap Spike-3 surfaces for EPIC-007/008.

Events that logically cross context boundaries:

1. `ArtifactCreated { id, kind, title, author, created_at }` — `db/store.rs::create_artifact` + changelog `action=create`.
2. `ArtifactActivated { id, from_status: "draft", to_status: "active", forced, must_errors }` — `lifecycle::activate`, `lifecycle/mod.rs:240`.
3. `ArtifactSuperseded { id, replacement_id, dependents, occurred_at }` — `lifecycle/mod.rs:332`.
4. `ArtifactDeprecated { id, reason, dependents }` — `lifecycle/mod.rs:399`.
5. `ArtifactRenewed { id, old_valid_until, new_valid_until, reason }` — `lifecycle/mod.rs:434`.
6. `ArtifactReopened { old_id, new_id: draft, new_kind, reason }` — `lifecycle/mod.rs:495`.
7. `ArtifactStale { id, valid_until, days_expired }` — `stale/mod.rs:17` (derived).
8. `LinkAdded { source, target, relation }` — `link::add_link` + `RelationStorage::add_relation`.
9. `EvidenceAttached { evidence_id, artifact_id, verdict, cl }` — `add_relation(EVID-*, artifact, "informs"|"supports")`.
10. `ClaimAcquired { artifact_id, agent_id, expires_at }` / `ClaimReleased { artifact_id, agent_id }` — `claim/mod.rs`.
11. `PhaseTransitioned { artifact_id, from, to, timestamp }` — `PhaseTransition`, `session/mod.rs:65`.
12. `ReffRecomputed { id, old_score, new_score, weakest_link }` — currently implicit (cached in `ArtifactRecord.r_eff_score`).

Published-language candidates: events 1–6 and 8 are stable enough to serialize to JSON for cross-context integration.

---

## 5. Context Integration Patterns

| Pair | Pattern | Notes |
|---|---|---|
| Authoring ↔ Persistence | **ACL** via `projection` + `scan::import` | Bidirectional: markdown is source of truth; LanceDB is derived. RFC-004 files-first. |
| Lifecycle → Scoring | **Customer-Supplier**, Scoring upstream | Activation gate calls `r_eff_recursive` transitively. |
| Lifecycle → Validation | **Customer-Supplier** | Single source of truth for both `review` and `activate` (DRY fix M-4). |
| Scoring ↔ Graph | **Shared Kernel** (relations) | `r_eff_recursive` walks `RelationStorage` edges. |
| Routing → LLM | **ACL** | Feature-flag + graceful fallback. Level 0 (keywords) always works. |
| Persistence ↔ CLI/MCP | **Open Host Service** via `StorageDriver` trait | ISP-segregated 5 traits. Blanket supertrait for back-compat (`driver/mod.rs:220`). |
| Multi-Agent → Persistence | **Customer-Supplier** | Workspace lock serializes writes at OS level. |
| Observability → all | **Observer** | Activity log + change_log are downstream consumers; never block writes (`activity/mod.rs:16`). |
| CLI ↔ MCP | **Separate Ways with Shared Kernel** | Both bind to `forgeplan-core` types directly. Candidate for ACL if surfaces grow. |
| FPF KB semantic search | **Conformist via feature-gate** | Embedding column unconditional in schema; only encoding feature-flagged. |

### Potential ACLs / translation points to add

1. **Domain events as published language** — today `ActivateResult`, `SupersedeResult`, `RenewResult`, `ReopenResult` are ad-hoc DTOs. A common `DomainEvent` trait with `event_type/aggregate_id/occurred_at/payload` would give cross-context and cross-process integration a stable wire format. Directly relevant for Spike-3's orchestrator-mapping goal.
2. **ArtifactRecord vs Artifact split** — persistence DTO and domain type coexist without a converter. Adapters silently couple lifecycle code to storage schema shape.
3. **Relation semantics duplicated** — `link::VALID_RELATIONS` and scoring's dependency-type list (`reff.rs:297`) are two lists. Scoring includes `depends_on`, which is NOT in VALID_RELATIONS — silent dead code or pending feature.

---

## 6. Category Errors Found

**E1 — Two "Artifact" types without explicit converter.** `artifact::types::Artifact` (domain) vs `db::store::ArtifactRecord` (persistence): conceptually same aggregate but drifted (`ArtifactRecord` adds `r_eff_score` cache + `body_hash`). Business logic operates on `ArtifactRecord` directly — persistence schema leaks into domain.

**E2 — `RefreshDue → Stale` rename exposes string-based lifecycle checks.** Per comment `artifact/types.rs:135`, lifecycle checks compared against string literal `"stale"` — enum variant was `RefreshDue` with serde-rendered `refresh_due`. Silent bug fixed PROB-040 C1. Root cause: enum-to-string coupling across context boundary without shared vocabulary.

**E3 — Role ≠ function confusion in driver supertrait.** `StorageDriver = ArtifactStorage + RelationStorage + SearchStorage + VectorStorage + FpfStorage` treats *what the backend can do* (role) and *specific function sets* as one. `InMemoryStore` panics on `search_fpf_by_vector` because it cannot fulfill the function — the trait comment at `driver/mod.rs:189` explicitly documents that this "must not silently Ok(empty)". Cleaner: capability-flags, not blanket-implemented traits with mandatory methods.

**E4 — "Method vs work" confusion in `phase` vs `session` vs `claim`.** Three parallel state machines. No common abstraction. Reader cannot tell without reading all three whether phase, claim, session are layers, alternatives, or competitors. EPIC-008 recommendation: explicit `WorkflowState` aggregate or documented orthogonality (claim = *who*, phase = *where in pipeline*, session = *workspace-wide current focus*).

**E5 — "Bounded context" used with two different meanings.** `fpf::contexts::BoundedContext` (`fpf/contexts.rs:12`) is an auto-detected artifact cluster from connected-component analysis — a metric. The project docs (`docs/methodology/UNIFIED-WORKFLOW.ru.md`) also use "bounded context" in the DDD sense. Two different domain nouns, same symbol. **Recommend**: rename FPF `BoundedContext` to `ArtifactCluster` or `ContextModule`; reserve "bounded context" for strategic DDD meaning.

**E6 — Relations aggregate boundary is implicit.** Relations live in a separate LanceDB table but are accessed as if they are part of the artifact aggregate (`collect_activation_gates` calls `get_relations` + `get_incoming_relations`). Small-aggregate trade-off: consistency is eventual across implicit aggregate. At ≤1000 artifacts this is fine; past that, a read model or explicit transactional boundary will be needed.

---

## 7. Deliverables from this spike

- **This document** — `docs/architecture/ddd-analysis-spike-3.md`.
- **Mapping file** — `plugins/forgeplan-brownfield-pack/mappings/ddd-to-forge.yaml` in peer marketplace repo. Demonstrates how this report can be mechanically ingested as forge artifacts (Epic per bounded context, PRD per aggregate root, Note per glossary term, Problem per category error).
- **Evidence artifact** — EVID-NNN (this session) — CL3 measurement linked to EPIC-007 + EPIC-008.

## 8. Implications for EPIC-008

This Spike-3 measurement directly validates three EPIC-008 Wave 1 claims:
- **6 new kinds needed**: especially `glossary` (for §3), `invariant` (for §2 aggregate rules), `hypothesis` (for §6 category errors with confidence levels), `domain-model` (for §1 bounded-context canonical).
- **12 MCP tools viable**: `forgeplan_hypothesis_*`, `forgeplan_contradictions`, `forgeplan_orphans` have concrete use cases proven here.
- **Factum/Intent separation is real and enforceable**: §1-§5 are factum (100% code-derived, cite-able), §6 is intent (reasoning about *why* things are the way they are).
