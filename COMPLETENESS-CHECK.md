# Completeness Check — PRD Process Engine

Итеративная проверка: есть ли всё необходимое для каждого слоя?

## Canonical Pipeline (подтверждён 4 источниками)

```
FPF/ADI (Design) → OpenSpec/BMAD (Spec) → Claude Code (Code) → Verify (CI/CD)
     ↑                    ↑                      ↑                   ↑
  Thinking             Documents              Implementation      Validation
```

---

## Checklist по слоям

### 🟢 = Есть и готово | 🟡 = Частично | 🔴 = Нет, нужно создать

---

### Layer 0: Методология и мышление
| # | Компонент | Статус | Источник | Действие |
|---|-----------|--------|----------|----------|
| 0.1 | ADI цикл (Abduction→Deduction→Induction) | 🟢 | FPF, Quint-code | Описать в PRD-SCHEMA как decision flow |
| 0.2 | Trust scoring (F-G-R) | 🟢 | FPF | Интегрировать в verification gates |
| 0.3 | R_eff = min(evidence_scores) | 🟢 | Quint-code | Формула для quality scoring |
| 0.4 | Depth calibration (4 levels) | 🟢 | Quint-code | Tactical/Standard/Deep/Critical routing |
| 0.5 | Evidence Decay (valid_until TTL) | 🟢 | Quint-code, FPF | TTL на каждый артефакт |
| 0.6 | Adversarial Review | 🟢 | BMAD | "Must find problems" gate |
| 0.7 | "Not a state machine" principle | 🟢 | Анализ видео | Pipeline = guideline, not rigid |

### Layer 1: Discovery & Research
| # | Компонент | Статус | Источник | Действие |
|---|-----------|--------|----------|----------|
| 1.1 | Quick research | 🟢 | `/research` | — |
| 1.2 | Deep research (4-7 agents) | 🟢 | `/deep-research` | — |
| 1.3 | Memory recall | 🟢 | Hindsight `memory_recall` | — |
| 1.4 | Context restore | 🟢 | `/recall` | — |
| 1.5 | Competitive analysis | 🟡 | — | Добавить шаблон |
| 1.6 | Problem framing | 🟡 | FPF `/q-frame` | Адаптировать из Quint-code |

### Layer 2: Requirements & PRD
| # | Компонент | Статус | Источник | Действие |
|---|-----------|--------|----------|----------|
| 2.1 | PRD шаблон | 🟢 | Создан в `templates/prd/` | Обогатить из BMAD |
| 2.2 | `/write-doc prd` | 🔴 | — | **Расширить write-doc.md** |
| 2.3 | User Stories генератор | 🔴 | BMAD (John PM) | **Создать skill** |
| 2.4 | Acceptance Criteria | 🟡 | В шаблоне есть, нет генератора | Создать генератор |
| 2.5 | Success Metrics шаблон | 🟢 | В PRD шаблоне | — |
| 2.6 | Problem Statement framework | 🟡 | FPF Bounded Contexts | Описать в PRD-SCHEMA |
| 2.7 | PRD-INDEX | 🔴 | — | **Создать (как RFC-INDEX)** |
| 2.8 | Product Brief (lightweight PRD) | 🔴 | BMAD Quick Flow | **Создать шаблон** |

### Layer 3: Architecture & Design
| # | Компонент | Статус | Источник | Действие |
|---|-----------|--------|----------|----------|
| 3.1 | Architecture review | 🟢 | `architecture-guardian`, `sparc/architect` | — |
| 3.2 | DDD decomposition | 🟢 | `v3-ddd-architecture` | — |
| 3.3 | C4 diagrams | 🟢 | C4 agents | — |
| 3.4 | Trade-off analysis | 🟢 | RFC шаблон (Options table) | — |
| 3.5 | Solutioning gate | 🟡 | BMAD | Формализовать в workflow |

### Layer 4: Specification & Contracts
| # | Компонент | Статус | Источник | Действие |
|---|-----------|--------|----------|----------|
| 4.1 | Spec шаблон | 🟢 | Создан в `templates/spec/` | — |
| 4.2 | API Contract-first design | 🔴 | OpenSpec | **Создать skill** |
| 4.3 | Data Model spec | 🟡 | В Spec шаблоне | — |
| 4.4 | Delta-specifications | 🔴 | OpenSpec | **Внедрить паттерн** |
| 4.5 | Verify against spec | 🟡 | OpenSpec `/opsx:verify` | Адаптировать |

### Layer 5: Decision Making (ADR)
| # | Компонент | Статус | Источник | Действие |
|---|-----------|--------|----------|----------|
| 5.1 | ADR шаблон | 🟢 | Создан в `templates/adr/` | — |
| 5.2 | `/write-doc adr` | 🟢 | write-doc command | — |
| 5.3 | DDR template (с obsolescence) | 🟡 | FPF, Quint-code | Добавить valid_until в ADR |
| 5.4 | Decision memory | 🟢 | Hindsight `memory_retain` | — |
| 5.5 | Verification Gate (5-point) | 🔴 | Quint-code | **Создать checklist** |

