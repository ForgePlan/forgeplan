# Eval-harness: кортеж → evidence → routing

> Ядро продуктового vision ForgeFarm и единственный настоящий пробел
> исследовательского корпуса (ни один из R1–R5 его не спроектировал; все
> оставили сокеты). Этот документ фиксирует принятую двухслойную схему.
> Решение развилки №9 → [../synthesis/02-open-decisions.md](../synthesis/02-open-decisions.md).

## Vision (формулировка владельца продукта)

Оценивать не «модель на задачах», а **связку**:

```
(model + harness + task type) → (cost + result quality + human interventions)
```

Результаты этой оценки — **evidence для оркестратора**: маршрутизация задач по моделям/уровням становится evidence-backed решением, а не вкусовщиной. Китайские локальные модели, open-source модели и внешние API — разные слои исполнения за одним контрактом, не «новая зависимость вместо старой».

## Двухслойная архитектура

### Слой 1: сырой eval-корпус (projection DB, с первого дня)

Каждый TaskRun пишет структурированную outcome-строку. Схема — в Phase 1, чтобы данные копились до появления самого eval-контура:

| Поле | Откуда берётся (сокет уже есть в дизайне) |
|---|---|
| `model`, `provider`, `capability_class` | Runtime Broker (что было выбрано) |
| `harness` / runtime adapter | ExecutorDriver (какой адаптер исполнял) |
| `task_type` / task-class | классификация задачи + `calibrate` depth |
| `tier` (T0–T3) | поле рана |
| `tokens`, `cost` | OTel GenAI-метрики / RunEvents |
| `gate_results` | Policy/Gate Engine (`gate_decisions` таблица) |
| `retry_count`, `failure_class` | fail-loop |
| `human_interventions` | HAQ entries, human_required счётчики |
| `verifier_verdict` | T3/verifier run (машиночитаемый) |
| `duration`, `wall_time` | run events |

Это операционная истина: большой объём, быстрые агрегации, живёт в Postgres. Здесь исполняется routing-запрос («какая модель сейчас лучшая для T2 rust-impl в бюджете X»).

### Слой 2: дистиллированные routing-claims (ForgePlan EvidencePacks)

Периодический distillation job агрегирует строки per **(model × task-class)** и авторит EvidencePack:

> «claude-x на T2 rust-impl: gate-pass 78%, $0.40/task, 0.2 interventions/task, n=40» — verdict: supports.

Маппинг на Structured Fields ForgePlan (семантика ложится точно):

| Поле EVID | Eval-семантика |
|---|---|
| `verdict: supports/weakens/refutes` | «модель X адекватна task-class Y» |
| `congruence_level` | совпадение harness+task-type: CL3 = тот же harness и тот же класс задач; CL1–0 = перенос из другого контекста |
| `evidence_type: benchmark` | — |
| `valid_until` | **устаревание eval'а при выходе новой версии модели** — ровно TTL/decay-семантика R_eff |

EVID линкуется `informs` к routing-policy артефакту (ADR). **Routing-таблица (`model-routing.yaml`) меняется только со ссылкой на активный, непросроченный EVID.** Тогда R_eff делает правильную weakest-link работу: routing-решение, опирающееся на просроченное/опровергнутое eval-evidence, видимо падает до 0.1 и всплывает в `blindspots`/`health`.

### Почему именно два слоя

- **min() R_eff неправилен для агрегации популяции ранов** — один плохой ран занулил бы модель. Распределительная статистика считается в DB ДО авторинга claim'а; EVID несёт уже агрегированное утверждение.
- **Git-tracked markdown не должен впитывать high-volume per-run записи** — нарушает дух ADR-003. Runs — операционная истина (DB), claims — decision-истина (git, review).
- Это ровно 4-truths split корпуса, применённый к eval'у.

## Петля целиком

```
TaskRun завершён
  → outcome-строка в projection DB           (автоматически, каждый ран)
  → [периодически] distillation job          (агрегация per model×task-class)
  → EvidencePack в ForgePlan + link к ADR    (git-reviewed, malый объём)
  → Runtime Broker обновляет model-routing.yaml
        ТОЛЬКО с цитатой на активный EVID    (audit_event, policy diff)
  → следующие раны маршрутизируются новой таблицей
  → decay: вышла новая версия модели → valid_until истекает
        → health показывает routing на просроченном evidence → re-eval
```

## v1 policy (до накопления кортежей)

Пока данных нет, маршрутизация — по `forgeplan calibrate` depth (R4): Critical/Deep → T0/T1 (дорогой reasoning); Tactical/часть Standard → T2/T3 (дешёвые модели/инструменты). Это осознанная заглушка, которую eval-петля постепенно замещает.

## Отвергнуто

- **LangSmith как eval-spine** — vendor lock, противоречит model-agnostic vision (все прогоны через одну экосистему). Допустим как опциональный sink трейсов, не как хранилище истины.
- **Отдельная eval-платформа/сервис на MVP** — сокеты уже в дизайне; отдельный сервис = ещё одна истина для reconcile.
- **Откладывание в Phase 5** (позиция R3) — воспроизводит слепое пятно корпуса; схема слоя 1 обязана попасть в Phase 1, ADR — в Phase 0.

## Flip-сигналы

- Если после ~100–200 ранов дистилляция в EVID — церемония, которую ни одно routing-решение не читает → отбросить слой 2, оставить eval в DB.
- Если routing-решения оспариваются («почему X на T1?») → это валидация слоя 2: ответ — linked EvidencePack.
- Если ручные golden-task прогоны понадобятся до накопления органических ранов → добавить `run_source: organic | golden` в схему слоя 1 (дёшево), НЕ строить отдельный бенчмарк-харнесс.
