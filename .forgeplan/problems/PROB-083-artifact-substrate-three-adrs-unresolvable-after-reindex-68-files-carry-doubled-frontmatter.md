---
depth: standard
id: PROB-083
kind: problem
last_modified_at: 2026-08-01T18:54:47.164394+00:00
last_modified_by: claude-code/2.1.220
links:
- target: EPIC-009
  relation: informs
status: draft
title: 'Artifact substrate: three ADRs unresolvable after reindex, 68 files carry doubled frontmatter'
---

# Artifact substrate: three ADRs unresolvable after reindex, 68 files carry doubled frontmatter

Дефекты **субстрата**, обнаруженные при подготовке к работе с графом. К vNext
отношения не имеют — существуют независимо и подрывают любые выводы, считаемые
по графу.

## Problem 1 — активные ADR невидимы резолверу даже после полного реиндекса

```
forgeplan get ADR-016  ->  Error: Artifact 'ADR-016' not found
forgeplan get ADR-017  ->  Error: Artifact 'ADR-017' not found
forgeplan get ADR-013  ->  Error: Artifact 'ADR-013' not found
```

Ключевое: это **не** устаревший индекс. Запущен `forgeplan scan-import`
(9 imported, 0 failed) — после него все три по-прежнему не резолвятся.

Причины разные:

- **ADR-013** — YAML-frontmatter отсутствует полностью, файл начинается с
  `# ADR-013: CI Security Gate Policy…`, статус записан прозой `**Status:** Accepted`.
  Невидимость объяснима.
- **ADR-016** и **ADR-017** — frontmatter **корректный** (`id:`, `kind: adr`,
  `status: active`). Невидимость необъяснима и является дефектом.

ADR-017 управляет LLM-провайдером, который переключён коммитом `d855abc`
(`chore(config): switch LLM provider to claude-code`). Активное решение, управляющее
текущей конфигурацией, недостижимо через резолвер.

Расхождение диск/индекс по `forgeplan health`:

| kind | файлов | в индексе | разрыв |
|---|---:|---:|---:|
| adr | 18 | 15 | −3 |
| evidence | 148 | 135 | −13 |
| problem | 77 | 74 | −3 |
| prd | 65 | 64 | −1 |
| note | 44 | 43 | −1 |

## Problem 2 — 68 артефактов несут сдвоенный frontmatter с противоречивым статусом

```
files in .forgeplan/*/ with more than one '^id: ' line  ->  68
```

Пример, `PRD-008-cli-ux-redesign-consistent-output-json-error-format.md`:

- строки 1–10 — канонический frontmatter: `status: deprecated`, `depth: tactical`
- строки 12–22 — второй, легаси-блок: `status: Draft`, `depth: standard`,
  плюс поля `author`, `created`, `updated`, `epic`, `priority`, `domain`

Парсер читает первый блок; второй остаётся в теле как мёртвый текст. Человек и
любой grep видят два взаимоисключающих статуса в одном файле.

## Problem 3 — коллизия id внутри evidence

Два файла объявляют `id: EVID-143`:

- `EVID-143-prob-073-broad-create-roundtrip-profile-13ms-p50-lancedb-commit-hot-spot.md`
- `EVID-143-id-collision-detector-prd-008-cleanup-scan-import-flags-two-md-with-one-id-...md`

Второй из них — доказательство работы **детектора коллизий id**. Живой экземпляр
дефекта, который сам же документирует.

## Problem 4 — scan-import резолвит файлы из docs/ в id артефактов (PROB-047 живой)

`forgeplan scan-import` сообщает 4 коллизии, и все — вне `.forgeplan/`:
`docs/schemas/EPIC-SCHEMA.md` → EPIC-001, `docs/methodology/PRD-RFC-ADR-FLOW.md` →
PRD-001, `docs/audit/PROB-060-*.md` → PROB-060, `docs/schemas/SPEC-SCHEMA.md` →
SPEC-001. Митигация Tier 3 под `docs/` работает частично.

Положительный контроль: **ни один** из 60 файлов пакета `docs/vnext/` в граф не
попал (проверено — 0 артефактов с путём, содержащим `vnext`).

## Problem 5 — forgeplan_contradictions не показывает явные рёбра contradicts

Созданы три ребра и записаны на диск (frontmatter PROB-082):

```yaml
links:
- target: ADR-001
  relation: contradicts
- target: ADR-009
  relation: contradicts
- target: ADR-011
  relation: contradicts
```

После этого, включая повторный `forgeplan scan-import`:

```
mcp__forgeplan__forgeplan_contradictions  ->  {"contradictions": [], ...}
```

Инструмент в поле `limitations` перечисляет только отложенные до LLM категории
(`invariant_conflict`, `glossary_divergence`, `scenario_vs_invariant`) и **не**
сообщает, что явные рёбра `contradicts` им не рассматриваются.

Для сравнения, `forgeplan anomalies` работает — 154 находки, включая
`duplicate_artifact` по DM/GLOS/HYP.

**Практическое следствие.** Расхожая рекомендация «зафиксируй коллизию ребром
`contradicts`, чтобы она стала машинно-видимой» **не достигает цели**: ребро
пишется, инструмент молчит. Коллизия остаётся видимой только при чтении файла.
Это тихий провал ровно того класса, который методология призвана исключать.

## Impact

Любое число, посчитанное по графу — «368 артефактов», blind spots, contradictions,
R_eff — считается по корпусу, из которого выпал 21 файл, включая два активных ADR.
Связи к ADR-016/ADR-017 построить нельзя, и провал будет тихим.

## Next

1. Диагностировать, почему корректный frontmatter ADR-016/017 не индексируется.
2. Выяснить, рассматривает ли `contradictions` явные рёбра; если нет — либо
   добавить, либо честно записать это в `limitations`.
3. Разрешить коллизию EVID-143 (оставить один файл).
4. Решить судьбу 68 сдвоенных frontmatter — чистка скриптом или принять, но записать.
5. ADR-013 — привести к YAML-frontmatter.

Все мутации `.forgeplan/**` — только через MCP/CLI (RED LINE #11).

