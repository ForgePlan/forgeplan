[English](GLOSSARY.md) · [Русский](GLOSSARY.ru.md)

# Глоссарий — ключевые термины Forgeplan

Справочник терминов, используемых в проекте Forgeplan. Термины сгруппированы по категориям.

---

## Артефакты

### ADR (Architecture Decision Record)
Запись архитектурного решения с контекстом, rationale, альтернативами. Фиксирует **почему** было принято решение, а не только **что** решили.

### Artifact DAG
Directed Acyclic Graph артефактов: Proposal → Specs → Design → Tasks. Из OpenSpec. Граф зависимостей без циклов — каждый артефакт знает своих родителей и потомков.

### DDR (Detailed Decision Record)
Расширенный ADR с invariants, rollback plan, valid_until, pre/post-conditions. Из Quint-code. Четырёхкомпонентная структура: Problem Frame → Decision → Rationale → Consequences.

### Delta-spec
Описывает **ТОЛЬКО** изменения: ADDED / MODIFIED / REMOVED. Предназначена для brownfield проектов, где полная спецификация избыточна. Из OpenSpec.

### Epic
Стратегическая инициатива, группирует PRD[], RFC[], ADR[]. Имеет aggregated progress — прогресс вычисляется из дочерних артефактов. Префикс: `epic-`.

### Evidence Pack
Набор доказательств: тесты, benchmarks, measurements. Тип артефакта (`evid-`). Подкрепляет решения измеримыми данными.

### ID формат
Display ID имеет вид `TYPE-NNN` (uppercase), например `PRD-001`, `EPIC-042`, `ADR-007`. Файл обычно называется `TYPE-NNN-kebab-case-title.md`. Slug (`prd-auth-system`) — отдельная identity, но её глобальная уникальность не доказана; текущий creator обычно назначает display number сразу, а nullable assignment и CI-finalization относятся к поэтапному дизайну ID-assignment.

### Note
Микро-решение без полного rationale. Интервал review в 90 дней — локальная конвенция, не автоматический default создания. Префикс: `note-`.

### PRD (Product Requirements Document)
Документ требований: **что**, **зачем**, **для кого**. Определяет scope и acceptance criteria. Не описывает реализацию. Префикс: `prd-`.

### RFC (Request for Comments)
Архитектурное предложение с фазами реализации. Описывает **как** будет реализована фича/изменение. Подлежит Adversarial Review. Префикс: `rfc-`.

### Spec (Specification)
Формальная спецификация — API contracts, data models, protocols. Описывает точные контракты между компонентами. Префикс: `spec-`.

---

## Скоринг и качество

### CL (Congruence Level)
Локальная шкала конгруэнтности Forgeplan — 0–3. Фиксированные штрафы ниже — эвристика продукта, не универсальные штрафы upstream FPF:

| Уровень | Penalty | Описание |
|---------|---------|----------|
| CL3 | 0.0 | Тот же контекст |
| CL2 | 0.1 | Похожий контекст |
| CL1 | 0.4 | Отличающийся контекст |
| CL0 | 0.9 | Противоположный контекст |

### DerivedStatus
Вычисляемый статус артефакта. **Никогда** не хранится напрямую — всегда рассчитывается на основе текущего состояния:

```
STUB → SHAPED → VALIDATED → EVIDENCED → ACTIVATED
```

### Evidence Decay
У evidence может быть дата `valid_until`. После её истечения scorer возвращает 0.1, без постепенного снижения со временем. Нет даты — нет expiry в этом расчёте. Обнаружение просрочки само по себе не меняет сохраняемый status.

### F-G-R Trust Calculus
Различайте upstream-понятия и адаптацию продукта:

