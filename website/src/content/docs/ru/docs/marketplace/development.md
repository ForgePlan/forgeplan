---
title: Разработка плагинов
description: Как создавать плагины и навыки для маркетплейса Claude Code
---

## Структура плагина

```
plugin-name/
├── .claude-plugin/plugin.json    # Обязательно: name, version, description
├── commands/                     # Слэш-команды (/command-name)
│   └── command.md               # Фронтматтер: name, description
├── agents/                       # Специализированные субагенты
│   └── agent.md                 # Фронтматтер: name, description, model
├── skills/                       # Базы знаний
│   └── skill-name/
│       ├── SKILL.md             # Роутер с навигационной таблицей
│       └── sections/            # Файлы контента (агентский RAG)
├── hooks/                        # Триггеры автоматизации
│   └── hooks.json               # События PostToolUse, PreToolUse
└── README.md
```

## Паттерн агентского RAG

Навыки используют **агентский RAG** — интеллектуальное извлечение, которое загружает только около 300 строк за раз, а не всю базу знаний. Пример использования этого паттерна в реальном мире можно найти в [плагине Forgeplan Workflow](/docs/marketplace/forgeplan-workflow/), который использует роутер `SKILL.md` для предоставления разделов методологии по запросу.

### Как это работает:

1. **SKILL.md** = роутер — сопоставляет потребности пользователя с разделами через таблицу
2. **sections/_index.md** = индекс раздела — перечисляет файлы с описаниями
3. **sections/topic.md** = контент — ~30-50 строк каждый

```markdown
<!-- SKILL.md -->
| Что вам нужно | Начать здесь |
|---|---|
| Декомпозировать систему | sections/decomposition/ |
| Оценить варианты | sections/evaluation/ |
```

Claude читает SKILL.md → выбирает нужный раздел → читает _index.md → загружает конкретный файл. Контекст остаётся сфокусированным.

## Автономные навыки (npx)

Для распространения через `npx skills add`:

```
skill-name/
├── SKILL.md              # Роутер
├── sections/
│   ├── 01-intro/_index.md
│   ├── 01-intro/overview.md
│   └── 02-usage/_index.md
└── README.md
```

Установка: `npx skills add ForgePlan/skill-name -g`

## Публикация на маркетплейсе

```bash
# 1. Скопируйте плагин на маркетплейс
cp -R my-plugin forgeplan-marketplace/plugins/

# 2. Добавьте в каталог marketplace.json
# Отредактируйте .claude-plugin/marketplace.json → plugins[]

# 3. Валидируйте
./scripts/validate-all-plugins.sh my-plugin

# 4. Создайте PR
git add -A && git commit -m "feat: add my-plugin v1.0.0"
gh pr create --base main
```

## Примеры плагинов

Просмотрите существующие плагины в репозитории [ForgePlan/marketplace](https://github.com/ForgePlan/marketplace) для ознакомления с эталонными реализациями. Плагины `forgeplan-workflow` и `dev-toolkit` демонстрируют полную структуру, включая команды, агенты, навыки и хуки.

## Рекомендации по внесению вклада

Полную информацию см. в [CONTRIBUTING.md](https://github.com/ForgePlan/marketplace/blob/main/CONTRIBUTING.md).

### Требования к PR:
- Структура плагина валидирована
- Версия обновлена в plugin.json + marketplace.json
- README с командами установки
- Отсутствие секретов или учётных данных
