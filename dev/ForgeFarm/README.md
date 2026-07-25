# ForgeFarm — база знаний проекта

**ForgeFarm** — model-agnostic оркестратор агентной разработки: отдельный
control plane НАД ForgePlan (artifact kernel) и форджем (GitHub/Forgejo,
ingress/egress), владеющий исполнительной истиной — projection DB, task state
machine, leases/claims, policy gates, fail-loop, worktree governance, audit log
и eval-контуром «(model + harness + task type) → (cost + quality + human
interventions) → evidence → routing». Формула: **не «система агентов», а
«система контрактов вокруг агентов»**. Человек участвует по policy
(review/approve high-risk), а не ведёт пайплайн руками.

> Статус: **pre-code, стадия knowledge base** (2026-07-02). Кода нет;
> репозитория продукта нет. Следующий физический шаг — Phase 0:
> см. [synthesis/00-master-synthesis.md](synthesis/00-master-synthesis.md) §5.

## Что это за каталог

Консолидация двух источников знания:

1. **Пять внешних deep-research отчётов** (`research/R1…R5`) — архитектурный
   аудит идеи, production-стек, Rust-first control plane, интеграция с
   Forgejo, нормализация чужой SDD-схемы.
2. **WSFold/constellation-анализ** (сессия 2026-06-29…07-02, три
   адверсариально верифицированных multi-agent workflow) — граница абсорбции
   fpl core vs ForgeFarm, constellation ingestion, worktree governance
   дисциплина, prior art gastown/swarm-forge.

Из них дистиллированы: консенсус (решённые инварианты), открытые развилки
с рекомендациями и flip-сигналами, нормализованная архитектура и фазированный
план.

## Структура

```
ForgeFarm/
├── README.md                ← вы здесь: что это и как с этим работать
├── INDEX.md                 ← RAG-роутер: «какой вопрос → какой файл»
├── research/                ← первоисточники (read-only, не редактировать)
│   ├── README.md            ← провенанс, дубликаты, известные особенности
│   └── R1…R5-*.md           ← пять отчётов с осмысленными именами
├── synthesis/               ← дистилляты (главные документы)
│   ├── 00-master-synthesis.md   ← ЧТО ДЕЛАТЬ: определение, план, анти-паттерны
│   ├── 01-consensus.md          ← 31 инвариант + 15 анти-паттернов (решено)
│   ├── 02-open-decisions.md     ← 9 развилок: позиции, рекомендации, flip-сигналы
│   ├── 03-wsfold-bridge.md      ← входы из WSFold/constellation-анализа
│   ├── 04-vision-alignment.md   ← vision владельца → маппинг + autonomy/self-hosting/
│   │                               methodology-router/bundle-composer (дельта к Phase 0)
│   ├── 05-herdr-patterns.md     ← идеи из Herdr: fallback-детекция состояния, ff top,
│   │                               wait/status примитивы, persistent sessions
│   └── 06-vibe-kanban-patterns.md ← глубокий разбор VK: брать-код/дизайн/уроки,
│                                   вердикт по executors, ACP 20:1, 12 пробелов KB
├── architecture/            ← нормализованные справочники
│   ├── planes.md                ← 6 плоскостей, компоненты, ExecutorDriver
│   ├── t0-t3-roles.md           ← контракт уровней T0–T3 + risk-policy
│   ├── state-and-truth.md       ← 4 истины, state machine, leases, reconcile
│   ├── security-trust.md        ← trust boundaries, RCE, write-таксономия
│   ├── model-routing.md         ← пары (harness × model): allowlist, gateway, tiers
│   ├── executor-sessions.md     ← спавн/resume/наблюдение сессий CC/Codex/OpenCode
│   ├── skill-forge.md           ← discovery + authoring скиллов/агентов, trust G1–G4
│   ├── ui-observability.md      ← доски, метрики, layers-вид, ff top, фазовая карта
│   ├── reliability-and-k8s.md   ← data-стек, безотказность, K8s-ready путь
│   └── rust-stack.md            ← крейты (по production-стеку VK) + осознанные отличия
├── evals/
│   └── eval-harness.md          ← eval-кортеж → EvidencePack → routing (ядро vision)
└── decisions/               ← зафиксированные решения (pre-ADR формат)
    └── README.md                ← правила; мигрируют в ForgePlan ADR при создании репо
```

## Как работать с этой базой

**Человеку:** начать с [synthesis/00-master-synthesis.md](synthesis/00-master-synthesis.md) —
он самодостаточен (определение → решённое → развилки → план → анти-паттерны).
Остальное — по мере надобности через [INDEX.md](INDEX.md).

**Агенту:** правила чтения и маршрутизация — в [INDEX.md](INDEX.md).
Коротко: сначала INDEX → точечный файл; `research/` — только для сверки
с первоисточником; `synthesis/01` не оспаривать без нового evidence;
развилки решать только через `decisions/`.

## Ключевые решения одним экраном

| Вопрос | Ответ | Где подробно |
|---|---|---|
| Что такое ForgeFarm | control plane над ForgePlan+форджем, не замена | synthesis/00 §1 |
| Ядро | custom Rust control plane; фреймворки — за ExecutorDriver | synthesis/02 §1 |
| Инфраструктура MVP | local-first: docker compose + Postgres + 2 Rust-бинаря | synthesis/02 §2 |
| Лестница агентов | T0–T3 как role contract; модель = capability_class | architecture/t0-t3-roles.md |
| Истина | 4 плана: артефакты/планирование/исполнение/evidence | architecture/state-and-truth.md |
| Фордж | GitHub-first за tracker-agnostic адаптером; issues = mirror | synthesis/02 §7 |
| Eval | двухслойно: run rows (DB) → EvidencePacks (git) → routing | evals/eval-harness.md |
| Первый workload | map-pack на готовом spine, до кодящих агентов | synthesis/02 §8 |
| ForgePlan | «wrap, don't replace»; ноль изменений в core; CLI/MCP only | synthesis/01 §B |
| Память | ForgePlan-native + Hindsight; Mem0/Qdrant/LightRAG — reject | synthesis/02 §4 |
| Автономность | policy-профиль manual/assisted/autonomous; инварианты неотключаемы | synthesis/04 Т-1 |
| Self-development | ForgeFarm разрабатывает ForgeFarm; self-hosting milestone в Phase 3/4 | synthesis/04 Т-2 |
| Методологии (BMAD/TDD/SDD/RIPER/SPARC/CANVAS…) | routing-map `/smith` (14 строк) → машинные playbooks; single-row rule | synthesis/04 Т-3 |
| Скиллы/агенты per проект×tier×методология | Agent Bundle Composer; bundle-манифест = committed intent | synthesis/04 Т-4 |

## Связанные ресурсы вне каталога

- **ForgePlan** (artifact kernel): `~/Work/ForgePlan`, https://github.com/ForgePlan/forgeplan
- **Гайды по системному мышлению/артефактам/evidence**: https://forgeplan.dev/ru/guides/
- **Prior art** (изучить до постройки Worktree Governor): `../gastown/`, `../swarm-forge/`
- **Map generator spike** (стыкуется с map-pack): `../forge-understand/`
- **Исходная папка отчётов**: `../deeP-researches/` (устаревший вход; канонические копии здесь в `research/`)
- **Память сессии WSFold-анализа**: `~/.claude/projects/-Users-explosovebit-Work-ForgePlan/memory/project_wsfold_evaluation.md`
