---
title: Hint Contract — чтение вывода Forgeplan
description: Как агенты читают детерминированные next-action подсказки Forgeplan в CLI text, JSON и MCP ответах
---

> Статус: **Active** с Forgeplan v0.25.0 (PRD-071, 2026-04-27)

В Forgeplan v0.25.0 введён **унифицированный 5-правильный hint contract** — любой агент (Claude Code, Cursor, Windsurf, custom orchestrators) может читать вывод без повторного открытия методологии каждый раз. Эта страница — канонический агент-ориентированный референс.

## Зачем это нужно

Forgeplan — методологический движок. Каждая команда/MCP-вызов — один шаг в более длинном workflow (Shape → Validate → Code → Evidence → Activate). Когда агенты не знают что делать дальше, они:

- Перечитывают CLAUDE.md чтобы заново открыть методологию
- Гадают, иногда галлюцинируют
- Зацикливаются на одном шаге

Каждое из этого тратит токены и рискует корректностью. Контракт убирает неоднозначность — каждый вывод несёт явный детерминированный next-action.

## Контракт из 5 правил

Каждый вывод Forgeplan, независимо от surface, удовлетворяет:

1. **PRESENCE** — каждый ответ либо emit'ит next-action, либо явно терминальный. Тихих gaps не бывает.
2. **ACTIONABILITY** — next-action это полная copy-paste команда с реальными ID, никогда не фрагмент или placeholder.
3. **DETERMINISM** — одно и то же состояние всегда производит одинаковую hint строку. Без рандома.
4. **CONDITIONALITY** — подсказки появляются только когда actionable. Терминальные states emit'ят `Done.` вместо фиктивного "all done!".
5. **CONSISTENCY** — text и JSON renderings несут одинаковый семантический контент. CLI отражает MCP semantics.

## 5 hint маркеров

| Маркер | Когда emit'ится | Действие агента |
|---|---|---|
| `Next: <full command>` | Основное действие — рекомендуемый следующий шаг | Выполнить exactly как написано |
| `Or: <command>` | Альтернатива когда primary блокирован (например claim занят) | Использовать только если primary fails |
| `Wait: <condition>` | Async/TTL state | Retry после condition |
| `Done.` | Workflow завершён (terminal) | Перейти дальше, не зацикливаться |
| `Fix: <command>` | Error remediation (paired с `Error:`) | Сразу выполнить fix команду |

## Где читать hint

| Surface | Местоположение | Формат |
|---|---|---|
| **CLI text (success)** | последние строки stdout | `Next: <full command>` плюс опциональный rationale |
| **CLI text (error)** | после `Error:` строки | `Fix: <full command>` |
| **CLI JSON** | top-level field | `{"_next_action": "<command>" \| null, ...}` |
| **MCP success** | top-level field | `_next_action: "<command>" \| null` |
| **MCP error** | error data field | `error.data._next_action: "<command>"` |

**Особый случай**: `forgeplan list --json` и `forgeplan tree --json` сохраняют bare-array stdout (backward compat для `jq '.[]'` consumers). Hint emit'ится в **stderr** в JSON режиме.

## Хорошие vs. плохие подсказки

### Хорошо ✅

```
Next: forgeplan score PRD-001
  R_eff is 0 — link evidence to enable activation
```
Specific, full command, реальный ID, rationale объясняет *почему*.

```
Next: forgeplan dispatch --agents 3
Or: forgeplan claim PRD-054 --agent worker-2 --ttl-minutes 30
```
Одно primary действие, один явный fallback.

```
Error: Direct status change to 'active' is not allowed.
Fix: forgeplan activate PRD-001
```
Error имеет clear, executable remediation.

### Плохо ❌ (паттерны до v0.25.0 — не должны появляться в v0.25.0+ выводе)

```
suggested next: adi
```
Bare word, не команда. Агент должен догадаться.

```
Try a longer window: --since-hours 720
```
Фрагмент, не full command.

```
Either work on a different artifact, wait for TTL expiry,
or ask the orchestrator to force-release.
```
Три опции, ни одна не выбрана как primary. Парадокс выбора.

## Reading protocol для агента

Когда агент получает любой Forgeplan вывод:

