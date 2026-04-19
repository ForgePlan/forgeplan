---
created: 2026-04-19
depth: deep
id: ADR-008
kind: adr
status: active
title: Self-describing tools + agent-skills standard + brownfield-aware init
updated: 2026-04-19
---

# ADR-008: Self-describing tools + agent-skills standard + brownfield-aware init

## Context

Telegram bug report 2026-04-19 (см. PRD-058) выявил системный провал brownfield-сценария: пользователь с 33 Obsidian-ADR запустил `forgeplan scan-import`, команда отчиталась «33 imported», но `.forgeplan/adrs/` остался пустым, body артефактов был пустым template, `forgeplan reindex` удалил всех «orphan'ов». PRD-058 закрыл три самых тяжёлых бага (проекция .md, перенос body, status-map), но оставил **три архитектурных gap'а**, которые один точечный fix не решит:

1. **Output is opaque**: forgeplan-команды не говорят агенту что делать дальше. Агент должен сам знать workflow (из CLAUDE.md), помнить команды, понимать какой skill нужен. При brownfield это проваливается — агент не знает что есть dialogue-фаза, не знает что нужен LLM-classifier, не знает куда идти за специальным skill'ом.
2. **No cross-harness distribution**: наши workflow живут в `.claude/skills/` и не видны из Cursor/Windsurf/Cline/Roo/Copilot. Пользователь не-Claude-code harness'а теряется.
3. **No brownfield-aware init**: `forgeplan init -y` создаёт пустую структуру. Не детектит существующие `requirements/`, `docs/adr/`, Obsidian vault markers. Не предлагает migration-plan. Пользователь оказывается один на один с непонятным scan-import.

В adjacent open-source landscape это уже решено: **OpenSpec** («built for brownfield not just greenfield»), **ccpm** (Claude Code PM — «Agent Skills compatible»), **BMAD-METHOD** (modular skill-пакеты + skill-validator) — все трое используют **agent-skills standard** (emerging cross-harness), делают init-time detection legacy-форматов, распространяются через marketplace. Мы изобретаем велосипед если игнорируем этот tail wind.

Связано: ADR-003 (Markdown = source of truth, LanceDB derived) — этот ADR не должен быть нарушен, наоборот — self-describing output должен помочь ADR-003 держаться (напоминать агенту «пиши markdown, не прямо в DB»).

## Decision

**Выбрано: Unified approach — self-describing output + agent-skills standard adoption + brownfield-aware init (3-в-1 coherent decision).**

Конкретные обязательства:

1. **Self-describing output convention**. Каждая forgeplan CLI-команда и MCP-tool эмитит structured hint после основного output в stderr (CLI) или в отдельном поле response (MCP): *«next: команда X, нужен skill Y, установи: Z»*. Exit code и stdout не меняются — backward compat на уровне скриптов. Feature flag `FORGEPLAN_HINTS=0` отключает.
2. **New command `forgeplan agent-manifest`**. Возвращает JSON: какие skills/plugins рекомендуются per-operation, минимальная версия forgeplan, cross-harness install-hints. Versioned schema (semver). Источник правды для self-describing hints.
3. **Context injection через `.forgeplan/config.yaml`**. Новое поле `project.context:` + `project.rules_per_kind:`. MCP-server читает при старте и инжектит в **tool description** каждого MCP-инструмента (forgeplan_new, forgeplan_validate, etc.). Агент получает project conventions автоматически с каждым tool-call, без отдельной подгрузки CLAUDE.md. Pattern адоптирован из OpenSpec.
4. **Brownfield-aware `forgeplan init`**. Детектит artifacts (requirements/, docs/adr/, Obsidian vault markers — `.obsidian/`, `type: adr` frontmatter, типовые epic-folder patterns), предлагает migration-plan pre-filled, запускает `forgeplan discover` как первый шаг. Также детектит harness markers (см. п. 5).
5. **Cross-harness skill auto-install — новый crate `forgeplan-skill-installer`**. Canonical SKILL.md внутри marketplace-пакета → per-harness adapters пишут в правильные пути:

   | Harness detected | Marker | Skill path |
   |---|---|---|
   | Claude Code | `.claude/` | `.claude/skills/<name>/SKILL.md` |
   | Cursor | `.cursor/` | `.cursor/skills/<name>/SKILL.md` |
   | Windsurf | `.windsurf/` | `.windsurf/workflows/<name>.md` |
   | Cline | `.clinerules/` | `.clinerules/workflows/<name>.md` |
   | Roo | `.roo/` | `.roo/commands/<name>.md` |
   | GitHub Copilot | `.github/prompts/` | `.github/prompts/<name>.prompt.md` |
   | agentskills.io generic | `AGENTS.md` | `.agentskills/<name>/SKILL.md` |

   Commands: `forgeplan skill {list|doctor|install|uninstall|update}`. Opt-in (требует `--yes` или interactive confirm), conflict detection (не перезаписывает user-созданные skills без `--force`).