### Layer 6: Planning & Decomposition
| # | Компонент | Статус | Источник | Действие |
|---|-----------|--------|----------|----------|
| 6.1 | Epic шаблон | 🟢 | Создан в `templates/epic/` | — |
| 6.2 | `/write-doc epic` | 🔴 | — | **Расширить write-doc.md** |
| 6.3 | Epic→PRD→RFC decomposition | 🔴 | — | **Создать workflow** |
| 6.4 | Sprint planning | 🟢 | `/sprint` | — |
| 6.5 | Wave execution | 🟢 | `/wave` | — |
| 6.6 | EPIC-INDEX | 🔴 | — | **Создать** |
| 6.7 | Dependency graph (mermaid) | 🟡 | В Epic шаблоне, нет генератора | Для Go CLI |

### Layer 7: Implementation & Sprint
| # | Компонент | Статус | Источник | Действие |
|---|-----------|--------|----------|----------|
| 7.1 | Agent Teams | 🟢 | `/team-up`, TeamCreate | — |
| 7.2 | Build from research | 🟢 | `/build` | — |
| 7.3 | Pair programming | 🟢 | `pair-programming` skill | — |
| 7.4 | Contextual chain (phase output → next input) | 🟡 | BMAD | Формализовать |
| 7.5 | File ownership rules | 🟢 | CLAUDE.md | — |

### Layer 8: Quality & Review
| # | Компонент | Статус | Источник | Действие |
|---|-----------|--------|----------|----------|
| 8.1 | Code audit | 🟢 | `/audit` | — |
| 8.2 | Adversarial review protocol | 🔴 | BMAD | **Создать quality gate** |
| 8.3 | Acceptance testing vs PRD | 🔴 | — | **Создать validator** |
| 8.4 | Quality scoring (R_eff) | 🔴 | Quint-code | **Для Go CLI** |
| 8.5 | Module coverage tracking | 🔴 | Quint-code | **Для Go CLI** |

### Layer 9: Documentation
| # | Компонент | Статус | Источник | Действие |
|---|-----------|--------|----------|----------|
| 9.1 | `/write-doc` (RFC, Guide, ADR...) | 🟢 | — | — |
| 9.2 | Memory sync | 🟢 | `/sync-docs`, `/load-doc` | — |
| 9.3 | Auto-capture decisions | 🔴 | Quint-code | **Для Go CLI** |
| 9.4 | Archive cycle | 🔴 | OpenSpec `/opsx:archive` | **Для Go CLI** |

---

## Summary

| Статус | Количество | % |
|--------|-----------|---|
| 🟢 Готово | 32 | 62% |
| 🟡 Частично | 10 | 19% |
| 🔴 Нужно создать | 10 | 19% |
| **TOTAL** | **52** | — |

## Критический путь (10 items 🔴)

**Phase 1 — Быстрые wins (commands/templates):**
1. `/write-doc prd` — расширить write-doc.md
2. `/write-doc epic` — расширить write-doc.md
3. PRD-INDEX.md — создать
4. EPIC-INDEX.md — создать
5. Product Brief шаблон (lightweight PRD) — создать

**Phase 2 — Skills и gates:**
6. User Stories генератор — skill
7. Verification Gate checklist (5-point) — skill/gate
8. Adversarial Review protocol — quality gate

**Phase 3 — Go CLI (`forgeplan`):**
9. Contract-first API design
10. Quality scoring (R_eff), module coverage, auto-capture, archive

---

## Ключевые инсайты из анализа

### 1. `/do` = прото-PRD-engine
Уже есть: classifies intent → chains commands → executes. Главный gap — **продуктовая сторона** (PRD, user stories, acceptance criteria). Инженерная сторона покрыта на 90%.

### 2. Pipeline = guideline, NOT rigid sequence
FPF автор подтвердил: "произвольные траектории в пространстве мышления". PRD engine должен поддерживать:
- Quick Flow (3 commands для малых задач) — из BMAD
- Full Path (6+ phases для продуктов) — из BMAD
- Depth calibration (Tactical → Critical) — из Quint-code

### 3. Quint-code = reference implementation для Go CLI
Go + SQLite + MCP server + slash commands — это буквально то что мы хотим в `forgeplan`. Изучить `.quint/` структуру и адаптировать.

### 4. Brownfield > Greenfield
Все источники подтверждают: structured engineering нужнее для существующих проектов ("vibe-coded mountains of code") чем для новых.

### 5. Contextual chain — ключевой паттерн
Каждая фаза производит артефакт → артефакт становится input для следующей фазы. Это автоматическая передача контекста, без потерь.
