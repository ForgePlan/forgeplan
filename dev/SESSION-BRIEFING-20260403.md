# Session Briefing — Forgeplan (2026-04-03)

> Этот файл — контекст для нового чата. Прочитай его первым.

## Кто мы и что делаем

**Forgeplan** — Rust CLI + MCP server для ведения проектов через структурированные артефакты (PRD, RFC, ADR, Epic, Evidence...) с quality scoring (R_eff), semantic search, dependency graph и FPF reasoning.

- **v0.12.0** released, 56 CLI commands, 37 MCP tools
- Язык документации: русский. Код: Rust с английскими идентификаторами.
- Полная методология в `CLAUDE.md` (обязателен к прочтению)

## Что было сделано в последней сессии (Sprint 8)

### PR #95 — `fix/graph-integrity-prob020` (ждёт merge в dev)

10 багов исправлено + 6 improvements. Ветка `fix/graph-integrity-prob020`, PR открыт.

**Баги:**
1. `blocked`/`order` считали deprecated/superseded блокирующими (BUG-1, P1)
2. `delete` не каскадно удалял relations — фантомные ссылки (BUG-2, P1)
3. `unlink` не мог чистить фантомы (BUG-2b)
4. `kahn_sort` parameter name `active_ids` не соответствовал семантике (CC1)
5. Case-sensitivity mismatch в SQL filter vs Rust (CC2)
6. TOCTOU race в cascade delete (CC3)
7. O(n²) в order.rs (U4)
8. Double table scan в delete.rs (U5)
9. `route ""` принимал пустой input (S1)
10. Memory артефакты ложно показывались как orphan (S4)

**Improvements:**
- DRY `common::resolved_ids()` helper
- `delete_relations_for_artifact` добавлен в `RelationStorage` trait + InMemory + Lance
- 2 новых MCP tools: `forgeplan_blocked` + `forgeplan_order`
- `validate_id_for_filter()` whitelist (anti-injection)
- Memory excluded from orphan detection
- E2E test plan: `dev/E2E-TEST-PLAN.md`

**Аудит:** 5-agent panel (logic 7/10, rust 8/10, security 7/10, arch 7/10, test 6/10) — все findings исправлены.

### Результаты
- **740 unit tests**, 0 failures, 0 warnings
- **83 E2E commands** протестированы (clean tempdir + real workspace), 0 failures
- **131 artifacts**, 0 blind spots, 0 orphans, 0 stale
- PROB-020 active, EVID-047 R_eff=1.00
- Installed: `forgeplan 0.12.0` в `~/.cargo/bin/forgeplan`

## Текущее состояние

### Git
- Ветка: `fix/graph-integrity-prob020`
- PR #95 открыт → dev
- `TODO.md` modified (не закоммичен — обновлены Stats и Sprint 8)
- Untracked files: backups, dev/, docs/guides/ — не критичные

### Forgeplan Health
```
Artifacts: 131 | Blind spots: 0 | Stale: 0 | At risk: 0
Blocked: 5 (все реальные — draft deps)
Ready to work: 35
```

### 5 Blocked (реальные — draft зависимости)
| Artifact | Blocked by | Reason |
|----------|-----------|--------|
| ADR-001 | EPIC-002, NOTE-009 | EPIC-002 draft, NOTE-009 draft |
| PRD-013 | RFC-001 | RFC-001 draft |
| PRD-015 | RFC-002 | RFC-002 draft |
| RFC-002 | EPIC-002 | EPIC-002 draft |
| RFC-003 | NOTE-015 | NOTE-015 draft |

## Что нужно сделать дальше

### 1. Merge PR #95

```bash
# Вернуться на dev и merge
git checkout dev && git pull origin dev
gh pr merge 95 --merge   # merge commit, НЕ squash (по CLAUDE.md)
git pull origin dev
```

### 2. Полное E2E тестирование на обновлённой версии

E2E test plan лежит в `dev/E2E-TEST-PLAN.md` — 158 команд, 11 waves. В Sprint 8 прогнали 83 команды. Осталось:

- **Wave 8** (10 команд) — LLM commands: generate, reason, decompose, capture (нужен GEMINI_API_KEY)
- **Wave 11d** (5 команд) — Edge cases: corrupt data, stress test (50 artifacts)
- **Wave 11c** (3 команды) — No-workspace edge cases

### 3. Записать Rust integration tests в код

Сейчас E2E прогонялись вручную (shell). Нужно автоматизировать ключевые сценарии:

```
crates/forgeplan-cli/tests/
  cli_integration_test.rs      ← уже 73 теста
  e2e_cascade_delete_test.rs   ← NEW: delete cascades relations
  e2e_blocked_resolved_test.rs ← NEW: deprecated doesn't block
```

### 4. Продолжить с того, что планировали ДО Sprint 8

Изначальная цель сессии была:
1. ~~Выгрузить gaps/blocked/tree~~ — DONE
2. ~~Составить E2E test plan~~ — DONE (dev/E2E-TEST-PLAN.md)
3. ~~Прогнать на workspace и tempdir~~ — DONE (83 commands, 0 fail)
4. ~~Найти баги~~ — DONE (10 найдено, 10 исправлено)

Следующие задачи из TODO.md:
- **Backlog items** из audit (deferred):
  - MCP parity gaps (health --json, validate --json format)
  - Driver trait: more methods to trait
- **Draft artifacts** (30 штук) — review и deprecate/activate устаревшие
- **EPIC-002** (v2.0) — стратегическое планирование

### 5. Known issues (не баги, но стоит знать)

- `validate --json` возвращает list (не dict) — by design, один элемент на артефакт
- `health --json` ключ `total` (не `total_artifacts`)
- `stale` status НЕ в `resolved_ids` — by design, stale блокирует (нужен renew)
- 2 memory artifacts (`mem-api-prefix...`, `mem-postgresql...`) — из другого проекта E2E, не orphan

## Быстрый старт для нового чата

```bash
# 1. Восстановить контекст
forgeplan health
cat dev/SESSION-BRIEFING-20260403.md

# 2. Merge PR если ещё не merged
gh pr view 95 --json state

# 3. Продолжить E2E или следующую задачу
cat dev/E2E-TEST-PLAN.md
forgeplan gaps
forgeplan blocked
```

## Команды для reference

```bash
forgeplan health              # общее состояние
forgeplan blocked             # граф зависимостей
forgeplan tree --depth 2      # иерархия артефактов
forgeplan gaps                # compliance gaps
forgeplan blindspots          # без evidence
forgeplan search "query"      # smart search
forgeplan route "task"        # определить depth
forgeplan validate PRD-XXX    # проверить качество
forgeplan score PRD-XXX       # R_eff scoring
```
