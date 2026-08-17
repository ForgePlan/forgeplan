# Vision владельца → маппинг на KB + четыре явных требования

> 2026-07-03 владелец уточнил суть продукта. Этот документ (а) показывает,
> что из формулировки УЖЕ решено и где; (б) фиксирует четыре требования,
> которые до этого были в KB неявными, — теперь они первокласные.

## Формулировка (суть, дословно по смыслу)

Система разработки, которая **разрабатывает наше решение сама** — по уровням
(L0 и выше); вся работа ведётся по задачам, все задачи имеют ссылки на
артефакты, артефакты — через ForgePlan (или его самого); система **полностью
автономна, если такой режим включён**, внутри — ADI и FPF; люди нужны только
в местах **валидации и ревью**; система использует **CC (Claude Code), Codex
и другие агенты** — а те, в свою очередь, всё, что у них под капотом; скиллы
и агенты **подгружаются (нами или самой системой) в зависимости от проекта и
L-уровня**; всё ведётся **по методологиям по типу задач**: BMAD, TDD, SDD,
RIPER, SPARC и т.д.

## Маппинг: что уже решено и где

| Пункт формулировки | Статус в KB | Где |
|---|---|---|
| Уровни L0+ | ✅ решено — **T0–T3 role contract** (L0–L3 остаётся UI-лейблом; внутри T0–T3 из-за семантической коллизии) | `architecture/t0-t3-roles.md` |
| Работа по задачам | ✅ решено — issues как task envelope → projection DB → task state machine → leases | `architecture/state-and-truth.md` |
| Задачи ↔ артефакты по ссылкам | ✅ решено — typed cross-system links: `Issue→planned_by→PRD/RFC/ADR`, `Run→executes→Issue`, `Run→reads→Artifact[]`, `Run→produces→PR/evidence`, `VerifierRun→assesses→Run`; evidence-first close требует linked artifact | `architecture/state-and-truth.md` §6, R1 |
| Артефакты через ForgePlan | ✅ решено — ForgePlan = artifact kernel, «wrap don't replace», мутации только CLI/MCP (Artifact Gateway = ADR-001) | `synthesis/01-consensus.md` §B |
| ADI и FPF внутри | ✅ решено методологически, ⚠️ уточнено ниже (Т-1) — `forgeplan reason` (ADI) обязателен на Deep/Critical; FPF-роутинг встроен в методологическую матрицу | routing-map row 7; ForgePlan CLAUDE.md S10 |
| Человек только на валидации/ревью | ✅ решено — human-on-exception по policy: Auto / Required / Optional / «запрещено микроменеджить»; bounded HAQ | `architecture/t0-t3-roles.md`, consensus D5 |
| CC / Codex / другие агенты как исполнители | ✅ решено — pluggable runtime plane за типизированным **ExecutorDriver** (createRun/streamEvents/cancelRun/collectOutcome); адаптеры CC/OpenCode/Codex/LangGraph — сменные | `architecture/planes.md` |
| «а у них всё что под капотом» | ✅ следует из ExecutorDriver — control plane управляет раном и scope lease, НЕ микроменеджит внутренние tool-calls агента; внутренности executor'а — его дело, наружу идут типизированные RunEvents | `architecture/planes.md` |
| Методологии по типу задач | ✅ существует как артефакт, ⚠️ уточнено ниже (Т-3) | routing-map.md (fpl-skills), 14 строк |
| Скиллы/агенты по проекту и уровню | ⚠️ было неявным → **Т-4 Agent Bundle Composer** | этот документ |
| Система разрабатывает сама себя | ⚠️ было неявным → **Т-2 Self-development loop** | этот документ |
| Автономность как режим | ⚠️ было неявным → **Т-1 Autonomy profile** | этот документ |

## Четыре явных требования

### Т-1. Autonomy profile — автономность как policy-переключатель

Автономность — не свойство «вкл навсегда», а **профиль, параметризующий
risk-policy**: он определяет, какие gates требуют человека.

| Профиль | Семантика |
|---|---|
| `manual` | каждый merge/activation — через человека (режим недоверия/отладки пайплайна) |
| `assisted` | человек на high/critical gates + повторных фейлах (дефолт становления) |
| `autonomous` | человек ТОЛЬКО в точках, где risk-policy говорит `human_required` (валидация high/critical, security/architecture sign-off, HAQ) — всё остальное машина ведёт сама |

