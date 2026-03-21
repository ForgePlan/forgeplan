# TODO — Forgeplan

## Next: Eat Your Own Dogfood — применить процесс к Forgeplan

### P0 (прямо сейчас)
- [ ] Создать Epic: "Build Forgeplan" используя templates/epic/_TEMPLATE.md
- [ ] Создать PRD: "Forgeplan CLI" используя templates/prd/_TEMPLATE.md (обогащённый BMAD)
- [ ] Записать ADR: "Rust вместо Go" используя templates/adr/_TEMPLATE.md
- [ ] Записать ADR: "LanceDB вместо SQLite" используя templates/adr/_TEMPLATE.md

### P1 (Phase 2 — Workflow Integration)
- [ ] Расширить /write-doc (prd, epic, spec)
- [ ] /prd slash command
- [ ] PRD-INDEX.md template
- [ ] EPIC-INDEX.md template
- [ ] Hindsight memory integration
- [ ] Verification Gate checklist в /audit
- [ ] Adversarial Review protocol в /audit
- [ ] Обновить CLAUDE.md

## Backlog: Phase 3 — Rust CLI
→ См. PLAN.md Phase 3

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
