---
created: 2026-04-20
depth: tactical
id: PROB-041
kind: problem
status: active
title: CLI dotenvy loads only cwd .env — misses .forgeplan/.env via workspace walk-up
updated: 2026-04-20
---

# PROB-041: CLI dotenvy loads only cwd `.env` — misses `.forgeplan/.env`

## Problem Statement

`crates/forgeplan-cli/src/main.rs:651` вызывает `dotenvy::dotenv().ok()` — это загружает `.env` **только** из текущей рабочей директории (cwd). Канонический файл секретов forgeplan — `.forgeplan/.env` (gitignored, документирован в CLAUDE.md), и он никогда не находится dotenvy по умолчанию. Результат: `NEURALDEEP_API_KEY` / `OPENROUTER_API_KEY` / `OPENAI_API_KEY` не загружаются; LLM-вызовы тихо фолбэчатся на Level 0 (keyword routing) без понятного индикатора что API-ключ не увиден.

## Signal

Воспроизведение (до fix):
```
$ cat .forgeplan/.env
NEURALDEEP_API_KEY=sk-...

$ forgeplan route "implement OAuth2 refresh"
## Level: Level 0 (keywords)   # LLM не вызван

$ NEURALDEEP_API_KEY=sk-... forgeplan route "implement OAuth2 refresh"
## Level: Level 2 (FPF reasoning)   # LLM работает когда env передан явно
```

Тот же pattern наблюдался в параллельной сессии с OpenRouter ключом (aod-worker миграция) — user потратил ~30 минут выясняя почему routing всегда Level 0 несмотря на валидный `.forgeplan/.env`.

## Root Cause

`dotenvy::dotenv()` (без пути) ищет `.env` в cwd. Forgeplan хранит env-файл внутри workspace `.forgeplan/`. Функция `forgeplan_core::workspace::find_workspace(start)` уже существует в core и умеет walk-up от любой директории, но в CLI entrypoint не используется.

MCP-server стартует через CLI subcommand `forgeplan serve` → проходит через main.rs → страдает от того же бага.

## Proposed Solution

Добавить helper `load_workspace_env()` в `main.rs`, вызываемый перед `dotenvy::dotenv()`:

```rust
fn load_workspace_env() {
    if let Ok(cwd) = std::env::current_dir()
        && let Some(ws) = forgeplan_core::workspace::find_workspace(&cwd)
    {
        dotenvy::from_path(ws.join(".env")).ok();
    }
}
```

Precedence (от высшего к низшему):
1. Shell env vars (уже `export`'нутые) — dotenvy не override
2. Workspace `.forgeplan/.env` — walk-up от cwd
3. Cwd `.env` — fallback для не-workspace сценариев

Не требует breaking change, не меняет поведение для кейсов где shell env vars уже установлены.

## Acceptance Criteria

- **AC-1**: `forgeplan route "..."` из корня workspace — Level 2 (LLM) при заполненном `.forgeplan/.env`
- **AC-2**: `forgeplan route "..."` из subdir (e.g. `crates/forgeplan-core/`) — тоже Level 2 (walk-up работает)
- **AC-3**: `forgeplan route "..."` из `/tmp` (outside workspace) — Level 0 fallback без crash
- **AC-4**: Shell-переданный env var имеет приоритет над workspace `.env` (dotenvy не override)
- **AC-5**: MCP server (`forgeplan serve`) наследует env var из workspace `.env` — MCP tools работают с LLM без ручного `env` блока в `.mcp.json`
- **AC-6**: Все существующие тесты PASS (1405+)

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| PROB-022 | Problem | informs (brownfield onboarding — этот баг раздражал users) |
| ADR-003 | ADR | informs (markdown source of truth — .forgeplan/.env не в git, но канонично для workspace) |


