# TODO — Forgeplan

## Current: v0.8.0 Released

### Completed
- [x] **EPIC-001** — v1.0 Methodology Engine (PRD-002..007)
- [x] **PRD-008** — CLI UX Redesign: cliclack, styled output, --json, setup-skill
- [x] **PROB-006** — Routing keyword fix (redesign/overhaul → Standard+)

### Stats
- 34 CLI commands, 28 MCP tools, 231 tests
- 27 dogfood artifacts (11 active, 16 draft)
- v0.8.0 tagged, PRs #17-#23 merged

---

## Open Problems (из dogfood)

| ID | Priority | Title | Impact |
|----|----------|-------|--------|
| PROB-001 | **P0** | Data loss — no export/import, rm -rf destroys all | Lost 26 artifacts |
| PROB-002 | P2 | Auth reuse — separate API key barrier | Adoption friction |
| PROB-003 | Done | Dead statuses → solved by PRD-007 lifecycle | |
| PROB-004 | P1 | Agent drift — AI ignores methodology | 90% value lost |
| PROB-005 | P1 | Cold start — zero context in new chat | Bad onboarding |

---

## Backlog (приоритизированный)

### P0: Data Safety
- [ ] `forgeplan export` — dump all artifacts to JSON (git-trackable)
- [ ] `forgeplan import` — restore from dump
- [ ] Auto-export on every write (or periodic)

### P1: Distribution & Adoption
- [ ] brew tap formula (macOS)
- [ ] GitHub Actions release pipeline (cross-compile for linux/windows/mac)
- [ ] Install script (`curl -fsSL https://forgeplan.dev/install.sh | sh`)
- [ ] `fpl` alias symlink in install
- [ ] Publish to crates.io (`cargo install forgeplan`)

### P1: Agent Integration
- [ ] `forgeplan init --scan` — detect existing docs/ADRs and offer import (PROB-005)
- [ ] SessionStart hook → `forgeplan health --compact --json` (PROB-004/005)
- [ ] PostToolUse hook → remind capture decisions (PROB-004)

### P2: Polish
- [ ] `forgeplan validate --adversarial` mode (devil's advocate review)
- [ ] Custom prompts `.forgeplan/prompts/` (PROB-001)
- [ ] Binary size 152MB → investigate LanceDB feature flags / strip
- [ ] Markdown projection sync on `forgeplan update` (not just `new`)

### P3: Integrations
- [ ] Bidirectional sync with task trackers (Linear, Jira, Orchestra)
- [ ] Export to GitHub Issues / Linear tasks

### Phase 5: Desktop App
- [ ] Tauri 2.0 + React frontend (shared Rust core)

---

## Done

- [x] **v0.8.0** — CLI UX: cliclack init, styled output, --json, setup-skill, PROB-006 fix
- [x] **v0.7.0** — EPIC-001 complete, FPF engine, lifecycle, /forge skill
- [x] **v0.6.0** — Methodology Engine: routing, lifecycle, F-G-R
- [x] **v0.5.0** — Health, Journal, Validation v2
- [x] **Phase 4** — MCP Server + AI Features + CRUD
- [x] **Phase 3** — Core CLI + LanceDB Primary
- [x] **Phase 1** — Schemas, Templates & Docs
- [x] **Phase 0** — Foundation & Research
