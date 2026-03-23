# PRD-005: Depth-Aware Validation

## Summary

Расширить `forgeplan validate` с базовых schema checks до полной depth-aware validation. Каждый тип артефакта проверяется по правилам, калиброванным под его depth (tactical/standard/deep). Правила разделены по severity: MUST (блокирует activation), SHOULD (предупреждение), COULD (рекомендация).

## Problem

Первая версия validation проверяла только наличие секций — есть `## Summary` или нет. Этого недостаточно:
- PRD на tactical depth не должен требовать Risk Assessment — это overkill
- PRD на deep depth обязан иметь rollback plan и success metrics — это safety requirement
- Разные типы артефактов (PRD, RFC, ADR, Epic) имеют разные обязательные секции
- Нет разницы между критической ошибкой (нет Problem секции) и рекомендацией (секция короткая)

Без depth-aware validation: либо все артефакты проверяются одинаково (слишком строго для tactical, слишком мягко для deep), либо validation бесполезен.

## Goals

- Валидация калибруется по depth: tactical = минимум правил, deep = максимум
- Каждое правило имеет severity (MUST/SHOULD/COULD) — MUST блокирует activation через lifecycle gate
- Поддержка aliases секций (Problem = Motivation = Problem Statement) — гибкость без потери строгости
- Per-kind правила: PRD, RFC, ADR, Epic, Spec проверяются по своим правилам

## Target Audience

- AI агент — автоматическая проверка перед activation (lifecycle integration)
- Разработчик — `forgeplan validate PRD-001` для self-check перед review
- Reviewer — `forgeplan review PRD-001` запускает validation как часть review

## Functional Requirements

- [x] FR-001: Depth-aware PRD validation — Tactical: 9 rules, Standard: 12 rules, Deep: 20 rules
- [x] FR-002: Severity per rule — MUST (блокирует activate), SHOULD (warning), COULD (suggestion)
- [x] FR-003: Section aliases — Problem = Motivation = Problem Statement = Background (и 10+ других)
- [x] FR-004: Per-kind validation — PRD (9-20), Epic (8), Spec (6), RFC (8-9), ADR (6-8) rules
- [x] FR-005: Quality gate output — PASS / FAIL / PASS_WITH_WARNINGS с счётчиками
- [x] FR-006: Deep PRD checks — risk assessment, rollback plan, success metrics, dependencies, FR format
- [x] FR-007: Placeholder detection — {{...}}, TODO, FIXME (ignores code fences)
- [x] FR-008: Tech leakage detection — blocklist of 22 technology names in requirements
- [x] FR-009: Problem density check — minimum 50 words in Problem section (SHOULD)

## Non-Goals

- `forgeplan validate --adversarial` mode (BMAD adversarial review) — backlog
- Custom validation rules per project — все правила hardcoded
- ML-based quality scoring — validation = deterministic rules, quality = F-G-R scoring (отдельный модуль)

## Related Artifacts

- EPIC-001: родительский Epic
- PRD-007: Lifecycle (validation = gate перед activation)
- PRD-002: FPF Engine (F-G-R scoring использует validation results для Formality)

## Implementation Notes

Реализовано в `crates/forgeplan-core/src/validation/` (1174 LOC, 32 теста):
- `mod.rs`: dispatcher — выбирает правила по kind + depth
- `rules.rs`: 30+ правил как function pointers с severity
- `checks.rs`: helper functions — section_exists (with aliases), word_count, placeholders, tech leakage
- Lifecycle integration: `review()` вызывает `validate()` и gates на MUST findings