1. **Найди next-action**.
   - CLI text: scan для `Next:`, `Fix:`, `Wait:`, или `Done.` строки
   - CLI JSON: read `_next_action` field (или stderr `Next:` для list/tree)
   - MCP: read `_next_action` field response'а
2. **Выполни primary если present**.
   - Если `Next:` или `Fix:` — выполни команду **exactly как написано**
   - Не парафразируй, не подставляй placeholders
   - Не разбивай на несколько команд
3. **Используй `Or:` только если primary blocks**.
4. **На `Wait:`, retry после condition**.
5. **На `Done.`, workflow complete** — переходи к следующей задаче, не зацикливайся.
6. **На no hint и не terminal — сообщи о violation контракта**. Это bug в Forgeplan. Не импровизируй.

## Чего НЕ делать

1. **Не парафразируй hint** — full команда дана не просто так
2. **Не комбинируй `Next:` + `Or:`** в одном call — выбери одно
3. **Не игнорируй `Done.`** — явный terminal сигнал
4. **Не подставляй `EVID-NNN` placeholders** — сначала запусти `forgeplan_new evidence`, потом используй real ID
5. **Не паникуй на `Wait:`** — async/TTL это нормально; просто retry

## Практические workflow паттерны

### Pattern A: Shape → Validate → Activate

```
forgeplan_route("add OAuth login")
  → Next: forgeplan new prd "<title>"

forgeplan_new(kind: "prd", title: "OAuth login support")
  → Next: forgeplan validate PRD-042

forgeplan_validate("PRD-042")
  → Next: forgeplan activate PRD-042   (если PASS)
  → Fix: forgeplan validate PRD-042    (если errors)

forgeplan_activate("PRD-042")
  → Done.
```

### Pattern B: Recovery после error

```
forgeplan_activate("PRD-042")
  → Error: No evidence linked
  → Fix: forgeplan validate PRD-042

forgeplan_validate("PRD-042")
  → Result: PASS
  → Next: forgeplan score PRD-042
```

### Pattern C: Multi-agent dispatch

```
forgeplan_dispatch(--agents 3)
  → Next: forgeplan claim PRD-054 --agent worker-1 --ttl 30
  → Or:   forgeplan list --status draft

forgeplan_claim("PRD-054")
  → Error: Already held by worker-2
  → Or: forgeplan release PRD-054 --force
```

## Drift prevention

Контракт enforced через:

1. **Integration test** `crates/forgeplan-cli/tests/hint_contract.rs` — 36 тестов, fails CI если команда без contract marker
2. **Audit script** `scripts/audit-hints.sh` — coverage метрика, target 100%, currently 100% (70/70 commands)

## Quick reference card

```
┌─────────────────────────────────────────────────────────┐
│  FORGEPLAN HINT CONTRACT (v0.25.0+)                     │
├─────────────────────────────────────────────────────────┤
│  Next:  <command>   → execute as-is                     │
│  Or:    <command>   → fallback if primary blocks        │
│  Wait:  <condition> → retry after condition             │
│  Done.              → workflow complete, move on        │
│  Fix:   <command>   → error remediation (with Error:)   │
├─────────────────────────────────────────────────────────┤
│  Read from: stdout (text), _next_action (JSON/MCP)      │
│  Special:   list/tree --json → hint on stderr           │
└─────────────────────────────────────────────────────────┘
```

## Для пользователей плагина

Marketplace плагин **`forgeplan-workflow` v1.5.0+** учит Claude Code агентов читать эти маркеры автоматически. После установки:

```
/plugin marketplace update ForgePlan-marketplace
```

Твой агент будет читать `Next:`/`Fix:` подсказки в `/forge-cycle`, `forge-advisor` агенте и methodology skill — без manual configuration.

## Связанное

- **Forgeplan v0.25.0 release** — первая версия с контрактом ([CHANGELOG](/ru/docs/changelog/))
- **Marketplace плагин forgeplan-workflow v1.5.0** — agent-side awareness layer
- **Lifecycle model** ([читать дальше](/ru/docs/methodology/lifecycle/)) — подсказки интегрируются с переходами status (draft → active → terminal)
