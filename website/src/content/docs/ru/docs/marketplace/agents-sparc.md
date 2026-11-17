---
title: agents-sparc
description: "SPARC для одной фичи существующей системы. Независимые гейты на Specification, Architecture и Refinement; необязательный Pseudocode проверяется на Architecture."
---

Агенты SPARC проводят одну фичу существующей системы через Specification, Pseudocode, Architecture, Refinement и Completion. Обязательные независимые гейты проходят на Specification, Architecture и Refinement. У Pseudocode нет отдельного гейта: его результат проверяют на Architecture, а для MICRO этот шаг пропускают. Полная история в CHANGELOG.md.

**Стоимость**: лёгкая | **Стабильность**: стабильная | **Версия**: 1.8.0 | **Категория**: workflow

**Целевые платформы**: claude-code

**Требования**: Claude Code с делегированием через `Task`, подключённый Forgeplan MCP и активный родительский контекст существующей системы. Полный проход использует ревьюеров из `agents-pro` и `agents-core`; для защищённого TDD внутри Refinement отдельно нужен `agents-tdd`.

## Агенты

| Агент | Что делает |
|---|---|
| `architecture` | Специалист фазы Architecture методологии SPARC. |
| `pseudocode` | Специалист фазы Pseudocode методологии SPARC. |
| `refinement` | Специалист фазы Refinement методологии SPARC. |
| `sparc-orchestrator` | Главный оркестратор конвейера фич SPARC. |
| `specification` | Специалист фазы Specification методологии SPARC. |

## Навыки

- `sparc` - Точка входа для конвейера фич SPARC - координируемый мастером пятифазный проход, который ведёт ОДНУ фичу в СУЩЕСТВУЮЩЕЙ активной системе от спецификации до завершённой, независимо верифицированной фичи (Specification → Pseudocode → Architecture → Refinement → Completion), ТРЕТИЙ экземпляр контракта подцикла AD/AID-PDLC (ADR-010 / RFC-016).

В пакет входят **5 агентов и 1 навык**, без файлов slash-команд. `/sparc` — точка входа навыка. Установка делает роли доступными, но не запускает пятифазный проход.

## Порядок работы и активация

Оркестратор передаёт накопленный контекст каждой следующей фазе и вызывает независимых ревьюеров отдельно от создателей. До активного Architecture RFC он не делегирует запись исходников и тестов. Сам `sparc-orchestrator` не пишет продуктовые файлы и не вызывает `forgeplan_activate`: он передаёт `NEEDS_ACTIVATION` основной сессии-оркестратору. Ревьюер `guardian` также не активирует артефакты.

## Хуки

**Собственного `hooks/hooks.json` нет (`hook-gate=No`).** Ограничение «сначала проектирование, потом код» обеспечивается порядком делегирования, а не fail-closed перехватом записи. Внешняя правка человеком или другим инструментом не блокируется SPARC в момент записи; её должен выявить обязательный независимый code review.

Если требуется запрет изменения замороженных тестов, Refinement делегируется в отдельно настроенный TDD-цикл. SPARC не создаёт собственный `state-*.json` и не включает TDD-защиту одной установкой.

## Модели и переносимость

`sparc-orchestrator` требует уровня A; `model: opus` — привязка для Claude Code. В другом клиенте сопоставьте `## Model tier` каждого агента с моделью своей конфигурации, при сомнении — уровнем выше. OMP, OpenCode, Codex и Gemini CLI не обязаны интерпретировать эти имена или воспроизводить `Task`, MCP и ограничения инструментов Claude Code.

## Источники

- [Манифест 1.8.0](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-sparc/.claude-plugin/plugin.json#L1-L4).
- [Пять агентов и навык sparc](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-sparc/README.md#L23-L54).
- [Уровень модели и входной контекст](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-sparc/agents/sparc-orchestrator.md#L79-L142), [точные границы отсутствующего хука](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-sparc/agents/sparc-orchestrator.md#L238-L243).

## Установка

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-sparc@ForgePlan-marketplace
```

## См. также

- [Обзор Marketplace](/ru/docs/marketplace/overview/)
- [Справочник команд](/ru/docs/marketplace/commands/)