# CLAUDE.md

Instructions for Claude Code when working in this repository.
**Documentation language**: Russian. **Code**: Rust with English identifiers.

---

## 🔴 RED LINES (never do)

1. **DO NOT `rm -rf .forgeplan`** — first `forgeplan export --output backup.json` + `cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)`.
2. **DO NOT `git push`** until the user has explicitly approved the PR after review.
3. **DO NOT commit directly to `main` or `dev`** — always `feature branch → PR → merge`.
4. **DO NOT push to a branch after a PR is merged** — squash loses late commits.
5. **DO NOT create a PR before `Code → Audit → Fix → Test → Fmt → Lint → Verify`**. **Verify** означает: unit tests + **РЕАЛЬНЫЙ E2E каждой затронутой surface** (не один tool из 45 — все затронутые). Dogfood with actual user tooling (brew binary, real client). Silent failures (PROB-035, PROB-039) все от того что протестировали только happy path.
6. **DO NOT leave PRD stubs** — `forgeplan new prd` → immediately fill in the MUST sections.
7. **DO NOT activate an artifact without code and evidence** — R_eff must be > 0. **EvidencePack body MUST contain** `verdict:`, `congruence_level:`, `evidence_type:` — без этих structured fields parser тихо ставит CL0 (silent failure → R_eff = 0.1).

Everything else in this file is guidelines, not red lines.

---

## 🎯 Terminology precision (reasoning rule)

**Не использовать специализированные термины** (hexagonal, monadic, idempotent,
bounded context, SOLID, и т.д.) если не можешь точно обосновать как техническое
значение термина маппится на текущий контекст. Buzzword-matching звучит умно, но
вводит в заблуждение.

**Правильный порядок**: сначала назвать суть паттерна своими словами, потом
(если уверен) упомянуть official term как cross-reference — не наоборот.

**Пример неправильно**: «это hexagonal principle применённый к памяти» (hexagonal
про изоляцию domain от I/O — не про гранулярность storage).
**Правильно**: «разбиение на фокусированные записи с чёткими границами — это
bounded contexts из DDD, или просто separation of concerns».

---

## What is this project

**Forgeplan** — Rust CLI + MCP server (+ planned Tauri desktop) for running
a project from idea to implementation through structured artifacts: PRD, RFC, ADR,
Epic, Spec + Evidence, Problem, Note. Quality scoring via R_eff (weakest-link),
semantic search via BGE-M3, typed links, lifecycle with validation gates.

**Formula**: `Quint-code (R_eff, evidence) + BMAD (13-step PRD) + OpenSpec (DAG, delta-specs) + FPF (ADI, trust calculus) + git-adr (clap CLI) + LanceDB + Tauri`.

Маппинг "что откуда портировано" (reference repos в `sources/` → наши crates):
[`docs/operations/SOURCE-PORTING.ru.md`](docs/operations/SOURCE-PORTING.ru.md).

**CLI**: `forgeplan` (alias: `fpl`).

**Полный индекс документации**: [`docs/README.md`](docs/README.md) — map всех
гайдов (methodology, operations, schemas) и артефактов в `.forgeplan/`.

## Current status

- **v0.19.0** (2026-04-16) — `forgeplan mcp install`, website i18n RU (144 pages),
  Rust 1.95 clippy compliance, PRD-048/PROB-037 closed
- **~58 CLI commands**, **~47 MCP tools**, **1194 tests**, **0 warnings** on both feature configs
- **EPIC-001/002/003 ✅**. Phase 5 (Desktop Tauri) — backlog
- FPF KB semantic search via BGE-M3 (feature-gated, graceful fallback)

Details: `TODO.md` (priorities), `CHANGELOG.md` (history), `docs/ROADMAP.md` (gap analysis).

---

## Session Start — context priming

Three sources loaded in parallel. `MEMORY.md` is auto-loaded every turn — no command needed.
Call `memory_recall` only for records beyond the index.

| Source | Command | What it gives |
|---|---|---|
| Memory (long-term) | `memory_recall("Forgeplan")` | Prev sessions, lessons (only beyond auto-index) |
| Workspace (current) | `forgeplan health` | Blind spots, orphans, stale artifacts |
| Tasks (ongoing) | `mcp__orch__query_entities(status: "in_progress")` | Who does what now |

**Rules:**
- If a source is unavailable — continue without it, note it in the response.
- **Do NOT read at start**: `TODO.md`, `CHANGELOG.md`, `docs/ROADMAP.md` — only when directly relevant.
- **Re-warm mid-session** when switching area: `forgeplan list <kind>` or read the specific artifact.
- **"Enough context"** = can name current sprint, active PRD/RFC, any blind spots.
- If health shows blind spots (active without evidence) or orphans — **fix them before new work**.