- Текущий FPF (revision `34a1ae7145309f8c18b4c8864513e3456d43de71`, B.3/C.2.2/C.2.3): **F** — порядковая строгость выражения (default F0–F9), **G** — множественный ClaimScope, **R** — обоснованность конкретного утверждения. Единой универсальной числовой агрегации или фиксированных CL penalties он не задаёт.
- Forgeplan `scoring/fgr.rs`: **Formality** — доля пройденных validation rules; **Granularity** — пять проверок полноты содержания; **Reliability** объединяет R_eff, связи и свежесть. Каждая шкала 0–1; общий FGR — `cbrt(F * G * R)`. Это эвристики, не калиброванные вероятности.

### Pareto Front
Множество недоминируемых вариантов — ни один не строго хуже по **всем** измерениям одновременно. Используется при сравнении вариантов в SolutionPortfolio.

### R_eff (Effective Reliability)
```
R_eff = min(evidence_scores) с CL penalties
```
Forgeplan использует weakest-link minimum текущих допустимых evidence, а явный recursive score report — также допустимых зависимостей. Терминальные паки (`superseded`/`deprecated`) исключаются (ADR-020). Непросроченный `refutes` даёт ноль; просроченный evidence сначала получает 0.1. Recursive scorer вычитает также evidence-type modifiers (test/measurement 0, benchmark 0.1, audit 0.2) и штрафы зависимостей. Flat captured score их и рекурсию не включает, поэтому поверхности могут различаться. См. [QUALITY-GATES.ru.md](QUALITY-GATES.ru.md).

### Stepping Stone
Вариант, открывающий будущие возможности даже если не оптимален сейчас. Boolean flag в SolutionPortfolio. Учитывается при выборе варианта наряду с R_eff.

### Valid Until
TTL артефакта или evidence. По истечении срока:
- Истечение срока и refresh отделены от сохраняемого lifecycle status; в lifecycle используется `stale`, не `REFRESH_DUE`
- Evidence score → 0.1
- Требуется переоценка через RefreshReport

### Verification Gate
5-точечная проверка перед закрытием решения:

1. **Deductive consequences** — какие следствия вытекают из решения?
2. **Counter-argument** — какой самый сильный аргумент против?
3. **Self-evidence** — не является ли решение тавтологией?
4. **Tail failures** — какие маловероятные, но катастрофические сценарии возможны?
5. **WLNK challenge** — что является самым слабым звеном?

### WLNK (Weakest Link)
Что ограничивает надёжность системы. Надёжность системы ≤ min(надёжность компонентов). Каждый вариант в SolutionPortfolio **должен** иметь явно указанный WLNK.

---

## Рабочий процесс

### Adversarial Review
Локально адаптированный протокол: активно искать дефекты и обосновывать findings. Ноль подтверждённых дефектов допустим с указанием scope и непроверенных рисков; Rust adversarial checks выдают `adversarial-too-clean` с COULD. Pinned BMAD prompting просит минимум десять замечаний; native `/forge-cycle` отдельно сохраняет собственную квоту. Эти предписания не доказывают наличие дефектов; глоссарий native policy не меняет.

### Contextual Chain
Принятая конвенция передачи результатов: выход каждой фазы использовать как вход следующей. Она помогает явно передавать контекст, но не гарантирует автоматическую передачу без потерь между агентами или tools.

### Depth Calibration
4 уровня глубины документирования, определяющих набор создаваемых артефактов:

| Уровень | Сложность | Артефакты |
|---------|-----------|-----------|
| Tactical | Quick fix, 1 файл | Note или ничего |
| Standard | Фича 1–3 дня | PRD (tactical) → RFC |
| Deep | Новый модуль, 1–2 недели | PRD → Spec → RFC → ADR |
| Critical | Подсистема, кросс-команда | Epic → PRD[] → Spec[] → RFC[] → ADR[] |

### Forge Cycle
Локальная FPF-aligned конвенция цикла разработки: Observe → Route → Shape → Sprint → Build → Audit → Fix → Evidence → Commit → PR → Activate → Next. Prompt `/forge-cycle` в Claude Code направляет агента; это не Rust executor и не гарантия автоматического завершения или разрешения конфликтов.

