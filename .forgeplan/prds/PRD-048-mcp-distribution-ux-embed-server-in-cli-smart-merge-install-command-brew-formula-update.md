---
depth: standard
id: PRD-048
kind: prd
links:
- target: PROB-037
  relation: based_on
status: draft
title: MCP distribution UX — embed server in CLI, smart-merge install command, brew formula update
---

# PRD-048: MCP distribution UX — embed server in CLI, smart-merge install command, brew formula update

## Progress

```
Phase 1 Embed     ████████████████████████  3/3  (100%)  ✅ already in CLI as `serve`
Phase 2 Install   ░░░░░░░░░░░░░░░░░░░░░░░░  0/5  (  0%)
Phase 3 Brew      ████████████████████████  2/2  (100%)  ✅ formula already correct
Phase 4 QA        ░░░░░░░░░░░░░░░░░░░░░░░░  0/3  (  0%)
─────────────────────────────────────────────────
TOTAL                                       5/13 ( 38%)
```

**Pivot 2026-04-16**: после re-investigation Phase 1 + 3 уже сделаны — `forgeplan serve` существует, brew binary работает как MCP server из коробки. Реальный scope сократился до Phase 2 (install command) + Phase 4 (cross-platform QA).

---

## Problem

**Кому плохо**: новый юзер Forgeplan, который ставит CLI через `brew install forgeplan` и хочет подключить MCP сервер к Claude Code (или Cursor/Windsurf).

**Что происходит**:
1. `brew install forgeplan` ставит только CLI binary — `forgeplan-mcp` отдельный binary, в formula его нет
2. CLI не имеет subcommand `mcp` — стандартная конвенция (`mytool serve`) нарушена
3. Юзер должен: `git clone forgeplan` → `cargo build --release -p forgeplan-mcp` → вручную править `.mcp.json` с абсолютным путём
4. Time-to-first-MCP-call ~30 минут вместо ожидаемых <2 минут

**Impact**: каждый новый юзер натыкается на эту стену. Подтверждено 2026-04-16 на собственном опыте — `brew install forgeplan@0.18.0` → MCP не работает → ручная сборка. Конверсия в active MCP user стремится к нулю для тех, кто не готов читать исходники.

Деталь: `PROB-037`.

## Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| Brew installer | Ставит forgeplan через brew, не разработчик Rust | Не должен компилировать из исходников |
| AI agent user | Хочет подключить Forgeplan MCP к Claude Code / Cursor / Windsurf | Не должен править JSON руками |
| Course student | Пробует Forgeplan по туториалу | Не должен ломать flow на setup-стене |

## Goals

| ID | Criterion | Metric | Current | Target |
|----|-----------|--------|---------|--------|
| SC-1 | Single binary distribution | Binaries в brew formula | 1 (CLI only) | 1 (CLI с embedded MCP) |
| SC-2 | MCP server subcommand | `forgeplan mcp serve` exit 0 | does not exist | works, JSON-RPC handshake OK |
| SC-3 | Auto-install command | `forgeplan mcp install --client claude` | does not exist | smart-merges `.mcp.json` |
| SC-4 | Time-to-first-MCP-call | Минут от brew install до working MCP | ~30 | <2 |
| SC-5 | Idempotency | Повторный запуск install не ломает env | N/A | env preserved, command/args replaced |
| SC-6 | Multi-client support | Поддерживаемые MCP clients | 0 (manual) | claude, cursor, windsurf |
| SC-7 | E2E test | brew install → install → handshake | manual | automated test in CI |
| SC-8 | Backward compat | Existing `.mcp.json` со старым путём | breaks | continues to work |

## Non-Goals

- Не делаем GUI для install (CLI достаточно)
- Не плодим отдельный `forgeplan-mcp` brew package (single binary preferred)
- Не делаем auto-update `.mcp.json` при апгрейде версий (юзер вызывает install сам)
- Не поддерживаем Windows-specific MCP setup в этой итерации (Linux + macOS only)
- Не реализуем `forgeplan mcp uninstall` (юзер удалит секцию руками или brew uninstall)

---

## Functional Requirements

