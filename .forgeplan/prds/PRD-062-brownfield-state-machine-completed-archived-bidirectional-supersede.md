---
depth: standard
id: PRD-062
kind: prd
links:
- target: EPIC-006
  relation: refines
- target: ADR-008
  relation: based_on
status: draft
title: Brownfield — state machine completed archived + bidirectional supersede
---

# PRD-062: Brownfield — init-time detection + multi-harness skill installer

## Problem

`forgeplan init -y` создаёт пустую структуру. Не детектит legacy-артефакты (requirements/, docs/adr/, Obsidian vault markers) — brownfield adopter не знает что делать дальше. Skills живут только в `.claude/skills/` — пользователи Cursor/Windsurf/Cline/Roo/Copilot/generic agentskills.io harness'ов не получают автоматический skill setup. Результат: zero-friction onboarding невозможен ни для brownfield, ни для non-Claude-Code users.

## Goals

1. `forgeplan init --from-brownfield` детектит legacy-артефакты, предлагает migration plan pre-filled.
2. `forgeplan init` также детектит coding-agent harness markers (7 типов) и предлагает auto-install skills.
3. Новый crate `forgeplan-skill-installer` с pluggable harness adapters.
4. Commands `forgeplan skill {list|doctor|install|uninstall|update}` для lifecycle.
5. Opt-in install (interactive confirm или `--yes` flag), conflict detection.

## Non-Goals

- NOT автоматическая установка без user consent (кроме CI flag `FORGEPLAN_AUTO_YES=true`)
- NOT overwrite user-created skills без `--force` confirm
- NOT добавляет NEW detection форматов за пределами документированных 7 harnesses + Obsidian/MADR/ADR-tools/log4brains/ad-hoc requirements/

## Target Users

- **Brownfield adopter** — `forgeplan init --from-brownfield` → detected legacy → wizard с confirm'ами → migration-plan готов для PRD-059 migrate
- **Multi-harness user** — Cursor/Windsurf/etc. detected → skill installed в правильное место без ручной работы
- **Solo maintainer** — greenfield init behavior не меняется (no false-positives detection)
- **CI** — `--yes` + env flags → non-interactive reproducible setup

## Success Criteria / Acceptance

- **AC-1**: `forgeplan init --from-brownfield` на проекте с `requirements/` + `.obsidian/` детектит 44 файлов, создаёт `migration-plan.json`, запускает `discover` автоматически.
- **AC-2**: На testbed с маркерами `.claude/` + `.cursor/` + `.windsurf/` `forgeplan skill install brownfield-pack` создаёт корректные файлы в 3/3 локациях (dry-run показывает preview).
- **AC-3**: `forgeplan skill doctor` возвращает green при консистентных skills, с diagnosis если есть stale/missing.
- **AC-4**: `forgeplan skill uninstall brownfield-pack` удаляет только created installer'ом файлы, user-modified файлы сохраняются (hash check).
- **AC-5**: Idempotent install: повторный install = skip или update based on version.
- **AC-6**: Backward compat: `forgeplan init -y` без `--from-brownfield` работает как раньше.
- **AC-7**: Новый crate `forgeplan-skill-installer` изолирован — forgeplan-core не depends на nego в runtime path (только для CLI commands).

## Functional Requirements

- **FR-1** New crate `forgeplan-skill-installer` в workspace.
- **FR-2** Trait `HarnessAdapter` с methods `detect() -> bool`, `skill_path(name) -> PathBuf`, `write(canonical_skill, path) -> Result`, `uninstall(path) -> Result`.
- **FR-3** 7 adapter implementations: ClaudeCode, Cursor, Windsurf, Cline, Roo, GitHubCopilot, AgentskillsGeneric.
- **FR-4** Brownfield detector: `BrownfieldScanner` модуль — ищет `requirements/`, `docs/adr/`, `.obsidian/`, frontmatter patterns (type:adr, layout: etc.), epic-folder conventions.
- **FR-5** `forgeplan init --from-brownfield` CLI: detect → interactive confirm → pre-fill migration-plan → optionally run `discover` immediately.
- **FR-6** Commands `forgeplan skill list|doctor|install|uninstall|update`: full lifecycle.
- **FR-7** Hash-tracking: installer пишет `.forgeplan/.skill-installs.json` — records которые файлы создал (чтобы uninstall знал что удалять без user content).
- **FR-8** Conflict detection: перед write — если файл существует и hash не совпадает с installer record → prompt `--force` или abort.
- **FR-9** MCP exposed: `forgeplan_skill_list`, `forgeplan_skill_install` MCP tools.

## Implementation Plan

### Phase 1: Crate + trait
- [ ] **1.1** `forgeplan-skill-installer` crate scaffold
- [ ] **1.2** `HarnessAdapter` trait + canonical skill type

### Phase 2: 7 adapters
- [ ] **2.1** ClaudeCode, Cursor, Windsurf (priority 1)
- [ ] **2.2** Cline, Roo, GitHubCopilot, AgentskillsGeneric (priority 2)

### Phase 3: Brownfield detector + init
- [ ] **3.1** BrownfieldScanner с Obsidian/MADR/ADR-tools/log4brains detection
- [ ] **3.2** `forgeplan init --from-brownfield` CLI
- [ ] **3.3** Interactive wizard UI

### Phase 4: Skill lifecycle commands + MCP
- [ ] **4.1** `skill {list|doctor|install|uninstall|update}` CLI
- [ ] **4.2** MCP tool exposure
- [ ] **4.3** Hash-tracking + conflict detection

### Phase 5: Tests + docs
- [ ] **5.1** Per-adapter unit tests + integration test на testbed
- [ ] **5.2** Docs: `docs/operations/SKILL-INSTALLATION.ru.md`

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| ADR-008 | ADR | based_on |
| EPIC-006 | Epic | refines |
| PRD-061 | PRD | consumes (installs brownfield-pack skill) |
| PRD-059 | PRD | informs (init --from-brownfield runs discover) |
| RFC-003 | RFC | informs (crate встраивается через trait pattern) |




