# Forgeplan — Forge Your Plan

**Vision**: Универсальный процесс и Rust CLI для ведения любого проекта
от идеи до реализации, используя FPF, PRD, RFC, ADR, Epics и автоматизацию.

**Статус**: Phase 1 — Schemas & Templates
**Alias**: `fpl` (опциональный короткий alias)

---

## Что это

Система документов + CLI-приложение (Rust), которые помогают:

1. **Декомпозировать** любую задачу на артефакты (PRD → Spec → RFC → ADR → Sprint)
2. **Генерировать** шаблоны через FPF reasoning + LLM
3. **Трекать** прогресс через индексы и progress bars
4. **Группировать** связанные артефакты в Epics
5. **Валидировать** полноту и качество документов

## Use Cases

| Сценарий | Артефакты | Flow |
|----------|-----------|------|
| Новая фича | PRD → Spec → RFC → Sprint | Idea → Requirements → Architecture → Code |
| Рефакторинг / Decompose monolith | ADR → RFC → Sprint | Decision → Design → Execute |
| Баг / Incident | ADR (root cause) → RFC (fix) | Investigate → Decide → Fix |
| Инфраструктура | RFC → ADR → Sprint | Design → Decide → Deploy |
| Продуктовый roadmap | Epic → PRD[] → RFC[] | Strategy → Requirements → Execution |
| Миграция (framework, DB, cloud) | Epic → ADR → RFC → Sprint | Plan → Decide → Design → Migrate |
| Новый проект с нуля | Epic → PRD → Spec → RFC → ADR → Sprint | Full lifecycle |
| Оценка технического долга | Analysis → ADR → Epic → RFC | Audit → Decide → Plan → Fix |
| API Design | Spec → RFC → ADR | Contract → Architecture → Decisions |
| Security Audit | Analysis → ADR → RFC | Findings → Decisions → Remediation |

## Структура каталога

```
frameworks/
├── README.md              ← этот файл
├── TODO.md                ← задачи проекта
├── PLAN.md                ← план реализации (4 фазы)
│
├── docs/
│   ├── schemas/           ← схемы артефактов (PRD-SCHEMA, EPIC-SCHEMA, etc.)
│   ├── guides/            ← workflow guides (PRD→RFC→ADR flow)
│   └── references/        ← собранные референсы (FPF, skills, commands)
│
├── templates/             ← шаблоны документов
│   ├── prd/               ← PRD шаблон
│   ├── epic/              ← Epic шаблон
│   ├── spec/              ← Specification шаблон
│   ├── rfc/               ← RFC шаблон (copy from existing)
│   └── adr/               ← ADR шаблон (copy from existing)
│
├── research/              ← исследования методологий
│
├── sources/              ← reference implementations (quint-code, OpenSpec, BMAD, git-adr)
│
└── src/                   ← Rust CLI application (Phase 3+)
    └── (будет позже)
```

## Методологии и фреймворки

| Метод | Из | Применение |
|-------|-----|-----------|
| **FPF** (First Principles Framework) | `.claude/skills/fpf-simple/` | Декомпозиция, bounded contexts, reasoning |
| **SPARC** | `.claude/commands/sparc/` | Specification → Pseudocode → Architecture → Refinement → Completion |
| **C4 Model** | `.claude/agents/c4-architecture/` | Context → Container → Component → Code diagrams |
| **DDD** | `.claude/agents/v3/ddd-domain-expert.md` | Bounded contexts, aggregates, domain events |
| **RFC Process** | `docs/methodology/PRD-RFC-ADR-FLOW.md` | When to use RFC / ADR / PRD |
| **ADR Process** | `.forgeplan/adrs/` | Project ADRs (managed by `forgeplan` CLI) |
| **Wave/Sprint** | `.claude/commands/sprint.md` | Agent Teams parallel execution |

## Quick Start

```bash
# 1. Изучить план
cat frameworks/PLAN.md

# 2. Посмотреть vision
cat frameworks/VISION.md

# 3. Использовать шаблон в любом проекте
cp frameworks/templates/prd/_TEMPLATE.md my-project/docs/prds/PRD-001-my-feature.md

# 4. (Phase 3+) CLI
forgeplan init
forgeplan new prd "My Feature"
forgeplan status
```
