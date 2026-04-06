# Глоссарий — ключевые термины Forgeplan

Справочник терминов, используемых в проекте Forgeplan. Термины упорядочены по алфавиту (русский → латиница).

---

## Термины

### ADI cycle
Abduction (3+ гипотез) → Deduction (логическая проверка) → Induction (практическая проверка). Цикл рассуждения из FPF. Каждая фаза фильтрует и уточняет результат предыдущей.

### ADR (Architecture Decision Record)
Запись архитектурного решения с контекстом, rationale, альтернативами. Фиксирует **почему** было принято решение, а не только **что** решили.

### Adversarial Review
Протокол ревью из BMAD: ревьюер **ОБЯЗАН** найти проблемы; 0 найденных проблем = повторить ревью. Обеспечивает качество через конструктивную конфронтацию.

### Artifact DAG
Directed Acyclic Graph артефактов: Proposal → Specs → Design → Tasks. Из OpenSpec. Граф зависимостей без циклов — каждый артефакт знает своих родителей и потомков.

### CL (Congruence Level)
Уровень конгруэнтности 0–3. Показывает насколько evidence переносится между контекстами:

| Уровень | Penalty | Описание |
|---------|---------|----------|
| CL3 | 0.0 | Тот же контекст |
| CL2 | 0.1 | Похожий контекст |
| CL1 | 0.4 | Отличающийся контекст |
| CL0 | 0.9 | Противоположный контекст |

### Contextual Chain
Паттерн из BMAD: выход каждой фазы = вход следующей. Автоматическая передача контекста без потерь. Гарантирует, что ни один результат промежуточного шага не теряется.

### DDR (Detailed Decision Record)
Расширенный ADR с invariants, rollback plan, valid_until, pre/post-conditions. Из Quint-code. Четырёхкомпонентная структура: Problem Frame → Decision → Rationale → Consequences.

### Delta-spec
Описывает **ТОЛЬКО** изменения: ADDED / MODIFIED / REMOVED. Предназначена для brownfield проектов, где полная спецификация избыточна. Из OpenSpec.

### Depth Calibration
4 уровня глубины документирования, определяющих набор создаваемых артефактов:

| Уровень | Сложность | Артефакты |
|---------|-----------|-----------|
| Tactical | Quick fix, 1 файл | Note или ничего |
| Standard | Фича 1–3 дня | PRD (tactical) → RFC |
| Deep | Новый модуль, 1–2 недели | PRD → Spec → RFC → ADR |
| Critical | Подсистема, кросс-команда | Epic → PRD[] → Spec[] → RFC[] → ADR[] |

### DerivedStatus
Вычисляемый статус артефакта. **Никогда** не хранится напрямую — всегда рассчитывается на основе текущего состояния:

```
UNDERFRAMED → FRAMED → EXPLORING → COMPARED → DECIDED → APPLIED → REFRESH_DUE
```

### Epic
Стратегическая инициатива, группирует PRD[], RFC[], ADR[]. Имеет aggregated progress — прогресс вычисляется из дочерних артефактов. Префикс: `epic-`.

### ID формат
Каноничный формат: `TYPE-NNN` (uppercase). Примеры: `PRD-001`, `EPIC-042`, `ADR-007`, `PROB-003`, `SOL-001`, `SPEC-015`, `RFC-128`. Для файлов: `TYPE-NNN-kebab-case-title.md` (например `PRD-001-social-login.md`). В коде Rust используется lowercase prefix с датой: `prd-20260321-001`.

### Evidence Decay
Доказательства имеют TTL (`valid_until`). Истёкшие доказательства получают score = 0.1 (слабые, но не отсутствующие). Graduated epistemic debt — чем дольше истёк срок, тем менее надёжно.

### Evidence Pack
Набор доказательств: тесты, benchmarks, measurements. Тип артефакта (`evid-`). Подкрепляет решения измеримыми данными.

### F-G-R Trust Calculus
Три оси оценки качества знания из FPF:

- **Formality** — насколько формализовано знание
- **Granularity** — уровень детализации
- **Reliability** — надёжность источника

### Forge Cycle
Полный FPF-aligned цикл разработки: Observe → Route → Shape → Sprint → Build → Audit → Fix → Evidence → Commit → PR → Activate → Next. Реализован как `/forge-cycle` команда в Claude Code. Автоматически разрешает конфликты через ADI + WLNK + Reversibility.

