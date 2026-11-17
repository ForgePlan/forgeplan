---
title: forgeplan-map-pack
description: "Карта проекта map.json через восемь агентов и детерминированную структурную проверку. Семантические замечания LLM рекомендательные и не меняют PASS/BLOCKER."
---

Строит навигируемую карту проекта — зоны, узлы, рёбра и потоки — в `.forgeplan/map/map.json` через восемь агентов. Сканирование, извлечение, проверка рёбер, запись и валидация разделены по контекстам: генератор не удостоверяет собственный результат. Детерминированный `map-guardian.mjs` решает, прошла ли карта структурные проверки; это не доказательство семантической правильности всех зон и описаний. LLM-часть `map-guardian` добавляет только рекомендательные `CONCERNS`, не меняя `PASS/BLOCKER`.

**Стоимость**: средняя | **Стабильность**: beta | **Версия**: 0.24.0 | **Категория**: workflow

**Цели**: claude-code

**Требования**: Forgeplan CLI >=0.25.0 и доступный read-only Forgeplan MCP (`graph`, `list`, `get`), репозиторий с `.forgeplan/`, Claude Code с `Task`, Node.js для guardian-скрипта, Git, Bash и `jq` для локальных проверок и хука. Пакет генерирует карту локально в сессии или headless-запуске; кнопки запуска анализа в forgeplan-web нет.

## Команды

| Команда | Что делает |
|---|---|
| `/map-build` | Сканирует текущий репозиторий с включённым forgeplan и генерирует валидированный .forgeplan/map/map.json (схема forgeplan.map/v1) для представления composed-map в forgeplan-web, запуская конвейер из 8 агентов forgeplan-map-pack. |
| `/map-build-layer` | Генерирует слой детализации ОДНОЙ зоны (валидированную подкарту с собственными зонами/узлами/рёбрами/потоками) для уже построенного .forgeplan/map/map.json, по требованию и только на добавление, запуская конвейер forgeplan-map-pack в режиме SCOPED. |
| `/map-doctor` | Быстрая проверка работоспособности только для чтения уже построенного .forgeplan/map/map.json + его слоёв - запускает детерминированный guardian на каждом файле, помечает устаревшие слои (контент изменён git/docs) и находит слои-сироты, БЕЗ регенерации чего-либо. |
| `/map-refresh` | Инкрементально обновляет уже построенный .forgeplan/map/map.json - находит, что изменилось через git + docs с момента построения карты, и перестраивает ТОЛЬКО затронутые слои, оставляя свежие слои нетронутыми. |

## Агенты

| Агент | Что делает |
|---|---|
| `code-scanner` | EMITTER-агент стадии SCAN для конвейера forgeplan-map-pack: сканирует дерево исходников целевого проекта, манифесты пакетов и точки входа в сырые факты (модули, точки входа, зависимости, объявленные в манифесте) для последующего `zone-extractor`, чтобы распределить их по зонам и превратить в узлы с хешем контента. |
| `docs-scanner` | EMITTER-агент стадии SCAN для конвейера forgeplan-map-pack: сканирует прозу README/docs в сырые факты повествования для шага присоединения `description_ru` в `zone-extractor`, извлекая RU-повествование ТОЛЬКО из РЕАЛЬНОЙ прозы - никогда не выдуманное; зона/узел без источника документации вообще не получает поле повествования, полностью опущенное (правило языка SPEC-003 SS D5, правило SS E3 отброшено-не-подделано, MASTER-SPEC SS15/SS22). |
| `edge-verifier` | Агент профиля EMITTER для стадии VERIFY в forgeplan-map-pack. |
| `forgeplan-scanner` | EMITTER-агент стадии SCAN для конвейера forgeplan-map-pack: сканирует граф артефактов `.forgeplan/` через MCP ТОЛЬКО ДЛЯ ЧТЕНИЯ (`forgeplan_graph`, `forgeplan_list`, `forgeplan_get` - мутатор никогда не вызывается) в сырые факты артефактов/рёбер для последующего `zone-extractor`, чтобы распределить и превратить их. |
| `map-emitter` | Агент профиля EMITTER для стадии EMIT в forgeplan-map-pack. |
| `map-guardian` | Структурный + консультативно-семантический гейт для сгенерированного `map.json`. |
| `map-orchestrator` | Главный оркестратор для конвейера генерации `map.json` в forgeplan-map-pack. |
| `zone-extractor` | СЕРДЦЕ конвейера forgeplan-map-pack (собственные слова RFC-023). |

## Навыки

