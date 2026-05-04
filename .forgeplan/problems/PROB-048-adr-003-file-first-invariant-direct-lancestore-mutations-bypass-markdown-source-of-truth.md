---
depth: tactical
id: PROB-048
kind: problem
links:
- target: ADR-003
  relation: based_on
- target: PRD-073
  relation: informs
status: draft
title: ADR-003 file-first invariant — direct LanceStore mutations bypass markdown source-of-truth
---

# PROB-048: ADR-003 file-first invariant violated by direct LanceStore mutations

## Context

ADR-003 (active) declares: **markdown files в `.forgeplan/` — source of truth; LanceDB — derived, gitignored index**. Invariant в том что каждая мутация артефакта должна:

1. Записать markdown файл FIRST (frontmatter + body)
2. Sync результат в LanceDB через `forgeplan_core::projection::*`
3. Никогда напрямую не вызывать `LanceStore::create_artifact / update_* / delete_* / add_relation / delete_relation` из command handler'а

Practical consequence: fresh `git clone` + `forgeplan reindex` должен воспроизвести точно такое же workspace state как у автора. Если handler обновил LanceDB но не файл — следующий reindex на клоне молча расходится.

## Observed symptoms (sprint 2026-04-28)

Three independent skews surfaced в одной сессии:

1. **MCP `forgeplan_deprecate` обновляет только LanceDB** — file's `status:` frontmatter оставался `draft` while LanceStore получил `deprecated`. CLI впоследствии отвергал re-deprecate как "deprecated → deprecated" (lance state) хотя файл утверждал что он draft.

2. **CLI `forgeplan link EVID-092 PROB-047 --relation informs` рендерил projection только для source** (EVID-092). Target's (PROB-047) frontmatter никогда не получил соответствующий link. Health checker который ожидает наличие relation с любой стороны пометил PROB-047 как blind spot пока я не добавил explicit reverse link вручную.

3. **PROB-047 file `status: draft` while LanceDB had `status: active`** — origin unclear, possibly an MCP-driven activate which bypassed projection. Workspace `forgeplan health` показал phantom blind spot пока файл не был отредактирован вручную.

## Root cause hypothesis

`crates/forgeplan-cli/src/commands/` и `crates/forgeplan-mcp/src/server.rs` содержат ~32 прямых call sites к мутирующим `LanceStore` методам (counted 2026-04-29: 27 в CLI, 5 в MCP production paths excluding `#[cfg(test)]`). Некоторые commands оборачивают их с `projection::sync_file_to_store` + `render_projection` (CLI lifecycle commands like `deprecate.rs` — корректные examples), другие — нет (большинство MCP handlers, несколько CLI commands like `tag.rs`, `update.rs` partial coverage, etc.).

Без единого enforced flow, каждый новый handler — coin-flip: запомнил ли автор отрендерить? Когда у 30+ handlers each имеет opportunity забыть, drift — статистическая certainty.

## Impact

- **Workspace skew**: clone reproduction broken — derived state на клоне отличается от author's machine
- **Phantom health signals**: blind spots / orphans / phase mismatches которые look real но являются LanceDB drift artefacts. Тратит audit time.
- **Silent data loss risk**: если LanceDB rebuilt from files через `reindex`, мутации которых нет в files — lost. Сегодня единственное что защищает users — это что они редко rebuild
- **Methodology dilution**: ADR-003 документирует invariant; код нарушает. New contributors learn from code, not docs.

## Mitigations (PRD-073 will execute)

1. **Regression guard test** (DONE in PR closing PROB-048 stage 1) — `tests/adr_003_invariant.rs` caps direct mutation count at current baseline. New PRs must not regress.

2. **Migrate MCP lifecycle handlers** — `forgeplan_deprecate` / `forgeplan_activate` / `forgeplan_supersede` / `forgeplan_renew` / `forgeplan_reopen` to use same `sync_file_to_store` + `render_projection` flow as CLI counterparts.

3. **Bidirectional link rendering** — `forgeplan link` должен re-render projections для BOTH source и target чтобы оба файла отражали новый edge корректно.

4. **Migrate remaining CLI commands** — `tag.rs`, `update.rs` body path, `score.rs`, `delete.rs`, etc. Каждая migration — measurable ratchet (lower the test baseline).

5. **Long-term**: сделать mutating `LanceStore` methods `pub(crate)` так что external consumers не могут bypass helper. Это architectural endpoint — когда reachable, все violations становятся compile-time errors.

## Acceptance Criteria for closure

- [ ] `tests/adr_003_invariant.rs` baseline reaches `CLI_BASELINE = 0` и `MCP_BASELINE = 0`
- [ ] New helper `forgeplan_core::projection::write_artifact(...)` exists, used by all mutations
- [ ] `LanceStore::{create_artifact, update_*, delete_*, add_relation, delete_relation}` are `pub(crate)`
- [ ] EVID with end-to-end measurement: clone → reindex produces identical workspace state to author's

## Related Artifacts

- ADR-003: source of truth invariant (this problem cites)
- PRD-073: full migration plan (this problem informs)
- EVID-XXX (TBD after PRD-073 completion)