**Это одно coherent решение**, не три разрозненных — все три части завязаны друг на друга: skill auto-install без agent-manifest не знает что устанавливать; manifest без self-describe hints нигде не surface'ится; self-describe без brownfield-init не решает onboarding; brownfield-init без cross-harness skills работает только в Claude Code.

## Alternatives Considered

| Option | Verdict | Why |
|---|---|---|
| **A. Только stderr hints** (minimum viable) | Rejected | Не решает cross-harness distribution; агент всё равно должен manually искать skill; не закрывает brownfield onboarding — user остаётся один на один с legacy файлами |
| **B. Unified (self-describe + agent-manifest + skills + brownfield-init)** — **Chosen** | Chosen | Coherent, покрывает brownfield end-to-end, валидирован adjacent proj (OpenSpec/ccpm/BMAD), inverts агент-first на tool-first flow |
| **C. Docs-only** (`docs/for-agents/brownfield.md` + skill файлы в репо, user копирует) | Rejected | Manual, zero runtime feedback, не адаптируется к harness пользователя; agent должен знать где искать docs — тот же провал что у scan-import |
| **D. Full OPSX port** (переписать forgeplan CLI под OpenSpec action-based model) | Rejected | Слишком большой scope, ломает существующих пользователей, теряем уникальное (R_eff, LanceDB, FPF) |
| **E. Only agent-manifest** (JSON-only, без cross-harness installer) | Rejected | Manifest без installer — pushes manual work на user; users без manifest-aware harness остаются за бортом |

## Consequences

### Positive
- **Zero-friction brownfield**: `forgeplan init --from-brownfield` guides user end-to-end: detect → classify → migrate → validate
- **Cross-harness reach**: out-of-the-box работа в 7+ harness'ах (Claude Code, Cursor, Windsurf, Cline, Roo, Copilot, generic agentskills.io)
- **Self-documenting surface**: агент знает следующий шаг из самого output — снижает нагрузку на CLAUDE.md
- **Context injection reliability** (OpenSpec insight): project rules всегда видны агенту с каждым MCP-call, а не «надеемся что прочитал»
- **Future-proof**: новый harness — добавляем adapter (~100 LOC) без core changes
- **Pattern legitimized**: 3 adjacent проекта (OpenSpec, ccpm, BMAD) уже прошли по этому пути — мы не piloting

### Negative (trade-offs)
- **+1 crate** (`forgeplan-skill-installer`) — +~1500 LOC поддержки
- **Stderr-hint convention** — требует careful design чтобы не ломать скрипты (убрать через `FORGEPLAN_HINTS=0`, не печатать когда stderr redirected)
- **7 harness adapters** — каждый требует рефреша при изменении upstream format
- **agent-manifest schema** — нужна governance, semver, migration guide при breaking changes
- **Config file пухнет** — `.forgeplan/config.yaml` получает `project.context:` + `project.rules_per_kind:` (mitigation: все поля optional)

### Risks
- **Stderr noise breaks CI scripts** → mitigation: env flag, respect TTY detection
- **Skill auto-install overwrite** → mitigation: conflict detection, `--dry-run` default для install, `--yes` для CI
- **agentskills.io standard immature** — может диверсифицировать → Weakest Link (см. ниже)
- **Cross-harness adapter drift** — Cursor/Windsurf меняют skill format → adapter-specific tests, CI matrix

## Invariants

- **ADR-003 holds**: все skill-файлы и migration-artifacts остаются markdown primary + LanceDB derived. `forgeplan skill install` пишет только markdown, DB не трогает.
- **Exit code semantics preserved**: 0 = success, stderr hints — informational. Скрипты, игнорирующие stderr, продолжают работать.
- **Backward compat для существующих пользователей**: текущий `forgeplan init` behavior сохранён (no breaking change). Brownfield-detection только при явном `--from-brownfield` или когда detection markers найдены И user confirms.
- **Idempotent skill install**: повторный запуск не ломает ничего (detect existing → skip или update based on version).
- **MCP tool description stability**: добавление `project.context` injection не меняет schema tool arguments, только description. Старые agents видят новое описание, но по-прежнему валидно вызывают tool.
- **Opt-in skill writes**: без явного consent (`--yes` или interactive) skill-installer ничего не пишет в `.claude/`/`.cursor/`/etc.

## Evidence Requirements

