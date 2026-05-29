---
depth: standard
id: ADR-015
kind: adr
links:
- target: PRD-078
  relation: based_on
- target: RFC-010
  relation: refines
- target: PROB-072
  relation: based_on
- target: PROB-067
  relation: informs
- target: PROB-073
  relation: informs
- target: ADR-003
  relation: informs
status: draft
title: MCP workspace resolution — error on multi-worktree detect
---

## Context

PROB-072 zafilen 2026-05-20 после dogfood feedback одного из активных пользователей: его subagent в git worktree вызывает `forgeplan_new` через MCP, projection лендится в main repo, Guardian не находит файл в worktree и отправляет переделывать. Loop неубиваем без core fix потому что MCP server фиксирует CWD на startup (`crates/forgeplan-mcp/src/main.rs:11`: `let cwd = std::env::current_dir()?;`).

Multi-worktree usage больше не edge case — этот user запускает **19 worktrees параллельно** во время sprint, и мы сами в команде используем worktrees per parallel teammate (см. memory feedback-use-worktrees-per-parallel-worker).

Решение принималось через два reasoning cycles:

1. **ADI cycle** (`forgeplan reason PROB-072`, gemini-3.1-pro-preview, 2026-05-22) — рекомендовал H1 (per-call `workspace` параметр) + H2 (`FORGEPLAN_WORKSPACE` env var) с resolution chain. Отверг H3 (git autodetect) как Low confidence из-за per-session MCP initialize limit.

2. **FPF Evaluate cycle** (2026-05-22) — после ADI выявил критический risk: «если agent забыл `workspace` parameter — silent fallback на main repo, та же PROB-072 опять, только без сигнала». Оцениваемые опции для адресации risk: B (stderr warning), C (plugin layer), D (stderr + opt-in strict), D' (response payload warning + opt-in strict), E (error on detect), F (noop).

Empirical test (16 дней Claude Code MCP логов forgeplan, 442 строки) показал что **stderr from forgeplan-mcp НЕ surface'ится** в Claude Code UI или логи. Это инвалидировало B и D как proven broken. F-G-R scoring оставшихся: E (3/3/2, Trust 0.85) > D' (3/2/2, Trust 0.80).

Cross-refs: PRD-078 (полный design + ACs), RFC-010 (implementation phases), PROB-072 (signal), PROB-067 (concurrent lock race — related surface).

## Decision

**Selected**: двухслойный механизм —
1. **H1 (PRIMARY)**: per-call `workspace` параметр на mutating MCP tools + `FORGEPLAN_WORKSPACE` env var (H2) с resolution chain `param → env → cwd`. Это **основной** механизм, закрывающий PROB-072: subagent в worktree передаёт `workspace=<свой worktree>`, projection лендится правильно.
2. **Option E (BEST-EFFORT NET)**: error response `-32602` когда detection видит multi-worktree env И ни param, ни env не переданы. Это **подстраховка** от silent fallback в подмножестве случаев (см. Known Limitation ниже), не самостоятельная гарантия.

**Why this split (honest re-assessment 2026-05-29, post-implementation)**:

Изначально ADR формулировал Option E как ГЛАВНУ�ю anti-silent-fallback гарантию. После реализации и e2e-тестов это оказалось **переоценкой**. Реальная картина:

1. **H1 (param) — настоящий fix**. Production-сценарий PROB-072 (Claude Code запускает MCP server в main repo, subagent работает в worktree, процесс общий) закрывается ТОЛЬКО тем, что subagent передаёт `workspace` param. Detection (Option E) здесь **не срабатывает** — cwd процесса сервера заморожен на main repo, а main repo сам по себе не «linked worktree», поэтому `git-common-dir == show-toplevel` → detection возвращает false → gate молчит.

