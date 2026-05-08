---
depth: tactical
id: PROB-062
kind: problem
links:
- target: ADR-003
  relation: informs
- target: PROB-060
  relation: informs
status: active
title: forgeplan init does not create canonical .gitignore + health does not detect gitignore drift + config.yaml not split for secrets
---

# PROB-062: `.gitignore` contract drift, missing init defaults, config.yaml secrets risk

## Context

При первичной инициализации `.forgeplan/` через `forgeplan init` инструмент **не создаёт** `.forgeplan/.gitignore`. Это оставляет правила tracking на усмотрение проекта/agent'а, что приводит к 3 классам ошибок:

1. **Artifact kinds попадают в gitignore** (e.g. `memory/`, `notes/`, `state/`) — graph workspace расходится между членами команды
2. **`config.yaml` либо leak'ит secrets если tracked, либо crash'ит CLI 0.28+ если missing** (no graceful degradation)
3. **`session.yaml` (runtime per-machine state) попадает в commits** — merge conflicts на каждом PR

Эти ошибки manifested in audit external doc (см. user proposal 2026-05-07).

## Problem Statement

`forgeplan init` НЕ создаёт canonical `.forgeplan/.gitignore` (verified на CLI 0.28+). Project teams вручную составляют gitignore — без single source of truth ошибки accumulate.