- **E1 — End-to-end brownfield migration**: реальный 44-файловый Obsidian vault (из Telegram bug report) мигрирован без data loss. Критерии: все ADR видны через `forgeplan get`, body сохранён, wikilinks разрешены в forge links, frontmatter dates преобразованы в `created_at`, status `accepted→active`.
- **E2 — Cross-harness skill install**: на testbed'е с тремя harness markers (`.claude/`, `.cursor/`, `.windsurf/`) `forgeplan skill install brownfield-pack` создаёт корректные файлы в 3/3 локациях. Установка идемпотентна.
- **E3 — Context injection proven**: MCP tool description для `forgeplan_new` содержит project.context при vault'е с заполненным `.forgeplan/config.yaml` project.context полем.
- **E4 — Backward compat**: существующие тесты 1405/1405 зелёные без изменений. Опциональная `--from-brownfield` флаг, дефолт без изменений.
- **E5 — Hints noise boundary**: скрипт, пишущий в stdout и игнорящий stderr, получает идентичный output с hints и без (`FORGEPLAN_HINTS=0`).
- **E6 — Bench**: `forgeplan init --from-brownfield` на 44-файловом vault'е завершается за < 30 сек (включая discover + dialogue skeleton).

## Valid Until

**Дата**: `2027-04-19` (12 месяцев).

**Обоснование TTL**: agent-skills standard всё ещё emerging. Год — реалистичный горизонт для: (а) наблюдать стабилизацию agentskills.io, (б) собрать feedback от brownfield cohort, (в) увидеть появление новых harnesses.

**Refresh Triggers** (оценить досрочно):
- Major version bump у agentskills.io standard или Claude Code plugin marketplace спецификации
- Breaking change в OpenSpec/ccpm/BMAD (мы основывали решение на их паттернах)
- >5 complaints из brownfield cohort на workflow friction
- Security incident в skill auto-install (CVE или реальный инцидент)

## Pre-conditions (DoR)

- [x] PRD-058 merged (scan-import ADR-003 compliance foundation)
- [ ] Decision confirmed: depth=critical, pipeline PRD→Spec→RFC→ADR (подтверждено `forgeplan route`)
- [ ] 6 ответов на open questions (Q1-Q6) зафиксированы (done — см. план brownfield сессии)
- [ ] Sprint slot v0.25 reserved

## Post-conditions (DoD)

- [ ] 6 PRDs (A-F) активированы с evidence R_eff > 0
- [ ] E2E test PASS: 44-файловый vault мигрирует без data loss (E1)
- [ ] Cross-harness install test PASS (≥3 harnesses, E2)
- [ ] `docs/operations/BROWNFIELD-MIGRATION.ru.md` опубликован
- [ ] `docs/schemas/agent-manifest.schema.json` опубликован
- [ ] `scan-import` помечен deprecated с migration path в docs
- [ ] `forgeplan skill doctor` возвращает green в workspace с установленным brownfield-pack

## Admissibility

- **NOT**: per-harness логика в `forgeplan-core`. Всё изолировано в `forgeplan-skill-installer` crate.
- **NOT**: unconditional stderr hints. Только когда `isatty(stderr)` и `FORGEPLAN_HINTS != 0`.
- **NOT**: skill auto-install без user consent. `--yes` только при explicit flag или CI mode (`CI=true` + `FORGEPLAN_AUTO_YES=true`).
- **NOT**: skills, обходящие forgeplan CLI/MCP (прямой LanceDB access из skill запрещён). Все skill operations идут через документированный surface.
- **NOT**: breaking change в tool argument schemas. Только description enrichment.
- **NOT**: разрастание `project.context` в config до мега-файла — hard cap 8 KB + warning при превышении.

## Rollback Plan

**Triggers**:
- User-reported workflow breakage >3 cases в 2 weeks
- agent-manifest schema дивергентна от agentskills.io
- CVE в skill-installer (path traversal, arbitrary write)

**Steps**:
1. Release v0.x.1 с `FORGEPLAN_HINTS=0` default (disable self-describing hints)
2. Disable `forgeplan skill install` (keep `list`/`doctor` для diagnostics)
3. Revert `init --from-brownfield` к noop, scan-import wrapper оставить как был
4. Existing skill files в `.claude/skills/` etc. — **не трогать** (user content)
5. Publish advisory + migration path в docs

**Blast Radius**: CLI output contracts меняются у users, включивших hints. Scripts parsing stderr могут сломаться — mitigation через env flag. **Medium** reputation risk mitigated clear rollback comms.

## Weakest Link

**agentskills.io standard maturity**. Стандарт emerging (OpenSpec/ccpm используют его но формальной спецификации нет). Наш cross-harness adapter может потребовать rewrite при финализации standard. R_eff floor capped на уровне CL2 (related evidence) до момента появления формальной спецификации.

**Mitigation**: (а) изолировать harness-specific logic в `forgeplan-skill-installer` — rewrite затрагивает один crate, (б) версионировать agent-manifest schema с явным breaking-change policy, (в) tracking issue на watch: когда spec финализируется, ре-оценить ADR.

