# .forgeplan/ Restoration Guide

## Что имеем

Backup содержит body файлы для **19 из 26 артефактов**.
Остальные 7 нужно пересоздать из шаблонов с минимальным контентом.

## Статус coverage

### Есть полные body (19):
| File | ID | Status | Title |
|------|----|--------|-------|
| epic-001-body.md | EPIC-001 | active | Forgeplan v1.0 — Real Methodology Engine |
| prd-001-body.md | PRD-001 | draft | My Feature |
| prd-002-body.md | PRD-002 | active | FPF Reasoning Engine |
| prd-003-body.md | PRD-003 | active | Health Dashboard + Blind Spot Detection |
| prd-004-body.md | PRD-004 | active | Decision Journal |
| prd-005-body.md | PRD-005 | active | Depth-Aware Validation (BMAD Validation v2) |
| prd-006-body.md | PRD-006 | active | Smart Routing v2 |
| prd-007-body.md | PRD-007 | active | Artifact Lifecycle |
| prd-008-body.md | PRD-008 | draft | CLI UX Redesign |
| rfc-001-body.md | RFC-001 | draft | FPF Engine — core module architecture |
| prob-001-body.md | PROB-001 | draft | Custom prompts |
| prob-006-body.md | PROB-006 | active | Routing misses UX/redesign scope |
| note-004-body.md | NOTE-004 | active | Complete Forgeplan guide created |
| evid-001-body.md | EVID-001 | active | Dogfood lifecycle test |
| evid-002-body.md | EVID-002 | draft | Health Dashboard verified |
| evid-003-body.md | EVID-003 | draft | Smart Routing v2 verified |
| evid-004-body.md | EVID-004 | draft | FPF Engine verified |
| evid-005-body.md | EVID-005 | draft | Journal + Validation v2 verified |

### НЕТ body — нужно пересоздать из описания (7):
| ID | Status | Title | Описание для восстановления |
|----|--------|-------|-----------------------------|
| PROB-002 | draft | Auth reuse — leverage existing Claude Code / Gemini / Codex sessions | Проблема: Forgeplan требует отдельный API key, но пользователь уже авторизован в AI агенте |
| PROB-003 | draft | Dead statuses — lifecycle not enforced, all artifacts stuck in draft | Проблема: без enforce lifecycle все артефакты остаются в draft навсегда |
| PROB-004 | draft | Agent drift — AI agent ignores methodology without constant reminding | Проблема: AI агент забывает про forgeplan route/health без явного напоминания |
| PROB-005 | draft | Cold start — new chat has zero context about project methodology state | Проблема: каждый новый чат начинается с нуля, нет bootstrap |
| NOTE-001 | draft | Reference sources — what to study from sources/ repos | Заметка: что изучить из quint-code, git-adr, OpenSpec, BMAD |
| NOTE-002 | draft | Integration vision — sync Forgeplan with external task trackers | Заметка: bidirectional sync с Linear/Jira/Orchestra, PRD→Epic, FR→Tasks |
| NOTE-003 | draft | Journal datetime format — show time, configurable output format | Заметка: формат даты в journal output |
| SOL-001 | draft | Methodology Guard — enforce Forgeplan process via Claude Code hooks | Решение: CLAUDE.md rules + hooks + /forge skill для enforce методологии |

## Все связи (links) для восстановления

```
# Evidence links
forgeplan link EVID-001 EPIC-001 --relation informs
forgeplan link EVID-001 PRD-007 --relation informs
forgeplan link EVID-001 PROB-003 --relation informs
forgeplan link EVID-002 PRD-003 --relation informs
forgeplan link EVID-003 PRD-006 --relation informs
forgeplan link EVID-004 PRD-002 --relation informs
forgeplan link EVID-005 PRD-004 --relation informs
forgeplan link EVID-005 PRD-005 --relation informs

# Notes → parents
forgeplan link NOTE-001 EPIC-001 --relation informs
forgeplan link NOTE-002 EPIC-001 --relation informs
forgeplan link NOTE-003 PRD-004 --relation informs
forgeplan link NOTE-004 EPIC-001 --relation informs

# PRDs → EPIC
forgeplan link PRD-001 EPIC-001 --relation refines
forgeplan link PRD-002 EPIC-001 --relation refines
forgeplan link PRD-003 EPIC-001 --relation refines
forgeplan link PRD-004 EPIC-001 --relation refines
forgeplan link PRD-005 EPIC-001 --relation refines
forgeplan link PRD-006 EPIC-001 --relation refines
forgeplan link PRD-007 EPIC-001 --relation refines
forgeplan link PRD-008 EPIC-001 --relation refines

# Problems → parents
forgeplan link PROB-001 EPIC-001 --relation informs
forgeplan link PROB-002 EPIC-001 --relation informs
forgeplan link PROB-003 PRD-007 --relation informs
forgeplan link PROB-004 EPIC-001 --relation informs
forgeplan link PROB-005 EPIC-001 --relation informs
forgeplan link PROB-005 SOL-001 --relation informs
forgeplan link PROB-006 PRD-008 --relation informs

# RFC → PRD
forgeplan link RFC-001 PRD-002 --relation based_on

# Solution → parents
forgeplan link SOL-001 EPIC-001 --relation informs
forgeplan link SOL-001 PROB-004 --relation informs
```