Инварианты, которые профиль НЕ может отключить (даже `autonomous`):
generator≠verifier; evidence-first close; fail-closed gate планирование→код;
ADI (`forgeplan reason`, ≥3 гипотезы) на Deep/Critical — T0/T1-ран обязан
прогнать ADI и приложить EVID до прохождения admission gate; append-only audit.
Автономность снимает человека с рутины, а не контракты с машины.
NB: у RIPER (row 4) человеческий Plan→Execute gate — load-bearing по дизайну;
в `autonomous` он остаётся человеческим (задокументированный accept-by-design).

→ В Phase 0 pack добавляется **ADR-006 autonomy profiles** (параметризация
risk-policy.yaml профилем; enforcement в Policy/Gate Engine).

### Т-2. Self-development loop — ForgeFarm разрабатывает ForgeFarm

Dogfooding — первый клиент ForgeFarm — сам ForgeFarm. Bootstrap-лестница:

1. **Phase 0–2 (ручной агентный режим):** ForgeFarm строится агентами CC по
   ForgePlan-методологии (артефакты, gates, evidence — руками/скиллами), как
   любой проект пользователя сегодня.
2. **Phase 3 (первая петля):** первые T2-раны ForgeFarm получают задачи ИЗ
   его же трекера — low-risk issues самого ForgeFarm.
3. **Self-hosting milestone (критерий, добавляется в DoD Phase 3/4):**
   *первый PR в репозиторий ForgeFarm, пройденный end-to-end раном самого
   ForgeFarm (T2 → worktree → code+tests → T3 verify → EVID → PR → merge
   по gate)*. С этого момента новые фичи ForgeFarm по умолчанию идут через
   ForgeFarm; ручной путь остаётся аварийным.
4. **Далее:** доля self-developed изменений — продуктовая метрика Control
   Room (и живой источник eval-кортежей: система непрерывно генерирует
   evidence о том, какие связки model×task-class работают).

### Т-3. Methodology Router — матрица `/smith` как машинная policy

Роутинг методологий **не изобретается**: он уже существует как зрелый артефакт
— `routing-map.md` (fpl-skills): **14 строк** «контекст → триггеры → primary/
secondary методология → dispatch-последовательность агентов → required
evidence»: BMAD (greenfield), Strangler Fig+DDD (brownfield), SPARC (фича),
RIPER (прод-баг), Tactical fast-path (hotfix), Branch-by-Abstraction+Mikado
(refactor), FPF ADI+ADR (решения), OWASP+STRIDE (security), DORA+SRE (perf),
JTBD+Lean (discovery), A3+Fishbone (техдолг), ICS (инцидент), enforced-TDD,
CANVAS (design→code). SDD присутствует как Spec Kit / SDD light path.

ForgeFarm поднимает эту матрицу из «скилла, который интерпретирует LLM» в
**машинный слой control plane**:

- классификация задачи (T0-ран) выбирает **ровно одну строку** (single-row
  rule; «methodology cocktails» — задокументированный анти-паттерн);
- строка резолвится в **playbook** (последовательность фаз/агентов + gates +
  required evidence) — исполняется Scheduler'ом как workflow, а не пересказом;
- required-evidence колонка строки = вход Policy/Gate Engine (какие EVID
  обязаны существовать до merge/activation);
- fail-closed hooks строк (bmad-gate, tdd-gate, canvas-gate) — прототипы
  runtime-политик ForgeFarm (no-code-before-plan и т.п.);
- матрица остаётся **committed intent** (versioned markdown/yaml в git);
  control plane читает её, не дублирует.

→ В Phase 0 pack добавляется **RFC `rfc-methodology-playbooks`** (формат
playbook + компиляция routing-строки в машинную форму); map-pack (Phase 2)
становится первым playbook'ом этого формата.

### Т-4. Agent Bundle Composer — комплектация под (проект × tier × методология)

Каждому рану собирается **bundle**: system prompt (per tier), скиллы, агенты,
MCP-серверы, память — в зависимости от проекта, T-уровня и выбранной
методологической строки. Прекурсоры уже есть: R5 «Memory → Bundle» edge
(bundle-agent.sh, hydrated from Memory); marketplace-паки пользователя
(agents-bmad / agents-sparc / agents-tdd / agents-canvas / fpl-skills);
per-tier prompts в scaffold R3; WSFold-паттерн «скиллы подключаются к любому
проекту композицией».

