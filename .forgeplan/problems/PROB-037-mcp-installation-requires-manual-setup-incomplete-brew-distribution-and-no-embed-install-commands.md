---
depth: tactical
id: PROB-037
kind: problem
links:
- target: PRD-048
  relation: informs
status: draft
title: MCP installation requires manual setup — incomplete brew distribution and no embed/install commands
---

# PROB-037: MCP installation requires manual setup — incomplete brew distribution and no embed/install commands

## Signal

User flow `brew install forgeplan` → setup MCP в Claude Code = **не работает out-of-the-box**:

1. `brew install forgeplan` устанавливает только CLI binary (`/opt/homebrew/bin/forgeplan`)
2. MCP server (`forgeplan-mcp`, отдельный binary в workspace) **не пакуется** в brew formula
3. CLI **не имеет** subcommand `mcp` / `serve` для запуска MCP server
4. Юзер должен: clone repo → `cargo build --release -p forgeplan-mcp` → вручную править `.mcp.json` с абсолютным путём
5. Нет команды `forgeplan mcp install --client claude` для авто-настройки

**Confirmed 2026-04-16**: после `brew install forgeplan@0.18.0` MCP сервер в Claude Code не подключился, пришлось танцевать с бубном (build from source + manual `.mcp.json` edit).

## Constraints

- Brew formula должна оставаться single-package (не плодить `forgeplan-mcp` отдельно — UX хуже)
- MCP protocol stdio transport — обязателен для Claude Code совместимости
- `.mcp.json` может содержать кастомизированный `env` (API keys, custom paths) — **сохранять**
- Backward compatibility: существующие `.mcp.json` со ссылкой на `forgeplan-mcp` binary должны работать

## Optimization Targets (1-3 макс)

- **Time-to-first-MCP-call** для нового юзера: с ~30 минут (clone+build+edit) до **<2 минут** (brew + 1 команда)
- **Number of manual steps** после `brew install`: с 5+ до **1** (`forgeplan mcp install --client claude`)
- **Distribution simplicity**: 1 binary вместо 2 (embed MCP server в CLI)

## Observation Indicators (Anti-Goodhart)

- НЕ оптимизировать число команд в CLI (избегать `forgeplan mcp install start stop status restart` — feature creep)
- НЕ оптимизировать только под Claude Code — учитывать Cursor/Windsurf/любой MCP-aware client
- НЕ ломать существующие workflows (separate `forgeplan-mcp` binary остаётся доступен для devs)

## Acceptance Criteria

После `brew install forgeplan` юзер выполняет **одну команду** и MCP работает в Claude Code:

```bash
brew install forgeplan
forgeplan mcp install --client claude
# → Detects ~/.mcp.json или project .mcp.json
# → Smart-merges forgeplan section (replaces command/args, preserves env)
# → Юзер перезапускает Claude Code, MCP подключается
```

Measurable:
- `forgeplan mcp serve` запускается из единого CLI binary, отвечает на JSON-RPC
- `forgeplan mcp install --client claude` идемпотентна (повторный запуск не ломает env)
- E2E тест: brew install → install → MCP handshake = OK

## Blast Radius

- **Затронуто**: brew formula (Formula/forgeplan.rb), CLI binary structure, `.mcp.json` файлы юзеров
- **Не затронуто**: methodology, artifacts, scoring engine, существующие CLI команды
- **Cross-cutting**: distribution + UX + DX

## Reversibility

**Medium**. Embed MCP server в CLI — обратимо (можно вернуть отдельный binary). `forgeplan mcp install` — новая команда, не сломает существующее. Smart-merge стратегия — единственное необратимое решение, нужно покрыть тестами.

---

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-048 | informs |

