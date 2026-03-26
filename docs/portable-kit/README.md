# Portable Kit — всё для нового проекта

> Скопируй эту папку в новый проект и получи готовую инфраструктуру: git workflow, hooks, методологию, шаблон CLAUDE.md.

## Что внутри

```
portable-kit/
├── README.md              ← ты здесь
├── CLAUDE-TEMPLATE.md     ← шаблон для CLAUDE.md (настрой под свой проект)
├── GIT-WORKFLOW.md        ← гайд по веткам и коммитам
├── METHODOLOGY.md         ← как вести проект: Shape → Code → Evidence
├── hooks/                 ← 5 enforcement hooks для .claude/hooks/
│   ├── install.sh         ← скрипт установки
│   ├── forge-safety.sh    ← блокирует опасные команды
│   ├── pr-todo-check.sh   ← проверяет P0 чекбоксы перед PR
│   ├── commit-test.sh     ← требует тесты для новых функций
│   ├── pre-code-check.sh  ← требует PRD перед кодом
│   └── pre-commit-health.sh ← предупреждает о blind spots
└── settings-template.json ← шаблон .claude/settings.json с hooks
```

## Как установить

```bash
# 1. Скопируй папку в проект
cp -r docs/portable-kit ~/Work/MyNewProject/.project-kit

# 2. Установи hooks
cd ~/Work/MyNewProject
bash .project-kit/hooks/install.sh

# 3. Скопируй CLAUDE.md
cp .project-kit/CLAUDE-TEMPLATE.md CLAUDE.md
# Отредактируй под свой проект

# 4. Готово — hooks активны, методология описана
```
