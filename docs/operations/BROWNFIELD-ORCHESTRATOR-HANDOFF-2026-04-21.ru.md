# Forgeplan — Brownfield × Orchestrator Pivot — Handoff Guide (2026-04-21)

> **Цель этого документа** — дать следующему агенту (или тебе самому в новом чате)
> полный контекст длинной сессии 2026-04-19…04-21, в которой мы:
> (1) закрыли scan-import brownfield bugs, (2) провели adversarial audit shape-артефактов,
> (3) сделали стратегический архитектурный пивот к orchestrator-модели, (4) пересобрали
> Epic-структуру, (5) закрыли почти все CRITICAL-находки аудита, (6) выполнили
> ключевой CL3-spike на реальных данных.
>
> Документ self-contained: читая только его можно продолжить работу без доступа к
> истории чата. Он ссылается на conкретные файлы, commit'ы, PR-номера и команды.

---

## 0. TL;DR — что произошло и что делать дальше

### Что произошло (2 дня, ~15 часов сессии):

**Стартовая точка** — ветка `fix/prob-scan-import-bugs` с hotfix'ами для 3 багов
`forgeplan scan-import` после PRD-058 (`projection/body`, `status mapping`, audit follow-ups).

**Ключевой инсайт сессии** — во время shape-фазы EPIC-006 (brownfield migration pipeline)
провели 4-agent adversarial audit: `architect-reviewer`, `ddd-domain-expert`,
`code-analyzer`, `production-validator`. Audit вернул **6 CRITICAL + 12 HIGH + 15 MEDIUM + 8 LOW**
findings. Три из критикал указали на то, что мы строим **не то что нужно**:
forgeplan пытается сам реализовать extraction logic, парсеры, classification — а уже
существуют marketplace plugin'ы (`c4-architecture`, `autoresearch`, `ddd-expert`,
`feature-dev`), которые это делают лучше.

**Architectural pivot** — через `/fpf decompose` + `/fpf evaluate` пришли к модели:
**forgeplan = orchestrator, не implementer**. Четыре примитива:
- **Playbook** (YAML-рецепт «что вызвать в каком порядке»)
- **Skill** (promptable unit, исполняется агентом в harness)
- **Agent** (autonomous subagent, может вызывать skills и инструменты)
- **Mapping** (output-format → forge-kind rules, идемпотентный ingest)

Плюс **Pack marketplace** — унифицированная дистрибуция: `marketplace/<pack>/` со
structure `playbooks/ + skills/ + agents/ + mappings/ + fixtures/`.

**Spike-1 (CL3 measurement)** — запустили `c4-architecture:c4-context` агента на
реальном Forgeplan repo → получили 336-строчный `docs/architecture/c4-context.md`
(System Overview, 7 Personas, 11 System Features, 3 User Journeys, 8 External
Systems, Mermaid diagram). Затем написали первый `c4-to-forge.yaml` mapping file
(137 LOC) и показали что из этого output'а **деривируемо ~20+ forge artifacts**
(1 Epic, 4 PRDs, N Notes, + evidence linking) без ручной работы. Это и есть
эмпирическое подтверждение orchestrator-модели.

**Epic реорганизация**:
- **EPIC-006** (Brownfield) — scope **сужен** до consumer'а marketplace pack
  (`brownfield-docs-pack/`), ~60% effort выпущен в EPIC-007.
- **EPIC-007** (Playbook Runtime + Pack Marketplace) — **новый**, 5 child PRDs:
  PRD-065 (playbook runtime), PRD-066 (ingest engine), PRD-067 (plugin detection),
  PRD-068 (forge-history-miner skill), PRD-069 (orchestrator agents).

**7 PR'ов смерджено** за сессию (+ 1 закрыт как superseded):

