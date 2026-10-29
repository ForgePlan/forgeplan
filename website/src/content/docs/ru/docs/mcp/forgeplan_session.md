---
title: forgeplan_session
description: "Показать текущее состояние сессии методологии - фазу (idle/routing/shaping/coding/evidence/pr), активный артефакт, глубину, статус принудительного применения. Используйте это, чтобы знать, на каком этапе рабочего процесса вы находитесь."
---

Показать текущее состояние сессии методологии - в какой фазе вы находитесь (`idle` / `routing` / `shaping` / `coding` / `evidence` / `pr`), активный артефакт, управляющий сессией, маршрутизированную глубину и включено ли принудительное применение методологии. Сессия - это конечный автомат Forgeplan, который связывает Route → Shape → Code → Evidence → Activate воедино (PRD-019).

Состояние сессии не равно рекомендательной фазе артефакта (`forgeplan_phase`). Чтение состояния или классификация задачи не доказывают завершение работы и не разрешают активацию.

**Категория**: Качество

## Когда агент вызывает его

-   **Возобновить работу** - "что я делал раньше?" - сессия запоминает активный артефакт и фазу.
-   **Перед переходом** - прочитать состояние, затем вызвать `forgeplan_guard`, чтобы проверить, разрешен ли следующий переход.
-   **Проверки согласованности** - если вы думаете, что вы "coding", но сессия говорит `shaping`, что-то не так.
-   **Онбординг новых агентов** - они могут найти точку входа в рабочий процесс с помощью этого инструмента.

## Входные параметры

_Входные параметры отсутствуют. Вызовите этот инструмент с пустым объектом `{}`._

## Возвращает

Возвращает `phase`, `active_artifact`, `route_depth`, `enforced`, `phase_started_at`, `history_count`, `next_action`, `_next_action`.

Иллюстративный фрагмент:

```json
{
  "phase": "coding",
  "active_artifact": "PRD-042",
  "route_depth": "standard",
  "enforced": true,
  "history_count": 3
}
```

Без активного артефакта `active_artifact` и `route_depth` могут быть null. Ответ содержит число записей истории, а не полную историю переходов.

## Пример вызова

```json
{}
```

## Типичная последовательность

1.  `forgeplan_session` - прочитать состояние.
2.  `forgeplan_guard` с целевой фазой - проверить, разрешен ли переход.
3.  Если разрешено, выполнить работу фазы (code / evidence / PR).
4. Явно проверьте состояние сессии; не путайте автоматические отметки фаз артефакта с переходами сессии.

## Эквивалент CLI

```bash
forgeplan session
```

## См. также

-   [`forgeplan_guard`](/ru/docs/mcp/forgeplan_guard/) - гейт для фазовых переходов.
- [`forgeplan_route`](/ru/docs/mcp/forgeplan_route/) — рекомендация глубины; выбранный MCP-обработчик route не сохраняет состояние сессии
-   [Руководство по методологии](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Show current methodology session state — phase (idle/routing/shaping/coding/evidence/pr), active artifact, depth, enforcement status. Use this to know where in the workflow you are.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| — | — | — | Параметров нет. |

<details>
<summary>JSON Schema</summary>

```json
{
  "properties": {},
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
