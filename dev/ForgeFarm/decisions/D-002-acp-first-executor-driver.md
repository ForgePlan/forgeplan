# D-002: agent-client-protocol (ACP) — первичный протокол ExecutorDriver

**Статус:** принято владельцем 2026-07-03, формулировка **ACP-first, не
ACP-only** (границы применимости дополняются по итогам vibe-kanban study).

## Контекст

ExecutorDriver (`../architecture/planes.md`) до сих пор предполагал
per-harness адаптеры с парсингом собственных JSON-потоков каждого CLI
(CC stream-json, codex exec --json, opencode --format json). Обнаружено:
**ACP (agent-client-protocol, стандарт от Zed)** — типизированный протокол
клиент↔кодинг-агент, и vibe-kanban уже использует его в продакшене
(`crates/executors`: `agent-client-protocol = "0.8"`), рядом с
нативными адаптерами.

## Решение

1. **ACP — первичный транспорт ExecutorDriver**: где агент говорит ACP
   (нативно или через официальный адаптер — например claude-code-acp,
   Gemini CLI), ран поднимается как ACP-сессия; RunEvents маппятся из
   ACP-сообщений, approvals — из ACP permission-запросов (→ HAQ).
2. **Нативные адаптеры остаются fallback'ом** для агентов без ACP и для
   возможностей, которые ACP (unstable, v0.x) ещё не покрывает —
   granular-настройки CC/Codex из `../architecture/executor-sessions.md`
   не отменяются.
3. PTY-эвристика (Herdr H-1) остаётся третьим каналом (cross-check/зависшие).
4. Карта «какой агент через что» — **подтверждена VK-study эмпирикой
   стоимости** (`../synthesis/06-vibe-kanban-patterns.md` §3.2): bespoke
   stdio-протоколы ≈ 4.5–5k LOC на агента (Claude 3.2k, Codex 2.9k),
   local HTTP+SSE ≈ 4.8k (OpenCode), **ACP ≈ 1.9k общих на ТРИ агента
   (Gemini/Qwen/Copilot) при ~230 marginal LOC на агента** — соотношение
   bespoke:ACP ≈ 20:1. Vendor-кандидат: VK `src/executors/acp/{harness.rs,
   client.rs}` (~990 LOC, Apache-2.0) как база ACP-драйвера, с заменой
   human-approval на policy/gate seam и JSONL pseudo-resume на declared
   capability. Большая тройка остаётся на нативных драйверах (CC stream-json
   control protocol с `can_use_tool` = точка in-run policy enforcement;
   Codex JSON-RPC; OpenCode HTTP+SSE) — их granular-возможности ACP не
   покрывает; новые агенты — через ACP по умолчанию.

## Последствия

- Меньше хрупкого парсинга per-CLI; новый ACP-агент = почти бесплатный
  адаптер; протокольные типы (permission request, tool call, diff) уже
  стандартизованы — ложатся на RunEvents/approvals.
- Version-риск: ACP v0.x с feature `unstable` — пиновать версию, smoke на
  релизы (та же дисциплина, что для harness-пар в
  `../architecture/model-routing.md` A.4).

## Что перевернёт

Если ACP-покрытие большой тройки (CC/Codex/OpenCode) окажется неполным или
деградирующим по качеству против нативных потоков (замер в eval-кортеже:
пара сравнивается через оба транспорта) — ACP опускается до «одного из
транспортов», нативные адаптеры возвращаются в primary.