2. **Option E ловит более узкий случай**: когда сам MCP server запущен с cwd внутри worktree (e.g. dev запускает `forgeplan-mcp` из worktree-папки, или per-worktree spawn). Тогда detection срабатывает и error не даёт молча писать не туда. AC-3 e2e тест проверяет именно этот случай (создаёт реальный worktree, запускает сервер с cwd внутри, ждёт -32602).

3. **Proven channel**. MCP error -32602 response path доказан работающим (lance / artifact-not-found errors доходят до user в чат через JSON-RPC `error` поле). Канал реальный, не theoretical.

4. **Полное покрытие требует cooperation от agent runtime** — agent ДОЛЖЕН передавать `workspace` param. Это не недостаток дизайна, а граница ответственности: core предоставляет механизм (param) + подстраховку (detection net), agent framework отвечает за передачу param. Plugin-layer workaround пользователя (forgeplan-marketplace `a9a825c`) как раз это и делает — и теперь может переехать на стандартный `workspace` param вместо dual-location verify hook.

## Alternatives Considered

| Option | Verdict | Why |
|--------|---------|-----|
| A — `workspace` обязателен всегда | Rejected | Breaks backward compat для single-worktree users (нарушает hard constraint PROB-072). Требует major version bump. |
| B — Warning через stderr + detection | Rejected | Empirically broken — stderr from forgeplan-mcp НЕ surface'ится в Claude Code (16 дней логов, 0 строк WARN). User не видит warning → silent fallback продолжается → false sense of security. |
| C — Push to plugin/agent layer | Rejected | Нарушает principle decomposition (proper fix принадлежит core, не каждому plugin layer-у). Дублирует существующий user workaround. Не generalizes на другие agent frameworks (Claude Code, OpenCode, Cursor — разные обходы). |
| D — Stderr warning + opt-in strict env | Rejected | Default mode (stderr) broken по той же причине что B. Strict-only часть превращается в opt-in A. |
| D' — Response payload warning + opt-in strict | Rejected as primary, kept as fallback | F-G-R close to E (0.80 vs 0.85) но проигрывает по «failure visibility». Soft signal через 6 месяцев деградирует. Удержан как Growth Vision PRD-078: config toggle `workspace_mismatch_policy: error \| warn`. |
| **E — Error on detect + opt-in strict env** | **Chosen as NET, not primary** | Подстраховка от silent fallback в случае server-cwd-in-worktree + smart detection sparing single-worktree users + proven error channel. Primary fix — H1 (param). См. Decision + Known Limitation для точной роли. |
| F — Noop, document как known limitation | Rejected | Не решает PROB-072. Users с забытым `workspace` param получают тот же drift который и привёл к filing. Через год тот же ticket. |

## Consequences

### Positive

- Корневая причина PROB-072 закрыта для production-сценария через H1 (agent передаёт `workspace` param → projection лендится в правильный worktree)
- Silent fallback дополнительно перехватывается Option E net в случае server-cwd-in-worktree (см. Known Limitation для точной границы покрытия)
- Backward compatibility сохранена для single-worktree workflows (detection не срабатывает → no error)
- Error message содержит actionable suggestion (auto-detected правильный path) — agent может self-correct в следующем вызове
- Plugin-layer workaround пользователя (forgeplan-marketplace `a9a825c` — dual-location verify, 11 agent definitions с Materialize sections) может быть **депрекейтнут** после v0.33 release
- Mental model для пользователей минимальный: single-worktree — ничего не меняется; multi-worktree — нужен `workspace` параметр
- Per-workspace lock natural'но мигрирует на resolved path — попутно закрывает PROB-067 race scenarios

### Negative (trade-offs)

- Latency cost git rev-parse на каждом mutating MCP call. Бenchmark (R-1 evidence gap) запланирован в PROB-073 sprint. Если >5ms p95 — mitigation через session-level cache в Growth Vision
- Cross-worktree convenience use case (agent в worktree-A осознанно пишет в main repo) теперь требует **явного** `workspace=<main path>` параметра вместо implicit silent fallback. Это явный gesture — допустимо, но less convenient
- Сложность тестов растёт — integration tests требуют `git worktree add` setup в `tempdir`, может быть flaky на ограниченных CI runners
- Detection logic зависит от `git` binary в PATH — graceful fallback (assume single-worktree если git not installed) добавляет ещё один edge case в test matrix

