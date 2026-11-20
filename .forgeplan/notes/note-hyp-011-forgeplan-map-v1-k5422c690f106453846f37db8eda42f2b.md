---
assigned_number: 71
author: null
created: 2026-11-20
depth: tactical
id: NOTE-071
kind: note
links:
- target: hyp-forgefarm-reuses-pinned-forgeplan-facts-kcfb7475c66f316ee60af886f1439a95c
  relation: based_on
parent_epic: null
predicted_number: 71
slug: note-hyp-011-forgeplan-map-v1-k5422c690f106453846f37db8eda42f2b
status: draft
tags: []
title: HYP-011 — граница forgeplan.map/v1 и закреплённых фактов
updated: 2026-11-20
valid_until: null
---

---
assigned_number: 71
created: 2026-11-20
predicted_number: 71
slug: note-hyp-011-forgeplan-map-v1-k5422c690f106453846f37db8eda42f2b
updated: 2026-11-20
---

## Наблюдение и граница проверки

Проверка HYP-011 выполнена 2026-11-21; это запись наблюдений, не валидация гипотезы.
HYP-011 остаётся draft/unvalidated; разрешения на реализацию, активацию или изменение RFC-023 нет.
Проверялись скопированные закреплённые входы, не runtime emitter, UI или вызов Farm.
Marketplace: вложенный git checkout be042515f00b54e8b3a5e7b23c681e14f0c56b47.
Использованы map.schema.json (JSON Schema Draft7) и Node map-guardian.mjs --smoke.
Web: 54ae12d6b64180919b09ca9d13654257cc4bd9b4; реально вызван validateMapDocument в Bun.
Baseline синтетический: checked-in fixture cleanDoc Guardian на указанной ревизии Marketplace.
Изменения ниже сравнивались с этим baseline, а не с реально произведённой картой.

## Матрица наблюдений

| Вариант относительно baseline | JSON Schema | Marketplace --smoke | Web |
| --- | --- | --- | --- |
| Baseline | pass | pass | pass |
| Добавлен meta.capture_probe | pass | pass | pass |
| В том же meta подставлена посторонняя placeholder revision | pass | pass | pass |
| imports → calls при неизменной форме ребра | pass | pass | pass |
| Неизвестная явная typed-link relation | pass | reject GC-4 | pass |
| schema = forgeplan.map/v2 | reject | reject GC-1 | reject |

Web сохранил capture revision и изменённую relation в принятых документах.
Принятие placeholder revision не подтверждает её принадлежность источнику или актуальность.
capture_probe — диагностический ключ эксперимента, не предложение wire schema.
--smoke пропускает GC-5/6 и XC-1/2; выводы не относятся к полной проверке Guardian.
Матрица также ничего не доказывает о фактическом производстве map.
RFC-023 C-MAP пишет forgeplan-map/v1, но реальный тег — forgeplan.map/v1 (с точкой); RFC не изменялся.
- Отдельный синтетический full check: закреплённый map-guardian.mjs --check-only --repo-root <isolated scratch git> --scan-fpl <synthetic valid scan>, пять случаев.
  Четыре детерминированных node ID, один typed-link в supplied scan и один code-dep с literal anchor: matching/foreign revision meta и imports → calls с тем же grep приняты.
  Отсутствующий literal anchor отклонён XC-2, typed-link relation вне supplied scan — XC-1; map побайтово неизменны, GC-6 pass, GC-5 WARN из-за целиком untracked .forgeplan (не pass).
  Это полный check только синтетических данных, не live emitter/UI/Farm; receipt full-guard-readback.json SHA-256: 45a05f3f13509b11338c95c58024de4a3db1e9ba2a21820df68f9c1048a55085.


## Core и Farm

Core b0c9259dc04d0c6d119844895c7a629fde6bea80, export.rs:44–86: экспортирует provenance артефактов.
Поставленный CLI constellation export --allow-partial --format jsonl вернул 4 читаемые записи источников,
692 документа, 1576 рёбер артефактов и запись failures; это не полный успешный экспорт всех источников.
Scope — live:forgeplan, источники имеют pinned generations; это не git commit snapshot.
Экспорт не предоставляет факты code-symbol, CFG или вызовов; artifact edges не равны call facts.
Farm HEAD — 2aaa29a130a464000837f8017c94231d5e9831a2.
kb/architecture/context-layer.md:133–138 содержит предложение, а не проверенный runtime-контракт.
Поиск текста символа CodeIntelProvider под tracked crates его не обнаружил.
В предложении neighbors/impact не принимают явный index/snapshot.
Вызывающий код мог бы переносить такой handle отдельно; это не проверялось.

## Вывод и следующий узкий E0

Вывод: принятие существующей presentation shape само по себе не переносит авторитетный pin/trust.
Исключено прямое использование принятой legacy map как доказательства, но не гипотеза provider.
Сохранить map как проекцию; сначала проверить перенос одного типизированного факта через явный pinned handle.
Проверить отказы для неверного, отсутствующего и устаревшего закрепления без второго анализатора.
Следующий E0: зафиксировать две ревизии, точное source-qualified ребро и ожидание потребителя.
Зафиксировать source/config/capabilities и независимого верификатора.
Запустить реальный adapter и readback вызывающего кода Farm с отказом при absent/foreign capture.
Web должен только отображать тот же источник; этот E0 здесь не выполнен, разрешения писать код нет.

## Квитанции

Локальные входы, raw logs и input.sha256 лежат в ignored .local/hyp011-map-probe-20261121/.
readback.json SHA-256: 91dbb7e5e6457d8cdc1a10b346983c17e4247cc36d810b9b711ef675a1ca05a3.
json-schema.json SHA-256 по фактическому readback: 1df8b4b5710ee3f6df2727a7269e9b0f3d875a825586926d972d494f3f441695.
Переданный для JSON Schema digest отличался; здесь записан хеш существующего файла, без его изменения.
core-export-summary.json SHA-256: 0ed0cfcc6754b21dc07ebd17f40fbcf945f5e069931d533b0119f3a4a0ee6c6e.
