# ForgePlan vNext — Engineering Contract Layer

Этот каталог является каноническим design-and-execution pack для программы ForgePlan vNext.

## Цель

Довести ForgePlan до зрелого repository-native слоя инженерных контрактов, authority и проверки результатов для AI coding agents, не превращая Core в task tracker, agent runtime, worktree manager или scheduler.

## Навигация

- `architecture/` — продуктовая граница, целевая архитектура и планы Core/Web/Extensions.
- `protocol/schemas/` — design drafts ForgePlan Protocol v1.
- `governance/` — правила исполнения, порядок зависимостей и требования к issues/PR.
- `prompts/` — готовые промпты coordinator, builder и verifier.
- `issues/` — issue manifest, bodies и отдельные читаемые задачи FPV-00…FPV-15.
- `scripts/` — валидация пакета, создание GitHub Issues и выбор следующей задачи.

## Каноническая граница

ForgePlan владеет engineering intent, graph, WorkContract, authority, Evidence, verification verdict и lifecycle. Внешние системы владеют backlog, workspace, worktree, session, agent process, scheduling, CI runner и deployment.

## Запуск

```bash
python3 docs/vnext/engineering-contract-layer/scripts/validate_pack.py
python3 docs/vnext/engineering-contract-layer/scripts/create_github_issues.py
python3 docs/vnext/engineering-contract-layer/scripts/next_issue.py
```

Перед реализацией прочитать:

1. `architecture/01-PRODUCT-BOUNDARY.md`
2. `architecture/02-TARGET-ARCHITECTURE.md`
3. `governance/EXECUTION-ORDER.md`
4. `governance/AGENTS-VNEXT.md`

Первая исполнимая задача программы — `FPV-01`. Master issue `FPV-00` координирует программу, но не является самостоятельным implementation PR.
