# PROB-060 Phase 2/3/4 — Autonomous Execution Handoff

**Дата**: 2026-05-07
**Состояние**: Phase 0b shipped (PR #263 open). Phase 1 + Phase 0b ready для merge → Phase 2 unblocked.
**Hindsight bank**: `forgeplan` — `memory_recall("PROB-060 Phase 0b")` для full context.

---

## TL;DR (вставить в новый чат как первое сообщение)

> Я продолжаю работу над PROB-060 (distributed artifact ID assignment) в Forgeplan. Phase 0b shipped — ADR-012 R_eff = 0.90 после EVID-114 (Variant B stress test, CL2) + EVID-115 (real-workspace migration dry-run, CL3). 14 audit findings closed (9 fixed, 5 deferred). Полный контекст в `docs/sessions/2026-05-07-PROB-060-phase-2-3-4-handoff.md` и Hindsight bank `forgeplan`. Сейчас нужно довести Phase 2 (CI bot productionization + MCP responses + hint protocol slug-aware), Phase 3 (ForgePlanWeb + Skills + docs), Phase 4 (legacy migration + EVID-B/D + activation gate). Запусти AgentTeams pattern (team lead + параллельные workers) с **strict red-line #11 enforcement** — все forgeplan artifact mutations через MCP/CLI tools, **не** через Edit/Write/sed. Прочитай handoff doc и подтверди понимание.

---

## 1. Project context (terse)

**Forgeplan** — Rust CLI + MCP server для управления проектом через структурированные artifacts. Phase 1 + Phase 0b merged: slug-canonical identity + lazy display number prototype работает на ветке. Phase 2/3/4 — production rollout.

Полная methodology — `CLAUDE.md`. Главные правила без которых ничего не делать:
- **Markdown = source of truth (ADR-003)**
- **Red-line #11**: artifact mutations ТОЛЬКО через `mcp__forgeplan__forgeplan_*` MCP tools или `forgeplan` CLI — никогда не Edit/Write/sed напрямую в `.forgeplan/{prds,adrs,specs,rfcs,evidence,notes}/*.md`
- Pipeline gate каждый commit: `cargo fmt && cargo fmt --check && cargo check --workspace && cargo test --workspace --lib && cargo clippy --workspace --all-targets -- -D warnings`
- Evidence body MUST contain `verdict:`, `congruence_level:`, `evidence_type:` structured fields
- 2-agent adversarial audit после significant changes (≥3 findings each)
- No push без explicit user approval after PR review

---

## 2. Что уже работает (Phase 1 + Phase 0b state)

### Phase 1 (production)
- Frontmatter schema: `slug`, `predicted_number`, `assigned_number`
- Slug regex validation: `^(prd|rfc|adr|epic|spec|prob|sol|evid|note|ref)-[a-z0-9]+(-[a-z0-9]+)*$`
- `forgeplan new <kind> "Title"` — Phase 1 frontmatter populated
- Pre-create check vs `origin/dev` (warn если slug exists)
- Resolver `LanceStore::resolve_id` — accepts slug or display ID
- Resolver wired в **6 commands**: `get`, `validate`, `activate`, `deprecate`, `link`, `score`
- Property test 2200 trials × 11 kinds

### Phase 0b (shipped via PR #263)
- `forgeplan ci-assign-id` binary — atomic assignment prototype (Variant B stress-tested)
- `forgeplan migrate-dry-run` binary — read-only collision scanner
- `.github/workflows/assign-id.yml` workflow — dormant (label trigger), never executed in real GH Actions
- `.github/SECURITY-PROB-060.md` — PR review checklist (cargo build RCE policy)
- `docs/operations/EVID-A-real-stress-test.md` runbook (Russian, 215 lines) + `scripts/stress-test-real-gh.sh` (idempotent helper)
- CLAUDE.md «Working with artifact IDs» section
- EVID-114 (CL2 test) + EVID-115 (CL3 measurement) — both linked PROB-060/ADR-012/PRD-076/RFC-009

### What's NOT working yet
- 15+ commands ещё не accept slug: `update`, `reason`, `decompose`, `delete`, `renew`, `reopen`, `supersede`, `estimate`, `calibrate_estimate`, `fgr`, `claim`, `release`, `claims`, `import`, `ingest`
- MCP tools не возвращают `slug`/`predicted_number`/`hint` в response
- Hint protocol (PRD-071) ещё не slug-aware
- `assign-id.yml` workflow никогда не запущен в production GH Actions (Variant A user gate)
- Filename rename при assign — Phase 2.1 (currently only frontmatter переписывается)
- ForgePlanWeb не renders slug / `?` marker
- Legacy 305 artifacts ещё не migrated (Phase 4)

---

## 3. Phase 2 — CI bot production + MCP integration (5 tasks)

### Task 2.1 — Productionize `assign-id.yml` workflow

**Цель**: первый реальный run в GH Actions, упгрейд EVID-114 → CL3.

**Steps**:
1. User manual: run `bash scripts/stress-test-real-gh.sh` (10 simulated parallel PRs). Confirm 0 race conditions.
2. Apply outcome to EVID-114 body via `mcp__forgeplan__forgeplan_update`:
   - Update `congruence_level: 2` → `3`
   - Append «Variant A real-runtime measurements» section с metrics
3. Add validation gate в `.github/workflows/ci.yml`: when PR touches `.forgeplan/{prds,rfcs,adrs,specs,evidence,notes}/*.md`, verify frontmatter contains valid slug + predicted_number per SPEC-005
4. Document policy enforcement в `.github/SECURITY-PROB-060.md` (cargo build trust assumption verified)

**Files**:
- Edit `.github/workflows/ci.yml` (validation gate addition)
- Update EVID-114 body via `forgeplan_update` (NOT direct Edit)

### Task 2.2 — Filename rename при assign

**Цель**: Phase 2.1 productionization — renames `prd-auth-system.md` → `prd-074-auth-system.md` after CI assignment.

**Steps**:
1. Extend `ci_assign_id::apply_plan` (`crates/forgeplan-cli/src/commands/ci_assign_id.rs`):
   - After frontmatter rewrite, if filename matches Phase 1 pattern `<kind>-<slug>.md` (no number), rename to `<KIND>-<NNN>-<slug>.md`
   - Use `git mv` semantics — call `Command::new("git").args(["mv", from, to])` to preserve history
   - Idempotent: if filename already has number, no-op
2. Update workflow YAML `Commit and push` step — `git add -A` already covers renames
3. New tests:
   - `apply_plan_renames_file_after_assign`
   - `apply_plan_idempotent_for_already_renamed`
4. Update EVID-114 body to reflect production scope (rename now included)

**Files**:
- `crates/forgeplan-cli/src/commands/ci_assign_id.rs`

### Task 2.3 — `forgeplan reconcile-ids` command

**Цель**: manual cleanup post-merge issues (rare, but needed для recovery).

**Steps**:
1. New CLI subcommand `forgeplan reconcile-ids [OPTIONS]`:
   - `--workspace <PATH>`: target workspace (default: cwd)
   - `--check-only`: report inconsistencies без fix (default: apply)
   - `--report-cross-pr`: detect ref drift в commit messages между branches
   - `--json`: emit JSON report
2. Detection logic:
   - Files с `assigned_number` but filename pattern doesn't match (rename suggestion)
   - Files с `slug` but missing `predicted_number` (legacy migration didn't include it)
   - Cross-artifact `Related:` table mentions IDs not in frontmatter `links:` array (body-links-drift)
   - Duplicate `assigned_number` per kind (corrupt state)
3. Apply fixes:
   - Auto-rename filenames
   - Auto-add `predicted_number` from `assigned_number`
   - Suggest `links:` array updates
4. Tests:
   - happy path: clean workspace → 0 actions
   - filename mismatch → suggested rename
   - missing predicted_number → autofill
   - duplicate assigned_number → flag для manual resolution

**Files**:
- `crates/forgeplan-cli/src/commands/reconcile_ids.rs` (new)
- `crates/forgeplan-cli/src/main.rs` (register subcommand)
- `crates/forgeplan-cli/src/commands/mod.rs`

### Task 2.4 — MCP `forgeplan_new` response shape

**Цель**: agents через MCP получают `slug`, `predicted_number`, `assigned_number`, `hint` в response.

**Steps**:
1. Update `mcp__forgeplan__forgeplan_new` tool в `crates/forgeplan-mcp/src/tools/new.rs`:
   - Response JSON includes:
     ```json
     {
       "id": "PRD-74?",
       "slug": "prd-auth-system",
       "predicted_number": 74,
       "assigned_number": null,
       "id_canonical": "prd-auth-system",
       "id_display": "PRD-74?",
       "hint": "Use slug 'prd-auth-system' in commit Refs: until merged.",
       "_next_action": "forgeplan validate prd-auth-system"
     }
     ```
2. Same shape для `forgeplan_get`, `forgeplan_list`, `forgeplan_search`
3. Update tool descriptions (JSON-RPC schema) to document new fields
4. Tests:
   - `forgeplan_new_response_includes_slug`
   - `forgeplan_get_accepts_slug_or_display_id`
   - `forgeplan_search_returns_slug_field`

**Files**:
- `crates/forgeplan-mcp/src/tools/new.rs`
- `crates/forgeplan-mcp/src/tools/get.rs`
- `crates/forgeplan-mcp/src/tools/list.rs`
- `crates/forgeplan-mcp/src/tools/search.rs`

### Task 2.5 — Hint protocol slug-aware (PRD-071 update)

**Цель**: pre-merge `Next:` hints используют slug; post-merge — display number.

**Steps**:
1. В `forgeplan-mcp/src/server.rs` hint rendering:
   - Detect if artifact is pre-merge (`assigned_number: null`)
   - Pre-merge: `Next: forgeplan validate prd-auth-system` (slug)
   - Post-merge: `Next: forgeplan validate PRD-074` (display)
2. Document в `docs/methodology/agent-protocol.md` hint sample updates
3. Update CLI hints in CLI commands (forgeplan get/list/etc.)
4. Tests:
   - `hint_uses_slug_for_pre_merge_artifact`
   - `hint_uses_display_id_for_post_merge_artifact`

**Files**:
- `crates/forgeplan-mcp/src/server.rs`
- `crates/forgeplan-cli/src/commands/get.rs` (and others с hint rendering)
- `docs/methodology/agent-protocol.md`

### Task 2.6 — Resolver wiring в 15+ остальных commands (Phase 1.5b extension)

**Цель**: agents и users могут use `forgeplan <command> prd-auth-system` для ВСЕХ commands, не только 6.

**Commands** to wire:
- `update`, `reason`, `decompose`, `delete`, `renew`, `reopen`, `supersede`, `estimate`, `calibrate_estimate`, `fgr`, `claim`, `release`, `claims`, `import`, `ingest`

**Pattern** (mirror Phase 1.5b precedent commit `4c37ddd`):
```rust
pub async fn run(id: &str, ...) -> Result<()> {
    let store = LanceStore::open(workspace).await?;
    let canonical_id = store.resolve_id(id).await?;  // ← add this line
    // ... rest unchanged using canonical_id
}
```

**Tests** for each: `<command>_accepts_slug_form` + `<command>_accepts_display_id_form`.

**Files**: each `crates/forgeplan-cli/src/commands/<cmd>.rs`

### Phase 2 GA gate

- [ ] Variant A real-stress-test pass → EVID-114 CL3 ✓
- [ ] All 15+ commands accept slug (Task 2.6 done)
- [ ] MCP tools return slug+predicted+assigned+hint (Task 2.4 done)
- [ ] Hint protocol slug-aware (Task 2.5 done)
- [ ] CI workflow live in production (Task 2.1 + 2.2 done)
- [ ] `reconcile-ids` command shipped (Task 2.3 done)
- [ ] 2-agent adversarial audit (≥3 findings each, fixed/deferred)
- [ ] EVID-114 upgraded to CL3 + EVID-D drafted

---

## 4. Phase 3 — ForgePlanWeb + Skills (4 tasks)

### Task 3.1 — ForgePlanWeb derived id rendering

**Цель**: Web UI shows `PRD-074` или `PRD-74?` (с `?` marker для draft) based on `assigned_number`.

**Files**:
- `template/src/widgets/artifact-panel/lib/markdown-export.ts:33` (NodeRef rendering)
- `template/src/widgets/dependency-graph/ui/{ForceView,SunburstView}.svelte` (graph node labels)
- `template/src/widgets/insights-rail/ui/InsightsRail.svelte`
- `template/src/entities/activity/model/types.ts` (ActivityEntry shape)
- `template/src/routes/api/get/[id]/+server.ts` (API accepts both formats)

### Task 3.2 — `?` marker styling

**Цель**: visual indicator for draft (pre-merge) artifacts.

- Dashed border на graph nodes
- Pulse animation для draft state
- Hover tooltip: «Predicted number — finalized on merge»

**Files**:
- `template/src/widgets/dependency-graph/ui/*.svelte` (CSS)

### Task 3.3 — Skills update

**Цель**: AI-agents reading skills see good/bad refs examples.

**Skills to update** (in `~/.claude/plugins/marketplaces/ForgePlan-marketplace/`):
- `forgeplan-workflow:forge-cycle` — full cycle с slug-in-Refs example
- `forgeplan-workflow:forge-audit` — audit examples с slug refs
- `forgeplan-workflow:forgeplan-methodology` — section «Working with artifact IDs» (mirror CLAUDE.md update)

### Task 3.4 — Documentation updates

- `docs/operations/GIT-WORKFLOW.ru.md` — slug в commit Refs section
- `docs/methodology/UNIFIED-WORKFLOW.ru.md` — Forgeplan × Orchestra slug naming convention
- `docs/operations/QUALITY-GATES.ru.md` — CI validation gate doc

### Phase 3 GA gate

- [ ] Visual regression suite на ForgePlanWeb pass
- [ ] Skills tested через sample agent runs (verify slug usage в emitted commits)
- [ ] Docs cross-references updated
- [ ] No regression в existing ForgePlanWeb features

---

## 5. Phase 4 — Migration + activation (5 tasks)

### Task 4.1 — Cutoff date announce

- CHANGELOG entry: «PROB-060 Phase 4 — legacy migration cutoff: <DATE>»
- Grandfather rules для open PRs at cutoff (старые auto-merge через legacy schema; new PRs must use Phase 2 schema)

**Files**: `CHANGELOG.md`

### Task 4.2 — Migration script

**Цель**: legacy 305 artifacts get `slug` + `predicted_number` + `assigned_number` frontmatter (additive only — никаких contents changes).

**Steps**:
1. New CLI subcommand `forgeplan migrate [OPTIONS]`:
   - `--apply-suggested`: auto-apply 6 collision suffixes from EVID-115 dry-run
   - `--workspace <PATH>`: target (default: cwd)
   - `--dry-run`: preview only
   - `--json`: machine-readable report
2. For each artifact in `.forgeplan/`:
   - Generate slug from existing title via `slug_from_kind_title`
   - Set `slug` + `predicted_number` (= existing assigned_number) + `assigned_number` (= existing) в frontmatter
   - Apply `--auto-suffix` rules для 6 collision artifacts (from EVID-115)
3. Validation gate: assert `forgeplan migrate-dry-run --auto-suffix` returns 0 collisions after migration
4. Tests:
   - `migrate_legacy_artifact_adds_slug`
   - `migrate_handles_collision_via_auto_suffix`
   - `migrate_idempotent_on_already_migrated`

**Files**:
- `crates/forgeplan-cli/src/commands/migrate.rs` (extend existing schema migration)
- `crates/forgeplan-cli/src/main.rs`

### Task 4.3 — EVID-B (multi-agent benchmark)

**Цель**: 10 параллельных запусков `forgeplan_dispatch` × 5 agents → 0 slug collisions при unique titles.

**Steps**:
1. Test fixture: 10 reference task pools, each with 5 unique titles
2. Driver script: spawn `forgeplan_dispatch` 10 times in parallel, captures all generated slugs
3. Assert: 50 unique slugs total, no duplicates per kind
4. Wall-time measurement
5. Document via `mcp__forgeplan__forgeplan_new evidence "EVID-B title"` flow (NOT direct Write)

**Files**:
- `tests/benchmarks/multi-agent.rs` или `crates/forgeplan-cli/tests/multi_agent_benchmark.rs`
- New EVID-XXX (created via MCP, не Write)

### Task 4.4 — EVID-D (AI-agent compliance benchmark)

**Цель**: 50 reasoning prompts → ≥95% slug usage в emitted commit Refs.

**Steps**:
1. Reference prompt corpus: 50 task descriptions from PRD-057 dispatch examples
2. Run each через `forgeplan_dispatch → ci-assign-id flow`, capture commit messages
3. Parser: count Refs: usage с slug vs display number
4. Target: ≥95% slug usage
5. EVID-D pack via MCP

**Files**:
- `benchmarks/agent-compliance/` (test fixture)
- New EVID-XXX

### Task 4.5 — Activation gate

**Цель**: переключить ADR-012 / PRD-076 / RFC-009 в `active` after EVID complete.

**Pre-conditions** (all must hold):
- EVID-A CL3 ✓
- EVID-B 0 slug collisions across 10 dispatch runs ✓
- EVID-C 0 unresolved legacy collisions ✓ (already done via Phase 0b auto-apply)
- EVID-D ≥95% AI-agent compliance ✓
- EVID-E (Web rendering correctness — manual visual check) ✓
- Migration completed (Task 4.2) ✓

**Activation commands** (use MCP, NOT direct Edit):
```bash
forgeplan activate ADR-012   # via mcp__forgeplan__forgeplan_activate
forgeplan activate PRD-076
forgeplan activate RFC-009
```

R_eff target after activation: > 0.7 (ADR-012 currently 0.90 — already meets gate).

### Phase 4 GA gate

- [ ] All 5 EVID collected (A-CL3, B, C, D, E) с structured fields
- [ ] Migration completed (305 artifacts have slug + predicted + assigned)
- [ ] R_eff > 0.7 для ADR-012, PRD-076, RFC-009
- [ ] Activation done via MCP
- [ ] Feature flag `id_assignment` default = `new` (legacy stays as rollback option до v0.34)

---

## 6. AgentTeams strategy — pattern для Phase 2/3/4

### Pattern A: Team Lead + Parallel Workers (durable state)

Use **`TeamCreate`** tool when:
- Multi-step coordinated work (>3 phases)
- State needs preserve между worker exchanges
- Lead orchestrates через `SendMessage`

**Phase 2 team example**:
```
Lead: api-scaffolding:backend-architect
Workers (parallel):
  - W1 systems-programming:rust-pro    → Task 2.2 (filename rename)
  - W2 systems-programming:rust-pro    → Task 2.3 (reconcile-ids)
  - W3 agents-pro:mcp-developer        → Task 2.4 (MCP response shape)
  - W4 agents-pro:mcp-developer        → Task 2.5 (hint protocol)
  - W5 systems-programming:rust-pro    → Task 2.6 (resolver wiring 15+ commands)
```

**Critical**: each worker prompt MUST contain:
- OWNED FILES list (exact paths)
- FORBIDDEN FILES list (other workers own)
- CONTRACT spec (CLI signature, JSON shape, integration points)
- **🔴 RED-LINE #11 reminder**: «forgeplan artifact mutations через MCP/CLI ONLY — no Edit/Write/sed на `.forgeplan/{prds,adrs,specs,rfcs,evidence,notes}/*.md`»
- Pipeline gate command list
- Acceptance criteria
- Anti-patterns с konkretnym warning

### Pattern B: Single-message parallel Agent (one-shot phases)

Use **multiple `Agent` tool calls в одном message** when:
- Independent tasks, no coordination needed
- Lead role isn't required
- Quick parallel close

**Example**: Phase 3 docs updates (Tasks 3.3 + 3.4 — independent files):
```python
# В одном assistant message:
Agent(subagent_type="agents-pro:documentation-engineer", prompt="Task 3.3 skills...")
Agent(subagent_type="agents-pro:documentation-engineer", prompt="Task 3.4 docs...")
```

### Multi-agent worktree pattern (PRD-057 follow-up)

**Lesson from Phase 0b**: shared `.git/HEAD` between parallel agents causes branch ref corruption. Two options:

**Option A** — separate worktrees per agent (recommended for >2 parallel workers):
```bash
git worktree add ../forgeplan-w1 feat/prob-060-phase-2-w1
git worktree add ../forgeplan-w2 feat/prob-060-phase-2-w2
# Each worker prompt includes: «Working dir: <worktree path>»
```

**Option B** — sequential workers (slower но safe):
- Spawn 1 agent at a time, wait for completion, spawn next
- Use when worktree setup overhead не justified

For Phase 2/3/4, **prefer Option A** для parallel speed.

### Team Lead role

Team lead does NOT write code. Lead's responsibilities:
1. **Read context** (handoff doc + ADR-012 + RFC-009 + relevant code)
2. **Validate cross-worker contracts** (e.g. CLI signature CD-1 between Worker 1 binary и Worker 2 workflow)
3. **Pre-flight Contract Decisions (CD-N)** — explicitly resolve ambiguities before workers spawn
4. **Worker briefs** (4-7 ready-to-spawn prompts с file ownership grid)
5. **Risk register** (top 5 risks с mitigations)

After workers complete:
6. **Integration** — merge workers' branches с CD-N conflict resolution
7. **Audit orchestration** (spawn 2-agent adversarial review)
8. **EVID pack authoring** через MCP/CLI tools
9. **PR description prep**

**Critical**: lead MUST mention red-line #11 в каждом worker brief — workers should never directly Edit forgeplan artifact files.

### Adversarial audit mandate

After significant changes (≥5 commits or new public API), spawn 2 audit agents:
- **agents-pro:security-expert** — CWE coverage, attack surfaces, injection vectors
- **code-documentation:code-reviewer** или **agents-core:reviewer** — code quality, idioms, error handling

Each MUST find ≥3 issues. Zero findings → re-spawn at deeper level (suspect superficial review).

---

## 7. fpl agents instructions (forgeplan MCP/CLI as agent tools)

### Канонические MCP tools (use these в worker prompts)

| Operation | MCP tool | CLI equivalent |
|---|---|---|
| Create artifact | `mcp__forgeplan__forgeplan_new(kind, title)` | `forgeplan new <kind> "Title"` |
| Read artifact | `mcp__forgeplan__forgeplan_get(id)` | `forgeplan get <id>` |
| Update body/metadata | `mcp__forgeplan__forgeplan_update(id, body=...)` | `forgeplan update <id> --body @path` |
| Add link | `mcp__forgeplan__forgeplan_link(source, target, relation)` | `forgeplan link <src> <tgt> --relation <r>` |
| Activate | `mcp__forgeplan__forgeplan_activate(id)` | `forgeplan activate <id>` |
| Validate | `mcp__forgeplan__forgeplan_validate(id)` | `forgeplan validate <id>` |
| Score (R_eff) | `mcp__forgeplan__forgeplan_score(id)` | `forgeplan score <id>` |
| Health check | `mcp__forgeplan__forgeplan_health()` | `forgeplan health` |
| ADI reasoning | `mcp__forgeplan__forgeplan_reason(id)` | `forgeplan reason <id>` |

### What workers MUST do (через MCP/CLI)

**Creating EvidencePack**:
```
WRONG: Write tool to .forgeplan/evidence/EVID-XXX-...md  ← red-line #11 violation
RIGHT: mcp__forgeplan__forgeplan_new(kind="evidence", title="...")
       then mcp__forgeplan__forgeplan_update(id="EVID-XXX", body=<body with structured fields>)
       then mcp__forgeplan__forgeplan_link(source="EVID-XXX", target="<artifact>", relation="informs")
```

**Updating progress trackers в PRD/RFC**:
```
WRONG: Edit tool на .forgeplan/prds/PRD-XXX-...md  ← red-line #11 violation
RIGHT: mcp__forgeplan__forgeplan_update(id="PRD-XXX", body=<new body>)
```

**Activating artifact**:
```
WRONG: sed -i 's/status: draft/status: active/' .forgeplan/...md
RIGHT: mcp__forgeplan__forgeplan_activate(id="ADR-012")
```

### What workers MAY edit directly (NOT in red-line #11 scope)

- `CLAUDE.md` (project-wide instruction file, not artifact)
- `docs/**` (documentation, not artifact)
- `crates/**` (Rust code)
- `.github/workflows/*.yml`, `scripts/*.sh`
- `README.md`, `CHANGELOG.md`, `KNOWN-ISSUES.md`
- `.changeset/*.md`
- Test fixtures NOT in `.forgeplan/`

### Recovery if worker accidentally Edit'нул artifact

Worker should:
1. Re-Read the file to capture current content
2. Strip YAML frontmatter (forgeplan_update body excludes frontmatter)
3. Call `mcp__forgeplan__forgeplan_update(id="<ID>", body=<full body>)`
4. Last-resort fallback: `forgeplan scan-import` rebuilds LanceDB from markdown

This recovery costs nothing (idempotent). Document the violation в commit message so reviewer knows pattern was caught.

### Existing Phase 1 + Phase 0b state

Workers should NOT re-create existing artifacts:
- PRD-076, RFC-009, ADR-012, SPEC-005 — already shape'нуты в Phase 1
- EVID-114 (CL2), EVID-115 (CL3) — already created в Phase 0b
- PROB-060 (the original problem) — active

For updates to those use `mcp__forgeplan__forgeplan_update`. For NEW artifacts (e.g. EVID-D for Phase 4) use `forgeplan_new`.

### Hint protocol reading

After every `forgeplan_*` MCP/CLI call, agents read:
- `Next: <command>` — primary action
- `Or: <command>` — alternative
- `Wait: <condition>` — async retry
- `Done.` — workflow complete
- `Fix: <command>` — error remediation

JSON: `_next_action` field. Following these = staying on methodology path (Shape → Validate → Code → Evidence → Activate).

---

## 8. Operational instructions для next session

### Session start protocol

```
1. memory_recall("PROB-060 Phase 0b complete")     # Hindsight context
2. memory_recall("PROB-060 Phase 2/3/4")            # this handoff context
3. forgeplan health                                 # workspace state
4. git status && git log feat/prob-060-id-assignment..HEAD --oneline
5. cat docs/sessions/2026-05-07-PROB-060-phase-2-3-4-handoff.md
6. Verify PR #263 status (merged or pending review)
```

### Pre-Phase-2 checklist (verify before starting)

- [ ] PR #263 (Phase 0b) merged into Phase 1 base
- [ ] Phase 1 base merged into dev (separate PR, user-managed)
- [ ] EVID-114 upgraded to CL3 via Variant A run (user manual, MUST happen before Phase 2.1)
- [ ] Hindsight memory loaded для context

### ADI requirements

For Phase 2/3/4, ADI (`forgeplan reason <id>`) REQUIRED для:
- Architecture decisions affecting multi-service or breaking changes
- Algorithm choice with multiple valid approaches
- Design pattern selection
- Resource allocation strategy

For implementation deviations within accepted ADR-012 scope:
- Apply manual FPF F-G-R reasoning (3+ hypotheses, weakest-link aggregation)
- Document в commit body
- Memory_retain decision trail

### Pipeline discipline

After every code change:
```bash
cargo fmt && cargo fmt --check && \
cargo check --workspace && \
cargo test --workspace --lib && \
cargo clippy --workspace --all-targets -- -D warnings
```

After significant changes: launch 2+ parallel audit agents (adversarial mandate).

### Commit format

```
<type>(<scope>): description

Body in Russian (or English for code-only changes).

Refs: PROB-060, PRD-076, RFC-009 §Phase X, FR-...

Co-Authored-By: Claude <ai-name> <noreply@anthropic.com>
```

### Red lines reminder

- ❌ NO `git push` until user explicitly approves
- ❌ NO commit to `dev`/`main` directly
- ❌ NO PR before audit closures
- ❌ NO direct Edit/Write/sed на `.forgeplan/{prds,adrs,specs,rfcs,evidence,notes}/*.md` (red-line #11)
- ❌ NO activation without evidence (R_eff > 0)

---

## 9. Files to read in new session

Order matters:

1. `docs/sessions/2026-05-07-PROB-060-phase-2-3-4-handoff.md` (this file) — operational context
2. `docs/sessions/2026-05-07-PROB-060-phase-1-handoff.md` — Phase 0b context (predecessor)
3. `CLAUDE.md` — project rules + red-lines (especially #11)
4. `.forgeplan/adrs/ADR-012-*.md` — decision + invariant matrix
5. `.forgeplan/specs/SPEC-005-*.md` — frontmatter contract + Phase Status
6. `.forgeplan/rfcs/RFC-009-*.md` — phase rollout plan + 4/4 Phase 0b status
7. `.forgeplan/prds/PRD-076-*.md` — product requirements + 10/24 progress
8. `.forgeplan/evidence/EVID-114-*.md`, `.forgeplan/evidence/EVID-115-*.md` — Phase 0b evidence
9. `.forgeplan/problems/PROB-060-*.md`, `.forgeplan/problems/PROB-061-*.md` — problem context

---

## 10. Quick reference — key code locations

### Phase 1 + Phase 0b code

| Concept | File | Function |
|---|---|---|
| Slug validation | `crates/forgeplan-core/src/artifact/types.rs` | `validate_slug`, `slug_from_kind_title`, `render_display_id` |
| Frontmatter accessors | `crates/forgeplan-core/src/artifact/frontmatter.rs` | `slug_from_frontmatter`, `set_assigned_number`, `augment_frontmatter_with_id_fields` |
| Resolver | `crates/forgeplan-core/src/db/store.rs` | `LanceStore::resolve_id` |
| Pre-create check | `crates/forgeplan-core/src/git/mod.rs` | `artifact_filenames_in_origin_dev`, `slug_exists_in_filenames`, `validate_git_ref` |
| CI assign binary | `crates/forgeplan-cli/src/commands/ci_assign_id.rs` | `run`, `discover_candidates`, `compute_assignment_plan`, `apply_plan` |
| Migration dry-run | `crates/forgeplan-cli/src/commands/migrate_dry_run.rs` | `run`, `discover_artifacts`, `detect_collisions` |
| Workflow | `.github/workflows/assign-id.yml` | concurrency: forgeplan-id-assign |

### Phase 2 will add

| New File | Purpose |
|---|---|
| `crates/forgeplan-cli/src/commands/reconcile_ids.rs` | Manual cleanup tool |
| `crates/forgeplan-mcp/src/tools/{new,get,list,search}.rs` (updates) | Slug в MCP responses |
| Workflow CI gate | PR validation |

### Phase 3 will add

| File | Purpose |
|---|---|
| `template/src/widgets/artifact-panel/lib/markdown-export.ts` (update) | Web NodeRef rendering |
| Skills SKILL.md updates | AI-agent guidance |

### Phase 4 will add

| File | Purpose |
|---|---|
| `crates/forgeplan-cli/src/commands/migrate.rs` (extend) | Legacy migration |
| `tests/benchmarks/multi-agent.rs` | EVID-B fixture |
| `benchmarks/agent-compliance/` | EVID-D corpus |
| New EVID-XXX (B, D, E) packs | Activation evidence |

---

## 11. Known limitations (transparency)

### Pre-existing issues NOT Phase 2/3/4 scope

1. **18 integration test failures на base branch** (`augment_frontmatter_with_id_fields` strict requirement) — file separately as PROB-XXX after PR #263 merges
2. **`forgeplan_dispatch` shares working tree** — multi-agent worktree contention. File as PROB-XXX (PRD-057 follow-up). Worker prompts должны mention worktree pattern.
3. **`set_assigned_number` reformats frontmatter** (CR-4 deferred) — surgical line-edit Phase 2 nice-to-have

### Phase 0b deferred audit findings (Phase 2 candidates)

- **SEC-4**: workflow `contents: write` over-broad — Phase 2.1 GitHub App migration
- **CR-3**: `extract_number_from_filename` test gaps — add tests in Phase 2 cleanup
- **CR-4**: `set_assigned_number` reformats frontmatter — Phase 2 surgical edit
- **E2E-1**: collision detection scope — Phase 2 productionization

---

## 12. Success criteria Phase 2/3/4 complete

When all 3 phases ship:
- ADR-012 R_eff > 0.7 (currently 0.90, должно survive activation gate)
- All 305+ artifacts have slug + predicted_number + assigned_number
- Multi-agent dispatch shows 0 slug collisions (EVID-B)
- AI-agent compliance ≥95% slug refs (EVID-D)
- ForgePlanWeb renders both formats correctly (EVID-E)
- v0.31.0 release tagged с feature flag `id_assignment: new` default

**Phase 5 (Desktop Tauri) starts after this.**

---

**Session ready. Phase 0b state: shipped via PR #263. Phase 2 unblocked. Read this doc + ADR-012 + RFC-009, then start Phase 2 team lead orchestration.**
