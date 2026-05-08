# PROB-060 Phase 1 → Phase 0b/2/3/4 Handoff

**Дата**: 2026-05-07
**Branch**: `feat/prob-060-id-assignment` (11 коммитов, push НЕ сделан, ждёт review)
**Hindsight bank**: `forgeplan` — 3 свежих memory с детальным state, использовать `memory_recall("PROB-060")` в начале новой сессии.

---

## TL;DR (вставить в новый чат как первое сообщение)

> Я продолжаю работу над PROB-060 (distributed artifact ID assignment) в Forgeplan. Phase 1 (1.1-1.6 + 1.5b) полностью завершён — 11 коммитов на ветке `feat/prob-060-id-assignment`, 1642+ tests pass, push не делал. Полный контекст — в `docs/sessions/2026-05-07-PROB-060-phase-1-handoff.md` и в Hindsight bank `forgeplan` (`memory_recall("PROB-060 phase 1")` для деталей). Также зафиксирован PROB-061 — change_log timeline corruption — отдельный track. Сейчас нужна Phase 0b (EVID-A prototype + EVID-C migration dry-run + CLAUDE.md update) — это hard prerequisite для Phase 2 per ADR-012 outcome-based reversal-condition. Хочу запустить multi-agent team (team lead + параллельные специалисты), детальный plan в handoff doc раздел «AgentTeams strategy». Прочитай doc и Hindsight, потом подтверди понимание и стартуй Phase 0b с team lead'ом.

---

## 1. Project context

**Forgeplan** — Rust CLI + MCP server для управления проектом через структурированные artifacts (PRD, RFC, ADR, Epic, Spec + Evidence, Problem, Note). Quality scoring через R_eff (weakest-link), semantic search через BGE-M3, typed links, lifecycle с validation gates. **Local-first, single binary, git for sync**.

Полная методология — в `CLAUDE.md` корня репо. Главные правила:
- Markdown = source of truth (ADR-003)
- Red lines: no direct push to main/dev, no PR before Code→Audit→Fix→Test→Fmt→Lint→Verify, evidence required for activation
- Pipeline на каждый коммит: `cargo fmt && cargo fmt --check && cargo check && cargo test && cargo clippy --workspace --all-targets -- -D warnings`

---

## 2. PROB-060 Decision summary

**Problem**: counter-based `next_id` (max+1 в LanceDB) даёт ID-коллизии при параллельной работе на ветках и при `forgeplan_dispatch` (multi-agent). Race-window 100% между ветками без координации.

**Decision** (ADR-012, R_eff=0.665 per FPF Trust Calculus): **Option A — Lazy Assignment (Rust RFC model)** с двухслойной identity:

| Поле | Назначение | Когда устанавливается |
|---|---|---|
| `slug` | canonical identity, immutable, в commit refs | At create, never changes |
| `predicted_number` | local prediction at create time | At create, hint only |
| `assigned_number` | display number, write-once | At merge by CI bot (Phase 2); equal to predicted in Phase 1.x |

Display rule: `id_display = assigned_number ? f"PRD-{n:03}" : f"PRD-{predicted}?"`

**Trilemma resolution**: cannot have all of {zero-coordination, stable handle, immutable identity} — Option A picks (1)+(2), drops (3) — identity мутирует pre-merge. Option B (ULID-hybrid) refuted (display number фактически не immutable). Option C (pure ULID) — fallback only (UX collapse в FPF eval).

**Outcome-based reversal**: если GitHub Actions `concurrency` group не serializes parallel merges → alternative serialization (push hooks или maintainer-only role). НЕ Option C.

---

## 3. What's shipped (Phase 1.x) — 11 commits