`forgeplan health` не детектит gitignore drift — agent читает workspace state, но не сообщает что:
- artifact kind directory tracked но `.gitignore` его исключает
- runtime state `session.yaml` коммитится
- `config.yaml` отсутствует (CLI потом crash'ит)

`config.yaml` смешивает project layout (need shared) и LLM API keys (need secret). Без split — командное использование заставляет выбирать: leak secrets или break newcomer workflow.

## Symptoms (где эти баги уже видны)

### S-1: Artifact kind ignored → graph divergence

Agent или developer создаёт `forgeplan_new note "Architecture decision"` → `.forgeplan/notes/NOTE-NNN-...md` появляется. Если `.gitignore` содержит `notes/`, файл untracked, не попадает в commit, у коллеги после `git pull` его нет. `forgeplan list` показывает разное у разных людей.

Same pattern для `memory/`, `refresh/`, `state/`.

### S-2: `config.yaml` missing → CLI crash

CLI 0.28+ требует `config.yaml` для **любой** subcommand. Если ephemeral worktree (e.g. для `@forgeplan/web` time-travel reconstruction) не имеет config — get `os error 2` immediately.

### S-3: `session.yaml` tracked → merge conflicts

`session.yaml` записывается forgeplan'ом при каждой операции (focus task, last-activity, claim TTL). Если tracked, каждый dev генерит свой diff — merge конфликты на каждом PR.

### S-4: 18 pre-existing integration tests failing on `feat/prob-060-id-assignment` (related root cause)

Templates в `templates/{kind}/_TEMPLATE.md` partially missing YAML frontmatter → `augment_frontmatter_with_id_fields` strict requirement falls. Это same family of «init-time defaults broken» problem.

## Impact

**High blast radius** — affects every new workspace using forgeplan. Specifically:
- New contributors clone repo → different forgeplan-experience (different config, different tracked artifacts)
- Time-travel reconstruction в `@forgeplan/web` breaks без config.yaml в ephemeral worktree
- CI smoke jobs могут pass locally + fail в CI from config drift
- Multi-agent dispatch through `forgeplan_dispatch` — claims в `session.yaml`, если tracked → conflicts

## Evidence (3 errors в external proposal doc)

User шарил `.gitignore` contract document 2026-05-07. Document содержит 3 ошибки против project authoritative state (CLAUDE.md storage section + actual artifact kinds в running workspace):

### Error 1 — `memory/` указано как gitignored

Document: «memory/ — per-agent contextual memory (Hindsight-style) → gitignored»

Reality: `memory/` это first-class artifact kind в Forgeplan. CLAUDE.md storage section явно tracked. `forgeplan health` показывает `memory: 2` artifact count. Forgeplan `.forgeplan/memory/` ≠ Hindsight bank (separate system).

### Error 2 — `config.yaml` без security caveat

Document: «config.yaml — конфиг проекта, должен быть tracked».

Reality: CLAUDE.md `config.yaml ← ⚠️ gitignored (LLM keys)`. Both аргументы valid но requires SPLIT:
- `config.toml` или `forgeplan.toml` (tracked: layout, embedding model, llm provider type, decay timings)
- `secrets.yaml` или `.env` (gitignored: API keys, tokens)

### Error 3 — `discovery/` категоризация неопределённа

Document gitignore'ит `discovery/` by default. `forgeplan_discover_*` MCP tools существуют — directory может быть artifact kind. Document acknowledges ambiguity но defaults без проверки.

## Proposed Fix Scope

### F-1 — `forgeplan init` creates canonical `.forgeplan/.gitignore`

Auto-write `.forgeplan/.gitignore` с canonical content:
```gitignore
# Forgeplan derived/cache state — NOT committed
lance/                 # LanceDB vector index — derived from markdown
logs/                  # local audit/ops logs — per-machine
.lock                  # runtime mutex during reindex/validate
.fastembed_cache/      # bge-m3 embedding model — ~600 MB
session.yaml           # runtime focus/claim state — per-machine
trash/                 # soft-deleted artifacts
secrets.yaml           # LLM API keys — NEVER commit
.env                   # alternate secret store
```

**NOT gitignored** (artifact kinds + project config):
`prds/`, `rfcs/`, `adrs/`, `specs/`, `epics/`, `evidence/`, `problems/`, `solutions/`, `refresh/`, `notes/`, `memory/`, `state/`, `config.toml` (if split).

### F-2 — `forgeplan health` detects `.gitignore` drift

Add check в health:
- Warn если artifact kind directory listed в `.gitignore`
- Warn если `session.yaml` tracked (`git ls-files .forgeplan/session.yaml` non-empty)
- Warn если `config.yaml`/`config.toml` tracked AND contains API key patterns (`*_api_key:`, `*_token:`)

Output как «◈ Gitignore drift (N)» section в health output.

### F-3 — Split `config.yaml` → `config.toml` + `secrets.yaml`

Refactor config schema:
- `config.toml` (tracked): `[project]`, `[embedding]`, `[llm]` (provider type only, NOT keys), `[scoring]`, `[decay]`
- `secrets.yaml` (gitignored): LLM API keys, optional tokens

`forgeplan` CLI loads both, merges. Backward-compat: if old `config.yaml` exists, parse + warn user to migrate via `forgeplan migrate-config`.

### F-4 — Graceful degradation if config missing

CLI 0.28+ falls с os error 2 если config.yaml missing. Fix:
- Default in-memory config с reasonable defaults
- Warn о missing config
- Allow read-only operations (list, get, search) without config
- Block mutation operations (new, update, link) until config present

### F-5 — Migration command `forgeplan migrate-config`

For existing projects с monolithic `config.yaml`:
- Splits into `config.toml` + `secrets.yaml`
- Adds `secrets.yaml` to `.gitignore` automatically
- Preserves all existing settings

## Affected Files

- `crates/forgeplan-cli/src/commands/init.rs` — auto-write canonical `.gitignore`
- `crates/forgeplan-core/src/health/mod.rs` — gitignore drift detector
- `crates/forgeplan-core/src/config/` — config split (`config.toml` + `secrets.yaml`)
- `crates/forgeplan-cli/src/commands/migrate_config.rs` (new)
- `templates/.gitignore` (new — canonical template)
- `docs/operations/GITIGNORE-CONTRACT.ru.md` (new — user-facing doc)
- `CLAUDE.md` — update storage section с canonical contract

## Related Artifacts

- ADR-003: «Markdown is source of truth, Lance is derived» — informs config split rationale
- PROB-060: workspace integrity (slug naming, ID assignment) — sibling problem class
- PRD-073 + PROB-048: ADR-003 invariant enforcement — related migration discipline
- 18 pre-existing test failures in `cli_integration_test.rs` on `feat/prob-060-id-assignment` — same root cause family («init-time defaults broken») — separate PROB still to file

## Reversibility

Reversible — config split has migration command in both directions. New `.gitignore` template is project-local, can be edited.

## Suggested Routing

Standard depth (per `forgeplan route` LLM advice): PRD + RFC. RFC justified by:
- Backwards compat для existing `config.yaml` projects
- Phased rollout (avoid breaking v0.30.x users)
- Multi-step impl: init template → health check → config split → migration → docs