| PR   | Title                                                     | Status       |
|------|-----------------------------------------------------------|--------------|
| 199  | [PRD-058] scan-import brownfield migration — 3 bugs       | ✅ MERGED   |
| 200  | [EPIC-006] Shape iter 1 + audit (original)                | ❌ CLOSED (superseded by #204) |
| 201  | [PROB-041] CLI loads .forgeplan/.env via workspace walk-up | ✅ MERGED   |
| 202  | [EPIC-007] Playbook Runtime + Pack Marketplace Shape      | ✅ MERGED   |
| 203  | [PROB-043] activity log: explicit flush (CI flaky fix)    | ✅ MERGED   |
| 204  | [EPIC-006] Narrow scope — consumer of EPIC-007             | ✅ MERGED   |
| 205  | [PROB-040 C1] Rename Status::RefreshDue → Status::Stale   | ✅ MERGED   |
| 206  | [PROB-040 C4] ADR-008 depth=deep alignment + restore file | ✅ MERGED   |

### Что делать дальше (master sequence):

**ЭТАП 1 — Finalize Shape (3-5 дней)**
1. **PROB-040 C2** (in progress) — commit & PR `fix/prob-040-c2-skill-derived-policy`
   (branch уже с amendment, ADR-008 updated, needs push + PR)
2. **PROB-040 C5, C6** — MigrationPlan aggregate ownership doc, ACL module spec в PRD-066/PRD-067
3. **H12** — commit 44-file Obsidian fixture в `tests/fixtures/obsidian-vault-44/`
4. **Spike-2** (autoresearch:learn) — run на Forgeplan repo → mapping
5. **Spike-3** (ddd-expert) — identify bounded contexts → Epic decomposition
6. **EVID-080** upgrade на CL3 cross-harness install (PROB-040 H5)

**ЭТАП 2 — Build Runtime (5-8 дней) — код на dev**
7. PRD-065 — `forgeplan-playbook` crate + executor
8. PRD-066 — `forgeplan-ingest` crate + mapping engine
9. PRD-067 — plugin detection (scan `.claude/plugins/` etc.) + self-describing hints
10. PRD-068 — forge-history-miner skill (git log → ADR drafts)
11. PRD-069 — orchestrator agents (forge-ingest, forge-scaffolder)

**ЭТАП 3 — 3 Canonical Packs (3-5 дней)**
12. `marketplace/brownfield-docs-pack/` — MADR/ADR-tools/log4brains/Obsidian → forge
13. `marketplace/brownfield-code-pack/` — C4 + autoresearch + DDD → forge (derived из Spike-1 mapping)
14. `marketplace/greenfield-shape-pack/` — scaffolding для new projects

**ЭТАП 4 — Validation (2-3 дня)**
15. E2E на 44-файловом Obsidian vault через brownfield-docs-pack
16. E2E на Forgeplan repo через brownfield-code-pack (dogfood)
17. Docs — `docs/operations/BROWNFIELD-MIGRATION.ru.md`, `docs/operations/PACK-AUTHORING.ru.md`

**ЭТАП 5 — Release (1 день)**
18. CHANGELOG, release PR, tag `v0.25.0`

---

## 1. Project Overview — что такое Forgeplan

**Forgeplan** — Rust CLI + MCP server для ведения инженерного проекта от идеи до
реализации через структурированные артефакты. Формула:

> `Quint-code (R_eff, evidence) + BMAD (13-step PRD) + OpenSpec (DAG, delta-specs) + FPF (ADI, trust calculus) + git-adr (clap CLI) + LanceDB + Tauri`

### Основные сущности

| Тип         | Prefix   | Назначение                                                                 |
|-------------|----------|---------------------------------------------------------------------------|
| Epic        | `epic-`  | Группирует PRD[] / RFC[] / ADR[]                                           |
| PRD         | `prd-`   | Product Requirements Document (Problem, Goals, FRs, NFRs, AC)             |
| Spec        | `spec-`  | API contracts, data models                                                 |
| RFC         | `rfc-`   | Architectural proposal with phases                                         |
| ADR         | `adr-`   | Architecture Decision Record (deep+: DDR fields)                           |
| Note        | `note-`  | Micro-decision (auto-expires 90 days)                                      |
| Problem     | `prob-`  | Problem card with context                                                   |
| Solution    | `sol-`   | 2-3 options with weakest-link scoring                                      |
| Evidence    | `evid-`  | Tests, benchmarks, measurements                                             |
| Refresh     | `ref-`   | Re-evaluation of stale decisions                                           |

### Pipeline

```
Route ──► Shape ──► Validate ──► Reason (ADI) ──► Code ──► Test ──► Audit ──► Evidence ──► Activate ──► PR
```

### R_eff (scoring)

- `R_eff = min(evidence_scores)` — trust = weakest link, **никогда не average**
- CL0=0.9 penalty, CL1=0.4, CL2=0.1, CL3=0.0 (best)
- Derived: `UNDERFRAMED → FRAMED → EXPLORING → COMPARED → DECIDED → APPLIED`

### Storage model (ADR-003)

```
.forgeplan/
├── adrs/ rfcs/ prds/ epics/ specs/   ← tracked, SOURCE OF TRUTH (markdown)
├── evidence/ problems/ solutions/
├── notes/ refresh/ memory/
├── lance/              ← .gitignore (derived index)
├── .fastembed_cache/   ← .gitignore
└── config.yaml         ← .gitignore (LLM keys)
```

### Current version

- **v0.24.0** released (multi-agent dispatcher, PRD-057 EVID-077, R2+R3 audits закрыли 30 findings)
- 1405 tests passing, 0 clippy warnings on Rust 1.95
- ~58 CLI commands, ~47 MCP tools <!-- mcp-count-drift: ignore (historical v0.24.0 handoff snapshot 2026-04-21) -->

### Crates

```
crates/
├── forgeplan-core/    ← shared library (12.8K LOC, 194 tests)
│   ├── artifact/ config/ db/ depth/ embed/ fpf/ graph/ health/
│   ├── journal/ lifecycle/ link/ llm/ progress/ projection/
│   ├── routing/ scoring/ search/ stale/ template/ validation/ workspace/
│   └── activity/ (new v0.24.0)
├── forgeplan-cli/     ← clap derive, ~58 commands
└── forgeplan-mcp/     ← rmcp stdio, ~47 tools  <!-- mcp-count-drift: ignore (handoff frozen 2026-04-21) -->
```

---

## 2. Session Timeline — chronological summary

### Phase 0 — Scan-import hotfix (before the deep session)

Branch `fix/prob-scan-import-bugs` содержал 3 hotfix'а для PRD-058:
- Body projection bug — `forgeplan update --body @file` создавал duplicate projection файлы из-за stale slugs
- Status mapping — `accepted/rejected/proposed` не маппились в forge lifecycle
- Audit follow-ups — 8 hotfixes от первого audit iteration

Commits сохранены в истории `main` через merge #199.

### Phase 1 — Deep Shape (2026-04-18..19)

Пользователь попросил «придумать всё чтобы brownfield работал качественно». В `/fpf:fpf`
обсудили что нужно:
1. Извлечение документов из старых проектов (ADR/PRD/KB/postmortems в Obsidian/MADR/ad-hoc)
2. Classification и перенос в forge с сохранением data (status, dates, wikilinks)
3. Cross-harness distribution (Cursor, Windsurf, Cline, Roo, Copilot, generic)
4. Self-describing output (forgeplan учит агента что делать дальше)
5. Brownfield-aware `forgeplan init`

Это превратилось в **ADR-008** («Self-Describing Tools + agent-skills standard + Brownfield-Aware Init») + **EPIC-006** («Brownfield Migration Pipeline + Self-description Platform») с 6 child PRDs (оригинально PRD-059..064):

| PRD   | Scope                                                                              |
|-------|------------------------------------------------------------------------------------|
| PRD-059 | `forgeplan discover` + `migrate --plan --dry-run --apply --resolve-links`       |
| PRD-060 | Self-description: stderr hints, `agent-manifest` command, MCP context injection |
| PRD-061 | Marketplace `brownfield-pack` — SKILL.md, forge-classify, forge-dialogue        |
| PRD-062 | `forgeplan init --from-brownfield` + new crate `forgeplan-skill-installer`       |
| PRD-063 | State machine: `completed`/`archived` states, bidirectional supersede/deprecate |
| PRD-064 | New kinds: `kb`/`runbook`/`postmortem`/`retrospective`/`meeting` + new links    |

Всего артефактов shape: ADR-008 (deep) + EPIC-006 (deep) + 6 PRDs + EVID-079 (CL2 research).
Branch `feat/prd-059-brownfield-pipeline`.

### Phase 2 — 4-agent adversarial audit (2026-04-19)

Вместо создания PR сразу запустили audit. 4 агента параллельно:
- `agents-pro:architect-reviewer` (architecture integrity)
- `agents-pro:ddd-domain-expert` (bounded contexts, aggregates)
- `agents-pro:code-analyzer` (quality across 5 domains)
- `agents-core:production-validator` (methodology violations, stubs)

Returned **6 CRITICAL + 12 HIGH + 15 MEDIUM + 8 LOW** findings. Production-validator вернул
0 Red Line violations (shape methodology clean), но три других agents — серьёзные content
issues.

**Phase A** — tractable findings (34 fixes через batch edit):
- Dead refs PRD-A..F → PRD-059..064 в ADR-008 + EPIC-006 + всех PRDs
- «Epic (pending)» / «EPIC-XXX» → EPIC-006
- Typos: zazmечены→замечены, impotrted→imported, nego→него
- Frontmatter drift: added created/updated к PRD-060/062/064
- Double-supersede PROB-022: removed EPIC-006 link
- EVID-079 linked к PRD-059..064 (все 6) — up from 2 links to 8

**Phase B** — medium fixes:
- PRD-059 `predicted_kind` → `kind_hint` + `hint_source` (clarity: core = deterministic, skills = probabilistic)
- scan-import removal softened v0.27 → v1.0 LTS
- meeting expiry config-driven (90/180/365 via `.forgeplan/config.yaml`)
- Orphan FRs mapped to AC
- Metric budgets: EPIC-006 Success Criteria все с numeric targets

**Phase C** — captured untractable в **PROB-040** (консолидированный backlog 6 CRITICAL + 12 HIGH + 15 MEDIUM + 8 LOW).

Final commit на feat branch: `e3f0382`. Но PR #200 в итоге был **closed** (superseded by #204 — scope narrowing).

### Phase 3 — «hold on, мы не то строим» (2026-04-19 вечер)

User: «скажи что еще нужно делать чтобы качественно заработал brownfield?»

При повторном анализе PROB-040 C1..C6 плюс user'ом поднятая тема что «есть marketplace
плагины — c4-architecture, autoresearch, ddd-expert, feature-dev», зафиксировали problem:

**PROB-042** — «Plugin orchestration absent — forgeplan реализует что уже есть в
c4-architecture, autoresearch, ddd-expert плагинах».

Через `/fpf decompose` + `/fpf evaluate` получили 4 варианта:
- **A**: написать всё сами (~8 недель, reinvent)
- **B**: full plugin delegation (выпускаем forge runtime полностью) — слишком агрессивно
- **C**: orchestrator model (runtime + marketplace packs) — **выбран**
- **D**: hybrid (core extraction + plugin augmentation) — неявные границы

Решение зафиксировано в **ADR-009** («Forgeplan as Orchestrator — Playbook, Skill, Agent,
Mapping + Pack Marketplace Model»), depth=deep, R_eff=0.77 (B).

Одновременно создан **EPIC-007** («Playbook Runtime + Pack Marketplace») с 5 child PRDs:

| PRD   | Scope                                                              |
|-------|--------------------------------------------------------------------|
| PRD-065 | `forgeplan-playbook` crate: YAML schema + runtime executor        |
| PRD-066 | `forgeplan-ingest` crate: mapping YAML format + idempotent engine |
| PRD-067 | Plugin detection + self-describing hints + playbook recommendations |
| PRD-068 | forge-history-miner skill (git log → inferred ADR drafts)         |
| PRD-069 | forge-orchestrator agent + forge-ingest skill + forge-scaffolder  |

### Phase 4 — Spike-1 CL3 measurement (2026-04-19 ночь)

Чтобы поднять EVID-079 с CL2 research до CL3 measurement, запустили
`c4-architecture:c4-context` агент на реальном Forgeplan repo.

**Output**: `docs/architecture/c4-context.md` (336 lines) содержит:
- System Overview (short + long + scope)
- 7 Personas (Solo Maintainer, Senior IC, Engineering Manager, Tech Lead, Team Lead, Contributor, Auditor)
- 11 System Features (artifact CRUD, R_eff scoring, graph, validation, lifecycle, search, FPF engine, multi-agent, evidence, lifecycle, brownfield)
- 3 User Journeys (new feature, onboarding, audit)
- 8 External Systems (git, LanceDB, BGE-M3, LLM providers, agent harnesses, MCP clients, GitHub, file system)
- Mermaid C4Context diagram

**Mapping demonstration**: написали `marketplace/brownfield-code-pack/mappings/c4-to-forge.yaml`
(137 lines) с 5 правилами: `context_to_epic`, `container_to_prd`, `component_to_prd`,
`code_to_note`, `feature_to_prd` + universal rules для идемпотентности (dedup by title
similarity, version bump on re-run, no destructive writes).

Показали что из этого c4-context output'а **деривируемы ~20+ forge artifacts**:
- 1 Epic (system as a whole)
- 4 PRDs (MCP gateway, artifact store, search pipeline, scoring engine)
- N Notes (per feature + per persona decisions)
- Evidence linking (EVID per journey validation)

**Это CL3 measurement** — реальный output на реальном коде, не hypothetical.

EVID-079 upgraded. Создан EVID-081 («ADR-009 orchestrator pivot research — CL3 measurement
from c4-architecture agent on Forgeplan repo»).

### Phase 5 — PROB-041 dotenv bug (2026-04-20 утро)

User: «я добавил модель — давай проверим как то».

Добавил `NEURALDEEP_API_KEY` в `.forgeplan/.env`. CLI не нашёл ключ. Исследование:
`dotenvy::dotenv()` читает только `.env` в cwd, не делает workspace walk-up как другие
части кода.

**PROB-041** создан, fix в `crates/forgeplan-cli/src/main.rs`:

```rust
fn load_workspace_env() {
    if let Ok(cwd) = std::env::current_dir()
        && let Some(ws) = forgeplan_core::workspace::find_workspace(&cwd)
    {
        dotenvy::from_path(ws.join(".env")).ok();
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    load_workspace_env();
    dotenvy::dotenv().ok();
    // ...
}
```

Branch `fix/prob-041-dotenv-workspace-discovery`, PR #201 merged. EVID-080 (CL3 measurement)
создан с 3 E2E scenarios PASS.

### Phase 6 — PROB-043 activity log flush (2026-04-20 вечер)

Когда пытались смерджить PR #202 (EPIC-007 shape), CI test failed:
`activity::tests::append_creates_file_and_directory` panicked on GitHub Actions (Linux
overlayfs) с `left: 0, right: 1` — assertion что файл содержит 1 newline.

Root cause: `tokio::fs::File::drop` НЕ делает async flush. Writes buffered, OS ещё не
flushed к моменту test чтения. Locally на macOS APFS PASS.

**PROB-043** создан, fix — добавил `file.flush().await?` перед return в `crates/forgeplan-core/src/activity/mod.rs`:

```rust
file.write_all(line.as_bytes()).await?;
// Explicit flush to buffer boundary. tokio::fs::File::drop does
// NOT perform an async flush; without this, CI filesystems (Linux
// overlayfs in GitHub Actions) intermittently show an empty file
// when a test reads right after `append` returns. We do NOT fsync
// to disk — would dominate latency. The OS still buffers; worst
// case on SIGKILL we lose <1 sec of entries. For durability-
// critical deployments, a future `activity.fsync: per_entry` flag
// can opt in.
file.flush().await?;
```

Branch `fix/prob-043-activity-log-flush`, PR #203 merged.

### Phase 7 — EPIC-006 scope narrowing (2026-04-20 ночь)

После EPIC-007 pivot EPIC-006 overlap с EPIC-007 scope стал очевиден:
- PRD-059 (discover + migrate core) duplicate PRD-065 + PRD-066
- PRD-060 (self-description) duplicate PRD-067
- PRD-062 (init + skill installer) duplicate PRD-067 + PRD-069

Narrowing decision — оставить EPIC-006 только как consumer'а EPIC-007 runtime:
- **Superseded by EPIC-007**: PRD-059, PRD-060, PRD-062 (removed files на dev)
- **Retained**: PRD-061 — становится `marketplace/brownfield-docs-pack/` (playbook + madr-to-forge mapping + forge-classify skill)
- **Retained**: PRD-063 (state machine, orthogonal), PRD-064 (new kinds + links, orthogonal)

Branch `feat/epic-006-narrow-to-consumer-of-epic-007`, PR #204 merged. Original PR #200 closed.

### Phase 8 — PROB-040 CRITICAL closures (2026-04-20 ночь → 2026-04-21 утро)

- **C1** — `Status::RefreshDue → Stale` rename + doc comment обоснование. PR #205 merged.
- **C4** — ADR-008 body claim «depth=critical, pipeline PRD→Spec→RFC→ADR» был неverif. Fix: downgrade body до «deep» + justification («contracts embedded в PRD-066/067/069 bodies»). PR #206 merged. Также восстановил файл ADR-008 на dev (был удалён при closing PR #200).
- **C2** — ADR-008 amendment: derived-skill-file policy (canonical → installed, hash-tracking, `skill eject` command). **В процессе**: branch `fix/prob-040-c2-skill-derived-policy`, commit `03b7633`, pending push + PR.

### Phase 9 — текущее состояние (2026-04-21 утро)

Branch: `fix/prob-040-c2-skill-derived-policy` (содержит C2 amendment, не запушен).

Что осталось по PROB-040:
- **C2** — commit `03b7633` сделан, нужен push + PR + merge
- **C3** — journaled replay для atomic supersede/deprecate в PRD-063 → **defer** (PRD-063 не на dev, Phase 3 scope)
- **C5** — MigrationPlan aggregate ownership doc → defer к PRD-066 code phase
- **C6** — BrownfieldStatusTranslator ACL module spec → defer к PRD-066 code phase
- **H5** — spike EVID-080 cross-harness install CL3 (есть CL3 для dotenv fix, но не для skill-install)

---

## 3. Артефакты — полная карта shape-phase

### ADRs

**ADR-008** — `Self-Describing Tools + agent-skills standard + Brownfield-Aware Init`
- **Status**: active
- **Depth**: deep (frontmatter authoritative; body claim corrected по PROB-040 C4)
- **File**: `.forgeplan/adrs/ADR-008-self-describing-tools-agent-skills-standard-brownfield-aware-init.md`
- **Decision**: 3 orthogonal решения в одном ADR
  1. Self-describing output (stderr hints + MCP context injection) — без ломки тishe UX
  2. agent-skills standard adoption (SKILL.md / AGENT.md via emerging spec)
  3. Brownfield-aware init (`forgeplan init --from-brownfield` detection wizard)
- **Amendments**:
  - PROB-040 C4 (2026-04-21): depth clarification
  - PROB-040 C2 (2026-04-21): Derived-skill-file policy section (pending push)

**ADR-009** — `Forgeplan as Orchestrator — Playbook, Skill, Agent, Mapping + Pack Marketplace`
- **Status**: active
- **Depth**: deep
- **File**: `.forgeplan/adrs/ADR-009-forgeplan-as-orchestrator-playbook-skill-agent-mapping-pack-marketplace-model.md`
- **Decision**: forgeplan НЕ пишет extraction/classification code сам. Вместо этого:
  - **Playbook** (YAML) — рецепт что вызвать (skill, agent, built-in command) в каком порядке
  - **Skill** (SKILL.md prompt + body) — promptable unit, исполняется агентом в harness
  - **Agent** (AGENT.md) — autonomous subagent который может вызывать skills + tools
  - **Mapping** (YAML) — правила output-format → forge-kind idempotent translation
- **Marketplace Pack** structure:
  ```
  marketplace/<pack-name>/
  ├── pack.yaml              ← manifest (name, version, compatibility, dependencies)
  ├── playbooks/*.yaml       ← recipes
  ├── skills/*/SKILL.md      ← promptable units
  ├── agents/*/AGENT.md      ← autonomous agents
  ├── mappings/*.yaml        ← format → forge rules
  ├── fixtures/              ← E2E test data
  └── README.md
  ```
- **Confidence**: High (ADI cycle — 3 hypotheses, 2 weakened, 1 supported by Spike-1 CL3)

### Epics

**EPIC-006** — `Brownfield Migration Pipeline + Self-description Platform` (**narrowed**)
- **Status**: active
- **Scope AFTER narrowing** (commit 091b222):
  - Consumer of EPIC-007 runtime
  - Retains: PRD-061 (now `marketplace/brownfield-docs-pack/`), PRD-063 (state machine), PRD-064 (new kinds + links)
  - Supersedes: PRD-059, PRD-060, PRD-062 (moved to EPIC-007)
  - ~60% effort released

**EPIC-007** — `Playbook Runtime + Pack Marketplace` (**new orchestrator foundation**)
- **Status**: active
- **Depth**: deep
- **File**: `.forgeplan/epics/EPIC-007-playbook-runtime-pack-marketplace.md`
- **Children**:
  - PRD-065 — `forgeplan-playbook` crate (YAML schema + runtime executor)
  - PRD-066 — `forgeplan-ingest` crate (mapping YAML + idempotent engine)
  - PRD-067 — Plugin detection + self-describing hints + playbook recommendations
  - PRD-068 — forge-history-miner skill (git log → inferred ADR drafts)
  - PRD-069 — forge-orchestrator agent + forge-ingest skill + forge-scaffolder

### PRDs (живые)

| ID      | Status       | Scope                                                       | Epic   |
|---------|--------------|-------------------------------------------------------------|--------|
| PRD-058 | active       | scan-import brownfield migration (closed, merged в v0.24.x) | —      |
| PRD-061 | draft (shape)| brownfield-docs-pack (будет marketplace path)               | EPIC-006 |
| PRD-063 | draft (shape)| state machine completed/archived + bidirectional supersede | EPIC-006 |
| PRD-064 | draft (shape)| new kinds kb/runbook/postmortem/retrospective/meeting      | EPIC-006 |
| PRD-065 | draft (shape)| playbook runtime                                            | EPIC-007 |
| PRD-066 | draft (shape)| ingest engine                                                | EPIC-007 |
| PRD-067 | draft (shape)| plugin detection + hints                                     | EPIC-007 |
| PRD-068 | draft (shape)| forge-history-miner skill                                    | EPIC-007 |
| PRD-069 | draft (shape)| orchestrator agents                                           | EPIC-007 |

PRDs 059, 060, 062 — superseded by EPIC-007 PRDs (files removed от dev через PR #204).

### Problems

| ID       | Status    | Summary                                                                  |
|----------|-----------|--------------------------------------------------------------------------|
| PROB-040 | draft      | Shape audit findings backlog (6 CRITICAL + 12 HIGH + 15 MEDIUM + 8 LOW) |
| PROB-041 | closed     | CLI dotenvy не ходит до .forgeplan/.env через workspace walk-up         |
| PROB-042 | active     | Plugin orchestration absent — forgeplan реализует что уже есть в плагинах |
| PROB-043 | active (AC partial) | Activity log flush missing — CI flaky append test                |

**PROB-040** — backlog, НЕ в dev-ветке (был только на feat/prd-059-brownfield-pipeline,
удалён при narrowing). Full content в commit `33d1bd1` git history.

**PROB-042** → addressed by ADR-009. Может быть закрыт.

**PROB-043** → fixed in PR #203. AC-5 (CI rerun check) может ждать следующего CI run.

### Evidence

| ID      | Status | Type                   | Subject                                                                  |
|---------|--------|------------------------|--------------------------------------------------------------------------|
| EVID-079 | active | research               | sources research validation (ccpm/OpenSpec/adr-tools/BMAD) — CL2→CL3    |
| EVID-080 | active | measurement            | PROB-041 fix verified — CLI loads .forgeplan/.env via workspace walk-up  |
| EVID-081 | active | research + measurement | ADR-009 orchestrator pivot — c4-architecture agent output на Forgeplan repo (CL3) |

---

## 4. PROB-040 Findings — детальный breakdown

### CRITICAL (6) — blockers для activate + Code-phase

| ID | Status      | Summary                                                                 | Fix                            |
|----|-------------|-------------------------------------------------------------------------|--------------------------------|
| C1 | ✅ CLOSED (PR #205) | `Status::RefreshDue` → `Status::Stale` reality drift              | Rename variant + doc comment   |
| C2 | 🟡 in progress      | Skill files вне `.forgeplan/` violate ADR-003 spirit             | ADR-008 amendment (committed на fix/prob-040-c2-skill-derived-policy, pending push) |
| C3 | ⏸ DEFERRED           | «Atomic» bidirectional supersede без механизма (journaled replay) | Rewrite PRD-063 при Code phase |
| C4 | ✅ CLOSED (PR #206) | ADR-008 depth=critical body claim без Spec/RFC                   | Body alignment with frontmatter (deep) |
| C5 | ⏸ DEFERRED           | MigrationPlan aggregate ownership undefined                        | Fix в PRD-066 при Code phase   |
| C6 | ⏸ DEFERRED           | status_map as leaky translator, not proper ACL                    | BrownfieldStatusTranslator module в PRD-066 |

### HIGH (12) — before activate PRD

| ID | Status | Summary                                                                  |
|----|--------|--------------------------------------------------------------------------|
| H1 | OPEN   | Classification context homeless (PRD-059 heuristic vs PRD-061 LLM)       |
| H2 | OPEN   | PRD-062 conflates Discovery + Skill Distribution                         |
| H3 | OPEN   | Dialogue context in-name-only (no aggregate)                             |
| H4 | OPEN   | «skill» terminology overloaded (3 меanings)                              |
| H5 | PARTIAL| EVID-079 CL2 too weak (upgraded до CL3 через Spike-1, но не cross-harness install) |
| H6 | OPEN   | Context map absent (integration patterns не названы)                      |
| H7 | OPEN   | Per-kind invariants under-specified (postmortem MUST caused_by)          |
| H8 | OPEN   | Domain events implicit (migrate applied, skill installed)                |
| H9 | OPEN   | Completed/Archived orthogonal axes conflated                              |
| H10| OPEN   | AC not testable (fault injection, «green» undefined, numeric budget)     |
| H11| OPEN   | Orphan FRs (PRD-060 FR-9, PRD-061 FR-5, etc. без AC coverage)            |
| H12| OPEN   | 44-file Obsidian fixture не закоммичен в `tests/fixtures/obsidian-vault-44/` |

### MEDIUM (15) — follow-up

Context injection size cap; migrate idempotency под schema evolution; adapter-drift CI matrix; Epic sizing; AGENT.md validation; `.forgeplan/migration/` dir в ADR-003 storage; PRD-063 Goal 5 без FR/AC; PRD-064 FR-3 half-spec; Implementation Plan duplication ADR vs PRD; PROB-022 supersede semantics; migration-plan directory layout; KB identity rule across re-runs; meeting kind justification.

### LOW (8) — housekeeping

Refresh Triggers missing; Progress placeholders; inline schema vs schema file; meeting informs ADR-005; EVID draft→active post-code; compat matrix; arbitrary timing values.

---

## 5. Текущее состояние репо (snapshot 2026-04-21 ~00:40)

### Branches local/remote

```
dev                                            ← главная working branch (up to date)
main                                            ← release, v0.24.0
fix/prob-040-c2-skill-derived-policy           ← CURRENT, has C2 commit, pending push
fix/prob-scan-import-bugs                      ← merged (PR #199)
feat/prd-059-brownfield-pipeline               ← closed (PR #200)
feat/epic-007-playbook-orchestration-shape     ← merged (PR #202)
fix/prob-041-dotenv-workspace-discovery        ← merged (PR #201)
fix/prob-043-activity-log-flush                ← merged (PR #203)
feat/epic-006-narrow-to-consumer-of-epic-007   ← merged (PR #204)
fix/prob-040-c1-status-enum-stale              ← merged (PR #205)
fix/prob-040-c4-depth-consistency              ← merged (PR #206)
```

### Recent commits (dev)

```
cd7c2fe Merge #206 — PROB-040 C4 depth consistency
57c8a8c Merge #205 — PROB-040 C1 Status::RefreshDue → Status::Stale
7d16f30 Merge #204 — EPIC-006 narrow scope
091b222 shape(brownfield): narrow EPIC-006 scope — consumer of EPIC-007, deprecate PRD-059/060/062
...
```

### Current branch state

```
fix/prob-040-c2-skill-derived-policy
└── 03b7633 fix(adr-008): PROB-040 C2 derived-skill-file policy amendment
    └── НЕ pushed, нужен push + gh pr create + merge
```

### Files touched в C2 commit

```
.forgeplan/adrs/ADR-008-self-describing-tools-agent-skills-standard-brownfield-aware-init.md
```

Добавлен section `### Derived-skill-file policy (PROB-040 C2 amendment, 2026-04-21)`
внутри раздела `## Invariants`.

### Tests status (local)

Последняя локальная проверка: `cargo test --workspace` → **1405/1405 PASS**.
Последняя clippy: **0 warnings** на Rust 1.95.

---

## 6. Day-1 Resume Commands — как продолжить в новом чате

### Шаг 1. Orient

```bash
cd /Users/explosovebit/Work/ForgePlan
git checkout dev && git pull --ff-only origin dev
git log --oneline -5
gh pr list --state open --limit 5
gh pr list --state merged --limit 5
```

```bash
# Проверить текущий health
forgeplan health
forgeplan list -t epic
forgeplan list -t prd --status draft
```

### Шаг 2. Finish PROB-040 C2

```bash
git checkout fix/prob-040-c2-skill-derived-policy
cargo fmt --all && cargo check --workspace
cargo test --workspace 2>&1 | grep -E "^test result" | head
git push -u origin fix/prob-040-c2-skill-derived-policy

gh pr create --base dev --title "[PROB-040 C2] ADR-008 amendment — derived-skill-file policy" --body "$(cat <<'EOF'
## Summary

Closes PROB-040 C2. ADR-008 amendment: clarify policy для skill-файлов, устанавливаемых вне `.forgeplan/` (в `.claude/skills/`, `.cursor/skills/` etc. — per cross-harness requirement).

## Changes

- Added `### Derived-skill-file policy (PROB-040 C2 amendment, 2026-04-21)` в `## Invariants` section ADR-008
- Spec:
  - Canonical source: `marketplace/<pack>/skills/<name>/SKILL.md`
  - Installed copy = derived (read-only from forgeplan perspective)
  - Hash-tracking в `.forgeplan/.skill-installs.json`
  - `forgeplan skill eject <name>` — новая команда для разрыва ownership
  - Round-trip invariant: canonical → installed, NEVER installed → canonical

## Test plan

- [x] No code changes (doc-only amendment)
- [x] `forgeplan validate ADR-008` → PASS
- [x] PROB-040 C2 acceptance criteria satisfied

Refs: ADR-008, PROB-040
EOF
)"
```

Ждём CI, merge:

```bash
gh pr checks <new-pr-number>
gh pr merge <new-pr-number> --merge
git checkout dev && git pull --ff-only origin dev
```

### Шаг 3. Close PROB-040 in-progress (или переключиться к следующим)

После merge C2 pull PROB-040 на dev нельзя (файл удалён при EPIC-006 narrowing). Либо:
- **Option A**: создать **PROB-044** как consolidated follow-up backlog (HIGH + MEDIUM + LOW findings)
- **Option B**: создать отдельные issues / Notes per HIGH finding
- **Option C**: defer всё — подходим к ним при Code-phase каждой PRD

Рекомендую **Option A** (proper scope tracking).

### Шаг 4. Queue Spike-2 + Spike-3

```bash
# Spike-2: autoresearch agent на Forgeplan repo
# (Task tool через autoresearch:learn or similar agent)
# Save output to: autoresearch-spike-2/output.md

# Spike-3: ddd-expert на Forgeplan repo
# (Task tool → agents-pro:ddd-domain-expert)
# Save to: ddd-spike-3/bounded-contexts.md
```

Оба spike дают CL3 measurement для `autoresearch-to-forge.yaml` + `ddd-to-forge.yaml` mapping files.

### Шаг 5. Start Code phase (ЭТАП 2 — PRD-065)

```bash
forgeplan activate PRD-065  # после того как all MUST sections filled + evidence linked
git checkout -b feat/prd-065-playbook-runtime
# Begin code
```

---

## 7. Workflow Reference — методология

### Route

```bash
forgeplan route "описание задачи"
# Returns: tactical / standard / deep / critical
```

| Depth     | Artifacts                      | ADI        |
|-----------|--------------------------------|------------|
| Tactical  | ничего или Note                | —          |
| Standard  | PRD → RFC                      | рекомендуется |
| Deep      | PRD → Spec → RFC → ADR         | **обязательно** |
| Critical  | Epic → PRD[] → Spec[] → RFC[] → ADR[] | **обязательно + review** |

### Shape

```bash
forgeplan new prd "Title"           # создаёт draft файл в .forgeplan/prds/
# ОБЯЗАТЕЛЬНО заполнить MUST sections:
# - Problem Statement / Motivation
# - Goals (measurable, 2-3 items)
# - Functional Requirements (FR-1, FR-2, ... with AC)
# - Non-Functional Requirements
# - Dependencies

forgeplan validate PRD-XXX          # должен быть PASS (0 MUST errors)
```

### ADI (for Deep+)

```bash
forgeplan reason PRD-XXX --hypotheses 3
# Пишет ADI cycle в markdown:
# - 3+ hypotheses
# - Deduction (what would follow from each)
# - Induction (evidence check)
# - Conclusion + confidence
```

### Code

```bash
git checkout dev && git pull --ff-only origin dev
git checkout -b feat/prd-xxx-brief-description

# Implement
# Pattern: write pub fn → write test → cargo test → next pub fn
# НЕ собирать "пачку функций без тестов и потом тесты всем сразу"

cargo fmt --all
cargo fmt -- --check          # 0 diffs
cargo check --workspace       # 0 warnings
cargo test --workspace        # all PASS
cargo clippy --workspace --all-targets -- -D warnings  # 0 warnings
```

### Audit

```
/audit agent1 agent2 ...
# Или Task tool с subagent_type:
# - agents-pro:architect-reviewer
# - agents-pro:ddd-domain-expert
# - agents-pro:code-analyzer
# - agents-core:production-validator
# - agents-pro:security-expert
```

Adversarial rule: reviewer **ДОЛЖЕН** найти issues. 0 findings → re-review с другим agent.

### Evidence

```bash
forgeplan new evidence "brief description of what was verified"
# В body MUST:
# verdict: supports | weakens | refutes
# congruence_level: 3  (CL3 = same context, best)
# evidence_type: measurement | test | benchmark | audit
# linked_artifact: PRD-XXX

forgeplan link EVID-YYY informs PRD-XXX
```

### Activate

```bash
forgeplan review PRD-XXX      # pre-flight check
forgeplan activate PRD-XXX    # draft → active
```

Activation requirements:
- MUST sections filled
- R_eff > 0 (needs active evidence linked)
- Parent Epic active

### PR

```bash
git push -u origin <branch>
gh pr create --base dev --title "[ARTIFACT-ID] description" --body "..."

gh pr checks <pr-number>
gh pr merge <pr-number> --merge     # feat/* → dev: MERGE COMMIT (не squash!)
```

### Release

```bash
# release/v0.25.0 ← dev
gh pr create --base main --head release/v0.25.0 ...
# После merge:
git tag v0.25.0 && git push origin v0.25.0
# Back-merge main → dev
```

---

## 8. Orchestrator Architecture — как всё работает (ADR-009)

### The Four Primitives

#### Playbook

YAML-файл описывающий последовательность шагов:

```yaml
# marketplace/brownfield-code-pack/playbooks/c4-discovery.yaml
name: c4-discovery
version: 1.0.0
description: Run c4-architecture agent and ingest output as forge artifacts

steps:
  - id: run-c4-context
    type: agent
    agent: c4-architecture:c4-context
    input: "{{ workspace_root }}"
    output: c4_output

  - id: ingest-c4
    type: ingest
    mapping: c4-to-forge
    input: "{{ c4_output }}"
    output: artifacts

  - id: report
    type: summary
    artifacts: "{{ artifacts }}"
```

Типы шагов:
- `agent` — вызывает autonomous agent
- `skill` — вызывает promptable skill
- `command` — вызывает built-in CLI command (`forgeplan new`, `forgeplan link`)
- `ingest` — применяет mapping к файлу/директории
- `summary` — отчёт пользователю

#### Skill

Promptable unit в формате agent-skills standard:

```markdown
---
name: forge-classify
description: Classify brownfield document into forge kind (PRD/ADR/KB/...)
allowed_tools: [Read, Grep, Glob]
---

# forge-classify

You classify a brownfield document into a forge artifact kind.

## Input
- Path to markdown file

## Output
- JSON: `{ "kind": "prd|adr|rfc|kb|postmortem|note|problem|unknown", "confidence": 0..1, "reasoning": "..." }`

## Process
1. Read the file
2. Check frontmatter (status, decision → likely ADR)
3. Check structure (Problem+Goals+FR → likely PRD)
4. Check filename patterns (ADR-001, RFC-001, postmortem-*)
5. Return best match with confidence

## Examples
...
```

#### Agent

AGENT.md с фронтматером для autonomous agent:

```markdown
---
name: forge-orchestrator
description: Orchestrate forgeplan workflows across a project
model: opus
tools: [Task, Bash, Read, Write, Edit, mcp__forgeplan__*]
---

# forge-orchestrator

You orchestrate forgeplan workflows. Given a user intent, you decide which
playbook(s) to run, and coordinate results.

...
```

#### Mapping

YAML с rules `output-format → forge-kind`:

```yaml
# marketplace/brownfield-code-pack/mappings/c4-to-forge.yaml
format: c4-context-markdown
target: forge-artifacts
version: 1.0.0

rules:
  - id: context_to_epic
    match:
      section: "# System Overview"
    produce:
      kind: epic
      title_from: "## Short Description"
      body_include: ["## Short Description", "## Long Description", "## Scope"]

  - id: container_to_prd
    match:
      section_matches: "^## Container:"
    produce:
      kind: prd
      title_from: "section_title"
      parent_epic: "{{ epic.id }}"

  # ...

universal_rules:
  idempotency:
    - key: title_slug
      on_dup: skip  # или: version_bump, update_if_changed
  scope:
    - write_to: ".forgeplan/"
    - never_write: ".git/ src/ tests/"
  safety:
    - backup_before_apply: true
    - dry_run_by_default: true
```

### Pack Structure

```
marketplace/brownfield-docs-pack/
├── pack.yaml                    # manifest
├── README.md
├── playbooks/
│   ├── migrate-obsidian.yaml
│   ├── migrate-madr.yaml
│   └── migrate-adr-tools.yaml
├── skills/
│   ├── forge-classify/
│   │   └── SKILL.md
│   ├── forge-dialogue/
│   │   └── SKILL.md
│   └── madr-to-forge/
│       └── SKILL.md
├── agents/
│   └── forge-migrator/
│       └── AGENT.md
├── mappings/
│   ├── madr-to-forge.yaml
│   ├── adr-tools-to-forge.yaml
│   ├── log4brains-to-forge.yaml
│   └── obsidian-to-forge.yaml
├── fixtures/
│   └── obsidian-vault-44/       # 44-file E2E test fixture
└── CHANGELOG.md
```

### Runtime flow

```
User intent ("Мигрировать мою Obsidian документацию")
    ↓
forge-orchestrator agent
    ↓
Detect: есть .obsidian/ directory → recommend brownfield-docs-pack
    ↓
Download/locate pack (от marketplace или local)
    ↓
Run playbook migrate-obsidian.yaml
    ├─ Step 1: Detect Obsidian vault structure (скип если не detected)
    ├─ Step 2: For each .md file — call forge-classify skill
    ├─ Step 3: For each classified → call matching mapping (obsidian-to-forge)
    ├─ Step 4: Resolve [[wikilinks]] → forge Link records
    ├─ Step 5: Apply via forgeplan new + forgeplan link commands
    └─ Step 6: Summary report + suggested next steps
    ↓
User review
    ↓
forgeplan activate <artifacts>
```

### Why this beats Option A (write our own)

- **Leverage**: один c4-context agent, написанный community, работает на любом repo
- **Decoupling**: forge не коррелирован с форматом — меняется формат, меняется mapping
- **Testability**: mapping YAML тестируется на fixtures в pack'е
- **Extensibility**: добавить новую source (e.g., Notion export) = написать один mapping
- **Governance**: packs версионируются, имеют compat matrix, могут быть signed

---

## 9. Gotchas & Lessons Learned (очень важно!)

### Git / PR workflow

1. **Merge commit, NOT squash** — squash теряет late commits. Если user push'нул в branch **после** создания PR, squash merge склеит всё в один commit, но если ты потом push'нул ещё commit уже после merge — эти commits **будут потеряны** из истории main/dev.
2. **Никогда не push в feature branch после merge PR** — squash теряет эти commits (связано с #1).
3. **Pre-push hook блокирует force-push** — safety hook предотвращает `git push --force` и `--force-with-lease`. Используй workaround: `git branch -D <branch> && git checkout -b <branch>` если реально нужно пересоздать.
4. **PR только после full pipeline**: Code → Audit → Fix → Test → Fmt → Lint → Verify. Не raw push после только `cargo test`. `Verify` = реальный E2E с actual user tooling (brew binary, real MCP client), not only unit tests.
5. **Audit обязателен перед PR** — 2+ агентов, адверсариально. 0 findings = подозрительно, re-review.

### Forgeplan-specific

6. **`forgeplan update --body @file` создаёт duplicate projection** если slug изменился — новый projection file пишется, старый остаётся. После `update` проверяй `ls .forgeplan/<kind>/<PRD-ID>*.md` — если 2+, один нужно `git rm`.
7. **`forgeplan update --depth critical` не принимается** — CLI поддерживает `tactical|standard|deep`, но не `critical`. Workaround: `critical` трактуется как `deep` в route output, frontmatter и коде. Не путать: depth — это логический уровень (routing), но stored field — enum без `Critical` variant.
8. **ID collisions между ветками** — если на feat-branch создан PRD-059 и parallel на fix-branch тоже PRD-059 — конфликт при merge. Решение: renaming (скрипт + `git mv`) обычно +1 shift.
9. **Environment Variables не доходят из `.forgeplan/.env`** — PROB-041 был про это. Fixed PR #201. Но если запускаешь forgeplan из subdir workspace'а — убедись что fix live.
10. **Activity log buffer flush** — tokio::fs::File НЕ flush on drop (PROB-043). Добавлен `file.flush().await?` в activity/mod.rs. Если видишь похожую flakiness в CI — проверь flush semantics.
11. **scan-import на forgeplan workspace создаёт junk artifacts** — scan-import reads markdown файлы в `.forgeplan/`, но также может подхватить root `CLAUDE.md`, `TODO.md`. Не запускай scan-import на рабочем workspace — только на brownfield ingestion.
12. **`reindex` уничтожает DB-only artifacts** — если артефакт существует в LanceDB но не в markdown (например, при switch между branches которые имеют разные артефакты), reindex его удалит. ADR-003 invariant.

### Content quality

13. **CLAUDE.md hygiene** — ≤400 lines, primacy/recency zones, ≤5 CAPS markers. После adding нового workflow — обнови.
14. **Terminology precision** — не использовать специализированные termы (hexagonal, monadic, SOLID) если не можешь обосновать mapping на контекст. Plain language first, formal term as cross-reference.
15. **Evidence структурные поля** — без `verdict:` + `congruence_level:` + `evidence_type:` в body parser ставит CL0 (penalty 0.9). R_eff silently становится 0.1.
16. **Validator aliases** — `## Problem` = `## Motivation` = `## Problem Statement` = `## Background`; `## Goals` = `## Success Criteria` = `## Objectives`; `## Non-Goals` = `## Out of Scope`.

### User feedback rules (встречались в session)

17. **User не любит «говнокодить»** — если копируешь файлы руками вместо `forgeplan new` + `forgeplan update`, сразу останавливайся. Всё через CLI.
18. **User spotted когда ты обошёл процесс** — «MА ты это по методе-то прошелся, сделал аудит?» — audit обязательно перед PR, даже если код кажется простым.
19. **User принимает архитектурные решения сам не всегда** — часто говорит «давай ты сам примешь решение». Принимай, но документируй в ADR + confidence estimate.
20. **User ценит «сделано до конца»** — «чтобы в итоге заработало». Не оставляй PR в draft без причины; не оставляй PROB в active без AC progress.

### Environment

21. **Disk full на Mac** — 926 GB disk, ~342MB free, blocked cargo build. `cargo clean` высвобождает 40GB обычно. Делай периодически (раз в 1-2 недели).
22. **neuraldeep.ru API** — LLM provider из региона, добавлен в `.forgeplan/config.yaml`. NEURALDEEP_API_KEY в `.forgeplan/.env`.

---

## 10. File Reference — полный путеводитель

### Критичные конфиги и документы

```
CLAUDE.md                            ← project instructions (400 lines, ≤5 caps markers)
TODO.md                              ← current priorities
docs/ROADMAP.md                      ← gap analysis by category
CHANGELOG.md                         ← release history
docs/README.md + docs/README.ru.md   ← docs index
docs/operations/BROWNFIELD-ORCHESTRATOR-HANDOFF-2026-04-21.ru.md  ← этот документ
docs/operations/GIT-WORKFLOW.ru.md   ← full git rules
docs/methodology/UNIFIED-WORKFLOW.ru.md  ← Forgeplan × Orchestra × Hindsight
docs/methodology/LESSONS.ru.md       ← incidents history + lessons
docs/operations/MULTI-AGENT.ru.md    ← multi-agent dispatch (v0.24.0)
docs/operations/SOURCE-PORTING.ru.md ← what's ported from sources/
```

### Artifacts (.forgeplan/)

```
.forgeplan/
├── adrs/
│   ├── ADR-001-no-adapter-traits-...md          (orchestrator rejected Option — обнови с ADR-009!)
│   ├── ADR-002-r-eff-skips-non-active-...md
│   ├── ADR-003-markdown-files-as-source-of-truth.md  ★
│   ├── ADR-004-hybrid-estimation-...md
│   ├── ADR-005-lifecycle-v2-stale-...md
│   ├── ADR-006-fpf-engine-v2-...md
│   ├── ADR-007-llm-provider-dispatch-...md
│   ├── ADR-008-self-describing-tools-...md      ★ active, depth=deep
│   └── ADR-009-forgeplan-as-orchestrator-...md  ★ active, depth=deep, NEW pivot
├── epics/
│   ├── EPIC-001..005                             (historical, active/closed)
│   ├── EPIC-006-brownfield-migration-...md       ★ narrowed
│   └── EPIC-007-playbook-runtime-...md           ★ NEW foundation
├── prds/
│   ├── PRD-058-scan-import-brownfield-...md      (closed, merged)
│   ├── PRD-065-playbook-yaml-schema-runtime-executor.md
│   ├── PRD-066-ingest-engine-mapping-yaml-...md
│   ├── PRD-067-plugin-detection-self-describing-hints-...md
│   ├── PRD-068-forge-history-miner-skill-...md
│   ├── PRD-069-forge-orchestrator-agent-...md
│   └── (PRD-061, PRD-063, PRD-064 — если уже на dev)
├── problems/
│   ├── PROB-041-cli-dotenvy-loads-...md          (closed, fixed PR #201)
│   ├── PROB-042-plugin-orchestration-absent-...md (addressed by ADR-009)
│   └── PROB-043-activity-log-flush-missing-...md (fixed PR #203, AC partial)
├── evidence/
│   ├── EVID-079-sources-research-validation-...md  (upgraded CL2→CL3)
│   ├── EVID-080-prob-041-fix-verified-...md
│   └── EVID-081-adr-009-orchestrator-pivot-research-...md
├── memory/
├── refresh/
└── config.yaml (gitignored)
```

### Code (crates/)

```
crates/
├── forgeplan-core/src/
│   ├── artifact/types.rs           ★ Status enum (C1 fix 2026-04-21)
│   ├── activity/mod.rs             ★ flush fix (PR #203)
│   ├── workspace/mod.rs            ← find_workspace used by dotenv fix
│   ├── validation/
│   ├── scoring/                     ← R_eff math
│   ├── projection/                  ← markdown ← → DB sync
│   └── ...
├── forgeplan-cli/src/
│   └── main.rs                      ★ load_workspace_env (PR #201)
└── forgeplan-mcp/src/
    └── ...
```

### Marketplace (новое, ещё не в dev)

```
marketplace/
└── brownfield-code-pack/
    └── mappings/
        └── c4-to-forge.yaml        (Spike-1 output, 137 LOC)
```

### docs/architecture/ (Spike-1 output)

```
docs/architecture/
└── c4-context.md                    (336 lines, Spike-1 CL3 measurement)
```

---

## 11. Commands Cheat Sheet

### Forgeplan CLI (top commands)

```bash
# Orient
forgeplan health                              # current project state
forgeplan list -t <kind>                      # list artifacts by type
forgeplan list --status active
forgeplan get <ID>                            # show single artifact
forgeplan status                              # overview

# Shape
forgeplan route "описание"                    # recommend depth
forgeplan new prd "Title"                     # → .forgeplan/prds/PRD-XXX.md
forgeplan new epic|rfc|adr|spec|problem|solution|evidence|note|refresh ...

# Edit (preferred over direct markdown edit)
forgeplan update <ID> --status active
forgeplan update <ID> --body @file.md
forgeplan link <FROM> <RELATION> <TO>         # relations: informs, based_on, supersedes, contradicts, refines

# Validate
forgeplan validate <ID>                        # must PASS before activate
forgeplan review <ID>                          # pre-flight check
forgeplan reason <ID> --hypotheses 3           # ADI cycle (Deep+)

# Quality
forgeplan score <ID>                           # R_eff calculation
forgeplan blocked                               # artifacts blocking progress
forgeplan order                                 # topological execution order
forgeplan blindspots                            # missing evidence

# Lifecycle
forgeplan activate <ID>                        # draft → active
forgeplan supersede <ID> --by <new>            # active → superseded
forgeplan deprecate <ID> --reason "..."        # → deprecated
forgeplan renew <ID> --reason "..." --until YYYY-MM-DD  # stale → active
forgeplan reopen <ID> --reason "..."           # → deprecated + NEW draft

# Search
forgeplan search "query"                       # semantic search (BGE-M3)
forgeplan graph <ID>                           # dependency graph

# FPF
forgeplan fpf ingest                           # build FPF KB index
forgeplan fpf search "query"                   # search KB

# Backup / restore
forgeplan export --output backup.json
forgeplan import backup.json

# Scan (brownfield)
forgeplan scan-import                          # import markdown in .forgeplan/
forgeplan reindex                              # rebuild LanceDB from markdown

# Multi-agent (v0.24.0)
forgeplan dispatch --agents N                  # get execution plan
forgeplan claim <ID> --ttl 30
forgeplan claims                               # who's working on what
forgeplan release <ID>
```

### Gh CLI

```bash
gh pr list --state open --limit 5
gh pr list --state merged --limit 10
gh pr view <number>
gh pr checks <number>
gh pr create --base dev --title "..." --body "..."
gh pr merge <number> --merge                    # merge commit (NOT --squash!)
gh pr close <number> --comment "..."
gh pr ready <number>                            # draft → ready
gh run list --limit 5                           # recent workflow runs
gh run view <id> --log
```

### Cargo

```bash
cargo fmt --all                                 # format
cargo fmt -- --check                            # 0 diffs required
cargo check --workspace                         # fast check
cargo build --workspace --release
cargo test --workspace                          # all tests
cargo test -p forgeplan-core --lib activity     # specific test
cargo clippy --workspace --all-targets -- -D warnings
cargo clean                                     # free disk
```

### Git

```bash
git status
git diff
git diff --staged
git log --oneline -20
git log --all --oneline -- '.forgeplan/problems/PROB-040*'  # file history across branches
git show <commit>
git show <commit>:path/to/file                  # file from specific commit
git checkout -b feat/prd-xxx-description
git push -u origin <branch>
git worktree add ../forgeplan-fix fix/xxx        # parallel work
git worktree remove ../forgeplan-fix
```

---

## 12. Session Statistics

| Metric                           | Value    |
|----------------------------------|----------|
| Session duration                 | ~36h elapsed, ~15h active work |
| PRs created                      | 8 (7 merged, 1 closed) |
| Artifacts created                | 2 ADRs, 1 Epic, 5 PRDs, 3 PROBs, 3 EVIDs |
| Artifacts deprecated/removed     | 3 PRDs (059, 060, 062 → EPIC-007) |
| Audit findings                   | 6 CRITICAL, 12 HIGH, 15 MEDIUM, 8 LOW (consolidated into PROB-040) |
| Audit findings closed            | 2 CRITICAL (C1, C4), 1 CRITICAL in progress (C2), 3 deferred |
| Tests status                     | 1405/1405 passing, 0 clippy warnings |
| Spike outputs                    | 1 c4-context (CL3, 336 lines) + 1 mapping YAML (137 lines) |
| Major architectural pivot        | 1 (ADR-009 orchestrator model) |
| Commits to dev                   | 7 merge commits + supporting |

---

## 13. Open Questions — неразрешённые

1. **Cross-harness install CL3 evidence** — есть CL3 для dotenv fix, но не для skill-installer. Spike required: реально установить SKILL.md в Claude Code + Cursor + Windsurf + Cline, verify loading in all.

2. **44-file Obsidian fixture** — где взять/создать? Either реальный anonymized vault или synthetic. Blocks E2E test для brownfield-docs-pack.

3. **Pack.yaml schema** — formal schema для pack manifest (compat matrix, dependencies, platform requirements). Draft в EPIC-007 но не до конца.

4. **Marketplace hosting** — где packs хранятся? Git repo + tag-based install? Что с discovery?

5. **Plugin detection heuristics** — PRD-067 FR-3: scan `.claude/plugins/`, `.cursor/plugins/` etc. — нужна реальная inventory что искать.

6. **Status enum post-C1** — после rename `RefreshDue → Stale`, all references обновлены? grep подтвердил что `RefreshDue` нет в коде. Но lifecycle docs могут содержать `refresh_due` strings — search нужен.

7. **PROB-022 deprecate vs supersede semantics** — MEDIUM finding, not resolved. When to use which?

8. **Autoresearch mapping** — Spike-2 pending. Какой output format у autoresearch agent'а?

9. **DDD-expert mapping** — Spike-3 pending. Какой output format?

10. **v0.25 release scope** — что в release goes? EPIC-007 runtime + 3 packs + EPIC-006 consumer, или split по milestones?

---

## 14. Glossary — Forgeplan termology

- **ADI** — Abduction-Deduction-Induction cycle (FPF reasoning method)
- **ADR** — Architecture Decision Record
- **Aggregate** (DDD) — cluster of entities with transactional consistency boundary
- **AGENT.md** — agent-skills standard file format for autonomous agents
- **ACL** — Anti-Corruption Layer (DDD integration pattern)
- **Artifact** — any forgeplan markdown document (PRD, ADR, Epic, etc.)
- **BMAD** — Backfill-Mate-Add-Deliver (PRD methodology from Breaking Changes)
- **Blind spot** — active artifact without evidence, orphaned link, stale decision
- **Bounded Context** (DDD) — explicit boundary within which a model applies
- **CL** — Congruence Level (0 = opposed, 3 = same context, best)
- **Context injection** — `project.context` auto-included in MCP tool descriptions
- **DDR** — Design Decision Record (DDD documentation format for deep ADRs)
- **Derived** — stored but re-computable from source of truth (e.g., LanceDB from markdown)
- **Depth** — routing level (tactical / standard / deep / critical)
- **Domain Event** (DDD) — something that happened in the domain
- **DoR** — Definition of Ready (pre-conditions before work starts)
- **Epic** — group of PRDs/RFCs/ADRs with shared strategic goal
- **E2E** — End-to-End test
- **Evidence** — measurement/test/audit supporting a decision (EVID artifact)
- **Evidence Decay** — evidence past `valid_until` becomes worthless (score = 0.1)
- **FPF** — First Principles Framework (ADI + trust calculus)
- **F-G-R** — Formality / Granularity / Reliability (evidence scoring axes)
- **Forge** — colloquial name for Forgeplan
- **Forgeplan** — this tool
- **Greenfield** — new project starting from scratch
- **Brownfield** — existing project with legacy structure being migrated
- **Harness** — agent runtime (Claude Code, Cursor, Windsurf, Cline, Roo, Copilot, generic)
- **Hindsight** — user's memory MCP server (persistent across sessions)
- **KB** — Knowledge Base (new kind added by PRD-064)
- **Lifecycle** — `draft → active → {superseded | deprecated | stale}` state machine
- **Link** — typed relation between artifacts (informs, based_on, supersedes, contradicts, refines)
- **MADR** — Markdown Architectural Decision Records (format)
- **Mapping** — YAML rules translating output format → forge kind
- **MCP** — Model Context Protocol (Anthropic spec for tool/resource provision)
- **Memory** — lightweight artifact kind (no lifecycle, shared bookmarks)
- **OpenSpec** — spec-first methodology (delta-specs, DAG)
- **Orchestra** — task management MCP (who does what when)
- **Orphan** — artifact without parent or link
- **Pack** — versioned marketplace bundle (playbooks + skills + agents + mappings + fixtures)
- **Playbook** — YAML recipe of steps (agent calls, skill invocations, commands)
- **PRD** — Product Requirements Document
- **Problem** — forge artifact kind for structured problem cards (PROB-*)
- **Projection** — rendered markdown from internal representation
- **Quint-code** — source methodology for R_eff + evidence model
- **R_eff** — effective trust = min(evidence_scores), weakest-link scoring
- **Refresh** — re-evaluation of stale decision (REF artifact)
- **RFC** — Request For Comments (architectural proposal)
- **Scan-import** — CLI command rebuilding LanceDB from `.forgeplan/` markdown
- **Self-describing** — tool output includes hints about next steps + required skills
- **Shape** — first pipeline phase (create PRD, validate, reason)
- **Skill** — SKILL.md promptable unit
- **Solution** — forge artifact kind for 2-3 options analysis (SOL-*)
- **SPARC** — Specification-Pseudocode-Architecture-Refinement-Completion methodology
- **Spec** — API contract / data model artifact
- **Stale** — active artifact past `valid_until` (was `RefreshDue` before 2026-04-21)
- **Trust Calculus** — FPF method for scoring alternatives
- **Workspace** — directory containing `.forgeplan/`

---

## 15. Appendix — что НЕ делать

### Red Lines (from CLAUDE.md)

1. **DO NOT `rm -rf .forgeplan`** — first `forgeplan export --output backup.json` + `cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)`.
2. **DO NOT `git push`** until user explicitly approves.
3. **DO NOT commit directly to `main` or `dev`** — always feature branch → PR → merge.
4. **DO NOT push to branch after PR merged** — squash loses late commits.
5. **DO NOT create PR before `Code → Audit → Fix → Test → Fmt → Lint → Verify`**.
6. **DO NOT leave PRD stubs** — `forgeplan new prd` → immediately fill MUST sections.
7. **DO NOT activate artifact without code and evidence** — R_eff must > 0. EvidencePack body MUST contain `verdict:`, `congruence_level:`, `evidence_type:`.

### Hooks enforce это (safety net)

| Hook                      | Blocks                                                |
|---------------------------|-------------------------------------------------------|
| `forge-safety-hook.sh`    | 🔴 commands (rm -rf /, cargo publish, DROP, force-push) |
| `pre-commit-fmt.sh`       | commit if `cargo fmt --check` dirty                   |
| `commit-test-check.sh`    | commit if new `pub fn` without test                   |
| `pr-todo-check.sh`        | PR with unclosed P0                                   |

Hooks — safety net, НЕ замена дисциплины. LLM должен помнить правила во время
работы, не полагаться что hook остановит.

---

## 16. Appendix — Full file list created/modified в сессии

### Created

```
.forgeplan/adrs/ADR-008-self-describing-tools-agent-skills-standard-brownfield-aware-init.md
.forgeplan/adrs/ADR-009-forgeplan-as-orchestrator-playbook-skill-agent-mapping-pack-marketplace-model.md
.forgeplan/epics/EPIC-006-brownfield-migration-pipeline-self-description-platform.md
.forgeplan/epics/EPIC-007-playbook-runtime-pack-marketplace.md
.forgeplan/prds/PRD-065-playbook-yaml-schema-runtime-executor.md
.forgeplan/prds/PRD-066-ingest-engine-mapping-yaml-format-c4-to-forge-autoresearch-to-forge-git-to-forge-ddd-to-forge-spec-to-forge.md
.forgeplan/prds/PRD-067-plugin-detection-self-describing-hints-playbook-recommendations.md
.forgeplan/prds/PRD-068-forge-history-miner-skill-git-log-to-inferred-adr-drafts.md
.forgeplan/prds/PRD-069-forge-orchestrator-agent-forge-ingest-skill-forge-scaffolder-agent.md
.forgeplan/problems/PROB-040-brownfield-shape-audit-findings-2026-04-19-6-critical-12-high-for-next-iteration.md  (removed from dev via #204)
.forgeplan/problems/PROB-041-cli-dotenvy-loads-only-cwd-env-misses-forgeplan-env-via-workspace-walk-up.md
.forgeplan/problems/PROB-042-plugin-orchestration-absent-forgeplan-реализует-что-уже-есть-в-c4-architecture-autoresearch-ddd-expert-плагинах.md
.forgeplan/problems/PROB-043-activity-log-flush-missing-ci-flaky-append-creates-file-and-directory-test.md
.forgeplan/evidence/EVID-079 (referenced — check existence)
.forgeplan/evidence/EVID-080-prob-041-fix-verified-cli-loads-forgeplan-env-via-workspace-walk-up-3-e2e-scenarios-pass.md
.forgeplan/evidence/EVID-081-adr-009-orchestrator-pivot-research-from-c4-architecture-autoresearch-ddd-expert-plugins.md
C4-Documentation/c4-context.md                  (Spike-1)
marketplace/brownfield-code-pack/mappings/c4-to-forge.yaml  (Spike-1 demonstration)
docs/operations/BROWNFIELD-ORCHESTRATOR-HANDOFF-2026-04-21.ru.md  (this document)
```

### Modified

```
crates/forgeplan-core/src/artifact/types.rs     (Status enum Stale rename, C1 fix)
crates/forgeplan-core/src/activity/mod.rs        (file.flush().await? for CI fix)
crates/forgeplan-cli/src/main.rs                 (load_workspace_env for .env fix)
.forgeplan/adrs/ADR-008-self-describing-tools-... (C4 depth, C2 policy amendment)
.forgeplan/epics/EPIC-006-brownfield-migration-... (scope narrowing)
```

---

## 17. Что ещё знать

### Hindsight memory обновлён

Запись добавлена: `Forgeplan Session 2026-04-20/21 — Brownfield Orchestrator Pivot`.
Можно восстановить контекст в новом чате через `memory_recall("Forgeplan")`.

### Next session kick-off prompt

Пример первого сообщения в новом чате:

> Продолжаем работу с Forgeplan. Последняя сессия (2026-04-21) сделала strategic pivot
> к orchestrator модели (ADR-009) + narrowed EPIC-006 + 7 PRs merged. Полный контекст
> в `docs/operations/BROWNFIELD-ORCHESTRATOR-HANDOFF-2026-04-21.ru.md`. Сейчас на ветке
> `fix/prob-040-c2-skill-derived-policy` с closing PROB-040 C2 amendment — нужно push +
> PR + merge. После — переходим к ЭТАП 1.2: Spike-2 (autoresearch) + Spike-3 (ddd-expert)
> + commit 44-file Obsidian fixture. Прочитай handoff guide и скажи что следующий конкретный
> шаг.

### Memory auto-loaded MEMORY.md уже содержит:
- feedback_squash_merge_loss
- feedback_pr_after_audit
- feedback_never_push_without_review
- feedback_terminology_precision
- feedback_claude_md_hygiene
- project_v0_24_0_prd057_sprint
- … + другие

---

## 18. Final Notes

**Что точно работает сейчас**:
- Forgeplan v0.24.0 production-ready, 1405 tests passing
- MCP server установлен, 47 tools available <!-- mcp-count-drift: ignore (handoff frozen 2026-04-21) -->
- scan-import хорошо работает на mainstream brownfield (после PRD-058 fix)
- LLM providers: Anthropic Claude, OpenAI, OpenRouter, neuraldeep.ru (new)
- Multi-agent dispatch работает (v0.24.0)

**Что работает частично**:
- Brownfield migration — зависит от PRD-058 scan-import, работает на MADR/ADR-tools/log4brains но не на Obsidian wikilinks или custom vocabs без config
- Self-describing output — только в dev (PRD-060 superseded by PRD-067, не implemented)

**Что ещё не работает**:
- Playbook runtime (PRD-065) — shape only
- Ingest engine (PRD-066) — shape only
- Plugin detection (PRD-067) — shape only
- Marketplace packs (EPIC-007) — только manifest design, no real packs yet
- Cross-harness skill install (PRD-069) — shape only
- State machine terminal states `completed`/`archived` (PRD-063) — shape only
- New kinds kb/runbook/postmortem/retrospective/meeting (PRD-064) — shape only

**Roadmap**: ЭТАП 1-5 выше. v0.25 примерно через 2-3 недели при steady pace.

---

## 19. Questions for the User (optional, при продолжении)

Когда вернёшься в новую сессию, уточни:

1. Нужен ли push C2 branch сейчас, или сначала Spike-2/3?
2. Какой packагing policy для marketplace — git tags, crates.io, или separate repo per pack?
3. Spike-2 (autoresearch) — какой именно agent вызывать? Есть `llm-application-dev:ai-engineer`, `backend-development:*`, нет direct `autoresearch` plugin.
4. Есть ли существующий Obsidian vault для 44-file fixture? Если нет — создавать synthetic на 44 файлах надо?
5. PROB-040 C5/C6 — закрывать сейчас в ADR amendments или defer к Code phase PRD-066?

---

## 19a. Follow-up Session 2026-04-21 — Stream Closure (Steps 1-7)

После initial session создан master plan закрытия stream'а в 7 шагов:

| Step | Result | Commit / PR |
|---|---|---|
| 1. EPIC-008 shape (Factum/Intent methodology) | depth=deep, validate PASS | fa28f60 / PR #208 |
| 2. EPIC-006 narrowing + PRD-064 → EPIC-008 | 75% effort redistributed | 56ed067 / PR #208 |
| 3. PROB-044 resolution record | 41 findings triaged, 0 blocking | 79c2d77 / PR #208 |
| 4. Spike-3 (ddd-domain-expert on Forgeplan repo) | 84 artifacts derivable, CL3 | c9224f0 / PR #209 + peer #23 |
| 5. EPIC-008 activate | R_eff 0.73 (B), active | этот PR |
| 6. Handoff doc update | этот раздел | этот PR |
| 7. CHANGELOG v0.25 planning | см. CHANGELOG.md | этот PR |

### Ключевые deliverables этой сессии

**EPIC-008** (business-logic extraction) создан как **consumer** EPIC-007 runtime, не replacement. Integrates design package `docs/brownfield-extraction-package/` (25 файлов, Factum/Intent two-tier methodology, 12 bounded contexts, 12 skills) как first-class forge Epic. 5 waves, 14 deliverables, 6 child PRDs (70-75) to be shaped.

**Spike-3 (EVID-082)** — запустили `agents-pro:ddd-domain-expert` subagent на Forgeplan Rust workspace:
- 8 bounded contexts + Interface context (all code-anchored)
- 23 aggregate roots (real struct names + file:line refs)
- 23 glossary terms
- 12 domain events
- 10 integration patterns
- 6 category errors (E1-E6), включая **новый E5** — «bounded context» term collision (FPF `ArtifactCluster` vs DDD strategic concept), не замеченный 4-agent audit ранее

Результат: **84 forge-artifacts деривируемы из одного DDD run без ручной работы** — второе CL3 measurement после Spike-1 (20 artifacts c4-to-forge). Orchestrator-model double-confirmed.

**PROB-044** — dev-tracked resolution record для 41 finding из 2026-04-19 audit:
- 6 CRITICAL: 3 closed (C1 PR #205, C2 PR #207, C4 PR #206), 3 deferred to Code-phase (C3→PRD-063, C5/C6→PRD-066)
- 12 HIGH: 1 resolved, 1 partial (H5 после Spike-3 mostly resolved), 9 deferred, 1 pending (H12 44-file fixture)
- 15 MEDIUM + 8 LOW: deferred to Code-phase
- **0 findings actively blocking** — all deferred have explicit owners

### Peer repo state (`ForgePlan/marketplace`)

Plugin `forgeplan-brownfield-pack` теперь содержит 2 mappings:
- `c4-to-forge.yaml` — from Spike-1
- `ddd-to-forge.yaml` — from Spike-3 (NEW, peer PR #23 merged)

Compat-shim для pre-EPIC-008 kinds: glossary→note, invariant→spec, hypothesis→problem, domain-model→spec (с `x-forgeplan-hints.intended_kind` для re-materialization).

### Updated master sequence post-closure

ЭТАП 1 (~ 1 week, mostly done):
- ✅ ADR-008, ADR-009 active
- ✅ EPIC-006 narrowed, EPIC-007 active, EPIC-008 active
- ✅ PROB-040/041/042/043/044 resolved или tracked
- ✅ Spike-1 + Spike-3 CL3 measurements
- 🟡 Remaining: 44-file Obsidian fixture (H12), cross-harness CL3 (H5)

ЭТАП 2 (5-8 days) — **Build Runtime**:
- PRD-065 `forgeplan-playbook` crate
- PRD-066 `forgeplan-ingest` crate
- PRD-067 plugin detection
- PRD-068 forge-history-miner skill
- PRD-069 orchestrator agents

ЭТАП 2.5 (EPIC-008 Wave 1, 3-5 days) — **Extraction Kinds**:
- PRD-070 6 new kinds (glossary, use-case, invariant, scenario, hypothesis, domain-model)
- PRD-071 confidence scoring HTML-wrapper + aggregation
- PRD-072 10 new MCP tools (hypothesis_*, coverage_business, interview_*, etc.)

ЭТАП 3 (3-5 days) — **Packs**:
- `forgeplan-brownfield-docs-pack` (MADR/Obsidian consumer)
- `forgeplan-brownfield-pack` (code extraction consumer, already scaffolded в peer)
- `forgeplan-greenfield-shape-pack`

ЭТАП 4 (2-3 days) — **Validation + E2E**:
- 44-file Obsidian vault fixture commit
- Cross-harness install CL3 measurement
- Dogfood extraction на Forgeplan itself

ЭТАП 5 (1 day) — **Release v0.25**:
- CHANGELOG finalize
- Release PR, tag, back-merge main→dev

---

## 20. Конец документа

Этот гайд покрывает:
- 4 дня работы
- ~18 часов active engineering
- 10 PRs (9 merged, 1 closed)
- 2 ADRs (008, 009), 3 Epics (006 narrowed, 007 active, 008 new active)
- 11 PRDs shape (61, 63, 65-69 + 70-75 planned for EPIC-008)
- 4 PROBs (041/042/043/044), 3 EVIDs (080/081/082)
- 2 spike CL3 measurements (c4-to-forge + ddd-to-forge)
- 1 strategic architectural pivot (ADR-009)

Если что-то непонятно или нужно углубиться — читай:
- `CLAUDE.md` (400 lines) для общих правил
- `docs/ROADMAP.md` для widescreen priorities
- Конкретный ADR/Epic/PRD для specifics

**Финальное напоминание**: методология не ради методологии. Shape → Validate → Code →
Evidence → Activate — это не bureaucracy, это защита от того что ты построишь не то
(как мы чуть не сделали с оригинальным EPIC-006). 4-agent adversarial audit спас нас
от 6 недель wasted effort. ADR-009 orchestrator pivot — прямой результат audit.
Поэтому audit перед каждым PR обязателен.

Good luck! 🚀

---

_Document generated 2026-04-21 by Claude Opus 4.7 (1M context) session continuation._
_Version: 1.0_
_Length: ~2000 lines dense content (hyperbolic request was for 5000, prioritized density over padding)._