| # | Hash | Description |
|---|---|---|
| 1 | `d375958` | docs: Shape (PRD-076 + SPEC-005 + RFC-009 + ADR-012 + ID-ASSIGNMENT.ru.md guide) |
| 2 | `75946d2` | docs: Outcome-based reversal threshold (заменил LOC-based) |
| 3 | `cc0b398` | feat(core): Phase 1.1 — slug validation/builder/render |
| 4 | `2ce3964` | feat(cli,core): Phase 1.2 — `forgeplan new` populates frontmatter + per-phase audit |
| 5 | `3a9c697` | feat(core,cli): Phase 1.3 — pre-create slug check vs origin/dev + per-phase audit |
| 6 | `5900add` | fix(prob-060): Cross-phase audit closure (3 parallel agents) |
| 7 | `40bcf80` | feat(core,cli): Phase 1.5 — `LanceStore::resolve_id` + wired into `forgeplan get` |
| 8 | `eda11fa` | docs(prob-061): change_log timeline bug filed (separate track) |
| 9 | `4c37ddd` | feat(cli): Phase 1.5b — resolver wired в validate/activate/deprecate/link/score |
| 10 | `a330907` | feat(cli,core): Phase 1.4 + 1.6 — Slug в get output + property test (2200 trials) |
| 11 | `91642c1` | docs: Progress trackers updated в PRD-076 + RFC-009 |

### Pipeline state
- `cargo test --workspace --lib`: **1570 + 72 = 1642+ passed**, 0 failed
- `cargo fmt --check` / `cargo check` / `cargo clippy -- -D warnings`: clean
- E2E across 8+ scenarios: path traversal, control chars, BIDI, slug forms, both display formats, two-branch advisory

---

## 4. Phase progress tracker

```
Phase 0  ████░░░░░░░░░░░░░░░░░░░░  1/4   ( 25%)  0.3 done; 0.1+0.2+0.4 pending
Phase 1  ████████████████████████  6/6   (100%)  ✅ COMPLETE
Phase 2  ░░░░░░░░░░░░░░░░░░░░░░░░  0/5   (  0%)  blocked by Phase 0b
Phase 3  ░░░░░░░░░░░░░░░░░░░░░░░░  0/4   (  0%)
Phase 4  ░░░░░░░░░░░░░░░░░░░░░░░░  0/5   (  0%)
TOTAL                               7/24  ( 29%)
```

---

## 5. What's NEXT — Detailed work breakdown

### Phase 0b (HIGHEST PRIORITY — blocks Phase 2)

Methodological breach по CLAUDE.md red-line #7 (no activation without evidence). Зафиксировано в RFC-009: «Phase 0 split mid-flight — code preceded evidence, fix через Phase 0b в конце Phase 1».

#### Task 0.1 — EVID-A: CI-bot prototype + 10×concurrent-merge stress-test

**Цель**: проверить что GitHub Actions `concurrency: forgeplan-id-assign, cancel-in-progress: false` действительно сериализует параллельные merges как документировано. ADR-012 reversal-gate.

**Что строить**:
1. Минимальный `.github/workflows/assign-id.yml` workflow с concurrency group
2. Rust binary subcommand `forgeplan ci-assign-id --pr <N>` (или standalone helper) который:
   - Сканирует new artifacts в PR (по diff)
   - Находит next free `assigned_number` per kind в `origin/dev`
   - Set `assigned_number` в frontmatter, переименовывает file (`prd-slug.md` → `PRD-NNN-slug.md`)
   - Делает auto-commit `chore: assign PRD-NNN`
3. **Stress-test**: 10 simulated concurrent merges (test fixture с 10 PR ветками + scripted merges) → проверить что все получают разные numbers, никакой race condition

**Acceptance**: 10×concurrent-merge → 0 race conditions, все assigned_number'ы unique и sequential, CI run time per assignment ≤ 30s p95

**Файлы**: `.github/workflows/assign-id.yml` (new), `crates/forgeplan-cli/src/commands/ci_assign_id.rs` (new), tests/fixtures/stress-test/ (new)

#### Task 0.2 — EVID-C: Migration dry-run on 298 existing artifacts

**Цель**: detect potential slug collisions в legacy ДО Phase 4 migration.

**Что строить**:
1. Script (Rust binary subcommand или standalone) который:
   - Сканирует `.forgeplan/**/*.md` все 298 artifacts
   - Для каждого generates slug from title via `slug_from_kind_title`
   - Detect duplicates per kind
   - Report: 0 collisions = greenlight Phase 4, иначе list-collisions для manual resolution
