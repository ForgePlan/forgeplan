# D-001: Rust-ядро + Tauri для приложения

**Статус:** принято владельцем 2026-07-03 (стек-детали дополняются по итогам
vibe-kanban study).

## Контекст

Развилка №1 (`../synthesis/02-open-decisions.md`) уже рекомендовала custom
Rust control plane. Владелец подтверждает и расширяет: **всё приложение —
Rust («быстро и надёжно»), UI — Tauri + проверенные Rust-либы**; vibe-kanban
берётся как образец паттернов хорошего софта этого класса (Rust workspace
30+ крейтов + React + Tauri, Apache 2.0 — код переиспользуем легально).

## Решение

1. **Ядро** — Rust workspace по образцу модульности VK: мелкие крейты по
   ответственности (`ff-core`, `ff-projection`, `ff-scheduler`, `ff-leases`,
   `ff-policy`, `ff-executors`, `ff-worktree`, `ff-audit`, `ff-api`… —
   scaffold из R3, сверенный с крейт-нарезкой VK).
2. **UI** — один web-фронт (проекция поверх `ff-api`), упакованный в
   **Tauri** для desktop — ровно паттерн VK (tauri-app оборачивает тот же
   React-код, что served локально). Это НЕ меняет фазовую карту UI
   (`../architecture/ui-observability.md`): Phase 3 = `ff top` (терминал),
   Phase 4 = Board + Run Inspector (web), Tauri-обёртка — дешёвый слой
   поверх Phase 4, не отдельная разработка. Согласуется с экосистемой:
   Tauri уже в формуле ForgePlan (Phase 5 Desktop backlog).
3. **Либы** — проверенный продакшеном набор VK, зафиксирован в
   [`../architecture/rust-stack.md`](../architecture/rust-stack.md)
   (tokio/axum/tower-http/serde/thiserror/tracing/git2/reqwest +
   command_group/os_pipe/json_patch/jsonc-parser/shlex из executors-слоя).
   Осознанные отличия: **Postgres вместо SQLite** (консенсус A4; их
   SQLite-preupdate-hook event-шина — в списке «не брать»), upstream `ts-rs`
   или `specta` вместо их личного форка, + petgraph/utoipa под наши нужды.
   Код VK (Apache-2.0) — selective vendor по списку из
   [`../synthesis/06-vibe-kanban-patterns.md`](../synthesis/06-vibe-kanban-patterns.md) §3.

## Последствия

- Один язык на ядро+ForgePlan; крошечный ops footprint; K8s-ready гарантии
  (`../architecture/reliability-and-k8s.md`) сохраняются — Tauri-слой на них
  не влияет.
- VK становится живым учебником: его крейты `executors`,
  `worktree-manager`, `db`, `services`, `review` — прямые прекурсоры наших.

## Что перевернёт

Flip-сигнал развилки №1 остаётся в силе (>30–40% времени в
durable-execution plumbing → Temporal как внешняя оболочка). Tauri
пересматривается только если desktop окажется ненужным (тогда web-only —
вычитание, не переделка).