### Risks

- **R-1**: latency cost git rev-parse превышает NFR-001 budget (<5ms p95) — mitigation: caching layer в Growth Vision. Evidence: bench в PROB-073 sprint
- **R-2**: MCP клиент (не Claude Code) не пропускает `FORGEPLAN_WORKSPACE` env var до child process — strict CI mode ломается в OpenCode/Cursor. Mitigation: short evidence test на OpenCode/Cursor до v0.33 release
- **R-3**: Error message wording не actionable достаточно — mitigation NFR-003 + AC-3 в PRD-078 enforce wording с auto-detected suggested path

### Known Limitation (Option E detection coverage)

**Option E detection НЕ покрывает главный production-сценарий PROB-072.** Это сознательно принятая граница, не баг:

| Сценарий | Кто запускает MCP server cwd | Detection (Option E) | Чем закрыт |
|----------|------------------------------|----------------------|------------|
| **Production** (главный): Claude Code в main repo, subagent в worktree, общий MCP процесс | main repo | ❌ не срабатывает (cwd == main, не worktree) | ✅ H1 — agent передаёт `workspace` param |
| **Dev/per-worktree spawn**: MCP server запущен с cwd внутри worktree | worktree | ✅ срабатывает → -32602 | ✅ Option E net + H1 |
| **Single-worktree** (обычный): всё в одном репо | main repo | ❌ не срабатывает (правильно) | n/a — backward compat, никаких изменений |

**Вывод**: anti-silent-fallback гарантия достигается **комбинацией** H1 (agent передаёт param) + Option E (net для случая server-cwd-in-worktree). Полная защита от «agent забыл param» в production-сценарии **невозможна на стороне core** без cooperation от agent runtime — потому что core физически не знает в каком worktree «должен» работать subagent, если subagent сам этого не сообщил. Это корректная decomposition: core даёт механизм, agent framework отвечает за его использование. Документировать в user-facing docs (`docs/operations/MULTI-AGENT.ru.md`): «при multi-worktree пайплайнах каждый subagent ОБЯЗАН передавать `workspace`».

## Invariants

Что должно выполняться всегда независимо от реализации:

- ADR-003 file-first invariant сохраняется — projection всегда лендится в `.forgeplan/` worktree из которого пришёл вызов (не в shared cache, не в Lance index-only)
- Single-worktree workflow остаётся zero-config — пользователь не должен знать про `workspace` параметр пока работает в обычном repo
- Error response для multi-worktree+missing case **MUST** содержать auto-detected suggested path в message body — иначе error не actionable
- Per-workspace lock acquired ДО любого file write — concurrent writes в разные worktrees не делят lock
- `git` binary недоступен → assume single-worktree → no error (graceful fallback)

## Evidence Requirements

Что нужно измерить/доказать для активации этого решения:

- **EVID-NNN bench**: `git rev-parse` cost (cold + warm) измерен через criterion в PROB-073 sprint. Verdict: supports если p95 <5ms; weakens если 5-50ms (триггер caching); refutes если >50ms (триггер pre-emptive caching mandatory)
- **EVID-NNN integration**: AC-1..AC-5 integration tests PASS (worktree setup в tempdir, real git operations)
- **EVID-NNN backward compat**: cargo test --workspace count 3084 → ≥3084 PASS, 0 регрессий
- **EVID-NNN cross-client**: short manual test что `FORGEPLAN_WORKSPACE` env var реально пропускается через OpenCode/Cursor spawn (если planned для эти клиентов; иначе documented limitation)

## Valid Until

