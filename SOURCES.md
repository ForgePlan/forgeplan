# SOURCES — Карта всех источников для PRD Process Engine

Все файлы, методологии, команды и скиллы которые нужно изучить и интегрировать.

---

## 1. Методологии и фреймворки мышления

### FPF (First Principles Framework) — КЛЮЧЕВОЙ
| Что | Путь | Зачем нужно |
|-----|------|-------------|
| FPF Skill | `.claude/skills/fpf-simple/` | Основной reasoning engine |
| FPF Sections (20 частей) | `.claude/skills/fpf-simple/sections/` | Декомпозиция, оценка, конфликты |
| Part A — Kernel | `sections/04-part-a-kernel-architecture-cluster/` | Holons, Bounded Contexts, Roles |
| Part B — Reasoning | `sections/07-part-b-trans-disciplinary-reasoning-cluster/` | Composition, Trust, Reasoning Cycles |
| Part C — Extensions | `sections/08-part-c-kernel-extensions-specifications/` | F-G-R scoring, Open-Ended Search |
| Part D — Ethics | `sections/09-part-d-multi-scale-ethics-conflict-optimisation/` | Conflict resolution |
| Part G — SoTA Kit | `sections/16-part-g-discipline-sota-patterns-kit/` | Discipline surveys, OperatorCards |

### SPARC Methodology
| Что | Путь | Зачем нужно |
|-----|------|-------------|
| SPARC Skill | `.claude/skills/sparc-methodology/` | Specification → Pseudocode → Architecture → Refinement → Completion |
| SPARC Commands | `.claude/commands/sparc/` | Orchestrator, coder, architect, tester modes |
| SPARC Agents | `.claude/agents/sparc/` | sparc-orchestrator, sparc-coder |

### C4 Architecture Model
| Что | Путь | Зачем нужно |
|-----|------|-------------|
| C4 Context Agent | `.claude/agents/c4-architecture/c4-context.md` | System context diagrams |
| C4 Container Agent | `.claude/agents/c4-architecture/c4-container.md` | Deployment architecture |
| C4 Component Agent | `.claude/agents/c4-architecture/c4-component.md` | Component boundaries |
| C4 Code Agent | `.claude/agents/c4-architecture/c4-code.md` | Code-level docs |

### DDD (Domain-Driven Design)
| Что | Путь | Зачем нужно |
|-----|------|-------------|
| DDD Domain Expert | `.claude/agents/v3/ddd-domain-expert.md` | Bounded contexts, aggregates |
| DDD Knowledge Skill | Skill: `ddd-knowledge` | Patterns, antipatterns |
| Clean Architecture | Skill: `clean-architecture` | Hexagonal, ports & adapters |

---

## 2. Существующие шаблоны документов

| Шаблон | Путь | Статус |
|--------|------|--------|
| RFC Schema | `docs/guides/RFC-SCHEMA.md` | ✅ Полный, скопировать |
| RFC Template | `docs/guides/project-template/docs/rfcs/_TEMPLATE.md` | ✅ Скопировать |
| ADR Template | `docs/guides/project-template/docs/adrs/_TEMPLATE.md` | ✅ Скопировать |
| Project Template | `docs/guides/project-template/` | ✅ Референс структуры |
| CLAUDE.md Template | `docs/guides/project-template/CLAUDE.md` | ✅ Референс |
| AGENTS.md Template | `docs/guides/project-template/AGENTS.md` | ✅ Референс |

---

## 3. Slash Commands (для интеграции)

| Команда | Путь | Что делает | Интеграция |
|---------|------|-----------|------------|
| `/write-doc` | `.claude/commands/write-doc.md` | Создаёт RFC, Guide, Report, ADR | **Расширить**: +prd, +epic, +spec |
| `/sprint` | `.claude/commands/sprint.md` | Wave-based sprint execution | Принимает PRD как input |
| `/do` | `.claude/commands/do.md` | Meta-pipeline: research→doc→sprint | Добавить PRD pipeline |
| `/research` | `.claude/commands/research.md` | Quick multi-agent research | Для PRD discovery phase |
| `/deep-research` | `.claude/commands/deep-research.md` | Deep 4-7 agent research | Для PRD deep analysis |
| `/synthesize` | `.claude/commands/synthesize.md` | Combine researches | Объединение PRD insights |
| `/audit` | `.claude/commands/audit.md` | Expert review panel | PRD quality validation |
| `/team-up` | `.claude/commands/team-up.md` | Agent Teams parallel work | Sprint execution |
| `/wave` | `.claude/commands/wave.md` | Quick wave execution | Sprint waves |
| `/build` | `.claude/commands/build.md` | Build from research | PRD → Implementation |

---

## 4. Агенты (для специализации)

### Planning & Architecture
| Агент | Путь | Роль в PRD |
|-------|------|-----------|
| `code-architect` | Feature-dev agent | Designs feature architectures |
| `architecture-guardian` | Custom agent | Validates structural changes |
| `architect-reviewer` | Custom agent | System design validation |
| `system-architect` | Custom agent | High-level technical decisions |
| `prd-specialist` | Plugin agent | PRD creation (needs config) |
| `planning-prd-agent` | Plugin agent | Task decomposition (needs config) |

### Documentation
| Агент | Путь | Роль в PRD |
|-------|------|-----------|
| `docs-architect` | Documentation agent | Technical documentation from code |
| `tutorial-engineer` | Documentation agent | Step-by-step guides |
| `mermaid-expert` | Documentation agent | Diagrams |
| `api-documenter` | Documentation agent | API docs |
| `reference-builder` | Documentation agent | Reference materials |

