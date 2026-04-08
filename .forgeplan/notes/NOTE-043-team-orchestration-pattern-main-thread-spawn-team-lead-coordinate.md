---
depth: tactical
id: NOTE-043
kind: note
links:
- target: PROB-025
  relation: informs
- target: PRD-043
  relation: informs
status: draft
title: Team Orchestration Pattern — main-thread spawn + team-lead coordinate
---

---
id: NOTE-043
title: "Team Orchestration Pattern — main-thread spawn + team-lead coordinate"
status: Draft
created: 2026-04-07
---

# NOTE-043: Team Orchestration Pattern — main-thread spawn + team-lead coordinate

## Контекст

Sprint 13.1 (PRD-043) — первый спринт с использованием `/sprint` и TeamCreate. Обнаружено фундаментальное ограничение Agent Teams system, которое мы обошли.

## Проблема

`/sprint` методология предполагает что team-lead агент сам спавнит teammates через Agent tool. Но team-lead, заспавненный через `subagent_type="general-purpose"`, **НЕ имеет Agent tool** в своём окружении.

Реальное сообщение от team-lead-2 в Sprint 13.1:
> "BLOCKER — cannot spawn teammates. I do not have an Agent tool in my available function set. Tasks #1 and #2 are already created in the task list with full descriptions, ready to be claimed as soon as teammates exist."

## Решение — Hybrid Pattern

**Main thread spawn + team-lead coordinate:**

```
┌──────────────────────────────────────────────────────────┐
│  Main Claude Code thread (you / orchestrator)            │
│  ├─ Has Agent tool ✅                                     │
│  ├─ Creates branch                                        │
│  ├─ TeamCreate(team_name)                                 │
│  ├─ Spawns team-lead via Agent                            │
│  └─ Spawns ALL teammates via Agent (bypassing team-lead) │
└──────────────────────────────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────┐
│  Team-lead (general-purpose subagent)                    │
│  ├─ Has SendMessage, TaskCreate, TaskUpdate ✅            │
│  ├─ NO Agent tool ❌                                      │
│  ├─ Coordinates teammates via messages                    │
│  ├─ Tracks progress via TaskList                          │
│  ├─ Verifies wave completion                              │
│  └─ Sends final report                                    │
└──────────────────────────────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────┐
│  Teammates (general-purpose subagents in team)           │
│  ├─ Read tasks from TaskList                              │
│  ├─ Implement assigned files                              │
│  ├─ Run cargo test/fmt/check                              │
│  ├─ Mark TaskUpdate completed                             │
│  └─ Message team-lead with results                        │
└──────────────────────────────────────────────────────────┘
```

## Workflow

1. **Main thread** делает `/sprint` planning (research, плана с волнами)
2. **Main thread** показывает план пользователю → ждёт approval
3. **Main thread** создаёт branch
4. **Main thread** делает `TeamCreate(team_name)`
5. **Main thread** спавнит team-lead с полным планом + инструкцией "координируй, не код"
6. **Main thread** спавнит teammates Wave 1 (НЕ team-lead)
7. **Team-lead** создаёт tasks через TaskCreate, отправляет teammates SendMessage с инструкциями
8. **Teammates** работают в своих процессах, отчитываются team-lead
9. **Team-lead** ждёт всех Wave 1, шлёт notification main thread'у
10. **Main thread** видит notification, спавнит Wave 2
11. Repeat для всех волн
12. После последней волны: main thread делает evidence/activate/PR
13. **Main thread** отправляет shutdown_request всем teammates
14. **Main thread** делает TeamDelete

## Преимущества

- ✅ Работает с текущим Agent Teams system
- ✅ Чёткое разделение ответственности (orchestration vs coordination vs work)
- ✅ Параллельность сохранена (главная цель teams)
- ✅ Skills и subagent prompts работают как обычно
- ✅ TaskList — единый источник правды о прогрессе
- ✅ SendMessage — peer communication между teammates

## Недостатки

- Main thread должен ждать каждую волну (нельзя fire-and-forget весь sprint)
- Больше touch points для main thread (но они короткие — spawn → wait → spawn)
- Team-lead становится "monitor" вместо "executor"

## Применимо к

- `/sprint` — ВСЕ sprint'ы 5+ агентов
- `/team-up` — большие команды (3+)
- Любая параллельная работа с file ownership

## Не применимо к

- Маленькие задачи (1-2 агента) — main thread сам спавнит, без TeamCreate
- Hotfix sprints (Sprint 13.0 Security) — слишком overhead

## Что нужно сделать чтобы это стало автоматическим

См. PROB-025 + PRD-044 (когда зашейпим):

1. **Update `/sprint` skill** — добавить hybrid pattern как первичный
2. **Update `/team-up` skill** — то же самое
3. **CLAUDE.md addition** — раздел "Team Orchestration"
4. **Marketplace agents** — в `agents/<name>/agent.md` добавить hint про pattern
5. **Forgeplan suggestion** — `forgeplan route` для big tasks предлагает sprint pattern
6. **Templates** — sprint plan template со ссылкой на этот pattern

## Real-world validation

Sprint 13.1 (PRD-043) — 11 agents (1 lead + 10 workers), 4 waves, ~22 минуты от start до PR. 879 tests pass, 1C+4H audit findings все исправлены. **Pattern работает в production.**

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-025 | informs (problem this note describes) |
| PRD-043 | informs (real case Sprint 13.1) |
| PRD-044 | informs (when shaped — solution) |