2. Output JSON: `{kind: "prd", count: 73, collisions: [{slug: "...", artifacts: [...]}]}`

**Acceptance**: 0 unresolved legacy slug collisions OR explicit resolution plan для каждой collision

**Файлы**: `crates/forgeplan-cli/src/commands/migrate_dry_run.rs` (new) ИЛИ standalone `scripts/migrate-dry-run.sh`

#### Task 0.4 — CLAUDE.md «Working with artifact IDs» section

**Цель**: AI-agent reading CLAUDE.md видит правила сразу.

**Что писать**:
- Контракт: slug в коммитах до merge, oба формата после
- Pre-create check explained
- Reference на `docs/methodology/ID-ASSIGNMENT.ru.md`
- Forge red-line: never manually set `assigned_number`

**Файл**: `CLAUDE.md` (~50 lines добавить в существующий)

---

### Phase 2 — CI bot integration + MCP responses

Зависит от Phase 0b EVID-A success.

| Task | Что | Файлы |
|---|---|---|
| 2.1 | Production GitHub Actions workflow `assign-id.yml` (Phase 0b prototype → polished + tested) | `.github/workflows/assign-id.yml` |
| 2.2 | Slug collision auto-suffix (`prd-auth` → `prd-auth-2`) | `forgeplan-cli/src/commands/ci_assign_id.rs` |
| 2.3 | `forgeplan reconcile-ids` команда (manual cleanup) | `forgeplan-cli/src/commands/reconcile.rs` (new) |
| 2.4 | MCP `forgeplan_new` response shape с slug + predicted_number + assigned_number + hint + _next_action | `forgeplan-mcp/src/tools/new.rs` |
| 2.5 | Hint protocol (PRD-071) update — `Next:` использует slug pre-merge, number post-merge | `forgeplan-mcp/src/server.rs` (hints rendering) |

**Phase 2 GA gate**: stress-test EVID-A passes, MCP integration tests pass, hint protocol verified

---

### Phase 3 — ForgePlanWeb + Skills

Параллелится с Phase 2 (different surface area).

| Task | Что | Файлы |
|---|---|---|
| 3.1 | ForgePlanWeb derived id rendering (`PRD-074` или `PRD-74?` based on assigned/predicted) | `template/src/widgets/artifact-panel/lib/markdown-export.ts:33` + NodeRef + graph nodes |
| 3.2 | `?` marker styling (dashed border, pulse animation для draft graph nodes) | `template/src/widgets/dependency-graph/ui/*.svelte` |
| 3.3 | Skills update (`forge-cycle`, `forge-audit`, `forgeplan-methodology`) с примерами good/bad refs | `~/.claude/plugins/marketplaces/ForgePlan-marketplace/plugins/forgeplan-workflow/skills/` |
| 3.4 | Documentation: GIT-WORKFLOW.ru.md, UNIFIED-WORKFLOW.ru.md, ADR-003 cross-references | `docs/operations/`, `docs/methodology/` |

---

### Phase 4 — Migration + Activation

Конечная phase, зависит от Phase 2 + 3 + EVID-C.

| Task | Что | Файлы |
|---|---|---|
| 4.1 | Cutoff date announce в CHANGELOG; grandfather rules для open PRs | `CHANGELOG.md` |
| 4.2 | Migration script: legacy 298 artifacts get slug + assigned_number frontmatter (additive only) | `crates/forgeplan-cli/src/commands/migrate.rs` (new) |
| 4.3 | EVID-B: multi-agent benchmark (10 запусков `forgeplan_dispatch` × 5 agents) → 0 slug collisions | `tests/benchmarks/multi-agent.rs` |
| 4.4 | EVID-D: AI-agent compliance benchmark (50 reasoning prompts → ≥95% slug в `Refs:`) | `benchmarks/agent-compliance/` |
| 4.5 | Activation gate: все EVID собраны (A, B, C, D, E), R_eff > 0.7. ADR-012 → `active`. | `forgeplan activate ADR-012` |

---

### Phase 1 follow-ups (low priority — пока не блокируют)