## Affected Files

| File | Baseline Hash | Notes |
|------|---------------|-------|
| `crates/forgeplan-core/src/config.rs` | — | Добавить `project.context` + `project.rules_per_kind` поля |
| `crates/forgeplan-cli/src/commands/init.rs` | — | `--from-brownfield` branch + detection |
| `crates/forgeplan-cli/src/commands/mod.rs` | — | New `agent-manifest`, `skill` subcommands |
| `crates/forgeplan-mcp/src/server.rs` | — | Tool description enrichment with project.context |
| `crates/forgeplan-skill-installer/` | — | **New crate** |
| `crates/forgeplan-core/src/lifecycle/mod.rs` | — | Extended state machine (PRD-E) |
| `crates/forgeplan-core/src/artifact/kind.rs` | — | New kinds (PRD-F) |
| `docs/schemas/agent-manifest.schema.json` | — | **New file** |
| `docs/operations/BROWNFIELD-MIGRATION.ru.md` | — | **New file** |

Baseline hashes заполняются при Code-phase первого PR.

## AI Guidance

> Правила для AI-агентов при работе с этим решением.

- Любой новый CLI-command ДОЛЖЕН эмитить self-describing hint (что можно делать дальше). Без исключений.
- Любой новый MCP-tool ДОЛЖЕН иметь description с `project.context` injection (если config имеет поле).
- Новый harness supportится через adapter в `forgeplan-skill-installer`, **не** через forgeplan-core.
- При добавлении kind/state/link type — сразу же обновлять `agent-manifest.schema.json` + docs.
- skill-файлы для marketplace пишутся в canonical format (`brownfield-pack/skills/<name>/SKILL.md`) — per-harness копии генерируются installer'ом, **не** руками.
- Если задача конфликтует с этим ADR — raise explicitly, не обходить.

## Implementation Plan

### Phase 0: Foundation (Shape)
- [ ] **0.1** ADR-008 активирован с evidence
- [ ] **0.2** Epic (placeholder until created) группирующий 6 PRDs
- [ ] **0.3** 6 PRDs (A-F) созданы и validated

### Phase 1: Core (PRD-A + PRD-B foundation)
- [ ] **1.1** `forgeplan discover` команда (core, без LLM)
- [ ] **1.2** `migration-plan.json` schema + reader/writer
- [ ] **1.3** `forgeplan migrate --plan --dry-run --apply`
- [ ] **1.4** `project.context` поле в config, MCP description injection
- [ ] **1.5** `forgeplan agent-manifest` command + schema

### Phase 2: Cross-harness (PRD-C + PRD-D)
- [ ] **2.1** Crate `forgeplan-skill-installer` bootstrap
- [ ] **2.2** Claude Code adapter
- [ ] **2.3** Cursor + Windsurf adapters (priority 2)
- [ ] **2.4** Cline + Roo + Copilot + agentskills.io (priority 3)
- [ ] **2.5** `forgeplan skill {list|doctor|install|uninstall|update}`
- [ ] **2.6** `forgeplan init --from-brownfield` detection + interactive wizard

### Phase 3: Extensions (PRD-E + PRD-F)
- [ ] **3.1** Lifecycle extension: `completed`, `archived` states
- [ ] **3.2** Bidirectional supersede/deprecate
- [ ] **3.3** New kinds: `kb`, `runbook`, `postmortem`, `retrospective`, `meeting`
- [ ] **3.4** New link types: `references`, `responds_to`, `caused_by`, `discusses`

### Phase 4: Validation + Rollout
- [ ] **4.1** E2E test: 44-file Obsidian vault migration
- [ ] **4.2** Cross-harness CI matrix
- [ ] **4.3** `scan-import` deprecated в v0.25
- [ ] **4.4** `scan-import` removed в v0.27
- [ ] **4.5** BROWNFIELD-MIGRATION.ru.md + agent-manifest schema published

## Implementation Log

<!-- Wave entries заполняются по мере спринтов -->

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| PRD-058 | PRD | based_on (closed scan-import core bugs, exposed arch gaps) |
| ADR-003 | ADR | informs (MUST NOT violate: markdown = source of truth) |
| Epic (pending) | Epic | drives (to be created — Brownfield Migration Pipeline) |
| PRD-A | PRD | drives (discover + migrate core) |
| PRD-B | PRD | drives (self-description + agent-manifest + context injection) |
| PRD-C | PRD | drives (marketplace brownfield-pack) |
| PRD-D | PRD | drives (init-time detection + skill installer) |
| PRD-E | PRD | drives (state machine extension) |
| PRD-F | PRD | drives (new kinds + links) |



