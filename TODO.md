# TODO — Forgeplan

## Next: Phase 4 — MCP Server + AI

### P0 (прямо сейчас)
- [x] Phase 4.1: MCP server — `forgeplan serve` (rmcp crate, expose all 11 commands as tools)
- [x] RFC-004: MCP Server Architecture — document design decisions
- [x] ADR-006: Full LanceDB primary (no file fallback) — document decision
- [x] Update RFC-003 progress to 100%

### P1 (Phase 4 — AI Features)
- [ ] LLM integration — generate PRD from description
- [ ] FPF ADI cycle — Abduction→Deduction→Induction for decisions
- [ ] Auto-decompose — PRD → RFC tasks
- [ ] Evidence Decay — valid_until TTL + refresh alerts
- [ ] Depth calibration — auto-suggest Tactical/Standard/Deep/Critical
- [ ] Auto-capture — agent records decisions from conversation context

### P2 (Phase 2 — Superseded by MCP)
- [ ] ~~Workflow Integration~~ — superseded: MCP server covers these use cases

## Backlog: Phase 5 — Desktop App
- [ ] Tauri 2.0 + React frontend (shared Rust core)

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
- [x] **Phase 3C** — Search + Stale + Polish (2026-03-21):
  - [x] `forgeplan search` — keyword grep по body (FR-008)
  - [x] `forgeplan stale` — detect expired valid_until (FR-010)
  - [x] ForgeplanError enum (thiserror) — typed errors
  - [x] 13 integration tests (assert_cmd + tempdir)
  - [x] 29 tests total, все проходят
  - [x] Release binary: 3.3 MB (NFR-002: < 15MB)
- [x] **Phase 3D** — LanceDB Primary + Async Migration (2026-03-22):
  - [x] LanceDB as sole source of truth (no file fallback)
  - [x] ArtifactRecord + 8 new LanceStore methods
  - [x] Markdown projection module (write-only, git-tracked)
  - [x] All 10 CLI commands migrated to LanceStore
  - [x] impl FromStr for ArtifactKind (eliminated 3 parse_kind dupes)
  - [x] N+1 query fixes, ID validation, UTF-8 safety
  - [x] 5-agent audit: 15 findings, all critical+high fixed
  - [x] `forgeplan progress` — checkbox parser + ASCII bars (3.10)
  - [x] 4-agent Rust audit: CheckboxCount struct, clamp, no unwrap
  - [x] 158 tests pass (135 core + 16 CLI + 7 other)
  - [x] Dogfooding: all 11 commands verified end-to-end
- [x] **Phase 4A** — MCP Server (2026-03-22):
  - [x] `forgeplan-mcp` crate — rmcp 1.2.0 + stdio transport
  - [x] 11 MCP tools: init, new, list, status, validate, score, link, graph, search, stale, progress
  - [x] ForgeplanServer with Arc<RwLock<Option<LanceStore>>> for lazy init
  - [x] Structured JSON responses (schemars JsonSchema on all types)
  - [x] `forgeplan serve` subcommand in CLI
  - [x] Refactor: Mode::FromStr in core, eliminated record_to_frontmatter dupe
  - [x] Smoke test: initialize + tools/list verified via stdio
  - [x] 158 tests pass (all existing tests unaffected)
