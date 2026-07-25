# Rust-стек ForgeFarm (по образцу production-проверенного стека VK)

> Реализация D-001. Список — из фактических Cargo.toml vibe-kanban
> (workspace v0.1.44, Apache-2.0) + отличия ForgeFarm. Принцип: брать то, что
> VK проверил на 30k MAU, менять только там, где наши контракты требуют.

## Ядро (совпадает с VK — брать как есть)

| Concern | Крейт | Заметка |
|---|---|---|
| async runtime | `tokio` (features=full) + `tokio-util` | |
| HTTP API | `axum` 0.8 (macros, multipart, ws) + `tower-http` (cors, trace, compression) | `ff-api` |
| сериализация | `serde` / `serde_json` (preserve_order) / `serde_with` | |
| ошибки | `thiserror` (библиотеки) + `anyhow` (бинари) | FF: + typed retryable/terminal taxonomy поверх |
| трейсинг | `tracing` + `tracing-subscriber` (env-filter, json) | OTel-совместимые span'ы — уже наш инвариант |
| git | `git2` 0.20 (libgit2) | + conformance-набор из VK `git_ops_safety.rs` |
| HTTP-клиент | `reqwest` (rustls) | webhooks, форджи, Model Gateway |
| trait-абстракции | `async-trait`, `enum_dispatch`, `strum` | форма VK-driver-trait |
| процессы | `command_group` (AsyncGroupChild — kill всей группы), `os_pipe` (stdout dup) | run supervisor |
| live-проекции | `json_patch` (RFC-6902) + broadcast (см. VK MsgStore) | wire-протокол Board/Inspector |
| конфиги агентов | `jsonc-parser` (comment-preserving CST merge) | MCP-инъекция per worktree |
| shell-парсинг | `shlex` (+ winsplit на Windows) | CommandBuilder |
| схемы | `schemars` | JSON Schema для playbooks/policies |
| ACP | `agent-client-protocol` (пин версии; unstable) | D-002 |

## Отличия от VK (осознанные)

| Concern | VK | ForgeFarm | Почему |
|---|---|---|---|
| БД | `sqlx` + **SQLite** (+preupdate-hook как event-шина) | `sqlx` + **Postgres** (LISTEN/NOTIFY + advisory locks + SKIP LOCKED) | консенсус A4: leases, очередь, leader-election, multi-host; SQLite-hook-шина — в списке «не брать» |
| TS-типы для фронта | `ts-rs` (личный форк xazukx/use-ts-enum!) | `ts-rs` upstream или `specta`; **не наследовать чужой форк** | форк — риск из executors-вердикта |
| API-доки | ручные типы | + `utoipa` (OpenAPI из axum) — опционально | ff-api потребляют агенты |
| граф | — (нет DAG) | `petgraph` (in-memory кэш DAG в scheduler) | наш scheduler |
| desktop | `tauri` (tauri-app crate) | `tauri` v2 — тот же паттерн: оборачивает web-фронт | D-001 |
| дистрибуция | `npx-cli` thin-shim (скачивает бинарь) | тот же паттерн для `ff` (G-11) + brew | как forgeplan |

## Не тащить из VK dep-tree

- Форк `ts-rs` (xazukx) — заменить upstream/specta.
- 13 relay-* крейтов (WebRTC/tunnels/embedded-ssh) — scope-урок №4: удалённый
  доступ решается SSH + `ff top`, не собственным relay-стеком.
- `aws-lc-*` пины — следствие их rustls-выбора; принять только если сами
  выберем ту же криптобиблиотеку.