### Review & Validation
| Агент | Путь | Роль в PRD |
|-------|------|-----------|
| `code-reviewer` | Multiple plugins | Code quality, security |
| `security-auditor` | Multiple plugins | Security audit |
| `performance-engineer` | Multiple plugins | Performance review |

---

## 5. Skills (для генерации контента)

### Архитектура и дизайн
| Skill | Что даёт PRD процессу |
|-------|----------------------|
| `clean-architecture` | Hexagonal, Clean Architecture patterns |
| `microservices-patterns` | Service boundaries, event-driven |
| `api-design-principles` | REST, GraphQL API design |
| `api-design-knowledge` | Richardson Maturity, HATEOAS |
| `design-expert` | System design, scalability |
| `ddd-knowledge` | DDD patterns for decomposition |
| `hexagonal-knowledge` | Ports & Adapters |
| `cqrs-knowledge` | Command/Query separation |
| `event-sourcing-knowledge` | Event sourcing patterns |

### Документация
| Skill | Что даёт PRD процессу |
|-------|----------------------|
| `adr-template` | ADR generation |
| `architecture-doc-template` | ARCHITECTURE.md generation |
| `mermaid-template` | Diagram generation |
| `changelog-template` | Changelog automation |
| `readme-template` | README generation |
| `getting-started-template` | Onboarding guides |
| `documentation-knowledge` | Doc types, audiences |

### Анализ и аудит
| Skill | Что даёт PRD процессу |
|-------|----------------------|
| `scan-codebase-structure` | Architectural layer detection |
| `detect-architecture-pattern` | MVC, DDD, Hexagonal detection |
| `analyze-solid-violations` | SOLID principle analysis |
| `analyze-coupling-cohesion` | Coupling metrics |
| `extract-domain-concepts` | Domain model extraction |
| `extract-business-rules` | Business rules extraction |
| `bug-impact-analyzer` | Blast radius analysis |

### Security & Quality
| Skill | Что даёт PRD процессу |
|-------|----------------------|
| `security-audit` | CVSS scoring, vulnerability assessment |
| `check-12-factor-compliance` | 12-Factor App audit |
| `check-bounded-contexts` | DDD boundaries audit |

---

## 6. Внешние референсы (для исследования)

| Источник | Что изучить | Приоритет |
|----------|------------|-----------|
| Shape Up (Basecamp) | Appetites, Pitches, Bets, Cool-down | HIGH |
| PRFAQ (Amazon) | Press Release + FAQ формат | HIGH |
| Jobs-to-be-Done | Outcome-driven requirements | MEDIUM |
| Impact Mapping | Goal → Actor → Impact → Deliverable | MEDIUM |
| Story Mapping | User journey decomposition | MEDIUM |
| Opportunity Solution Tree | Teresa Torres' framework | MEDIUM |
| RICE Scoring | Reach, Impact, Confidence, Effort | LOW |
| MoSCoW | Must, Should, Could, Won't prioritization | LOW |
| Wardley Mapping | Value chain + evolution | LOW |

---

## 7. Go Ecosystem (для Phase 3)

| Package | Зачем |
|---------|-------|
| `github.com/spf13/cobra` | CLI framework |
| `github.com/spf13/viper` | Config management |
| `text/template` | Template engine (stdlib) |
| `github.com/charmbracelet/bubbletea` | Interactive TUI |
| `github.com/charmbracelet/lipgloss` | Terminal styling |
| `github.com/yuin/goldmark` | Markdown parser |
| `gopkg.in/yaml.v3` | YAML config |
| `embed` | Embed templates in binary |

---

## 8. Use Cases для `forgeplan` CLI

```
# Новый проект
forgeplan init my-saas-app
forgeplan new epic "User Authentication System"
forgeplan new prd "Social Login (Google, GitHub)"  --epic EPIC-001
forgeplan new spec "OAuth2 API Contract"           --prd PRD-001
forgeplan new rfc "OAuth2 Implementation Design"   --prd PRD-001
forgeplan new adr "Choose Passport.js over Auth0"  --rfc RFC-001
forgeplan status
forgeplan graph --format mermaid

# Рефакторинг монолита
forgeplan new epic "Monolith Decomposition"
forgeplan new prd "Extract Payment Service"        --epic EPIC-001
forgeplan new adr "Event-Driven vs REST"           --prd PRD-001
forgeplan new rfc "Payment Service Architecture"   --prd PRD-001

# Миграция
forgeplan new epic "React 18 → React 19 Migration"
forgeplan new prd "Server Components Adoption"     --epic EPIC-001
forgeplan new rfc "RSC Architecture"               --prd PRD-001
forgeplan validate --epic EPIC-001

# Технический долг
forgeplan analyze ./src --type coupling
forgeplan new epic "Reduce Technical Debt Q2"
forgeplan new adr "Replace Redux with Zustand"     --epic EPIC-001
```

---

## Приоритет сбора

### MUST (Phase 1)
1. RFC-SCHEMA.md → `docs/references/`
2. RFC _TEMPLATE.md → `templates/rfc/`
3. ADR _TEMPLATE.md → `templates/adr/`
4. FPF Part A (Kernel) summary → `docs/references/`
5. Sprint template reference → `docs/references/`

### SHOULD (Phase 2)
6. Write-doc command source → `docs/references/`
7. SPARC methodology summary → `docs/references/`
8. C4 model summary → `docs/references/`
9. DDD patterns summary → `docs/references/`

### COULD (Phase 3+)
10. Shape Up / PRFAQ research
11. External methodology comparison
12. Go package evaluation
