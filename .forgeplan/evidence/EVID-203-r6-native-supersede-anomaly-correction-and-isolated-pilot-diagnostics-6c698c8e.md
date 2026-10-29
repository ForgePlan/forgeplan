---
assigned_number: 203
author: null
created: 2026-10-27
depth: tactical
id: EVID-203
kind: evidence
links:
- target: RFC-018
  relation: informs
parent_epic: null
predicted_number: 203
slug: evid-r6-native-supersede-anomaly-correction-and-isolated-pilot-diagnostics
status: draft
tags: []
title: R6 native supersede anomaly correction and isolated pilot diagnostics 6c698c8e
updated: 2026-10-27
valid_until: null
---

---
assigned_number: 203
created: 2026-10-27
predicted_number: 203
slug: evid-r6-native-supersede-anomaly-correction-and-isolated-pilot-diagnostics
updated: 2026-10-27
---

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3
review_verdict: CONCERNS
source_pin: 6c698c8e59e4b1fccca8544fda46c388a46bed2b4e92147055bef495496569ad

## Measurement

2026-10-27. Исполнитель: Main, openai-codex/gpt-6.1-sol. Проверка ограничена исправлением направления связи supersedes и разбором диагностик изолированного пилота ForgePlan. Это не приёмка всех прежних команд и не релизный gate.

Код: /Users/explosovebit/Work/ForgePlan-Constellation-P0. Source pin содержит 442 входа; относительно R5 изменён только crates/forgeplan-core/src/anomalies.rs. Прежний SHA файла b8208e1ecb3a996544f846b3f00aca615ee4a8a739c65e56fc7b9653bfd28aa3, текущий 43b0056da9281268b81fb8e6339a0f031d903bfdf0b0fbc549cbf50060824563. Во время сборки все входы сохранились. Снимок: /Users/explosovebit/Work/ForgePlan-Pilots.SoPuaC/anomaly-investigation/source-freeze-anomaly-r6.json.

Пилот: /Users/explosovebit/Work/ForgePlan-Pilots.SoPuaC/ForgePlan; исходные ForgePlan, ForgePlanWeb и ForgePlanMarketplace не переключались. Платные provider-вызовы и загрузки моделей не выполнялись.

## Findings

1. Исправлен подтверждённый дефект: unbacked_displacement смотрел входящую связь, хотя native supersede записывает старый пакет -> заменяющий. До исправления реальный CLI ошибочно отмечал EVID-176, уже связанный с EVID-177. Дополнены существующие проверки: входящий предшественник не считается заменой; честная замена проходит через настоящий lifecycle::supersede. Параметры CLI/MCP и направление хранимых связей не менялись. Уточнён ошибочный текст ADR-020, история уточнения сохранена.
2. Исправлены данные только в пилоте: EVID-136 сохраняет исходный CONCERNS и восемь findings, scoring verdict weakens отделён от review_verdict CONCERNS, CL3/audit не изменены. Native supersede указывает на явную постпроверку EVID-137; исторические нерешённые MEDIUM/LOW не объявлены исправленными. Две контекстные заметки связаны с ADR-003 по явно указанному в них правилу хранения.
3. Остаются две реальные high-аномалии: EVID-157 informs отсутствующие ADR-021 и PROB-086. Точные draft-записи найдены в историческом workspace-export-before.json; текущего решения об их удалении/восстановлении не найдено. Автоматическое восстановление, удаление ссылок и выдуманные обратные связи не применялись. Триггер возврата: подтверждено, что записи отсутствуют случайно, либо найдено явное решение об их дальнейшей судьбе.

## Result