- `edge-verifier` - Алгоритм классификации доверия к рёбрам и grep-верификации для стадии VERIFY в forgeplan-map-pack (RFC-023 Proposed Direction SS1 строка реестра 6; PRD-075 FR-4; SPEC-003 SS D2 11 VALID_RELATIONS, SS D3 правило пространства имён по умолчанию typed-link/code-dep).
- `map-emitter` - Алгоритм сборки документа, предзаписной защиты инвариантов и атомарной записи для стадии EMIT в forgeplan-map-pack (RFC-023 Proposed Direction SS1 строка реестра 7 + Invariant #1; PRD-075 FR-5; SPEC-003 SS C0 контракт в одном предложении, SS C3 гейт G4).
- `zone-extractor` - Алгоритм извлечения зон/узлов/мега-узлов для стадии EXTRACT в forgeplan-map-pack (RFC-023 Proposed Direction SS1 строка реестра 5, "СЕРДЦЕ"; PRD-075 FR-3; SPEC-003 SS C1 INV-2, SS D4).

Поставляются **4 файла команд, 8 агентов и 3 внутренних навыка**. Навыки используются одноимёнными агентами; пользовательская точка входа — `/map-build`, а не прямой запуск `zone-extractor` или `map-emitter`.

## Результат и границы проверки

`map-emitter` атомарно записывает карту со статусом `proposed`. Только успешный детерминированный guardian-скрипт в обычном режиме переводит `meta.status` в `confirmed`; после этого агент должен перечитать файл, а не принять сообщение о записи за доказательство. `--smoke` не меняет статус и пропускает GC-5, GC-6, XC-1 и XC-2. В обычном режиме отсутствие контекста `--repo-root`/`--scan-fpl` также ослабляет набор выполненных проверок и должно быть отражено в отчёте.

`confirmed` относится к производной карте, а не к lifecycle-статусу артефакта Forgeplan. Пакет не создаёт EVIDENCE и не активирует PRD/RFC/ADR.

По умолчанию `/map-build` после подтверждения верхней карты автоматически строит первый уровень подходящих зон; `--no-layers` оставляет только верхнюю карту, `--layers-depth N` задаёт глубину. `/map-build-layer` строит отдельную зону по запросу, `/map-doctor` диагностирует без перегенерации, `/map-refresh` перестраивает изменившиеся слои.

## Хуки

`hooks/hooks.json` регистрирует **`PreToolUse`** для `Write|Edit|MultiEdit` → `hooks/scripts/map-emitter-gate.sh`. В отличие от TDD/BMAD/CANVAS, этот гейт не требует состояния ветки или отдельного init.

Внутри `.forgeplan/` разрешены только `map/map.json`, `map/.work/**` и `map/layers/**`. Остальные пути запрещены; отсутствие `jq`, повреждённый вход или отсутствующая библиотека дают отказ. Проверка личности писателя `map.json` выполняется лишь когда клиент передал соответствующий сигнал; разделение scratch-файлов остаётся правилом делегирования, не индивидуальным замком.

Matcher не включает `Bash`. Запись `meta.status` самим Node-скриптом — прямой файловый вызов и этим хуком не перехватывается. Поэтому гарантия единственного писателя складывается из профилей инструментов, порядка делегирования и последующей GC-5 проверки, а не из одного хука. В другом клиенте регистрация файла не означает, что такой контроль включён.

## Модели и переносимость

Требуемый уровень задан в `## Model tier` каждой роли. Например, `map-guardian` требует B: структурный вердикт даёт скрипт, а модель выполняет только рекомендательный смысловой проход. `model: sonnet` — привязка Claude Code. В OMP, OpenCode, Codex и Gemini CLI выберите модель своего уровня, при сомнении — выше; отдельно обеспечьте `Task`, MCP, запуск Node и хуков.

## Источники

- [Манифест 0.24.0](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-map-pack/.claude-plugin/plugin.json#L1-L4).
- [Состав, локальный запуск и скрипт](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-map-pack/README.md#L9-L68), [map-build и слои по умолчанию](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-map-pack/commands/map-build.md#L8-L39).
- [Структурный вердикт, smoke и рекомендательная семантика](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-map-pack/agents/map-guardian.md#L115-L210).
- [Регистрация PreToolUse](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-map-pack/hooks/hooks.json#L1-L16), [область действия и разрешённые слои](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-map-pack/hooks/scripts/map-emitter-gate.sh#L6-L147).


## Установка

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install forgeplan-map-pack@ForgePlan-marketplace
```

## См. также

- [Обзор Marketplace](/ru/docs/marketplace/overview/)
- [Справочник команд](/ru/docs/marketplace/commands/)