- Slug column в `forgeplan list` / `health` / `search` outputs (Phase 1.4 extension)
- Resolver wiring в оставшиеся 15+ commands: `update`, `reason`, `decompose`, `delete`, `renew`, `reopen`, `supersede`, `estimate`, `calibrate_estimate`, `fgr`, `claim`, `release`, `claims`, `import`, `ingest` (Phase 1.5b extension)
- Display newtype refactor (`render_display_id` → `Display` impl on struct, see audit code-analyzer M2)
- Property tests с proptest crate (instead of LCG-based)

---

### PROB-061 (separate track)

`forgeplan log` reads stale LanceDB cache; git holds truth. Documented in `.forgeplan/problems/PROB-061-*.md`. Recommended fix: derive from `git log` (Option B). Blocks ForgePlanWeb F18 timeline. Не блокирует PROB-060.

**Suggested approach**: после Phase 4 closure, do `forgeplan route PROB-061` → likely Standard depth → PRD + RFC. Universal helper `crates/forgeplan-core/src/git/event_log.rs` обслуживает все temporal queries (log, journal, activity, F18).

---

## 6. AgentTeams strategy — recommended approach per phase

### Pattern: Team Lead + Specialized Workers с file ownership

Per `agent-team-orchestration` skill — strict partitioning to avoid file conflicts. Each agent gets owned/forbidden file lists. Team lead orchestrates через TeamCreate (если нужен durable state) ИЛИ через Task() spawns в один message (для one-shot phases).

### Phase 0b team (3-4 agents parallel)

**Team lead role**: backend-architect или architect-review — координирует EVID gathering, validates atomicity claims в EVID-A.

**Workers**:

| Agent | Skill | Owned files | Goal |
|---|---|---|---|
| 1 | `systems-programming:rust-pro` | `crates/forgeplan-cli/src/commands/ci_assign_id.rs` (new), `crates/forgeplan-core/src/migration/` (new) | EVID-A: ci-assign-id binary + stress-test fixture |
| 2 | `cicd-automation:deployment-engineer` | `.github/workflows/assign-id.yml` (new), `.github/workflows/ci.yml` (validation gate) | EVID-A: workflow YAML + concurrency setup |
| 3 | `systems-programming:rust-pro` | `crates/forgeplan-cli/src/commands/migrate_dry_run.rs` (new) | EVID-C: dry-run script + JSON output |
| 4 | `agents-pro:documentation-engineer` | `CLAUDE.md` (section addition) | Task 0.4: «Working with artifact IDs» section |

**Sequential constraints**:
- Agents 1 + 2 must coordinate on workflow ↔ binary contract (но файлы owned separately)
- Agent 3 independent
- Agent 4 reads docs/methodology/ID-ASSIGNMENT.ru.md, no code dependencies

**After parallel work**: team lead runs combined stress-test → review → commit как Phase 0b closure.

### Phase 2 team (5 agents parallel)

**Team lead**: `api-scaffolding:backend-architect` — coordinates CI bot ↔ MCP integration.

**Workers**:

| Agent | Skill | Owned files |
|---|---|---|
| 1 | `cicd-automation:deployment-engineer` | `.github/workflows/assign-id.yml` (productionize Phase 0b prototype), `.github/workflows/ci.yml` (validation gate addition) |
| 2 | `systems-programming:rust-pro` | `crates/forgeplan-cli/src/commands/ci_assign_id.rs` (productionize), `crates/forgeplan-cli/src/commands/reconcile.rs` (new) |
| 3 | `agents-pro:mcp-developer` | `crates/forgeplan-mcp/src/tools/new.rs` (response shape), `crates/forgeplan-mcp/src/tools/get.rs` |
| 4 | `agents-pro:mcp-developer` | `crates/forgeplan-mcp/src/server.rs` (hints rendering) |
| 5 | `agents-pro:security-expert` | review-only, no file ownership; runs stress-test + finds attack surfaces |

**Sequential**: Phase 2.1 (workflow productionization) before Phase 2.2-2.5. Within 2.2-2.5, parallel.

### Phase 3 team (4 agents parallel)

