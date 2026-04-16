---
depth: tactical
id: PROB-037
kind: problem
links:
- target: PRD-048
  relation: informs
status: active
title: MCP installation requires manual setup — incomplete brew distribution and no embed/install commands
---

# PROB-037: MCP installation requires manual setup — incomplete brew distribution and no embed/install commands

## Signal

User flow `brew install forgeplan` → setup MCP в Claude Code = **требует ручной правки `.mcp.json`**.

**Что РЕАЛЬНО происходит (после re-investigation 2026-04-16)**:

1. `brew install forgeplan` ставит CLI binary который **уже содержит embedded MCP server** (`forgeplan serve` subcommand)
2. Brew binary полноценно работает как MCP server: `forgeplan serve` → JSON-RPC stdio handshake OK
3. **Реальная проблема**: нет команды `forgeplan mcp install --client claude` для **автоматического прописывания** правильной секции в `.mcp.json`
4. Юзер должен вручную править JSON, помнить правильный формат (`"command": "forgeplan", "args": ["serve"]`), и применять для каждого MCP-aware client отдельно (Claude Code / Cursor / Windsurf)

**Original misdiagnosis (2026-04-16)**: я предположил отсутствие embed, но не проверил `forgeplan --help | grep serve`. Verification показала что embed уже есть — gap только в install UX.

**Что РАБОТАЕТ из коробки** (после правильной конфигурации):
```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve"]
    }
  }
}
```

## Constraints

- Brew distribution уже корректна (single binary с embedded MCP) — **НЕ ломать**
- MCP protocol stdio transport — обязателен для Claude Code совместимости
- `.mcp.json` может содержать кастомизированный `env` (API keys, custom paths) — **сохранять**
- Cross-platform: macOS / Linux / Windows — equal quality
- Backward compatibility: существующие `.mcp.json` с любым форматом должны продолжать работать

## Optimization Targets (1-3 макс)

- **Number of manual steps** после `brew install` для подключения MCP: с 5+ (find docs, edit JSON, verify format) до **1** (`forgeplan mcp install --client claude`)
- **Multi-client coverage**: support Claude Code + Cursor + Windsurf одной командой
- **Idempotency**: повторный запуск install не дублирует/не ломает существующий конфиг

## Observation Indicators (Anti-Goodhart)

- НЕ оптимизировать число команд в CLI (избегать `forgeplan mcp install start stop status restart` — feature creep)
- НЕ оптимизировать только под Claude Code — учитывать Cursor/Windsurf/любой MCP-aware client
- НЕ ломать существующие workflows (separate `forgeplan-mcp` binary остаётся доступен для devs)

## Acceptance Criteria

После `brew install forgeplan` юзер выполняет **одну команду** и MCP работает в любом supported client:

```bash
brew install forgeplan
forgeplan mcp install --client claude    # или cursor / windsurf
# → Detects config path (cross-platform: macOS / Linux / Windows)
# → Smart-merges forgeplan section (replaces command/args, preserves env)
# → Idempotent: repeat invocations don't break user customization
# → Юзер перезапускает client, MCP подключается
```

Measurable:
- `forgeplan mcp install --client claude` exit code 0, .mcp.json содержит правильную секцию
- Repeat invocation = no diff (idempotent)
- Existing `env` в `.mcp.json` сохраняется через все апгрейды
- Works on macOS / Linux / Windows (CI matrix)
- 3 supported clients: claude, cursor, windsurf

## Blast Radius

- **Затронуто**: новый `forgeplan mcp install` command в CLI, `.mcp.json` файлы юзеров (write target)
- **Не затронуто**: brew formula, `forgeplan serve` (existing), MCP server logic, methodology, artifacts
- **Cross-cutting**: только UX/DX layer

## Reversibility

**High**. Новая команда install, не модифицирует existing logic. Юзер может откатить вручную (revert .mcp.json через git или backup). Smart-merge с `--dry-run` опцией показывает предполагаемые изменения до записи.

---

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-048 | informs |


