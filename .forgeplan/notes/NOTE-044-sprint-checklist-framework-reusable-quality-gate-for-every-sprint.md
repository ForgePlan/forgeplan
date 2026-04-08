---
depth: tactical
id: NOTE-044
kind: note
status: active
title: Sprint Checklist Framework — reusable quality gate for every sprint
---

# NOTE-044: Sprint Checklist Framework

Reusable quality gate for every sprint. Born out of Sprint 13.7 self-retrospective where multiple items were silently deferred or required user prompting ("что мы пропустили?") to catch. This checklist is meant to be **self-correcting**: if any box is unchecked at closeout, the sprint is not done.

Use by copying relevant sections into sprint planning + reviewing at each phase.

---

## Phase 0 — Planning (before TaskCreate, before spawning agents)

### PRD analysis
- [ ] Read PRD целиком, включая "Affected Files", "Out of Scope", "Growth Vision"
- [ ] Для каждого FR выписать **все surfaces** (CLI, MCP, TUI, desktop) — явно решить нужна ли parity
- [ ] CLAUDE.md check: "MCP-first tool" → есть ли MCP exposure для каждого FR?
- [ ] Проверить deliberate deviations от PRD — если трогаем файл не из PRD или пропускаем файл из PRD, **явно** задокументировать причину
- [ ] Cross-reference с EPIC — какие прошлые спринты влияют на этот?

### Feature flags & build configs
- [ ] Feature flag discipline: если фича gated, план covers BOTH on/off build configs в тестах
- [ ] Tests для `cargo check --features X` и без него
- [ ] Release binary test для default build

### Architecture
- [ ] Trait discipline: если расширяем signature — **все** trait impls обновить (production + tests + mocks + in_memory variants)
- [ ] Schema discipline: если меняем schema, план covers migration + idempotency + **real legacy workspace** preservation test
- [ ] Bounded contexts respected (core vs cli vs mcp — никаких leakages)

### Test strategy
- [ ] Для каждого нового public function: positive + negative + corner cases
- [ ] External dependencies (network, models, services): план для gradual test skip в CI, OR explicit `#[ignore]` + docs how to run
- [ ] Migration path: тест на **реальном** legacy fixture, не только fresh create
- [ ] Cross-surface tests: если фича в CLI AND MCP, нужны тесты для обоих handler'ов (integration, не только unit)
- [ ] E2E regression: верифицирует ВСЕ предыдущие спринты + новый сприт

### Team structure
- [ ] Декомпозиция учитывает file ownership — нет shared files между parallel agents
- [ ] Если spec требует trip в файл outside ownership — **расширить ownership**, не хитрить (иначе агент будет работать через json! macro вместо типизированного struct)
- [ ] Plan **ВСЕ waves upfront** (не "fixer + потом wave 2 + completer" — это signal of incomplete planning)
- [ ] Если агент должен возможно коммитить partial work — план как это обработать

---

## Phase 1 — Implementation

- [ ] Agent tasks имеют **точный commit message** в спеке
- [ ] Typed structs everywhere — никаких `json!` макросов для response shapes в production коде
- [ ] Input validation на boundaries (CLI args, MCP params, store methods, trait signatures)
- [ ] Error types: distinct variants для разных failure modes, не `Ok(empty)` скрывающий real errors
- [ ] If agent hits file outside ownership → STOP и SendMessage team-lead, **не work around**
- [ ] Idle teammates shutdown immediately после completion, не висят hours
- [ ] Commit messages соответствуют спеке (exact match на первую строку)

---

## Phase 2 — Audit (обязательно 4 parallel auditors)

- [ ] Rust / Security / Architecture / Tests в параллель
- [ ] Each audit report должен включать explicit "что я НЕ проверил" section
- [ ] Findings classified: CRITICAL / HIGH / MEDIUM / LOW / Positive / Deferred
- [ ] Architecture auditor specifically checks: trait honesty, file ownership, surface parity

---

## Phase 3 — Fixer

- [ ] ВСЕ CRITICAL + HIGH fix'ятся
- [ ] Medium quick-wins (< 5 LOC) bundled в fixer pass
- [ ] Deferred items explicitly logged в fixer report с причиной
- [ ] Commit message точно specified

---

## Phase 4 — Re-audit (НЕ ПРОПУСКАТЬ!)