### Forge Mode
Локальная конвенция разрешений Green/Yellow/Red, не стандарт разрешений FPF B.3. Фактический доступ зависит от настроек host и подключённых hooks; иллюстративный config в [FORGEPLAN-GUIDE.ru.md](FORGEPLAN-GUIDE.ru.md) эти контроли не устанавливает и не доказывает.

### Invariants
Что **ДОЛЖНО** выполняться всегда. Ненарушимые ограничения. Часть DDR. Если invariant нарушен — решение считается невалидным и требует пересмотра.

### Mode
Режим глубины решения, определяет требуемый уровень обоснования:

| Mode | Описание |
|------|----------|
| note | Микро-решение, не требует rationale |
| tactical | Обратимое решение, срок < 2 недель |
| standard | Большинство решений |
| deep | Необратимое, security-critical решение |

### Scope Drift
Незаметное переключение из одного типа работы в другой (тактика → стратегия). Anti-pattern в FPF B.4. Решение: Scope Lock в `/forge-cycle` Phase 0 фиксирует тип сессии и предупреждает при drift.

### Scope Lock
Механизм в `/forge-cycle` Phase 0: фиксирует SESSION_SCOPE (tactical/strategic). При переключении — предупреждение с вариантами: вернуться, bookmark, разделить сессию, осознанно переключиться.

---

## AI и интеграция

### ADI cycle
Abduction (кандидатные объяснения) → Deduction (проверяемые следствия) → Induction (сопоставление с evidence), по FPF B.5. Prompt Forgeplan просит 3+ различных альтернативы как локальную эвристику против привязки к первому варианту; parser это число не обеспечивает. Текущий upstream B.5.2 требует серьёзного конкурента или обоснования его отсутствия, не универсального порога в три варианта.

### FPF (First Principles Framework)
«Операционная система мышления». Транс-дисциплинарная архитектура для рассуждений. Источник ADI cycle, F-G-R Trust Calculus, Verification Gate и других паттернов Forgeplan.

### FPF auto-resolve
Native prompt `/forge-cycle` Build использует ADI, weakest-link и обратимость для рекомендации выбора. Его локальные stop conditions включают необратимость, одинаково слабые варианты и влияние на другие PRD/сквозные вопросы. Это инструкции агенту, не Rust resolver и не разрешение сверх текущего задания.

### LanceDB
Embedded база для структурированных таблиц и векторов. Это производный, восстанавливаемый индекс над canonical Markdown в `.forgeplan/<kind>/`, не источник истины Forgeplan. Constellation публикует неизменяемые поколения индекса; legacy и migrated layout не взаимозаменяемы.

---

## См. также

- [ARTIFACT-MODEL.md](ARTIFACT-MODEL.md) — иерархия артефактов и lifecycle
- [PRD-RFC-ADR-FLOW.md](PRD-RFC-ADR-FLOW.md) — decision tree: какой документ создать
- [VISION.md](../../VISION.md) — архитектура и data model

## Сохраняемые lifecycle statuses

Core lifecycle kinds используют этот набор статусов; у других типов lifecycle
может отличаться или отсутствовать.

```text
draft --activate--> active
active --> superseded | deprecated | stale
stale --renew--> active
stale --reopen--> deprecated + new draft
stale --deprecate--> deprecated
```

`superseded` и `deprecated` — terminal. Review, Approved, Implementing,
Implemented, Done, Archived, Resolved, Decided, Expired и Complete — описания
процесса, не дополнительные значения сохраняемого status. `review` — проверка,
не переход status. Session phases и маркеры lifecycle phase артефакта учитываются
отдельно, как и DerivedStatus (STUB→SHAPED→VALIDATED→EVIDENCED→ACTIVATED).
Его labels вычисляются из входов, а не доказывают выполнение всех прежних проверок.
