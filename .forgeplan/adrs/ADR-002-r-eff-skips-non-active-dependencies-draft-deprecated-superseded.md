---
depth: standard
id: ADR-002
kind: adr
status: active
title: R_eff skips non-active dependencies (draft, deprecated, superseded)
---

## Context

R_eff recursive вычисляет weakest link по всему дереву зависимостей. Если артефакт A зависит от артефакта B, R_eff(A) = min(self_score, R_eff(B)).

Проблема: если B — draft (ещё не начат) или deprecated (закрыт), его R_eff = 0. Это тянет весь chain к нулю.

## Decision

**R_eff пропускает non-active зависимости** (draft, deprecated, superseded) при рекурсивном вычислении. Пропуск логируется в factors: 'Skipped X (status: draft)'.

## Alternatives Considered

| Вариант | Результат | Отклонён потому что |
|---------|-----------|---------------------|
| Считать draft как 0 | Всё дерево красное | Наказывает за планирование вперёд |
| Считать draft как 1.0 | Эквивалентно пропуску | Менее явно |
| **Пропускать с логом** | R_eff отражает реальную работу | **Выбрано** |

## Consequences

- Зелёные scores отражают подтверждённую работу, не планы
- Пользователь видит пропущенные зависимости в выводе score
- Когда draft станет active → R_eff автоматически включит его при следующем score

## Affected Files

- crates/forgeplan-core/src/scoring/reff.rs