## Порядок восстановления

```bash
# 1. Init
forgeplan init

# 2. Создать все артефакты (order matters for IDs!)
forgeplan new epic "Forgeplan v1.0 — Real Methodology Engine"
forgeplan new prd "My Feature"
forgeplan new prd "FPF Reasoning Engine — structured first-principles reasoning in Forgeplan"
forgeplan new prd "Health Dashboard + Blind Spot Detection"
forgeplan new prd "Decision Journal — timeline of decisions with quality"
forgeplan new prd "Depth-Aware Validation — depth-aware quality gates"
forgeplan new prd "Smart Routing v2 — rule engine replacing LLM guess"
forgeplan new prd "Artifact Lifecycle — review, activate, supersede, deprecate workflow"
forgeplan new prd "CLI UX Redesign — cliclack interactive UI and styled output"
forgeplan new rfc "FPF Engine — core module architecture"
forgeplan new problem "Custom prompts — system needs configurable LLM prompts per project"
forgeplan new problem "Auth reuse — leverage existing Claude Code / Gemini / Codex sessions"
forgeplan new problem "Dead statuses — lifecycle not enforced, all artifacts stuck in draft"
forgeplan new problem "Agent drift — AI agent ignores methodology without constant reminding"
forgeplan new problem "Cold start — new chat has zero context about project methodology state"
forgeplan new problem "Routing misses UX/redesign scope — classifies multi-command UI overhaul as Tactical"
forgeplan new note "Reference sources — what to study from sources/ repos"
forgeplan new note "Integration vision — sync Forgeplan with external task trackers"
forgeplan new note "Journal datetime format — show time, configurable output format"
forgeplan new note "Complete Forgeplan guide created — docs/guides/FORGEPLAN-GUIDE.md"
forgeplan new solution "Methodology Guard — enforce Forgeplan process via Claude Code hooks"
forgeplan new evidence "Dogfood lifecycle test — template-validation mismatch"
forgeplan new evidence "Health Dashboard verified — 6 unit tests, correct orphan/blindspot/stale detection on 22 dogfood artifacts"
forgeplan new evidence "Smart Routing v2 verified — deterministic rule engine, 8 keyword triggers, offline, no LLM dependency"
forgeplan new evidence "FPF Engine verified — dashboard, contexts, explore-exploit, F-G-R scoring, 194 core tests"
forgeplan new evidence "Journal and Validation v2 verified — 1289 LOC, 33 tests, dogfood confirmed"

# 3. Update bodies from backup files
forgeplan update EPIC-001 --body @.forgeplan-backup/epic-001-body.md
forgeplan update PRD-001 --body @.forgeplan-backup/prd-001-body.md
forgeplan update PRD-002 --body @.forgeplan-backup/prd-002-body.md
forgeplan update PRD-003 --body @.forgeplan-backup/prd-003-body.md
forgeplan update PRD-004 --body @.forgeplan-backup/prd-004-body.md
forgeplan update PRD-005 --body @.forgeplan-backup/prd-005-body.md
forgeplan update PRD-006 --body @.forgeplan-backup/prd-006-body.md
forgeplan update PRD-007 --body @.forgeplan-backup/prd-007-body.md
forgeplan update PRD-008 --body @.forgeplan-backup/prd-008-body.md
forgeplan update RFC-001 --body @.forgeplan-backup/rfc-001-body.md
forgeplan update PROB-001 --body @.forgeplan-backup/prob-001-body.md
forgeplan update PROB-006 --body @.forgeplan-backup/prob-006-body.md
forgeplan update NOTE-004 --body @.forgeplan-backup/note-004-body.md
forgeplan update EVID-001 --body @.forgeplan-backup/evid-001-body.md
forgeplan update EVID-002 --body @.forgeplan-backup/evid-002-body.md
forgeplan update EVID-003 --body @.forgeplan-backup/evid-003-body.md
forgeplan update EVID-004 --body @.forgeplan-backup/evid-004-body.md
forgeplan update EVID-005 --body @.forgeplan-backup/evid-005-body.md

# 4. Create all links (see full list above)
# ... (copy link commands from above)

# 5. Activate artifacts that were active
forgeplan activate EPIC-001
forgeplan activate EVID-001
forgeplan activate PRD-003
forgeplan activate PRD-006
forgeplan activate PRD-007
forgeplan activate PRD-002
forgeplan activate PRD-004
forgeplan activate PRD-005
forgeplan activate PROB-006
forgeplan activate NOTE-004

# 6. Verify
forgeplan health
forgeplan list
```
