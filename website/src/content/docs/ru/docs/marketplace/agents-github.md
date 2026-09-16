---
title: agents-github
description: "Агенты для операций с GitHub: жизненный цикл pull-request, триаж issue, автоматизация релизов, координация между репозиториями, project boards, инженерия workflow и скаффолдинг репозиториев."
---

Агенты для операций с GitHub: жизненный цикл pull-request, триаж issue, автоматизация релизов, координация между репозиториями, project boards, инженерия workflow и скаффолдинг репозиториев. Намеренно не интегрированы с Forgeplan - они работают с GitHub, а не с графом артефактов. Используйте, когда работа связана непосредственно с GitHub: открытие PR с реальным чек-листом, поиск того, что заблокировано на CI, создание релиза или распространение одного изменения на множество репозиториев. Полная история в CHANGELOG.md.

**Стоимость**: лёгкая | **Стабильность**: стабильный | **Версия**: 1.3.1 | **Категория**: developer-tools

**Цели**: claude-code

**Требует**: CLI: gh

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

## Установка

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-github@ForgePlan-marketplace
```

## См. также

- [Обзор Marketplace](/ru/docs/marketplace/overview/)
- [Справочник команд](/ru/docs/marketplace/commands/)