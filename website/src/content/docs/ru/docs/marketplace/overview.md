---
title: Обзор Marketplace
description: Экосистема плагинов для Claude Code - навыки, агенты, команды, хуки
---

[ForgePlan Marketplace](https://github.com/ForgePlan/marketplace) распространяет
плагины Claude Code и переносимые skills. Другим клиентам нужны свои поддержанные
адаптеры: переносимый текст не делает команды, hooks и permissions Claude
автоматически равнозначными в Cursor, Windsurf или Codex.

## Что внутри

Marketplace поставляется с плагинами, охватывающими методологию, инструменты разработки, рассуждения, оркестрацию и UX. Точное количество варьируется от релиза к релизу - просмотрите [репозиторий GitHub](https://github.com/ForgePlan/marketplace) для получения актуального каталога.

## Плагины

| Плагин | Назначение | Основные команды | Страница |
|--------|---------|-------------|------|
| **forgeplan-workflow** | Полный цикл методологии Forgeplan | `/forge-cycle`, `/forge-audit`, навык `forgeplan-methodology` | [Подробнее](/ru/docs/marketplace/forgeplan-workflow/) |
| **fpl-skills** | Текущие skills аудита, спринтов, исследования и реализации | `/audit`, `/sprint`, `/research`, `/build` | [История dev-toolkit](/ru/docs/marketplace/dev-toolkit/) |
| **fpf** | Рассуждения по First Principles Framework | Режимы `/fpf`; отдельные `/fpf-decompose`, `/fpf-evaluate` | [Руководство по FPF](/ru/docs/guides/fpf/) |
| **forgeplan-orchestra** | Синхронизация с управлением задачами Orchestra | `/session`, `/sync` | -- |
| **laws-of-ux** | Справочник психологии UX, перечисленный в реестре | Проверьте установленный пакет: наличие команд здесь не подтверждено | -- |
| **agents-sparc** | SPARC для одной фичи в существующей системе | Навык `sparc` и агенты фаз | [Подробнее](/ru/docs/marketplace/sparc/) |

Команды описаны в [Справочнике команд](/ru/docs/marketplace/commands/); сверяйте их с установленной версией пакета.

## Установка

### Через маркетплейс плагинов клиента

```bash
# Установить конкретный плагин (изнутри сессии Claude Code)
/plugin marketplace add ForgePlan/marketplace
/plugin install fpl-skills@ForgePlan-marketplace

# Отдельный офлайн-установщик навыка запускается в shell, не как slash-команда
forgeplan setup-skill
```

### Через встроенный CLI (офлайн, без сети)

Если у вас уже установлен бинарный файл `forgeplan`, навык `/forge` можно установить без доступа к сети:

```bash
forgeplan setup-skill
```

Это записывает встроенный файл навыка в `~/.claude/skills/forge/SKILL.md`. Подробнее см. в [`forgeplan setup-skill`](/ru/docs/cli/setup-skill/).

[Каталог, проверенный на ревизии b4e8a1b9](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/.claude-plugin/marketplace.json)
помечает `dev-toolkit` deprecated в пользу `fpl-skills`, а `agents-sparc` —
stable. Запись в реестре не доказывает установку или работу пакета.

## Как работают плагины

Skills могут использовать **agentic RAG**: `SKILL.md` направляет агента
в нужные разделы. Это способ организации контента, не гарантия, что каждый
плагин автоматически читает лишь 300 строк или одинаково загружается каждым клиентом.

Подробнее о создании собственного плагина см. в [Разработка плагинов](/ru/docs/marketplace/development/).

## Узнать больше

- [Forgeplan Workflow](/ru/docs/marketplace/forgeplan-workflow/) - workflow plugin и отдельный встроенный `/forge`
- [Dev Toolkit](/ru/docs/marketplace/dev-toolkit/) - исторический пакет, заменённый `fpl-skills`
- [Справочник команд](/ru/docs/marketplace/commands/) - команды пакетов
- [Разработка плагинов](/ru/docs/marketplace/development/) - создайте свой собственный плагин
- [Репозиторий GitHub](https://github.com/ForgePlan/marketplace)