Контракт:
- **bundle-манифест = committed intent** (per project: какие паки/скиллы
  доступны какому tier'у и какой методологии); резолвинг на машине = local
  resolution (двухклассовая дисциплина из `state-and-truth.md` §8);
- Bundle Composer — компонент control plane рядом с Runtime Broker: Broker
  выбирает executor+model (`capability_class`), Composer — содержимое
  (скиллы/агенты/промпты/MCP);
- ExecutorDriver-адаптер транслирует bundle в родной формат executor'а
  (CC: plugins/skills/agents dirs; Codex: AGENTS.md/config; OpenCode: свои);
- подгрузка и человеком («подгружаем мы»), и системой («сама подгружает» —
  T0-ран запрашивает pack по методологической строке) — оба пути через один
  манифест, авто-путь пишет audit_event.

→ В Phase 3 (первый orchestration loop) bundle-манифест v1 — статический YAML;
динамический выбор — после накопления eval-данных.

## Дополнение 2026-07-03: Т-5 и Т-6 (research-верифицированы)

Владелец дозаявил два требования; оба прошли фактический web-research +
адверсариальную проверку «есть ли смысл», результаты — отдельными
документами:

- **Т-5. Skill Forge** — система сама находит скиллы из известных источников
  и сама пишет скиллы/суб-агентов по best practices →
  [`../architecture/skill-forge.md`](../architecture/skill-forge.md).
  Ключ: механический discovery реален (allowlisted-каталог источников),
  механический trust — фикция (36.8% публичных скиллов с security-flaw) →
  trust state machine `discovered→quarantined→trusted(pinned)` c гейтами
  G1–G4; authoring-pipeline из 8 стадий переиспользует существующие ассеты
  (skill-creator, plugin-dev, agent-creator, skill-reviewer,
  AGENT-AUTHORING-GUIDE); **autonomous никогда не допускает новые чужие
  скиллы** — допуск структурно supervised.
- **Т-6. Model routing** — подключение «других моделей» (Cerebras,
  OpenRouter, DeepSeek…) через настроенные CC/Codex/OpenCode →
  [`../architecture/model-routing.md`](../architecture/model-routing.md) +
  [`../architecture/executor-sessions.md`](../architecture/executor-sessions.md).
  Ключ: атомарная единица маршрутизации — **пара (harness × model)**, не
  модель; вся инъекция per-process при спавне; default-deny allowlist пар,
  промоушен только через EvidencePack; model-swap окупается только в T2/T3;
  OpenCode — канонический дом произвольных моделей, Codex — только
  GPT-семейство/Responses (+ cross-vendor T1-верификация), CC — Claude
  native support-first, vendor-endpoints (DeepSeek/GLM/Kimi) gate-behind-eval;
  Model Gateway (задеплоенный LiteLLM) — условие честности cost в eval-кортеже.

«Наша система должна уметь настраиваться» — реализуется единым паттерном
всех шести требований: **вся конфигурация = committed intent (git, PR,
evidence-gated) + local resolution (env/secrets/DB), и ни один исполняющий
компонент не имеет записи в правила собственного допуска.**

## Дельта к Phase 0 pack (поверх списка из `00-master-synthesis.md` §5)

| Добавить | Что фиксирует |
|---|---|
| **ADR-006 autonomy profiles** | manual/assisted/autonomous как параметризация risk-policy; неотключаемые инварианты |
| **RFC rfc-methodology-playbooks** | компиляция routing-map строк в машинные playbooks; single-row rule как policy |
| **ADR-007 model-routing allowlist** (Т-6) | пары (harness × model) default-deny; per-process инъекция; Model Gateway scope; eval-промоушен |
| **ADR-008 skill trust & authoring** (Т-5) | trust state machine + гейты G1–G4; authoring-pipeline; autonomous ≠ допуск чужих скиллов |
| **Self-hosting milestone** в DoD Phase 3/4 | первый PR ForgeFarm, проведённый самим ForgeFarm |
| **Bundle-манифест** (в rfc-runtime-and-lease-model или отдельным RFC в Phase 3) | комплектация (проект × tier × методология); материализация pinned-ассетов в executor-форматы |

Остальная формулировка vision дельты не требует — она уже зафиксирована
(см. таблицу маппинга выше).