---

## Full cycle (single source, not duplicated)

```
1. Route:    forgeplan route "task"          → determine depth
2. Shape:    forgeplan new prd "Title"       → immediately fill MUST sections
3. Validate: forgeplan validate PRD-XXX      → PASS (0 MUST errors)
4. ADI:      forgeplan reason PRD-XXX        → 3+ hypotheses (Deep/Critical: REQUIRED)
5. Branch:   git checkout dev && git pull && git checkout -b feat/xxx
6. Code:     implementation + test for every pub fn
7. Test:     cargo test                      → 0 failures
8. Fmt:      cargo fmt && cargo fmt --check  → 0 diffs
9. Lint:     cargo check                     → 0 warnings
10. Audit:   /audit (at least 2 agents)      → Fix all HIGH/CRITICAL
11. Evidence: forgeplan new evidence + link  → score > 0
12. Activate: forgeplan activate PRD-XXX
13. PR:      git push && gh pr create --base dev
14. Merge:   gh pr merge (merge commit — feat → dev; release → main)
15. Sync:    Orchestra task → Done + memory_retain
16. Progress: update FR checkboxes in PRD/RFC + TODO.md
```

**Tactical depth** (trivial, reversible, 1 file): Route → Branch → Code → Test → Fmt → Lint → Commit. No artifact, ADI, evidence, PR.

**Work is not done** until: PRD filled + validate PASS + ADI (Standard+) + evidence + R_eff > 0 + activated.

---

## Routing — one question determines depth

| Complexity | Depth | Artifacts | ADI |
|---|---|---|:---:|
| Trivial, reversible within a day | Tactical | nothing or Note | — |
| Feature 1–3 days, has a choice | Standard | PRD → RFC | recommended |
| Irreversible, 1–2 weeks | Deep | PRD → Spec → RFC → ADR | **required** |
| Cross-team, strategy | Critical | Epic → PRD[] → Spec[] → RFC[] → ADR[] | **required + review** |

**5 artifacts = 5 questions:**

| Question | Artifact | NOT needed if |
|---|---|---|
| WHAT and why? | PRD / Brief | bug-fix, refactor |
| HOW EXACTLY does it work? | Spec | no API / data model changes |
| HOW DO WE BUILD IT? | RFC | architecture is obvious, <1 day |
| WHY exactly this? | ADR | decision is trivial and reversible |
| GROUPING? | Epic | task = single PRD |

Pipeline = guideline, not bureaucracy. Don't create all 10 types for every task.

---

## EvidencePack — structured fields (critical for R_eff)

Without these fields the R_eff parser sets CL0 (penalty 0.9) and score = 0.

```markdown
## Structured Fields

verdict: supports            # supports / weakens / refutes
congruence_level: 3          # CL3 = same context (best) … CL0 = opposed (worst)
evidence_type: measurement   # measurement / test / benchmark / audit
```

---

## Lifecycle commands

```bash
forgeplan review <id>                   # check readiness
forgeplan activate <id>                 # draft → active (validation gate)
forgeplan supersede <id> --by <new>     # active → superseded (TERMINAL)
forgeplan deprecate <id> --reason "..." # → deprecated (TERMINAL)
forgeplan renew <id> --reason --until   # stale → active (extend)
forgeplan reopen <id> --reason          # stale/active → deprecated + NEW draft
```

**State machine**: `draft → active → {superseded|deprecated|stale}` ; `stale → {active via renew | deprecated + new draft via reopen}`. `superseded`/`deprecated` are terminal.

---

## Validator aliases

The validator accepts section synonyms:
- `## Problem` = `## Motivation` = `## Problem Statement` = `## Background`
- `## Goals` = `## Success Criteria` = `## Objectives`
- `## Non-Goals` = `## Out of Scope` = `## Product Scope`
- `## Related` = `## Related Artifacts` = `## Dependencies`
- `## Target Users` = `## Target Audience` = `## Users`

---

## Git — brief rules

**Branching (dev-based)**: `main` ← `release/v0.x.0` ← `dev` ← `feat/*`, `fix/*`, `docs/*`.

**Commit format** (Conventional Commits + Forgeplan refs):
```
<type>(<scope>): <description>

[body in Russian]

Refs: RFC-001, FR-001..004
```
Types: `feat`, `docs`, `fix`, `refactor`, `test`, `chore`, `progress`.
Scope: module (`cli`, `core`, `store`) or artifact (`rfc`, `prd`, `adr`).