| ID | Priority | Requirement | Journey |
|----|----------|-------------|---------|
| FR-001 | Must | CLI имеет subcommand `forgeplan mcp serve` который запускает MCP server (stdio) | Юзер запускает напрямую для debugging |
| FR-002 | Must | MCP server logic embedded в CLI binary — `forgeplan-mcp` остаётся как thin wrapper для backward compat | Brew ставит один binary |
| FR-003 | Must | CLI имеет subcommand `forgeplan mcp install --client <name>` | Юзер настраивает MCP за одну команду |
| FR-004 | Must | Поддерживаемые clients: `claude` (~/.mcp.json), `cursor` (.cursor/mcp.json), `windsurf` (.codeium/windsurf/mcp_config.json) | Multi-client support |
| FR-005 | Must | **Smart merge strategy** для `.mcp.json`: replace `command` + `args` (для version bumps), preserve `env` (для user customization), создать новую секцию если её нет | Идемпотентность + сохранение env |
| FR-006 | Must | Install детектит абсолютный путь к binary (`which forgeplan` или `std::env::current_exe`), вписывает в config | Юзер не указывает путь руками |
| FR-007 | Must | Install идемпотентна — повторный запуск даёт тот же результат, не дублирует секции | Safe to re-run |
| FR-008 | Should | `forgeplan mcp install --scope user|project` — выбор между ~/.mcp.json и ./.mcp.json | Контроль scope |
| FR-009 | Should | `forgeplan mcp install --dry-run` — показать что будет сделано, не записывать | Юзер видит diff перед commit |
| FR-010 | Should | Brew formula update — single binary в `bin/forgeplan` (без forgeplan-mcp) | Cleaner brew install |
| FR-011 | Could | `forgeplan mcp status` — показать в каких clients MCP настроен и работает ли | Diagnostic UX |
| FR-012 | Could | Auto-detect installed clients (Claude Code/Cursor present in standard locations) | Discovery UX |

---

## Technical Approach

**Embed strategy** (FR-001/FR-002):
- Переместить `crates/forgeplan-mcp/src/main.rs` логику в `crates/forgeplan-cli/src/commands/mcp.rs` как subcommand
- `forgeplan-mcp` binary остаётся как 5-строчный wrapper: `fn main() { forgeplan_cli::commands::mcp::run() }` — для backward compat существующих `.mcp.json`
- Все 47 MCP tools остаются в `forgeplan-mcp` lib (логика), CLI и thin wrapper оба её используют

**Smart merge** (FR-005, ключевое решение):
```rust
// Pseudocode
let existing = parse_json(&mcp_json_path)?;
let forgeplan_section = existing.get("mcpServers").get("forgeplan");

let new_section = if let Some(prev) = forgeplan_section {
    Section {
        command: detect_binary_path(),  // ALWAYS replace (version bumps)
        args: vec!["mcp".into(), "serve".into()],  // ALWAYS replace
        transport: "stdio".into(),  // ALWAYS replace
        env: prev.env.clone(),  // PRESERVE (user customization)
    }
} else {
    Section::default_with_path(detect_binary_path())
};

write_json_atomic(&mcp_json_path, merged)?;
```

**Multi-client paths** (FR-004):
| Client | Config path |
|--------|-------------|
| claude | `~/.claude.json` (mcpServers section) или `./.mcp.json` (project) |
| cursor | `~/.cursor/mcp.json` |
| windsurf | `~/.codeium/windsurf/mcp_config.json` |

**Brew formula** (FR-010):
- `Formula/forgeplan.rb`: `bin.install "forgeplan"` (только CLI, без `forgeplan-mcp`)
- Cargo workspace: keep `forgeplan-mcp` crate как thin wrapper для backward compat
- cargo-dist обновить чтобы пакетировать только `forgeplan` в release archive

---

## Dependencies

| Dependency | Type | Status |
|-----------|------|--------|
| `serde_json` | External | Already in Cargo.toml |
| `clap` subcommands | External | Already used |
| `directories` crate (XDG paths) | External | Add to deps |
| Brew formula access | Process | Tap maintainer (we own it) |
| cargo-dist config | Internal | Update `cargo-dist.toml` |

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Smart merge сломает кастомизированный env | High | Unit tests на 10+ scenarios (empty/full/missing/extra fields) + `--dry-run` |
| Embed раздует CLI binary размер | Medium | Замерить до/после; release profile уже агрессивный (43MB → ожидается ~50MB) |
| Backward compat `.mcp.json` со старым путём | Medium | Wrapper binary `forgeplan-mcp` остаётся, продолжает работать |
| Разные пути для разных clients ломаются | Medium | E2E test per client + version detection |
| Brew formula обновление не доходит до tap | High | CI pipeline для tap update + manual verification post-release |

---

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-037 | based_on |
| PRD-024 | based_on |
| EPIC-002 | belongs_to |