| Проверка на R6 | Наблюдение |
|---|---|
| cargo fmt --all -- --check | exit 0 |
| cargo test --offline --locked -p forgeplan-core --lib --features test-helpers anomalies:: -- --test-threads=1 | 37 passed, 0 failed, 0 ignored, 2748 filtered; 1 реально запущенный test binary |
| Свежие production CLI + standalone MCP, semantic-search, без test-helpers | build exit 0; source input bytes не изменились |
| Фактический CLI anomalies --json | 3 -> 2 аномалии; остались только две исходные orphan_link, unbacked_displacement отсутствует |
| Фактический full MCP | каталог 76 tools, финальные get NOTE-057 / get ADR-020 / health / anomalies успешны; EOF/exit 0 |
| Фактический read-only MCP | каталог 9 tools; open/search/close успешны; ровно NOTE-057, partial false, failures пуст, EOF/exit 0 |
| Предупреждение EVID-136 invalid scoring verdict | отсутствует в CLI и финальных MCP stderr |
| health пилота | 449 записей, healthy; orphans 2 -> 0, at_risk 2 -> 0 |
| Полный R_eff после явной постпроверки | PRD-079 0.8, RFC-011 0.7000000000000001; EVID-136 видна в breakdown как excluded superseded |
| Байты NOTE-057 | MCP body SHA-256 равен каноническому файлу: 6293b00d52cd09d7f81283da5e9c99dacebe44cbb6de25128ed97f32c882d7ec; точный новый текст и пять старых metadata-полей сохранены |

CLI SHA-256: fb600a03e06218502a5dc020c618c55e821b96759c3a07699ed48bd083309386.
MCP SHA-256: ec79bf1c5e62c29128fe41623f05f196f9af32ee4473219ee171bf7e3d840e4a.
Бинарники: /Users/explosovebit/Work/ForgePlan-Constellation-P0/.local/constellation-verification/compatibility-20261025/quality-r6-anomaly-sole/production-semantic. Выбраны только для private pilot/bin; R5 сохранён в pilot/bin/history/r5.

## Interpretation

Наблюдения подтверждают ограниченную гипотезу: настоящий native successor больше не вызывает ложную unbacked_displacement, а входящий предшественник не прячет отсутствие замены. Технические чтения обоих MCP-профилей и сохранность изменённых данных наблюдались в реальных процессах.

healthy у health не означает отсутствия всех аномалий: отдельный anomalies продолжает честно показывать две отсутствующие цели. Ни один артефакт не активирован этой проверкой.

Неудачный wrapper-вызов случайно попал в чужую команду update и не признан native-исправлением; успешные native-вызовы записаны отдельно. Ошибка prose-only guard не признана дефектом: тот же metadata-only body prefix уже присутствовал до R6; независимый Sol подтвердил действующий контракт сохранения данных. Новый production fix для этого не делался. Потеря последнего newline в ranged read выявлена как ограничение инструмента; совпадение реальных байтов проверено native SHA-256, а не trim.

## Congruence Level Justification

CL3: точный снимок исходников, production-сборка и адресные тесты относятся к одной версии R6; реальные CLI/MCP запущены из проверенного бинарного набора на той же macOS/arm64 и одной явно заданной копии данных. Результаты R3/R4/R5 не прибавлены к числу тестов R6.

## Limits and independent review

SolCoreFinalReview и PilotEvidenceSuccessor, фактическая модель openai-codex/gpt-6.1-sol, независимо нашли 0 подтверждённых дефектов в ограниченных правках; честный static-only CONCERNS сохранён. Нет заявления о reasoning telemetry или формальном PASS.

Полные Rust matrix, clippy, code coverage/80%, внешний provider, Windows/NTFS/crash cases, G1/G2/G3 и #542 не закрыты. Default production binaries на R6 отдельно не пересобирались. В этом свежем пилоте keyword-режим; semantic-search скомпилирован, но новый runtime autoembedding/BGE-прогон не выполнялся. Git provenance для незафиксированного дерева не сертифицирован; вымышленные base/result commit SHA не подставлялись. Статус этого Evidence и RFC-018 остаётся draft.

## Related Artifacts

| Artifact | Relation |
|---|---|
| RFC-018 | informs |

## References

- ADR-020: в ходе исправления уточнено направление существующей native-связи; это не новое подтверждение всего ADR.
- EVID-202: отдельное историческое измерение R5; его результаты не заимствованы для R6.

Raw receipts, независимые reviews, before/after payloads и условия возврата: /Users/explosovebit/Work/ForgePlan-Pilots.SoPuaC/anomaly-investigation. Финальные stdio transcripts: /Users/explosovebit/Work/ForgePlan-Pilots.SoPuaC/receipts/mcp-forgeplan-anomaly-r6-final.json и mcp-forgeplan-constellation-r6-final.json.
