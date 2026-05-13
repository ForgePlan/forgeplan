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
8. **DO NOT call `LanceStore::create_artifact / update_* / delete_* / add_relation / delete_relation` directly from `commands/*.rs` или `server.rs`** — нарушает ADR-003 (markdown — source of truth). Используй `forgeplan_core::projection::sync_file_to_store` + `render_projection` flow (см. canonical `crates/forgeplan-cli/src/commands/deprecate.rs`). Regression guard: `tests/adr_003_invariant.rs` блокирует рост counts. Migration tracker: PROB-048 + PRD-073.
9. **DO NOT skip the post-release sync step** — после каждого `release/v* → main` PR merge обязательно открывай sync-PR `chore/sync-main-to-dev-after-vX.Y.Z` (branch protection blocks direct push to dev). Без этого dev forever lags `Cargo.toml` version, и следующий release создаст merge conflicts. См. PR #223 как канонический пример flow.
10. **DO NOT ignore Dependabot alerts at release time** — перед каждым `release/v* → main` PR проверь `gh api repos/.../dependabot/alerts` и в release notes пометь: addressed / scheduled / accepted-with-justification. Накопление alerts — отдельный architectural debt (PROB-XXX TBD при первом triage).
11. **STRICT: Forgeplan artifacts мутировать ТОЛЬКО через MCP/CLI** —
  файлы в `.forgeplan/{prds,adrs,specs,rfcs,evidence,notes}/*.md` нельзя
  редактировать через `Edit`/`Write`/`sed` напрямую. Все изменения тела/статуса
  идут через `mcp__forgeplan__forgeplan_update`, `forgeplan_new`,
  `forgeplan_link`, `forgeplan_activate`, `forgeplan_deprecate` (или
  эквивалентный CLI `forgeplan update|new|link|activate|...`). Прямой Edit
  десинхронизирует LanceDB index, state machine (`.forgeplan/state/<ID>.yaml`)
  и canonical body — `forgeplan_get` начнёт возвращать stale данные,
  semantic search промахнётся. Если случайно отредактирован — recover через
  `forgeplan_update id=<ID> body=<full new body>` (читаешь файл, формируешь
  полное новое body без YAML frontmatter, пушишь через MCP). Last-resort
  fallback: `forgeplan scan-import` пересоберёт LanceDB из markdown.
  Direct Edit OK ТОЛЬКО для не-forgeplan markdown (READMEs, CLAUDE.md,
  KNOWN-ISSUES, src code, .changeset/*.md).

Everything else in this file is guidelines, not red lines.

---

## 🤖 Hint protocol (PRD-071 — agent reading)

Каждый CLI/MCP вывод эмитит **один** контрактный маркер для следующего шага:
- **`Next: <full command>`** — основное действие (run as-is, real IDs, no placeholders)
- **`Or: <command>`** — альтернатива к Next (если primary blocks)
- **`Wait: <condition>`** — async/TTL — retry после condition
- **`Done.`** — workflow complete (terminal)
- **`Fix: <command>`** — error remediation (paired with `Error:`)

JSON: `{"_next_action": "<command>" | null, ...}`. Полный контракт + bad/good примеры:
[`docs/methodology/agent-protocol.md`](docs/methodology/agent-protocol.md).

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

- **v0.31.0** (2026-05-13) — Wave 9 polish: 19-finding adversarial audit
  closure. SEC-C1+C2 input gate (`validate_title` lifted to core, gated at all
  4 mutation paths — CLI new/update + MCP new/update). SEC-H1 output gate
  (`sanitize_for_hint` on 8 CLI command print sites, completes LOG-001 from
  v0.30). SEC-H2 HOME sanitiser bare-string masking. SEC-H3 MCP error chain
  sanitisation across 40+ `McpError::internal_error` sites. ARCH-C1
  `health_report_to_json` helper extract — single source of truth for
  CLI/MCP wire shape. PROB-051 closed end-to-end (R_eff=0.80 grade B).
- **76 CLI commands**, **72 MCP tools**, **2724 tests**, **0 warnings** on both feature configs
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

## Working with artifact IDs (PROB-060)

Двухслойная identity: **slug** (`prd-auth-system`) — каноничный, immutable,
пишется в `forgeplan new`. **Display number** (`PRD-074`) — выставляется
CI-ботом на merge в `dev`. До merge артефакт виден как `PRD-74?`
(маркер `?` = «номер предсказан, не финален»). Frontmatter: `slug`,
`predicted_number`, `assigned_number` (последний — `null` до merge).

**Три правила для коммитов и refs:**

1. **До merge — только slug в `Refs:`**. Predicted/displayed номер не
   попадает в commit messages — он ещё не финален.
   ```
   ✅ Refs: prd-auth-system, FR-001..003
   ❌ Refs: PRD-74?, FR-001..003
   ❌ Refs: PRD-074, FR-001..003   # broken pointer — номер не assigned
   ```
2. **После merge — работают оба формата**: `Refs: PRD-074` или
   `Refs: prd-auth-system`. Резолвер маппит в один артефакт.
3. **`assigned_number` — write-once, выставляется только CI-ботом**
   (workflow `.github/workflows/assign-id.yml`, concurrency-serialized).
   Ручная правка `assigned_number` в frontmatter — нарушение контракта
   (mirrors §RED LINES #7/#8: artifact integrity).

**Pre-create check**: `forgeplan new` warning'ит если slug уже существует
в `origin/dev` и предлагает alt-slug. Игнор без явной причины запрещён.

**Legacy artifacts compatibility (Phase 2.3 audit, 2026-05-08)**: артефакты,
созданные **до Phase 1.5** (PRD-001..073, ADR-001..011, RFC-001..008, etc.) **не
имеют** `slug` поля во frontmatter — и это **сознательное решение**. Такие
артефакты работают как **first-class citizens** через display id path и
**миграция не требуется** (она demoted с MUST до OPTIONAL CLEANUP в RFC-009 §4.1).

Resolver, MCP DTOs и hint emission обрабатывают missing slug graceful через
fallback к display id:
- **Resolver**: `crates/forgeplan-core/src/artifact/store.rs` — если `slug` отсутствует,
  lookup идёт по `assigned_number` через display id (`PRD-074`).
- **MCP DTOs**: `slug: Option<String>` с `skip_serializing_if = "Option::is_none"` —
  legacy артефакты возвращаются без поля `slug` в JSON, agent видит только `id` /
  `id_display`.
- **`refs_form_from_body`**: если parse slug fails, возвращается canonical id
  (display number), что валидно для post-merge legacy.
- **E2E coverage**: `crates/forgeplan-cli/tests/legacy_compat_e2e.rs` фиксирует все
  fallback paths (resolver → display id, MCP serialization без slug, refs lookup).

**Практическое следствие**: agent **не должен** добавлять slug в legacy artifact
вручную (через `forgeplan_update`) — миграция, если будет, идёт через cosmetic
script, не как часть feature workflow. Проверяй через resolver, что artifact
доступен по display id (`PRD-074`) — этого достаточно.

Подробности (FAQ, migration, multi-agent dispatch, slug regex):
[`docs/methodology/ID-ASSIGNMENT.ru.md`](docs/methodology/ID-ASSIGNMENT.ru.md).

Cross-refs: PROB-060, PRD-076, ADR-012, SPEC-005, RFC-009.

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

CI также содержит **drift detector** (`scripts/check-mcp-tool-count.sh`) — блокирует PR если число MCP-инструментов в docs расходится с кодом. Полный CI-гейт reference: [`docs/operations/QUALITY-GATES.ru.md`](docs/operations/QUALITY-GATES.ru.md).

---

## AI-agents (non-interactive hygiene)

- `forgeplan init` — **always** with `-y` (no interactive prompt)
- Config `.forgeplan/config.yaml` — tracked (env var refs only, no hardcoded secrets); shared LLM provider/model settings между членами команды
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

---

## AgentTeams orchestration patterns

Два паттерна спавна workers для сложных multi-task фаз. Выбор зависит от
координации между задачами.

### Pattern A: Team Lead + Parallel Workers (durable state)

Использовать когда:
- ≥3 задач, требующих cross-worker contracts (CLI signature shared между binary
  и workflow YAML; JSON shape consumed multiple workers)
- State preserve между worker exchanges (lead returns после wave 1, spawns wave 2)
- Конфликты файлов нужны explicit resolution (CD-N protocol, see Phase 0b)

Mechanism:
```
Agent({ subagent_type: "api-scaffolding:backend-architect",
        description: "Phase X team lead",
        prompt: <team-lead-brief> })
# Lead returns 4-6 worker briefs с CD-N decisions
# Затем main thread spawns workers одним сообщением:
Agent({ subagent_type: "systems-programming:rust-pro", prompt: <W1-brief> })
Agent({ subagent_type: "cicd-automation:deployment-engineer", prompt: <W2-brief> })
# ... и т.д.
```

Lead's responsibilities (НЕ пишет код):
1. Read context (handoff doc + ADR + RFC + relevant code)
2. Validate cross-worker contracts (binding CD-N decisions)
3. Worker briefs с file ownership grid
4. Risk register (top 5 рисков + mitigations)
5. After workers complete: integration / conflict resolution / EVID authoring

### Pattern B: Single-message parallel Agent (one-shot)

Использовать когда:
- Independent tasks без shared state
- Lead role не нужен (workers self-sufficient)
- Quick parallel close

Mechanism:
```python
# В одном assistant message:
Agent({ subagent_type: "systems-programming:rust-pro", prompt: <T1-brief> })
Agent({ subagent_type: "agents-pro:documentation-engineer", prompt: <T2-brief> })
```

### Multi-agent worktree pattern (PRD-057 follow-up)

**Lesson из Phase 0b**: shared `.git/HEAD` между параллельными agents в одном
worktree вызывает branch ref corruption (W4's `git update-ref` recovery сбила
W2's branch). Для ≥3 параллельных workers — separate worktrees:

```bash
# Pre-spawn (in main thread):
git worktree add ../forgeplan-w1 feat/prob-060-phase-2-w1
git worktree add ../forgeplan-w2 feat/prob-060-phase-2-w2
# Worker prompt: «Working dir: ../forgeplan-w1»
```

Cleanup после merge: `git worktree remove ../forgeplan-w1 && git branch -D feat/prob-060-phase-2-w1`.

### Worker brief required fields (any pattern)

Каждый worker prompt должен явно содержать:

1. **Working directory** + branch instructions (off which base, naming convention)
2. **OWNED FILES** list (exact paths, mark `(new)` для new files)
3. **FORBIDDEN FILES** list (exact paths + which worker owns each)
4. **CONTRACT spec** (CLI signature / YAML structure / JSON schema / markdown shape)
5. **🔴 RED-LINE #11 reminder** (artifact mutations через MCP/CLI ONLY)
6. **Pipeline gate** command list (cargo fmt + check + test + clippy)
7. **Acceptance criteria** (testable bullets)
8. **Anti-patterns** (что НЕ делать с конкретными warnings)
9. **Final report format** (что worker возвращает)

### Adversarial audit mandate

После significant changes (≥5 commits OR new public API) — 2-agent audit:
- `agents-pro:security-expert` — CWE coverage, injection vectors
- `code-documentation:code-reviewer` или `agents-core:reviewer` — code quality

Each MUST find ≥3 issues. Zero findings → re-spawn (suspect superficial review).

---

## Sub-agents и forgeplan MCP/CLI tools

**Red-line #11 enforcement в worker prompts**: artifact mutations ИСКЛЮЧИТЕЛЬНО
через MCP tools или CLI — никогда `Edit`/`Write`/`sed` напрямую на
`.forgeplan/{prds,adrs,specs,rfcs,evidence,notes}/*.md`.

### Канонические operations

| Operation | MCP tool | CLI equivalent |
|---|---|---|
| Create artifact | `mcp__forgeplan__forgeplan_new(kind, title)` | `forgeplan new <kind> "Title"` |
| Read | `mcp__forgeplan__forgeplan_get(id)` | `forgeplan get <id>` |
| Update body/metadata | `mcp__forgeplan__forgeplan_update(id, body=...)` | `forgeplan update <id> --body @path` |
| Add typed link | `mcp__forgeplan__forgeplan_link(source, target, relation)` | `forgeplan link <src> <tgt> --relation <r>` |
| Activate | `mcp__forgeplan__forgeplan_activate(id)` | `forgeplan activate <id>` |
| Validate | `mcp__forgeplan__forgeplan_validate(id)` | `forgeplan validate <id>` |
| Score (R_eff) | `mcp__forgeplan__forgeplan_score(id)` | `forgeplan score <id>` |
| ADI reasoning | `mcp__forgeplan__forgeplan_reason(id)` | `forgeplan reason <id>` |
| Health | `mcp__forgeplan__forgeplan_health()` | `forgeplan health` |

### What sub-agents MUST use MCP/CLI for

- Creating EvidencePack (`forgeplan_new evidence` + `forgeplan_update body=...` + `forgeplan_link`)
- Updating PRD/RFC/ADR/Spec progress trackers (`forgeplan_update id=... body=<full new body>`)
- Activating artifact (`forgeplan_activate id=...`)
- Linking artifacts (`forgeplan_link source=... target=... relation=...`)
- Adding/updating Notes, Problems, Solutions, Refresh

### What sub-agents MAY edit directly (NOT in red-line #11 scope)

- `CLAUDE.md` (project-wide instruction file, not artifact)
- `docs/**` (documentation, not artifact)
- `crates/**` (Rust code)
- `.github/workflows/*.yml`, `scripts/*.sh`, `templates/**`
- `README.md`, `CHANGELOG.md`, `KNOWN-ISSUES.md`, `.changeset/*.md`
- Test fixtures NOT under `.forgeplan/`

### Recovery if accidentally Edit'нул artifact

1. Re-Read file to capture current content
2. Strip YAML frontmatter (forgeplan_update body excludes it)
3. `mcp__forgeplan__forgeplan_update(id="<ID>", body=<full body>)`
4. Last-resort fallback: `forgeplan scan-import` rebuilds LanceDB from markdown

Document the violation в commit message so reviewer recognizes the pattern was caught.

### Hint protocol reading после fpl calls

После каждого `forgeplan_*` MCP/CLI call agent reads:
- `Next: <command>` — primary action (run as-is)
- `Or: <command>` — alternative
- `Wait: <condition>` — async retry
- `Done.` — workflow complete (terminal)
- `Fix: <command>` — error remediation

JSON: `_next_action` field. Following hints = staying на methodology path
(Shape → Validate → Code → Evidence → Activate).

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

Automated via `scripts/smoke-test.sh` in CI pipeline (see `smoke-e2e` job in `.github/workflows/ci.yml`).

Manual testing (if CI not available):
```bash
cargo fmt && cargo fmt --check && cargo check && cargo test  # 0 diffs, 0 warnings, all PASS
bash scripts/smoke-test.sh --verbose                          # 13 operations, 8 artifact kinds
```
Covers: init, new (8 kinds), validate, score, list, search, blocked, order, health, link, graph, fpf.

Any fail → do not commit, fix.

---


### Standard flow для фичи (Standard+)

```bash
forgeplan health                                         # observe
forgeplan route "implement ad-account dashboard tile"
forgeplan new prd "Ad-account dashboard tile"            # shape
$EDITOR .forgeplan/prds/PRD-NNN-*.md                     # заполнить MUST sections
forgeplan validate PRD-NNN                               # 0 MUST errors
forgeplan reason PRD-NNN                                 # ADI (Standard+)
# write code + tests (через subagent / orchestrator)
forgeplan new evidence "PRD-NNN: vitest 14 pass, p95 180ms на staging"
$EDITOR .forgeplan/evidence/EVID-MMM-*.md                # ## Structured Fields!
forgeplan link EVID-MMM PRD-NNN --relation informs
forgeplan score PRD-NNN                                  # R_eff > 0?
forgeplan activate PRD-NNN                               # draft → active
# gh pr create --base develop  (PR body: "Refs: PRD-NNN")
```

### Multi-agent (`dispatch → claim → spawn → release`)

```bash
forgeplan dispatch --agents 3 --json    # планер conflict-free buckets (НЕ спавнер!)
forgeplan claim PRD-NNN --agent <subagent-name> --ttl-minutes 60
# … работа …
forgeplan release PRD-NNN
```

`dispatch` возвращает план, **спавнит main thread / orchestrator** через `Agent({subagent_type, prompt})` (несколько `Agent`-блоков в одном сообщении = параллель). `SendMessage` — НЕ спавнер; адресует только уже запущенные процессы.

### Команды-однострочники (на каждый день)

```bash
forgeplan health              # session-start sanity check
forgeplan list                # все артефакты
forgeplan graph               # mermaid-граф связей
forgeplan stale               # артефакты с истёкшим valid_until
forgeplan blindspots          # решения без evidence
forgeplan claims              # кто что захватил
```


## Storage (ADR-003): Markdown primary, LanceDB derived

```
.forgeplan/
├── adrs/ rfcs/ prds/ epics/ specs/   ← tracked, source of truth
├── evidence/ problems/ solutions/
├── notes/ refresh/ memory/
├── config.yaml         ← tracked (env var refs only, no hardcoded secrets)
├── lance/              ← ⚠️ gitignored (derived index — forgeplan scan-import)
├── .fastembed_cache/   ← ⚠️ gitignored
└── session.yaml        ← ⚠️ gitignored (per-machine runtime state)
```

**Fresh clone**: `git clone → forgeplan init -y → forgeplan scan-import → forgeplan list`.

**Rules**: edit via `forgeplan` CLI; direct markdown edits require `forgeplan scan-import`; DO NOT commit `lance/` or `session.yaml`.

---

## Rust Architecture

```
crates/
├── forgeplan-core/    ← shared library (12.8K LOC, 194 tests)
│   ├── artifact/ config/ db/ depth/ embed/ fpf/ graph/ health/
│   ├── journal/ lifecycle/ link/ llm/ progress/ projection/
│   ├── routing/ scoring/ search/ stale/ template/ validation/ workspace/
├── forgeplan-cli/     ← clap derive, 76 commands
└── forgeplan-mcp/     ← rmcp stdio, 72 tools
```

**Project structure**: `docs/README.md` — map of all documentation. Reference repositories in `sources/` (read-only).

---

## Non-Goals

- NOT project management (not Jira/Linear)
- NOT CI/CD, NOT SaaS, NOT a code generator
- Local-first, single binary, git for sync
