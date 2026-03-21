# TODO — Forgeplan

## Next: Phase 3C — Polish & Tests

### P0 (прямо сейчас)
- [ ] Phase 3C: `forgeplan search` — keyword search (FR-008)
- [ ] Phase 3C: `forgeplan stale` — detect expired valid_until (FR-010)
- [ ] Phase 3C: Integration tests (assert_cmd + tempdir)
- [ ] Phase 3C: Error handling refinement (thiserror enum)
- [ ] Phase 3C: >80% test coverage
- [ ] LanceDB tables schema — adapt quint-code schema (3.4)

### P1 (Phase 2 — Workflow Integration)
- [ ] Расширить /write-doc (prd, epic, spec)
- [ ] /prd slash command
- [ ] PRD-INDEX.md template
- [ ] EPIC-INDEX.md template
- [ ] Hindsight memory integration
- [ ] Verification Gate checklist в /audit
- [ ] Adversarial Review protocol в /audit
- [ ] Обновить CLAUDE.md

## Backlog: Phase 4 — AI
→ См. PLAN.md Phase 4

## Done ✅
- [x] **Phase 0** — Foundation & Research (10/10)
- [x] **Phase 1** — Schemas, Templates & Docs (12/12):
  - [x] PRD-SCHEMA.md, EPIC-SCHEMA.md, SPEC-SCHEMA.md
  - [x] PRD-RFC-ADR-FLOW.md
  - [x] PRD шаблон обогащён из BMAD (13 validation steps, YAML frontmatter)
  - [x] Product Brief шаблон (lightweight PRD для Quick Flow)
  - [x] Problem Card шаблон (из quint-code)
  - [x] Solution Portfolio шаблон (из quint-code, weakest link)
  - [x] DDR шаблон (FPF E.9: invariants + rollback + valid_until)
  - [x] DEPTH-CALIBRATION.md (4 уровня + auto-escalation)
  - [x] QUALITY-GATES.md (Verification Gate + Adversarial Review + BMAD 13 steps + R_eff)
  - [x] GLOSSARY.md (31 термин)
- [x] Rust workspace scaffold (forgeplan-core + forgeplan-cli)
- [x] Artifact types (11 kinds) + R_eff scoring (4 tests pass)
- [x] Git initialized, 2 commits
- [x] **Phase 3A** — Core CLI (2026-03-21):
  - [x] RFC-001: CLI Architecture (модули, data flow, phases)
  - [x] `forgeplan init` — workspace initialization (FR-001)
  - [x] `forgeplan new` — template engine + auto-ID (FR-002)
  - [x] `forgeplan list` — frontmatter parser + table output (FR-003)
  - [x] `forgeplan status` — project dashboard (FR-004)
  - [x] Config module + YAML loader
  - [x] Artifact store (CRUD, slugify, next_id)
  - [x] 11 tests pass (4 R_eff + 3 frontmatter + 4 workspace)
- [x] **Phase 3B** — Validate + Score + Link + Graph (2026-03-21):
  - [x] RFC-002: Validation Engine Architecture
  - [x] `forgeplan validate` — schema rules engine per kind per depth (FR-005)
  - [x] `forgeplan score` — R_eff CLI wrapper with evidence lookup (FR-006)
  - [x] `forgeplan link` — typed relationships in frontmatter (FR-009)
  - [x] `forgeplan graph` — mermaid dependency graph (FR-007)
  - [x] validation/ module (checks, rules — PRD/Epic/Spec/RFC/ADR)
  - [x] link/ module (add_link, list_links, normalize_relation)
  - [x] graph/ module (build_edges, render_mermaid)
  - [x] 16 tests pass (5 validation + 4 R_eff + 3 frontmatter + 4 workspace)