- [ ] 1 re-auditor agent проверяет каждый fix **against real code** (не keywords)
- [ ] Verdict: READY TO MERGE / NEEDS FIXES / BLOCKED
- [ ] **Если добавляется Wave 2 / completer work — еще один re-audit после** (no "manual UX covers it" shortcut)
- [ ] Re-auditor verifies: тесты actually pass, не только что файлы закоммичены

---

## Phase 5 — Manual UX verification (team-lead, не agent)

- [ ] Build **release** binary (not debug)
- [ ] Tested: все новые commands с default build
- [ ] Tested: все новые commands с feature-on build (если применимо) — even if via `#[ignore]` test
- [ ] Tested: regression для **каждого** предыдущего sprint'а того же EPIC'а
- [ ] Tested: error paths (empty input, invalid format, missing workspace)
- [ ] Tested: JSON outputs валидируются (serde round-trip, field names verified)
- [ ] Tested: MCP tools через реальный MCP client (не только unit тесты)
- [ ] Tested: migration на **pre-existing** workspace (не только fresh init)
- [ ] UX rated per command (★/★★/★★★/★★★★/★★★★★) с justification

---

## Phase 6 — Closeout

- [ ] PRD Progress 0/N → N/N с реальным FR → file:line → test mapping
- [ ] PRD **deliberate deviations** от "Affected Files" задокументированы
- [ ] EVID артефакт с structured fields (verdict/CL/type)
- [ ] EVID explicit **deferred list** с причинами каждого пункта
- [ ] `forgeplan score PRD-XXX` показывает R_eff > 0
- [ ] `forgeplan activate` выполнен
- [ ] Superseded PRDs переведены в terminal state через `forgeplan supersede`
- [ ] **Closeout committed together** с wave work (не separate PR unless explicit hotfix)
- [ ] PR body includes **full test plan checklist** со всеми quality gates
- [ ] Hindsight `memory_retain` sprint summary
- [ ] Team shutdown + TeamDelete cleanup
- [ ] Stale tasks cleaned up из task list

---

## Phase 7 — Meta / self-correction

- [ ] Сразу после closeout провести **self-retrospective** по этому чеклисту
- [ ] Если user должен был спросить "что пропустили" во время sprint'а — **флаг методологического fail**
- [ ] Log всех deferred items в одно место (backlog PRD or NOTE) — не теряется между спринтами
- [ ] Compare actual work vs original plan: сколько tasks появилось "по дороге"? Если > 20%, планирование было неполным
- [ ] If retrospective finds HIGH/CRITICAL items — open hotfix branch, не игнорировать

---

## Red flags (stop-the-world signals)

- 🚩 User говорит "а что мы пропустили?" → planning phase было неполным
- 🚩 Agent использует `json!` macro вместо типизированной struct → file ownership constraint слишком узкий
- 🚩 Тот же gap deferred 2+ спринта подряд (например "MCP handler harness") → institutional avoidance, нужен force fix
- 🚩 Trait forwarder passes `None` или `empty` → trait врёт, расширять signature
- 🚩 Test проходит но reality не проверена (`#[ignore]` тест который никто не запускал) → нужен ручной run
- 🚩 `serde_json::json!` в response path → потерян JSON contract type safety
- 🚩 Фича gated на compile-time но runtime failure сценарии не покрыты → fallback incomplete

---

## Lessons from Sprint 13.7

Конкретные ошибки из 13.7 которые этот чеклист должен предотвратить:

1. **MCP parity было unplanned** — `Phase 0 → Cross-reference → MCP-first tool check` должен был поймать это
2. **Wave 2 было unplanned** (types.rs cleanup, CLI runtime fallback, corner cases) — `Phase 0 → Team structure → Plan ВСЕ waves upfront`
3. **MCP handler harness deferred 2 спринта подряд** — `Red flag → institutional avoidance`
4. **Real semantic path 0 tested до post-closeout hotfix** — `Phase 1 → Test strategy → external deps planned with #[ignore]` 
5. **FpfStorage::search_fpf_by_vector не в trait** (только insert) — `Phase 2 → Architecture auditor → trait honesty`
6. **Fixer stalled 15 min, team-lead думал что он завис** — process issue, checklist note: "check commits exist before spawning replacement"
7. **cli-impl висел idle час** — `Phase 1 → Idle teammates shutdown immediately`

---

## Related Artifacts

| Artifact | Relation |
|---|---|
| NOTE-045 | sibling (Sprint 13.7 deferred debts list) |
| EVID-064 | context (Sprint 13.7 evidence that triggered this retrospective) |
| EPIC-003 | context (v0.17.0 series) |
| CLAUDE.md | authoritative (governing conventions) |
