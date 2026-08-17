# T0–T3: контракт уровней исполнения

> Принятая семантика лестницы агентов. Решение развилки №3
> ([../synthesis/02-open-decisions.md](../synthesis/02-open-decisions.md)):
> нейминг T0–T3 (R3), семантика role contract (R2), T1 владеет verification,
> цикл T2↔T3 разрешён.

## Два принципа, на которых всё стоит

1. **Tier ≠ мощность модели.** Tier определяется **разрешёнными типами действий + требуемыми evidence + ценой ошибки**, а модель — лишь параметр `capability_class`, который резолвит routing policy (и который со временем начинает назначаться eval-данными, см. [../evals/eval-harness.md](../evals/eval-harness.md)). «Какая модель» меняется ежемесячно; «что разрешено и что доказывать» — стабильно.
2. **Нейминг: внутри кода/API/схем — только T0–T3.** L0–L3 несёт задокументированную семантическую коллизию (SDD-слайды: L0 = исполнители; узус автора: L0 = сильнейшие модели). L0–L3 допустимы только как человекочитаемые UI-лейблы. Rename после schema freeze дорог — поэтому T0–T3 с первого дня.

## Контракт уровней

| Tier | Роль | Делает | НЕ МОЖЕТ | Профиль модели | Параллелизм |
|---|---|---|---|---|---|
| **T0** | Strategy / Planning | routing, decomposition, планирование PRD/RFC/ADR, scope reasoning, разрешение конфликтов, dependency planning, admission package | напрямую писать production-код | лучшие reasoning-модели | низкий |
| **T1** | Design / Verification | spec drafting, design review, dependency resolution, map zone extraction, typed edge verification, **verification reasoning** | пушить merge-ready код без T3-прогона; менять policy | сильные модели | средний |
| **T2** | Implementation | код, тесты, документация, миграции — в изолированном worktree, в пределах approved scope lease | менять policy, обходить gates, редактировать критичные emitted-файлы (map.json) | средний ценовой класс | высокий |
| **T3** | Fast guard / Validate-Repair | lint/test/retry/fix, авто-review, малые rewrite-циклы, форматирование, детерминированные guardians, evidence-normalization, классификация фейлов, подготовка merge | архитектурные решения без артефакта | дешёвые/быстрые модели + детерминированные инструменты | высокий |

## Поток и циклы

```
T0 admission package
   ↓ (fail-closed gate: артефакты стабилизированы, readiness PASS)
T1 executable slices + specs
   ↓
T2 patch + tests + docs  ⇄  T3 validate/repair   ← плотный цикл разрешён
   ↓ (T3 PASS, машиночитаемый вердикт)
Merge Queue / Done
   ↓ (T3 FAIL)
Fail Loop → repair ticket в T1 (архитектурная причина) или локальный фикс в T2
```

- **Generator ≠ verifier — сквозной инвариант, не уровень:** верифицирующий ран всегда отдельный (другой контекст/агент), на каком бы tier он ни исполнялся.
- Строгая последовательная цепочка без циклов (форма R5) — отвергнута: запрещает плотные repair-циклы.
- До любого T2-рана агент обязан зарезервировать scope lease (path-globs) через Lease Manager; каждый leased run привязан ровно к одному tier (`agent_level`/`tier` — поле lease-записи).

## Привязка к риску: risk-policy.yaml (скелет из R3)

| Риск-класс | allowed_tiers | human | обязательные gates / артефакты |
|---|---|---|---|
| low | T2, T3 | нет | базовые (tests, drift clean) |
| standard | T1, T2, T3 | нет | + design/spec наличие |
| high | T0, T1, T2 | **required** | design_review + drift_clean + evidence_present |
| critical | по политике | **required** | обязательные артефакты epic+prd+spec+rfc+adr; architecture_signoff + security_signoff + evidence_present + activation_gate |

Вход риск-классификации на v1 — `forgeplan calibrate` depth (Tactical/Standard/Deep/Critical → tier-маппинг из R4), пока eval-кортежи не накопились.

## Роль человека (human-on-exception)

Четыре категории (R3): **Auto** (зелёный пайплайн, без человека) / **Human required** (high/critical риск, повторные фейлы, policy overrides) / **Human optional** (по подписке) / **«Human запрещено микроменеджить»** (низкорисковые зелёные раны — принудительно без per-diff review). Точка входа — bounded Human Attention Queue; критерий приёмки: очередь остаётся ограниченной.

## Соответствие ForgePlan-методологии

Лестница — не чужеродна, а «практическое расширение существующей state machine ForgePlan» (R4): T0 ≈ route/calibrate/ADI reasoning//smith; T1 ≈ shaping + link/graph/order; T2 ≈ coding phase в worktree; T3 ≈ guardian/validate/evidence-enforcement. ForgePlan lifecycle hooks оборачивают T2: readiness до кода, evidence + validate/drift после.