### Forge Mode
Модель разрешений для AI агентов с 3 зонами доверия (FPF B.3): Green (авто — cargo, forgeplan, git read), Yellow (acceptEdits — файлы), Red (blocked — force push, rm -rf). Реализован через whitelist в settings + PreToolUse blacklist hook.

### FPF auto-resolve
Автоматическое разрешение конфликтов/выборов во время `/forge-cycle` Build фазы. Использует ADI цикл: Abduction (3 гипотезы) → Deduction (последствия каждой) → Induction (WLNK + Reversibility → выбор). Спрашивает юзера только при необратимых решениях.

### FPF (First Principles Framework)
«Операционная система мышления». Транс-дисциплинарная архитектура для рассуждений. Источник ADI cycle, F-G-R Trust Calculus, Verification Gate и других паттернов Forgeplan.

### Invariants
Что **ДОЛЖНО** выполняться всегда. Ненарушимые ограничения. Часть DDR. Если invariant нарушен — решение считается невалидным и требует пересмотра.

### LanceDB
Embedded база данных: structured tables + vector embeddings в одной DB. Source of truth для Forgeplan. Позволяет комбинировать точный поиск по полям и семантический поиск по содержимому.

### Mode
Режим глубины решения, определяет требуемый уровень обоснования:

| Mode | Описание |
|------|----------|
| note | Микро-решение, не требует rationale |
| tactical | Обратимое решение, срок < 2 недель |
| standard | Большинство решений |
| deep | Необратимое, security-critical решение |

### Note
Микро-решение. Не требует rationale. Авто-истекает через 90 дней. Самый лёгкий тип артефакта. Префикс: `note-`.

### Pareto Front
Множество недоминируемых вариантов — ни один не строго хуже по **всем** измерениям одновременно. Используется при сравнении вариантов в SolutionPortfolio.

### PRD (Product Requirements Document)
Документ требований: **что**, **зачем**, **для кого**. Определяет scope и acceptance criteria. Не описывает реализацию. Префикс: `prd-`.

### R_eff (Effective Reliability)
```
R_eff = min(evidence_scores) с CL penalties
```
Trust = weakest link, **НИКОГДА** average. Надёжность решения определяется самым слабым доказательством, а не средним.

### RFC (Request for Comments)
Архитектурное предложение с фазами реализации. Описывает **как** будет реализована фича/изменение. Подлежит Adversarial Review. Префикс: `rfc-`.

### Scope Drift
Незаметное переключение из одного типа работы в другой (тактика → стратегия). Anti-pattern в FPF B.4. Решение: Scope Lock в `/forge-cycle` Phase 0 фиксирует тип сессии и предупреждает при drift.

### Scope Lock
Механизм в `/forge-cycle` Phase 0: фиксирует SESSION_SCOPE (tactical/strategic). При переключении — предупреждение с вариантами: вернуться, bookmark, разделить сессию, осознанно переключиться.

### Spec (Specification)
Формальная спецификация — API contracts, data models, protocols. Описывает точные контракты между компонентами. Префикс: `spec-`.

### Stepping Stone
Вариант, открывающий будущие возможности даже если не оптимален сейчас. Boolean flag в SolutionPortfolio. Учитывается при выборе варианта наряду с R_eff.

### Valid Until
TTL артефакта или evidence. По истечении срока:
- Статус → `REFRESH_DUE`
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

## См. также

- [ARTIFACT-MODEL.md](ARTIFACT-MODEL.md) — иерархия артефактов и lifecycle
- [PRD-RFC-ADR-FLOW.md](PRD-RFC-ADR-FLOW.md) — decision tree: какой документ создать
- [VISION.md](../../VISION.md) — архитектура и data model

## Lifecycle статусы по типу артефакта

| Тип | Lifecycle |
|-----|-----------|
| PRD | Draft → Review → Approved → Implementing → Implemented → Closed (или Rejected) |
| Epic | Draft → Active → Done → Archived (или Cancelled) |
| Spec | Draft → Approved → Implemented |
| RFC | Draft → Discussion → Accepted → Implemented → Superseded |
| ADR | Proposed → Accepted → Deprecated → Superseded |
| ProblemCard | Draft → Active → Resolved |
| SolutionPortfolio | Draft → Active → Decided |
| EvidencePack | Draft → Active → Expired |
| Note | Active → Expired (auto-expires 90 days) |
| RefreshReport | Draft → Complete |

**Важно**: это type-specific lifecycles. Не путать с DerivedStatus (UNDERFRAMED→...→APPLIED), который вычисляется автоматически по полноте цепочки ProblemCard→SolutionPortfolio→ADR→EvidencePack и НЕ хранится как поле.
