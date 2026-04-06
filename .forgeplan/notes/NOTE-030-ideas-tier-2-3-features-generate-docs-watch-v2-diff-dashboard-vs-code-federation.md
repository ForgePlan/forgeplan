---
depth: tactical
id: NOTE-030
kind: note
links:
- target: EPIC-002
  relation: informs
status: draft
title: 'Ideas: Tier 2-3 features — generate-docs, watch v2, diff, dashboard, VS Code, federation'
---

## Tier 2 — Средние усилия, трансформационный эффект

### forgeplan generate-docs (1 неделя)
Из графа артефактов генерировать:
- ARCHITECTURE.md — из PRD/RFC/ADR графа
- ADR log — из journal --json
- Risk matrix — из journal --risk --json
- Coverage matrix — из coverage --json
forgeplan graph уже выводит Mermaid = 80% ценности.

### Watch v2 — bidirectional sync (2 недели)
Разработчик редактирует .md в VS Code → watch подхватывает → обновляет артефакт → пересчитывает validate + score → уведомление если score упал. Сейчас watch синкает .md↔LanceDB, но нет обратной связи.

### forgeplan diff PRD-001 (1 неделя)
Что изменилось с момента активации (git-aware). Показать delta между текущим состоянием и snapshot при activate. Полезно для review и re-validation.

### Dashboard TUI (1 неделя)
forgeplan dashboard в терминале (типа htop для архитектуры). Real-time view: artifacts by status, R_eff scores, blind spots, recent changes.

### forgeplan export --format mermaid (2 часа)
Граф прямо в README. Уже есть forgeplan graph (Mermaid), нужно только --format flag в export. Низковисящий фрукт.

## Tier 3 — Большие усилия, новая категория

### VS Code extension (2-3 недели)
Inline validate, score в редакторе при изменении .md. Gutter icons для R_eff. Tree view для artifact graph. Используем forgeplan serve (MCP) как backend.

### Multi-project federation (1 месяц)
Связи между артефактами из разных проектов. Например: PRD-001 в project-A зависит от ADR-005 в shared-infra. Cross-project graph + R_eff propagation.

### Desktop app — Tauri (1-2 месяца)
Визуальный граф (drag-n-drop связи), real-time scoring, interactive validate. Shared Rust core (forgeplan-core). Уже в Phase 5 backlog.

