---
depth: tactical
id: NOTE-029
kind: note
links:
- target: EPIC-001
  relation: informs
status: draft
title: 'Idea: CLI UX Polish — links command, doctor, --ci mode, error consistency'
---

## CLI UX Polish (из E2E findings + обсуждения)

### Tier 1 (малые усилия, большой эффект)
- forgeplan links PRD-001 — показать все связи артефакта (1 день)
- forgeplan export --format mermaid — граф прямо в README (2 часа)
- forgeplan validate --ci — exit code по MUST-ошибкам (1 день)
- Документировать capture = LLM-dependent в --help (10 минут)
- forgeplan doctor — проверить workspace, LLM key, feature flags (2 дня)

### UX Findings из E2E (не записанные ранее)

**Вывод команд:**
- `list` — вывод мог бы быть табличнее (сейчас просто список)
- `journal --risk` — слишком шумный: 10 из 11 at-risk. Порог нужно калибровать
- `--semantic` — плохой UX: feature flag, узнаёшь только при вызове. Нужно graceful fallback

**Подсказки и ошибки:**
- `activate` без evidence — говорит 'no evidence linked', но не подсказывает КАК добавить. Сравни с validate который конкретен
- `supersede/deprecate` — Clap ошибки сухие, нужно 'Missing --by. Usage: forgeplan supersede PRD-001 --by PRD-002'
- `capture` — выглядит offline, а требует LLM. Ноль предупреждений в --help

**Качество данных:**
- `estimate` — не находит FR в свободном тексте, возвращает 'no estimable items'. Бесполезен без формализованных FR
- `route` на пустой строке — 'Tactical, Confidence 0%'. Нет fallback-логики
- `remember/recall` — зачем отдельная от Note система? Непонятный UX

### Error Consistency Issues
- scan --path /tmp → exit 0 (FIXED в BUG-001)
- scan-import --path /etc → 'path traversal rejected' (правильно)
- init повторный → exit 0, info-сообщение (мягко, idempotent)
- activate повторный → exit 1, ошибка (строго)
Нет единой философии: где-то idempotent, где-то strict. Нужно выбрать одно.

### Case Sensitivity
IDs регистрозависимые (PRD-001 != prd-001). By design, но нигде не документировано.

### Design Decisions (не документированы как ADR)
- Повторный init = exit 0 (idempotent design)
- Target not found при link = warning + created (forward references)
- Self-link разрешён (→ PROB-019)