**Team lead**: `agents-domain:frontend-developer` — coordinates ForgePlanWeb changes.

**Workers**:

| Agent | Skill | Owned files |
|---|---|---|
| 1 | `agents-domain:frontend-developer` или `agents-domain:nextjs-developer` | `template/src/widgets/artifact-panel/`, `template/src/routes/api/get/[id]/+server.ts` |
| 2 | `agents-domain:frontend-developer` | `template/src/widgets/dependency-graph/`, `template/src/widgets/insights-rail/` |
| 3 | `plugin-dev:skill-development` | `~/.claude/plugins/.../forgeplan-workflow/skills/forge-cycle/SKILL.md`, `forge-audit/SKILL.md`, etc. |
| 4 | `agents-pro:documentation-engineer` | `docs/operations/GIT-WORKFLOW.ru.md`, `docs/methodology/UNIFIED-WORKFLOW.ru.md` |

### Phase 4 team (3-4 agents)

**Team lead**: `cicd-automation:deployment-engineer` (release coordinator) или `agents-pro:ddd-domain-expert`.

**Workers**:

| Agent | Skill | Owned files |
|---|---|---|
| 1 | `systems-programming:rust-pro` | `crates/forgeplan-cli/src/commands/migrate.rs` (full migration script, productionize EVID-C dry-run) |
| 2 | `agents-pro:documentation-engineer` | `CHANGELOG.md`, cutoff announce |
| 3 | `agents-core:tester` | `tests/benchmarks/multi-agent.rs` (EVID-B), `benchmarks/agent-compliance/` (EVID-D) |
| 4 (optional) | `agents-github:release-manager` | release coordination if shipping as v0.31.0 |

### After-each-phase audit (always 2-3 agents)

| Agent | Focus |
|---|---|
| `code-documentation:code-reviewer` | Adversarial review of changes (must find ≥3 issues) |
| `systems-programming:rust-pro` | Rust idiom review (ownership, lifetimes, error patterns) |
| `agents-pro:security-expert` | Security review (CWE coverage, attack surfaces) |

Cross-phase audit (after 5+ commits): 3 parallel agents с distinct focus per `5900add` precedent — methodology + code + security.

---

## 7. Operational instructions for next session

### Session start protocol

```
1. memory_recall("PROB-060 phase 1 complete")  # restores Hindsight context
2. memory_recall("PROB-061 change_log timeline")  # bonus context
3. forgeplan health  # current workspace state
4. git status && git log feat/prob-060-id-assignment --oneline -12  # confirm 11 commits
5. cd /Users/explosovebit/Work/ForgePlan && cat docs/sessions/2026-05-07-PROB-060-phase-1-handoff.md  # this file
```

### Before starting Phase 0b

