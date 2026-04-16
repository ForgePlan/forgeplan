---
title: Обзор Marketplace
description: Экосистема плагинов для Claude Code — навыки, агенты, команды, хуки
---

[ForgePlan Marketplace](https://github.com/ForgePlan/marketplace) — это официальная экосистема плагинов для Claude Code и совместимых ИИ-агентов для кодирования (Cursor, Windsurf, Codex и т. д.).

## Что внутри

Marketplace поставляется с плагинами, охватывающими методологию, инструменты разработки, рассуждения, оркестрацию и UX. Точное количество варьируется от релиза к релизу — просмотрите [репозиторий GitHub](https://github.com/ForgePlan/marketplace) для получения актуального каталога.

## Плагины

| Плагин | Назначение | Основные команды | Страница |
|--------|---------|-------------|------|
| **forgeplan-workflow** | Полный цикл методологии Forgeplan | `/forge`, `/forge-cycle`, `/forge-audit` | [Подробнее](/docs/marketplace/forgeplan-workflow/) |
| **dev-toolkit** | Аудит кода, спринты, исследования, сборки | `/audit`, `/sprint`, `/recall`, `/research`, `/build` | [Подробнее](/docs/marketplace/dev-toolkit/) |
| **fpf** | Рассуждения по First Principles Framework | `/fpf`, `/fpf decompose`, `/fpf evaluate` | [Руководство по FPF](/docs/guides/fpf/) |
| **forgeplan-orchestra** | Синхронизация с управлением задачами Orchestra | `/session`, `/sync` | -- |
| **laws-of-ux** | Принципы психологии UX для фронтенда | `/ux-review`, `/ux-law` | -- |
| **agents-sparc** | Методология SPARC (экспериментально) | -- | [Подробнее](/docs/marketplace/sparc/) |

Полный список слеш-команд для всех плагинов см. в [Справочнике команд](/docs/marketplace/commands/).

## Установка

### Через npx (реестр marketplace)

```bash
# Установить конкретный плагин
npx skills add ForgePlan/marketplace --plugin dev-toolkit

# Установить навык forge (методология)
npx skills add ForgePlan/marketplace --skill forge
```

### Через встроенный CLI (офлайн, без сети)

Если у вас уже установлен бинарный файл `forgeplan`, навык `/forge` можно установить без доступа к сети:

```bash
forgeplan setup-skill
```

Это записывает встроенный файл навыка в `~/.claude/skills/forge/SKILL.md`. Подробнее см. в [`forgeplan setup-skill`](/docs/cli/setup-skill/).

## Как работают плагины

Плагины используют **agentic RAG** — интеллектуальный поиск, который загружает только релевантный контент (~300 строк на шаг) вместо целых баз знаний. Файл `SKILL.md` навыка действует как маршрутизатор: он сопоставляет потребности пользователя с определёнными разделами контента, поэтому агент читает только то, что ему нужно для текущего шага.

Подробнее о создании собственного плагина см. в [Разработка плагинов](/docs/marketplace/development/).

## Узнать больше

- [Forgeplan Workflow](/docs/marketplace/forgeplan-workflow/) — интеграция методологии и команда `/forge`
- [Dev Toolkit](/docs/marketplace/dev-toolkit/) — аудит кода, спринты, исследования
- [Справочник команд](/docs/marketplace/commands/) — документация по каждой слеш-команде
- [Разработка плагинов](/docs/marketplace/development/) — создайте свой собственный плагин
- [Репозиторий GitHub](https://github.com/ForgePlan/marketplace)