**Дата**: `valid_until: 2026-11-22` (frontmatter — 6 месяцев)

**Обоснование TTL**: Решение фундаментальное (затрагивает MCP tool schema), но базируется на текущем состоянии Claude Code MCP stderr handling. Если Claude Code в течение 6 месяцев начнёт surface'ить stderr в UI — re-evaluate D'/E balance. Также если появится MCP protocol-native workspace negotiation (workspaceFolders extension) — пересмотреть H3.

**Refresh Triggers** (когда пере-оценить досрочно):
- Claude Code добавляет stderr-to-UI surface (D' становится viable)
- MCP protocol gains native workspace negotiation (H3 переоценивается)
- User feedback indicates cross-worktree convenience use case критичен (toggle config `workspace_mismatch_policy: warn` migration trigger)
- Latency bench показывает >50ms overhead (mandate session cache, possibly invalidates per-call detection model)

## Pre-conditions (чеклист ДО реализации)

- [x] PROB-072 documented с reproduction steps
- [x] ADI cycle complete (3+ hypotheses generated, recommended H1+H2)
- [x] FPF Evaluate cycle complete (E winner over D'/B/C/D/F)
- [x] Empirical evidence: stderr surface test → negative result
- [x] PRD-078 draft validated (PASS, 0 errors)
- [x] RFC-010 draft validated (PASS, 0 errors)
- [ ] Latency bench infra ready (создаётся в PROB-073 sprint Day 4-5)
- [ ] User approval для feature branch + work plan

## Post-conditions (Definition of Done)

- [ ] All 7 FRs (FR-001..007) реализованы в коде
- [ ] All 4 NFRs (NFR-001..004) измерены и PASS
- [ ] All 5 ACs (AC-1..AC-5) integration tests PASS
- [ ] cargo test --workspace count: 3084 → ≥3084+N (no regressions, new tests added)
- [ ] Adversarial audit complete (≥2 agents, all CRITICAL/HIGH findings closed)
- [ ] Evidence packs linked: bench (R-1), integration tests, backward compat regression
- [ ] PRD-078 activated (draft → active), R_eff > 0
- [ ] PR merged в dev с user approval
- [ ] CHANGELOG обновлён для v0.33 release
- [ ] User уведомлён о возможности deprecate plugin-layer workaround

## Admissibility

Что НЕ допускается в рамках этого решения:

- **NOT**: silent fallback на main repo при multi-worktree detected — должен быть explicit error
- **NOT**: detection через MCP `initialize` workspaceFolders без explicit signal — отвергнут как H3
- **NOT**: warning-only response (без error) когда multi-worktree detected — D' fallback design, не MVP
- **NOT**: каeshing detection result globally — per-call evaluation (session cache OK как mitigation если bench требует)
- **NOT**: запуск `git` binary с user-controlled input напрямую — все git invocations через `std::process::Command` с фиксированными args (`rev-parse --git-common-dir`, `rev-parse --show-toplevel`), без shell expansion

## Rollback Plan

**Triggers** (когда откатывать):
- Production выявляет breaking case для single-worktree workflow (despite tests PASS)
- Latency overhead в production >100ms per call (10× over budget) и caching не помогает
- Cross-worktree convenience use case оказывается критичен для >1 user в течение месяца после release

**Steps** (шаги отката):
1. Feature flag `FORGEPLAN_DISABLE_WORKTREE_DETECT=1` env var — short-term escape для аффектed users (добавить в Phase 2 как defensive net)
2. Если flag недостаточен: `git revert` PR merge commit на dev. Re-release как hotfix v0.33.1
3. Re-evaluate с D' (config toggle defaulting to warn) — отдельный sprint, supersede ADR-015

**Blast Radius**: revert затрагивает MCP server только (CLI workflow unchanged). Users на v0.33.0 → v0.33.1 hotfix получают rollback transparently через `brew upgrade`. Plugin-layer workaround у user'а должен оставаться функционален (не зависит от core поведения).

## Weakest Link

R_eff = min(evidence_scores). Самое слабое звено решения:

- **Detection reliability** в edge cases (git submodules, nested worktrees, symlinked .git, bare repos) — наша detection logic покрывает common case (worktree через `git worktree add`), но edge cases могут давать false positives/negatives. Mitigation: AC-3 integration tests должны включать минимум 3 edge case scenarios. Без этого weakest link оценивается ~0.6.

После integration test coverage scenarios weakest link поднимется к 0.85+.

## Affected Files

| File | Baseline Hash |
|------|---------------|
| `crates/forgeplan-mcp/src/convert.rs` | TBD (snapshot перед Phase 1) |
| `crates/forgeplan-mcp/src/server.rs` | TBD |
| `crates/forgeplan-core/src/workspace/init.rs` | TBD |
| `crates/forgeplan-core/src/workspace/lock.rs` | TBD |

## AI Guidance

Правила для AI-агентов при работе с этим решением:

- **Prefer this pattern in all new MCP mutating tools** — добавление нового `forgeplan_*` mutating tool должно использовать `resolve_workspace(params.workspace.as_deref())` chain, не `self.workspace_root` напрямую
- **Do not introduce alternative routing approaches without new RFC** — никаких альтернативных workspace resolution mechanisms (auto-detect, config files, parent process inheritance) без supersede ADR-015
- **When generating code, assume Option E is binding** — multi-worktree detection + error response, не warning. Если silent fallback кажется acceptable trade-off в new context — escalate с reasoning
- **If a task conflicts with this ADR, raise it explicitly** — например, если новая фича требует cross-worktree default behaviour, документировать в issue и предложить supersede vs feature toggle

## Implementation Plan

### Phase 0: Foundation

- [x] **0.1** PROB-072 documented (2026-05-20)
- [x] **0.2** ADI cycle complete (2026-05-22)
- [x] **0.3** FPF Evaluate cycle complete (2026-05-22)
- [x] **0.4** PRD-078 + RFC-010 + ADR-015 drafted (2026-05-22)

### Phase 1: Resolution chain + params (cross-ref RFC-010 Phase 1)

- [ ] **1.1** Worker W1 — `workspace: Option<String>` в NewParams/LinkParams/UpdateParams
- [ ] **1.2** Worker W1 — `resolve_workspace` chain в server.rs
- [ ] **1.3** Worker W1 — replace `self.workspace_root` в mutating handlers
- [ ] **1.4** Worker W1 — unit tests resolution chain priority

### Phase 2: Detection + error response (cross-ref RFC-010 Phase 2)

- [ ] **2.1** Worker W2 — `detect_multi_worktree` в forgeplan-core/src/workspace/init.rs
- [ ] **2.2** Worker W2 — error-on-detect path в `resolve_workspace`
- [ ] **2.3** Worker W2 — integration test AC-3

### Phase 3: Lock refactor + e2e (cross-ref RFC-010 Phase 3)

- [ ] **3.1** Worker W3 — per-workspace lock refactor
- [ ] **3.2** Worker W3 — `resolved_workspace` + `resolved_via` в response payload
- [ ] **3.3** Worker W3 — e2e tests AC-1, AC-4, AC-5 (AC-2 уже covered регрессией)

## Implementation Log

<!-- Add wave entries as phases complete -->

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| PRD-078 | PRD | based_on (этот ADR — decision record для PRD-078 design) |
| RFC-010 | RFC | refines (RFC-010 — phased implementation plan этого решения) |
| PROB-072 | Problem | based_on (parent signal triggering decision) |
| PROB-067 | Problem | informs (per-workspace lock refactor — Phase 3) |
| PROB-073 | Problem | informs (latency budget shared, bench evidence cross-link) |
| ADR-003 | ADR | preserves (file-first invariant сохраняется) |