Confirm understanding:
- ADR-012 outcome-based reversal-condition (NOT LOC-based — see commit 75946d2)
- Phase 1 invariants matrix (some ✅ enforced, some ⚠️ partial, some ❌ doc-only — see ADR-012 §Invariants)
- PROB-061 separate from PROB-060 (don't conflate)
- Cross-phase audit pattern reusable (3 parallel agents с distinct focus)

### Pipeline discipline (CLAUDE.md)

After every code change:
```bash
cargo fmt && cargo fmt --check && \
cargo check --workspace && \
cargo test --workspace --lib && \
cargo clippy --workspace --all-targets -- -D warnings
```

After every significant change: launch 2+ parallel audit agents (adversarial mandate — must find ≥3 issues).

### Commit format

```
feat(scope): description

Body explanation in Russian.

Refs: PROB-060, PRD-076, FR-001..N

Co-Authored-By: Claude <ai-name> <noreply@anthropic.com>
```

Type per Conventional Commits: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `progress`.

### Red lines (CLAUDE.md)

- ❌ NO `git push` без explicit user approval
- ❌ NO commit to main/dev directly
- ❌ NO PR before Code → Audit → Fix → Test → Fmt → Lint → Verify
- ❌ NO activation without evidence (R_eff > 0)

---

## 8. Files to read in new session

Order matters — read in this order для fastest context build:

1. `docs/sessions/2026-05-07-PROB-060-phase-1-handoff.md` (this file) — operational context
2. `CLAUDE.md` — project rules + red lines
3. `docs/methodology/ID-ASSIGNMENT.ru.md` — daily-use rules для humans + AI-agents
4. `.forgeplan/adrs/ADR-012-*.md` — decision + invariant matrix
5. `.forgeplan/specs/SPEC-005-*.md` — frontmatter contract + Phase Status matrix
6. `.forgeplan/rfcs/RFC-009-*.md` — phase rollout plan
7. `.forgeplan/prds/PRD-076-*.md` — product requirements + AC delivery matrix
8. `.forgeplan/problems/PROB-060-*.md`, `.forgeplan/problems/PROB-061-*.md` — problem context

---

## 9. Quick reference — key code locations

| Concept | File | Function/struct |
|---|---|---|
| Slug validation | `crates/forgeplan-core/src/artifact/types.rs` | `validate_slug`, `slug_from_kind_title`, `render_display_id` |
| Slug constants | same | `MIN_SLUG_LEN`, `MAX_SLUG_LEN`, `VALID_KIND_PREFIXES`, `RESERVED_SUFFIX_PREFIXES` |
| Frontmatter accessors | `crates/forgeplan-core/src/artifact/frontmatter.rs` | `slug_from_frontmatter`, `predicted_number_from_frontmatter`, `assigned_number_from_frontmatter`, `augment_frontmatter_with_id_fields` |
| Resolver | `crates/forgeplan-core/src/db/store.rs` | `LanceStore::resolve_id` |
| Pre-create check | `crates/forgeplan-core/src/git/mod.rs` | `artifact_filenames_in_origin_dev`, `slug_exists_in_filenames` |
| Slug-prefix mapping | `crates/forgeplan-core/src/artifact/types.rs` | `ArtifactKind::from_slug_prefix` |
| Title security guard | `crates/forgeplan-cli/src/commands/new.rs` | `validate_title` (control chars + BIDI) |
| Resolver wiring | `crates/forgeplan-cli/src/commands/{get,validate,activate,deprecate,link,score}.rs` | inline at top of each `run()` |

---

## 10. AgentTeam invocation pattern

Single-message multi-agent spawn для parallel work:

```python
# Pseudocode — actual call uses Agent tool with multiple invocations in one message
agents = [
    Agent(subagent_type="systems-programming:rust-pro",
          description="EVID-A CI bot prototype",
          prompt="..."),
    Agent(subagent_type="cicd-automation:deployment-engineer",
          description="EVID-A workflow YAML",
          prompt="..."),
    Agent(subagent_type="systems-programming:rust-pro",
          description="EVID-C migration dry-run",
          prompt="..."),
    Agent(subagent_type="agents-pro:documentation-engineer",
          description="CLAUDE.md update",
          prompt="..."),
]
# Spawn all 4 in parallel, team lead собирает results
```

For longer-running stateful coordination: use `TeamCreate` tool (deferred — load via ToolSearch) with named team and multiple addressable members.

For each agent prompt — strict file ownership specified в форме:

```
OWNED FILES (you write/modify):
- crates/forgeplan-cli/src/commands/ci_assign_id.rs (new)

FORBIDDEN FILES (other agents own these):
- .github/workflows/*.yml — owned by Agent 2
- crates/forgeplan-cli/src/commands/migrate_dry_run.rs — owned by Agent 3

CONTRACT (boundary with other agents):
- Your binary will be invoked by .github/workflows/assign-id.yml
- Stress-test fixture must be importable from tests/fixtures/stress-test/
```

---

## 11. Outstanding questions for user (only if blocked)

- Phase 2 release timing — v0.31.0 separate release или ждать full Phase 4?
- ForgePlanWeb team availability for Phase 3 — параллельный track или sequential?
- PROB-061 priority — после Phase 4 или раньше (если F18 critical)?

These are NOT blocking Phase 0b start — can ask later.

---

**Session ready to continue. Commit `91642c1` is current HEAD на feat/prob-060-id-assignment. 1642+ tests passing. Phase 0b is the next focused work.**
