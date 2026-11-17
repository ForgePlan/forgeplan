---
title: agents-github
description: "Агенты для операций с GitHub: жизненный цикл pull-request, триаж issue, автоматизация релизов, координация между репозиториями, project boards, инженерия workflow и скаффолдинг репозиториев."
---

Агенты для операций с GitHub: жизненный цикл pull-request, триаж issue, автоматизация релизов, координация между репозиториями, project boards, инженерия workflow и скаффолдинг репозиториев. Намеренно не интегрированы с Forgeplan - они работают с GitHub, а не с графом артефактов. Используйте, когда работа связана непосредственно с GitHub: открытие PR с реальным чек-листом, поиск того, что заблокировано на CI, создание релиза или распространение одного изменения на множество репозиториев. Полная история в CHANGELOG.md.

**Стоимость**: лёгкая | **Стабильность**: стабильная | **Версия**: 1.3.1 | **Категория**: developer-tools

**Цели**: claude-code

**Требования**: Claude Code; установленный и авторизованный GitHub CLI `gh` с доступом к нужным репозиториям.

## Агенты

| Агент | Что делает |
|---|---|
| `issue-manager` | Специалист по жизненному циклу GitHub issue. |
| `multi-repo-manager` | Специалист по оркестрации между репозиториями. |
| `pr-manager` | Специалист по жизненному циклу pull request. |
| `project-board-manager` | Специалист по GitHub Projects V2 board. |
| `release-manager` | Специалист по координации релизов ПО. |
| `repo-architect` | Специалист по скаффолдингу репозиториев и управлению. |
| `workflow-engineer` | Специалист по GitHub Actions CI/CD. |

В пакет входят **7 агентов**, без собственных команд и навыков. Они ограничены своими списками инструментов и не получают Forgeplan MCP для изменения графа артефактов. Такие операции передаются ролям из `agents-core` или `agents-pro`.

## Модели и переносимость

Требуемый уровень указан в `## Model tier` конкретного агента. Например, `pr-manager` требует B, а `model: sonnet` — привязка этого уровня для Claude Code, не универсальное имя модели. В OMP, OpenCode, Codex и Gemini CLI сопоставьте уровень с собственной моделью; при сомнении выбирайте более высокий уровень. Отдельно настройте доступ к `gh` и поддерживаемые клиентом инструменты: перенос текста агента не переносит автоматически разрешения.

## Хуки

Собственного `hooks/hooks.json` нет. Установка регистрирует роли, но не запускает PR, релиз или CI автоматически.

## Источники

- [Манифест 1.3.1](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-github/.claude-plugin/plugin.json#L1-L4).
- [Требование авторизации gh и 7 ролей](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-github/README.md#L11-L25), [граница Forgeplan MCP](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-github/README.md#L51-L58).
- [Уровень модели pr-manager](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-github/agents/pr-manager.md#L16-L28).


## Установка

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-github@ForgePlan-marketplace
```

## См. также

- [Обзор Marketplace](/ru/docs/marketplace/overview/)
- [Справочник команд](/ru/docs/marketplace/commands/)