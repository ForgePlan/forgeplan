---
created: 2026-04-19
depth: standard
id: PRD-059
kind: prd
links:
- target: EPIC-006
  relation: refines
- target: ADR-008
  relation: based_on
- target: PROB-028
  relation: informs
- target: PROB-027
  relation: informs
- target: PROB-022
  relation: supersedes
status: draft
title: Brownfield — discover + migrate core commands
updated: 2026-04-19
---

# PRD-059: Brownfield — discover + migrate core commands

## Problem

`forgeplan scan-import` (даже после предыдущего hotfix) — жёсткий deterministic CLI без возможности интерактивной классификации, редактируемого plan'а между фазами, resume при partial failure, resolve wikilinks в forge-граф. Brownfield миграция (33 Obsidian-ADR Telegram vault) требует разделения на **detect** (быстро, без LLM) → **classify** (LLM, skill) → **apply** (deterministic). Сейчас всё замешано в одну команду, агент не может между этапами скорректировать plan или спросить пользователя.

## Goals

1. `forgeplan discover --source <dirs>` создаёт `migration-plan.json` — inventory файлов, без LLM, без изменений в forge DB.
2. `forgeplan migrate --plan plan.json --dry-run` показывает detailed mapping per-file без записи.
3. `forgeplan migrate --plan plan.json --apply` выполняет transform+projection write atomically, rollback DB insert при projection failure.
4. `forgeplan migrate --plan plan.json --resolve-links` резолвит Obsidian `[[wikilinks]]` в forge graph links после того как все ID известны.
5. Idempotent: повторный `migrate --apply` с тем же plan = no-op или update.
6. Partial failure resilient: 5/33 fail → остальные 28 не блокируются, failed помечены в plan.

## Non-Goals

- NOT содержит semantic classification logic (отдельно в PRD-061 skill — LLM layer)
- NOT перекрывается с PRD-062 `BrownfieldScanner`: PRD-059 `discover` = library entry-point для inventory + deterministic hints; PRD-062 BrownfieldScanner = wizard shell over PRD-059 discover (UI-level). PRD-059 owns the data model (MigrationPlan).
- NOT пишет JSON в LanceDB DB напрямую — plan.json живёт в `.forgeplan/migration/` как markdown-adjacent artifact
- NOT заменяет `forgeplan new` — это отдельный поток (brownfield имеет свой bulk entry)

## Target Users

- **Brownfield adopter** — первый вызов `discover`, потом редактирует plan (через skill или вручную), потом `migrate --apply`.
- **Existing forgeplan user** — не использует эти команды, backward compat.
- **CI/automation** — `--apply --yes --plan plan.json` non-interactive для reproducible imports.

## Success Criteria / Acceptance

- **AC-1**: `forgeplan discover --source tests/fixtures/obsidian-vault-44` → `migration-plan.json` с 44 entries, корректным frontmatter schemas summary, detected formats list.
- **AC-2**: `migrate --dry-run --plan plan.json` печатает per-file: source → target forge ID, kind, status-map, projection path. Zero writes.
- **AC-3**: `migrate --apply --plan plan.json` создаёт N projection .md + N DB entries. ADR-003 invariant: для каждой DB entry есть .md.
- **AC-4**: Projection write fail → DB entry rollback. Audit проверяется unit test.
- **AC-5**: `migrate --resolve-links --plan plan.json` после apply — парсит `[[ID]]` в body, создаёт forge `references` link records.
- **AC-6**: Partial failure: 5/44 fail (malformed frontmatter и т.п.) → plan обновлён с per-file error, остальные 39 успешны.
- **AC-7**: Re-run `migrate --apply` с тем же plan → Skipped для existing, healing missing projections.
- **AC-8**: Determinism — `discover` on same input двукратно → identical migration-plan.json (byte-equal, modulo timestamps).

## Functional Requirements

- **FR-1** `forgeplan discover`: scan source dirs, parse frontmatter, collect: file path, deterministic structural kind hints (frontmatter type / path / filename heuristic — not semantic classification), frontmatter schema, size, detected wikilinks count. Output — migration-plan.json по schema.
- **FR-2** Migration plan JSON schema: fields version, files с path, size, detected kind candidates, frontmatter, kind_hint + hint_source (frontmatter|path|filename), decision, mapped id, warnings. Schema файл — docs/schemas/migration-plan.schema.json.
- **FR-3** `forgeplan migrate --dry-run`: read plan, для каждого файла с decision/kind_hint + hint_source (frontmatter|path|filename) показать destination в stdout. Без writes.
- **FR-4** `forgeplan migrate --apply`: transform loop. Per file: read source → parse frontmatter → status-map → projection write → DB upsert → update plan entry mapped_id, applied_at. Atomic per-file.
- **FR-5** `forgeplan migrate --resolve-links`: post-apply stage. Parse все body на предмет wikilink patterns, emit forge link records типа references.
- **FR-6** Idempotency: check DB entry existence before apply, skip if exists + projection exists, heal if missing projection.
- **FR-7** Partial failure mode: collect errors, continue, final report с successful и failed count.
- **FR-8** scan-import становится thin wrapper: равно discover + migrate --apply --auto-classify --yes. Deprecation warning v0.25. Removal deferred to v1.0 (3+ minor releases of deprecation window); each invocation prints exact migration command.

## Implementation Plan

### Phase 1: Core scaffolding
- [ ] **1.1** Migration plan schema + serde types в forgeplan-core migration::plan module
- [ ] **1.2** `forgeplan discover` command + inventory logic (reuse from existing scan::discovery)

### Phase 2: Migrate command
- [ ] **2.1** `forgeplan migrate --dry-run`
- [ ] **2.2** `forgeplan migrate --apply` — transform + projection + DB (reuse maybe_write_projection)
- [ ] **2.3** Rollback-on-projection-failure (перенос из предыдущего hotfix)
- [ ] **2.4** Partial failure tracking в plan

### Phase 3: Resolve-links + scan-import wrapper
- [ ] **3.1** `migrate --resolve-links` body parser + forge link records
- [ ] **3.2** scan-import deprecated wrapper

### Phase 4: Tests
- [ ] **4.1** Unit tests для plan schema, atomic apply, rollback
- [ ] **4.2** E2E test 44-файлового fixture (Obsidian vault reproduction)

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| ADR-008 | ADR | based_on |
| EPIC-006 | Epic | refines |
| ADR-003 | ADR | informs (markdown = source of truth) |