**PR rules (minimal)**:
- Title: `[ARTIFACT-ID] description`
- feat/* → dev: **merge commit (NOT squash)** — squash loses late commits
- release/* → main: merge commit
- Before merge: `git log origin/dev..HEAD` — all commits pushed
- After merge: `git checkout dev && git pull`, **do not delete branches** (history)
- Tags: `v{major}.{minor}.{patch}` on main after release PR is merged

**Worktrees**: `git worktree add ../forgeplan-fix fix/xxx` for parallel tasks, delete after merge.

**Full rules + edge cases + lessons learned**: `docs/operations/GIT-WORKFLOW.ru.md`.

---

## Forge Mode — permission zones

| Zone | What | Mode | Examples |
|---|---|---|---|
| 🟢 Green | read-only, build, test, `forgeplan` | auto-allow | `cargo test`, `forgeplan health`, `git status` |
| 🟡 Yellow | files, `git add/commit` | acceptEdits | `Write`, `Edit`, `git commit` |
| 🔴 Red | irreversible | **BLOCKED hook** | `git push --force`, `rm -rf /`, `cargo publish`, `DROP TABLE` |

Hook: `.claude/hooks/forge-safety-hook.sh`. Whitelist: `settings.local.json`.

---

## Rust coding rules

1. **Complex patterns** → activate skills: `rust-expert`, `m01-ownership`, `m06-error-handling`, `m07-concurrency`
2. **Every new `pub fn` = test immediately** — не переходить к следующей функции без теста. Pattern:
   ```
   write pub fn → write test → cargo test → next pub fn
   ```
   НЕ собирать «пачку функций без тестов и потом тесты всем сразу» — теряется изоляция багов.
3. **Before commit** (mandatory order):
   - `cargo fmt` — форматирование
   - `cargo fmt -- --check` — 0 diffs
   - `cargo check` — 0 warnings
   - `cargo test` — все PASS
   - `cargo clippy --workspace --all-targets -- -D warnings` — 0 warnings (строже с Rust 1.95)
4. **After significant changes** — `/audit` с 2+ агентами (адверсариально: reviewer ДОЛЖЕН найти issues; 0 findings → re-review)
5. **Tools**: `/fpf` для архитектурных решений; `/forge` для structured workflow; `/forge-cycle` для полного FPF-aligned цикла

---

## Hooks enforcement (safety net)

Hooks в `.claude/hooks/` блокируют нарушения методологии на уровне shell:

| Hook | Блокирует | Когда |
|------|-----------|-------|
| `forge-safety-hook.sh` | 🔴 команды (rm -rf /, cargo publish, DROP, force-push) | pre-tool-use |
| `pre-commit-fmt.sh` | коммит если `cargo fmt --check` dirty | git commit |
| `commit-test-check.sh` | коммит если новая `pub fn` без теста | git commit |
| `pr-todo-check.sh` | PR с незакрытыми P0 | pre-push |

**Hooks — safety net, НЕ замена дисциплины**. LLM должен помнить правила во время работы, а не полагаться что hook остановит.

---

## AI-agents (non-interactive hygiene)

- `forgeplan init` — **always** with `-y` (no interactive prompt)
- Config `.forgeplan/config.yaml` — в gitignore, теряется на reinit → настроить LLM provider после init
- **Backup перед reinit** (4-step для LanceDB migration или corruption):
  1. `forgeplan export --output backup-$(date +%Y%m%d).json`
  2. `cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)`
  3. `rm -rf .forgeplan && forgeplan init -y`
  4. `forgeplan import backup-ДАТА.json`
- **Dependent sprints**: если новый sprint зависит от кода другого (ещё не merged) — `git log <base> --oneline | grep <PR-id>` перед началом. Если нет — wait for merge или branch from feature branch. Details: `docs/methodology/LESSONS.ru.md`
- **Smoke test после каждого спринта** перед commit:
  ```bash
  cargo fmt && cargo fmt --check && cargo check && cargo test       # Rust pipeline
  forgeplan init -y && forgeplan new prd "Smoke" && forgeplan validate PRD-XXX
  forgeplan score PRD-XXX && forgeplan blocked && forgeplan order   # Methodology
  forgeplan fpf ingest && forgeplan fpf search "trust"              # FPF KB
  ```
  Any fail → **НЕ коммитить**, fix first.

---

## Unified Workflow (Forgeplan × Orchestra × Hindsight)

- **Forgeplan** = WHAT to do and WHY (artifacts, quality, evidence)
- **Orchestra** = WHO does it and WHEN (tasks, deadlines, assignments)
- **Hindsight** = MEMORY (context between sessions)

**Synchronization:**
1. New artifact → task in Orchestra (if available)
2. `forgeplan activate` → mark Orchestra task Done
3. PR merged → update Orchestra + `memory_retain` in Hindsight
4. Orchestra unavailable → record in TODO.md what to sync

**Task naming in Orchestra:**
- With artifact: `[ARTIFACT-ID] description` (`[PRD-019] MCP session state machine`)
- Without artifact (bug/feature): description + Tags

**Fields**: Status (Backlog/To Do/Doing/Review/Done), Phase (Shape/Validate/Code/Evidence/Done), Depth, Artifact, Type, Sprint, Branch, Tags.

Full guide: `docs/methodology/UNIFIED-WORKFLOW.ru.md`.

## Multi-agent (v0.24.0+)

При работе 2-5 суб-агентов в одном workspace используй MCP tools:
- `forgeplan_dispatch --agents N` — получить план (buckets + serial queue)
- `forgeplan_claim <id> --ttl 30` — «я беру этот артефакт»
- `forgeplan_release <id>` — «закончил»
- `forgeplan_claims` — кто сейчас что делает

Guide: `docs/operations/MULTI-AGENT.ru.md`.

## Docs — update on every release

**Red line** для методологии: release, который добавляет user-facing MCP tool или CLI flag, **не мерджится в main** без:
- раздела в `CHANGELOG.md`
- соответствующего документа в `docs/operations/` или `docs/methodology/`
- обновлённого индекса `docs/README.md` и `docs/README.ru.md`
- упоминания новой фичи в этом CLAUDE.md (если меняет workflow)

Иначе пользователи читают stale-docs, а фичи висят undocumented. Проверяй в последнем commit перед PR.

---

## Artifacts (10 types, 6 actively used)

| Kind | Prefix | Description |
|---|---|---|
| **PRD** | `prd-` | Product Requirements Document |
| **Epic** | `epic-` | Groups PRD[]/RFC[]/ADR[] |
| **Spec** | `spec-` | API contracts, data models |
| **RFC** | `rfc-` | Architectural proposal with phases |
| **ADR** | `adr-` | Architecture Decision Record (deep+: DDR fields) |
| **Note** | `note-` | Micro-decision (auto-expires in 90 days) |
| Problem | `prob-` | Problem with context |
| Solution | `sol-` | 2–3 options (weakest-link) |
| Evidence | `evid-` | Tests, benchmarks, measurements |
| Refresh | `ref-` | Re-evaluation of stale decisions |

**Hierarchy**: Epic → PRD[] → Spec[] + RFC[] + ADR[]. Child references parent.
**Rule**: supersede, do not delete.

---

## Key formulas

### R_eff (scoring)
- `R_eff = min(evidence_scores)` — trust = weakest link, **never average**
- Evidence Decay: `valid_until` TTL, expired = 0.1
- CL penalty: CL3=0.0, CL2=0.1, CL1=0.4, CL0=0.9
- DerivedStatus: UNDERFRAMED → FRAMED → EXPLORING → COMPARED → DECIDED → APPLIED

### Smoke test (every sprint)
```bash
cargo fmt && cargo fmt --check && cargo check && cargo test  # 0 diffs, 0 warnings, all PASS
forgeplan init -y && forgeplan new prd "Smoke" && forgeplan validate PRD-XXX && forgeplan score PRD-XXX
forgeplan blocked && forgeplan order
forgeplan fpf ingest && forgeplan fpf search "trust"
```
Any fail → do not commit, fix.

---

## Storage (ADR-003): Markdown primary, LanceDB derived

```
.forgeplan/
├── adrs/ rfcs/ prds/ epics/ specs/   ← tracked, source of truth
├── evidence/ problems/ solutions/
├── notes/ refresh/ memory/
├── lance/              ← ⚠️ gitignored (derived index — forgeplan scan-import)
├── .fastembed_cache/   ← ⚠️ gitignored
└── config.yaml         ← ⚠️ gitignored (LLM keys)
```

**Fresh clone**: `git clone → forgeplan init -y → forgeplan scan-import → forgeplan list`.

**Rules**: edit via `forgeplan` CLI; direct markdown edits require `forgeplan scan-import`; DO NOT commit `lance/` or `config.yaml`.

---

## Rust Architecture

```
crates/
├── forgeplan-core/    ← shared library (12.8K LOC, 194 tests)
│   ├── artifact/ config/ db/ depth/ embed/ fpf/ graph/ health/
│   ├── journal/ lifecycle/ link/ llm/ progress/ projection/
│   ├── routing/ scoring/ search/ stale/ template/ validation/ workspace/
├── forgeplan-cli/     ← clap derive, 33 commands
└── forgeplan-mcp/     ← rmcp stdio, 26 tools
```

**Project structure**: `docs/README.md` — map of all documentation. Reference repositories in `sources/` (read-only).

---

## Non-Goals

- NOT project management (not Jira/Linear)
- NOT CI/CD, NOT SaaS, NOT a code generator
- Local-first, single binary, git for sync
