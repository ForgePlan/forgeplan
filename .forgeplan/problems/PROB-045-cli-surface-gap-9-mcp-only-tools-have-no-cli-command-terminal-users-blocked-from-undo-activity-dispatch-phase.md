---
depth: standard
id: PROB-045
kind: problem
links:
- target: PRD-055
  relation: informs
status: draft
title: CLI surface gap — 9 MCP-only tools have no CLI command (terminal users blocked from undo, activity, dispatch, phase)
---

# PROB-045: CLI surface gap for v0.21-v0.24 features

## Signal

E2E test (2026-04-26) против release v0.24.0 binary показал, что **9 фич, поставленных в v0.21-v0.24, существуют только как MCP tools** — без CLI команд:

- `forgeplan_activity` / `forgeplan_activity_stats` (v0.21, PRD-055)
- `forgeplan_undo_last` / `forgeplan_restore` (v0.22, PRD-055)
- `forgeplan_phase` / `forgeplan_phase_advance` (v0.23, PRD-056)
- `forgeplan_dispatch` / `forgeplan_claim` / `forgeplan_claims` / `forgeplan_release` (v0.24, PRD-057)

Все остальные command surfaces в Forgeplan (`validate`, `score`, `health`, `link`, `new`, etc.) имеют **обе** реализации — CLI команда + MCP tool. Эти 9 — единственные исключения.

**Воспроизведение**:
```bash
$ forgeplan undo-last
error: unrecognized subcommand 'undo-last'
$ forgeplan activity
error: unrecognized subcommand 'activity'
$ forgeplan dispatch --agents 3
error: unrecognized subcommand 'dispatch'
```

## Constraints

- НЕ ломать существующее поведение MCP tools — они должны продолжать работать как сейчас
- НЕ менять ABI / JSON output форматы существующих команд
- Каждая новая CLI команда должна иметь `--json` флаг для machine consumption (как остальные команды)
- Output текстовый mode должен быть consistent с другими CLI commands (Forge tone, без emoji)
- Tests required для каждой новой команды (test-every-pub-fn rule из CLAUDE.md)

## Optimization Targets (1-3 max)

1. **CLI parity** — все 10 MCP tools (включая `forgeplan_dispatch` который имеет sub-action возможности) получают first-class CLI команды
2. **Reuse existing logic** — CLI handlers вызывают тот же core code что и MCP tools (no logic duplication)
3. **Documentation auto-regen** — после merge запустить `generate-cli-docs.mjs` чтобы website docs были актуальны

## Observation Indicators (Anti-Goodhart)

- НЕ оптимизировать "количество новых команд" — если фича не имеет смысла из терминала, лучше не добавлять CLI surface (например `dispatch` для агентов реально нужен только агентам, но human может использовать как dry-run plan)
- НЕ оптимизировать "lines of code" — handlers должны быть тонкими wrappers над core
- НЕ дублировать parsing/validation между CLI и MCP — общий код в `forgeplan-core`

## Acceptance Criteria

- [ ] `forgeplan activity [--since-hours N] [--tool X] [--status ok|err] [--limit N] [--json]` работает
- [ ] `forgeplan activity-stats [--since-hours N] [--json]` работает
- [ ] `forgeplan undo-last [--within-hours N] [--json]` работает
- [ ] `forgeplan restore <ID> [--json]` работает
- [ ] `forgeplan phase <ID> [--json]` работает
- [ ] `forgeplan phase-advance <ID> --to <PHASE> [--reason "..."] [--json]` работает
- [ ] `forgeplan dispatch --agents N [--epic ID] [--kind K] [--status S] [--json]` работает
- [ ] `forgeplan claim <ID> [--agent A] [--ttl-minutes N] [--note "..."] [--json]` работает
- [ ] `forgeplan claims [--json]` работает
- [ ] `forgeplan release <ID> [--agent A] [--force] [--json]` работает
- [ ] Каждая команда имеет integration test в `tests/cli_*.rs`
- [ ] `cargo test` PASS, 0 failures
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` PASS
- [ ] `cargo fmt --check` PASS
- [ ] `forgeplan --help` shows all 10 new commands
- [ ] Website CLI docs regenerated and re-translated to RU
- [ ] PR merged to dev

## Blast Radius

- **Medium**: `crates/forgeplan-cli/src/main.rs` (Commands enum + dispatcher), `crates/forgeplan-cli/src/commands/` (10 new modules)
- **Low**: `forgeplan-core` — переиспользует существующий код, минимальные изменения если нужны
- **Low**: `forgeplan-mcp` — не трогаем, MCP tools продолжают работать
- **Medium**: `tests/` — добавляются 10 integration test files
- **Medium**: `website/src/content/docs/` — regen после merge добавит 10 EN + 10 RU pages

## Reversibility

**High** — добавление новых subcommand'ов аддитивно. Если что-то сломается — feature flag или revert PR. Existing CLI/MCP не затронуты.

---

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-055  | informs (soft-delete + undo + activity, MCP-only ship) |
| PRD-056  | informs (phase state machine, MCP-only ship) |
| PRD-057  | informs (multi-agent dispatcher, MCP-only ship) |
| PRD-070  | informs (solution: full CLI surface) |
| PRD-046  | based_on (docs catch-up triggered this finding) |

