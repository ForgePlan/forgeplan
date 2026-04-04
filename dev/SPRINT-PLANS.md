# Forgeplan Sprint Plans — Road to v1.0

> Создано: 2026-04-01 | Обновлено: 2026-04-01
> Источники: hindsight, git log (82 PRs), E2E test report (193 теста), forgeplan health/tree/gaps/score/blocked
> **Каждый спринт = 1 час = 1 чат = полный цикл по методе = PR на выходе**

---

## Обязательные инструкции для КАЖДОГО спринта

Каждый спринт выполняется в отдельном чате. В начале чата:

### 1. Context Restore (обязательно)
```bash
# Восстановить контекст
memory_recall("Forgeplan sprint plans backlog")
memory_recall("Forgeplan v0.12 session")

# Прочитать текущее состояние
forgeplan health
forgeplan gaps
forgeplan blocked

# Прочитать этот файл
cat dev/SPRINT-PLANS.md
cat TODO.md
cat CLAUDE.md
```

### 2. Полный цикл по методе (для каждой фичи/фикса в спринте)
```
Route  → forgeplan route "<описание задачи>" — определить depth
Shape  → forgeplan new <kind> "<title>" / обновить существующий артефакт
         Заполнить MUST секции. forgeplan validate <id> — PASS required.
Code   → Реализация. cargo test после каждой pub fn.
Audit  → Минимум 2 агента (/audit). Исправить все HIGH/CRITICAL.
Test   → cargo test (ALL pass). cargo check (0 warnings).
Lint   → cargo check = 0 warnings, 0 errors
Verify → Ручная проверка каждой фичи/фикса (не поверхностно!)
Evidence → forgeplan new evidence "<что подтверждено>"
           Body: verdict: supports, congruence_level: 3, evidence_type: test
           forgeplan link EVID-XXX <artifact> --relation informs
           forgeplan activate EVID-XXX
Score  → forgeplan score <id> — R_eff > 0
PR     → Code → Audit → Fix → Test → Lint → Verify → PR (ОБЯЗАТЕЛЬНЫЙ порядок!)
         git checkout dev && git pull origin dev
         git checkout -b <branch-name>
         ... commits ...
         git push -u origin <branch-name>
         gh pr create --base dev
         gh pr merge <N> --merge
Activate → forgeplan review <id> && forgeplan activate <id>
Progress → Обновить TODO.md, RFC progress bars, hindsight
```

### 3. Git конвенции
```
Branch: feat/<slug> или fix/<slug>
Commit: <type>(<scope>): <description>
PR: НЕ удалять ветки после merge
НЕ пушить в ветку после merge PR (коммиты потеряются!)
НЕ коммитить напрямую в dev или main
```

### 4. Safety
```bash
# ПЕРЕД любым reinit
forgeplan export --output backup.json
# ПЕРЕД destructive git
git stash / git log origin/dev..HEAD
# ОБЯЗАТЕЛЬНО
cargo test   # ALL pass
cargo check  # 0 warnings
forgeplan health  # после каждого спринта
```

---

## Sprint 1: Housekeeping — Close Done, Unblock, Fix Stale (45 min)

**Depth**: Tactical (cleanup, нет новых фич)
**Branch**: `fix/housekeeping-v1`
**Артефакты**: Нет новых — только lifecycle transitions + TODO fix

### Wave 1: Close 5 solved PROBs (5 min)
```bash
forgeplan deprecate PROB-006 --reason "Fixed in v0.11 — routing keywords expanded"
forgeplan deprecate PROB-010 --reason "Fixed by RFC-004 files-first architecture"
forgeplan deprecate PROB-012 --reason "Fixed in v0.11 — 5 integrity fixes, 2 audit rounds"
forgeplan deprecate PROB-014 --reason "Fixed in v0.12 — smart search v2, real cosine"
forgeplan deprecate PROB-016 --reason "Fixed in v0.11 — 13 CLI fixes, 6-agent audit"
forgeplan delete PROB-013 --yes   # draft, fix already shipped
```

### Wave 2: Activate ADRs + RFC-004 + PRD-019 (10 min)
```bash
# ADR-002 — R_eff skip rule (implemented)
forgeplan review ADR-002 && forgeplan activate ADR-002
# ADR-003 — files as truth (RFC-004 fully done)
forgeplan review ADR-003 && forgeplan activate ADR-003
# RFC-004 — all 4 phases done, NOW unblocked by ADR-003
forgeplan review RFC-004 && forgeplan activate RFC-004
# PRD-019 — Layer 3 enforcement delivered
forgeplan review PRD-019 && forgeplan activate PRD-019
```

### Wave 3: Activate 3 draft evidence (5 min)
```bash
# Заполнить structured fields если не заполнены, потом activate
forgeplan activate EVID-033  # Smart search v2
forgeplan activate EVID-034  # PROB-016 CLI Quality
forgeplan activate EVID-035  # RFC-004 Phase 1
```

### Wave 4: Fix TODO.md stale entries (5 min)
```
Mark [x] в TODO.md:
- PRD-019 Layer 3 MCP session state machine
- RFC-004 Phase 2: file watcher (PR #69)
- RFC-004 Phase 3: change_log (PR #69)
- forgeplan reindex (PR #71)
- RFC-005 3.2: estimate MCP tool
- e2e_coverage_backfill test (fixed PR #80)
```

### Wave 5: Fix MCP grade unused warning (2 min)
```
File: crates/forgeplan-mcp/src/server.rs
Wire EstimateParams.grade → estimate logic, или убрать поле
```

### Wave 6: Evidence boost for R_eff=0.60 artifacts (10 min)
```bash
forgeplan new evidence "PRD-013..018 implementation verified — all features shipped in v0.9-v0.12"
# Body: verdict: supports, congruence_level: 3, evidence_type: test
forgeplan link EVID-XXX PRD-013 --relation informs
forgeplan link EVID-XXX PRD-014 --relation informs
forgeplan link EVID-XXX PRD-015 --relation informs
forgeplan link EVID-XXX PRD-018 --relation informs
forgeplan activate EVID-XXX
```

### Wave 7: Verify + PR (5 min)
```bash
cargo check  # 0 warnings
forgeplan health   # orphans <=2 (memory), 0 blind spots
forgeplan gaps     # 0 MUST
forgeplan blocked  # <=5 (EPIC-002 dependents only)
forgeplan score --all  # all R_eff >= 0.70

git add . && git commit && git push
gh pr create --base dev && gh pr merge
```

**Exit criteria**: 0 active solved PROBs, ADR-002/003 + RFC-004 + PRD-019 activated, 3 evidence activated, TODO consistent, all R_eff >= 0.70

---

## Sprint 2: Bug Fixes from E2E Report (1 hour)

**Depth**: Standard (3 бага, один P1 security)
**Branch**: `fix/e2e-bugs`
**Артефакты**: PROB (обновить или создать для BUG-001/002/003)

### Методология
```bash
forgeplan route "Fix 3 bugs from E2E test: scan path traversal, unlink phantom, display message"
# Ожидаем: Standard → PROB/fix
```

### Wave 1: BUG-001 — scan path traversal (P1 security) (20 min)
```
File: crates/forgeplan-cli/src/commands/coverage.rs (или scan.rs)
Fix: validate_project_path() — reject paths outside project root
     Аналогично scan-import который уже блокирует /etc
Test: cargo test (unit test: scan --path /tmp → error)
Verify: forgeplan scan --path /tmp → "Path outside project root" exit 1
        forgeplan scan --path ./src → works normally
```

### Wave 2: BUG-002 — unlink non-existent relation (15 min)
```
File: crates/forgeplan-core/src/db/store.rs (delete_relation)
Fix: check relation exists before delete, return error if not
Test: unit test: unlink A B --relation contradicts → Err("Relation not found")
Verify: forgeplan unlink PRD-001 EPIC-001 --relation contradicts → error exit 1
```

### Wave 3: BUG-003 — deprecated→active display message (10 min)
```
File: crates/forgeplan-cli/src/commands/activate.rs
Fix: use record.status (actual from_status) in message, not hardcoded "draft"
Test: verify message shows real transition
Verify: deprecate → activate → message says "deprecated → active"
```

### Wave 4: Audit (2 agents) + evidence (15 min)
```bash
# Audit минимум 2 агента
/audit — security reviewer + correctness reviewer

# Fix все HIGH/CRITICAL findings
cargo test  # ALL pass
cargo check # 0 warnings

# Evidence
forgeplan new evidence "BUG-001/002/003 fixed — E2E report 193 tests"
# verdict: supports, congruence_level: 3, evidence_type: test
forgeplan link EVID-XXX PROB-XXX --relation informs
forgeplan activate EVID-XXX

# PR
git push && gh pr create --base dev && gh pr merge
```

### Wave 5: Update progress
```bash
# TODO.md — mark BUG fixes done
# hindsight — retain results
memory_retain("BUG-001/002/003 fixed: scan path, unlink check, display msg")
```

**Exit criteria**: BUG-001/002/003 fixed, 2-agent audit passed, tests pass, evidence created, PR merged

---

## Sprint 3: PROB-017 — Router Alternatives (1 hour)

**Depth**: Standard (новая фича route)
**Branch**: `feat/prob-017-router-alternatives`
**Артефакты**: Обновить PROB-017 (не создавать новый PRD — per feedback rule)

### Shape (15 min)
```bash
forgeplan route "Enhance forgeplan route to return 2-3 alternative depth/pipeline suggestions"
# Обновить PROB-017 с Goals, Criteria, FR
forgeplan update PROB-017 --body @prob-017-body.md
forgeplan validate PROB-017
```

### Code (30 min)
```
File: crates/forgeplan-core/src/routing/mod.rs
- RouteResult: добавить alternatives: Vec<RouteAlternative>
- Каждый route возвращает primary + 2 alternatives с reasoning
- Detect existing PROB/PRD by topic keywords

File: crates/forgeplan-cli/src/commands/route.rs
- Display alternatives после primary result

File: crates/forgeplan-mcp/src/server.rs
- forgeplan_route response includes _alternatives field
```

### Audit + Test + Evidence (15 min)
```bash
/audit — 2 agents
cargo test -- routing
forgeplan route "Add OAuth2"  # primary + 2 alternatives

forgeplan new evidence "PROB-017 router alternatives implemented"
forgeplan link EVID-XXX PROB-017 --relation informs
forgeplan activate EVID-XXX
forgeplan activate PROB-017

git push && gh pr create --base dev && gh pr merge
memory_retain("PROB-017 done: route returns alternatives")
```

**Exit criteria**: route shows alternatives, MCP includes _alternatives, 2-agent audit, evidence, PR merged

---

## Sprint 4: PROB-015 — EmbedDriver + ISP Split (1 hour)

**Depth**: Standard (refactoring, architecture cleanup)
**Branch**: `refactor/prob-015-embed-driver-isp`
**Артефакты**: Обновить PROB-015, создать evidence

### Shape (10 min)
```bash
forgeplan route "Extract EmbedDriver trait and split StorageDriver into focused traits (ISP)"
forgeplan update PROB-015 --body @prob-015-updated.md
forgeplan validate PROB-015
```

### Wave 1: EmbedDriver trait (25 min)
```
File: crates/forgeplan-core/src/driver/mod.rs
- trait EmbedDriver { fn embed_text(&self, text: &str) -> Result<Vec<f32>>; fn embed_batch(...); }
- FastEmbedDriver implements EmbedDriver (existing embed/ module)
- NoOpEmbedDriver fallback (returns empty vec)
- Test: both drivers pass
```

### Wave 2: ISP split StorageDriver (20 min)
```
File: crates/forgeplan-core/src/driver/mod.rs
Split ~29 methods into focused traits:
- ArtifactStorage: create/get/update/delete/list artifacts
- RelationStorage: add/delete/get relations
- SearchStorage: search_body, vector_search
- ChangeLogStorage: log_change, get_change_log
LanceDriver implements all 4.
```

### Audit + Evidence (5 min)
```bash
/audit — 2 agents (architecture + correctness)
cargo test  # ALL pass, no breaking changes
forgeplan new evidence "PROB-015 resolved — EmbedDriver + ISP split"
forgeplan link EVID-XXX PROB-015 --relation informs
forgeplan activate EVID-XXX

git push && gh pr create --base dev && gh pr merge
```

**Exit criteria**: EmbedDriver trait extracted, StorageDriver split into 4, all tests pass, 2-agent audit, PR merged

---

## Sprint 5: RFC-003 Phase 3 — SQLite Driver (1 hour)

**Depends on**: Sprint 4 (ISP split provides traits to implement)
**Depth**: Standard
**Branch**: `feat/rfc-003-sqlite-driver`
**Артефакты**: RFC-003 progress update, evidence

### Shape (10 min)
```bash
forgeplan route "Implement SQLite storage driver as alternative to LanceDB"
# RFC-003 Phase 3 уже описан
forgeplan get RFC-003  # read phases
```

### Code (35 min)
```
New dep: rusqlite (workspace Cargo.toml)
New file: crates/forgeplan-core/src/driver/sqlite.rs
- Implement ArtifactStorage + RelationStorage + SearchStorage + ChangeLogStorage
- Schema: CREATE TABLE artifacts, relations, change_log
- Feature flag: default=lancedb, --features sqlite

New/update: crates/forgeplan-core/src/driver/factory.rs
- Read config.yaml storage.driver: "lancedb" | "sqlite"
- Instantiate correct driver
```

### Audit + Test + Evidence (15 min)
```bash
/audit — 2 agents
cargo test -- driver
cargo test -- sqlite (if feature enabled)

forgeplan new evidence "RFC-003 Phase 3 — SQLite driver implemented"
forgeplan link EVID-XXX RFC-003 --relation informs
forgeplan activate EVID-XXX
# Update RFC-003 progress: Phase 3 [x]

git push && gh pr create --base dev && gh pr merge
```

**Exit criteria**: SQLite driver works, feature flag, factory reads config, 2-agent audit, PR merged

---

## Sprint 6: Promote + Evidence Calibration (1 hour)

**Depth**: Tactical (2 small features, no RFC needed)
**Branch**: `feat/promote-calibration`

### Wave 1: forgeplan promote (30 min)
```bash
forgeplan route "forgeplan promote — upgrade memory bookmark to full artifact"
```
```
New file: crates/forgeplan-cli/src/commands/promote.rs
- forgeplan promote <memory-id> --kind prd
- Read memory text → create artifact of kind with memory content → delete memory
- Add to mod.rs + main.rs

Test: promote mem-xxx --kind note → creates NOTE, deletes memory
Verify: forgeplan remember "test fact" && forgeplan promote mem-test-fact --kind note
```

### Wave 2: Evidence calibration (25 min)
```bash
forgeplan route "Estimate calibration — compare estimated vs actual hours"
```
```
New: forgeplan calibrate-estimate <artifact-id> --actual-hours 5
- Reads last estimate result (from LanceDB or re-calculates)
- Compares with actual hours
- Outputs ratio: actual/estimated
- Hints: "Your estimates are 1.3x optimistic for this type"
```

### Audit + Evidence + PR (5 min)
```bash
/audit — 2 agents
cargo test
forgeplan new evidence "promote + calibrate-estimate implemented"
git push && gh pr create --base dev && gh pr merge
```

**Exit criteria**: promote works, calibrate-estimate shows ratio, tests pass, PR merged

---

## Sprint 7: Distribution — brew + GH Actions + crates.io (1 hour)

**Depth**: Tactical (infra, no code logic changes)
**Branch**: `feat/distribution`

### Wave 1: GitHub Actions release workflow (30 min)
```
New file: .github/workflows/release.yml
- Trigger: push tag v*
- Matrix build: linux-x86_64, macos-arm64, macos-x86_64
- cargo build --release
- gh release create with binary assets
- Checksums SHA256

Test: create test tag → verify workflow runs
```

### Wave 2: brew tap + install script (20 min)
```
New repo: ForgePlan/homebrew-tap (or in-repo Formula/)
- forgeplan.rb formula downloads binary from GH release

New file: install.sh
- Detects OS + arch
- Downloads correct binary from latest GH release
- Installs to ~/.cargo/bin/forgeplan
```

### Wave 3: crates.io (10 min)
```bash
# Verify metadata
cargo package -p forgeplan-core --list
cargo package -p forgeplan-cli --list

# Publish (requires crates.io token)
cargo publish -p forgeplan-core
cargo publish -p forgeplan-cli
```

### Verify + PR
```bash
# Verify brew
brew install forgeplan/tap/forgeplan && forgeplan --version

# Verify curl
curl -fsSL https://raw.githubusercontent.com/ForgePlan/forgeplan/main/install.sh | sh

git push && gh pr create --base dev && gh pr merge
```

**Exit criteria**: GH Actions builds on tag, brew works, install.sh works, crates.io published

---

## Sprint 8: Release v1.0.0 (30 min)

**Depends on**: ALL sprints 1-7 complete
**Branch**: `release/v1.0.0`

### Pre-release checklist
```bash
# Context
memory_recall("Forgeplan sprint plans")
forgeplan health        # clean
forgeplan gaps          # 0 MUST
forgeplan blocked       # only EPIC-002 dependents
forgeplan score --all   # all R_eff >= 0.70

# Full test suite
cargo test              # ALL pass
cargo check             # 0 warnings

# Smoke test key commands
forgeplan init -y && forgeplan new prd "Smoke" && forgeplan validate PRD-XXX
forgeplan estimate PRD-XXX && forgeplan estimate PRD-XXX --llm-score
forgeplan score PRD-XXX && forgeplan review PRD-XXX
```

### Version bump + build
```bash
# Version
sed -i 's/0.12.0/1.0.0/' Cargo.toml
git add Cargo.toml
git commit -m "chore: bump version to v1.0.0"

# Release build
cargo build --release
./target/release/forgeplan --version  # 1.0.0
cp target/release/forgeplan ~/.cargo/bin/
```

### Release
```bash
git push -u origin release/v1.0.0
gh pr create --base main --title "Release v1.0.0"
gh pr merge --merge
git checkout main && git pull
git tag -a v1.0.0 -m "Release v1.0.0: Forgeplan — Forge your plan"
git push origin v1.0.0

# Sync dev
git checkout dev && git merge main && git push origin dev

# Hindsight
memory_retain("Forgeplan v1.0.0 released — full feature set")
```

**Exit criteria**: v1.0.0 tagged, binary installed, GH release published, brew updated

---

## Dependency Graph

```
Sprint 1 (Housekeeping)     ─── нет зависимостей
Sprint 2 (Bug Fixes)        ─── нет зависимостей
Sprint 3 (Router Alt)       ─── нет зависимостей
Sprint 4 (EmbedDriver/ISP)  ─── нет зависимостей
Sprint 5 (SQLite Driver)    ─── зависит от Sprint 4
Sprint 6 (Promote/Calibrate)─── нет зависимостей
Sprint 7 (Distribution)     ─── нет зависимостей
Sprint 8 (Release v1.0)     ─── зависит от ALL (1-7)
```

### Параллельные группы (разные чаты одновременно):
```
Chat A: Sprint 1 + Sprint 2    → cleanup + bugfix
Chat B: Sprint 3               → router alternatives
Chat C: Sprint 4 → Sprint 5    → drivers (sequential)
Chat D: Sprint 6               → promote + calibration
Chat E: Sprint 7               → distribution
Chat F: Sprint 8               → release (after all)
```

**Total: ~6.5 часов AI-time**

---

## P3 Backlog (Post v1.0)

| Task | Estimate |
|------|----------|
| Phase 5: Desktop App (Tauri + React) | weeks |
| Integrations (Linear, Jira, Orchestra) | days |
| RRF hybrid search | 3-4h |
| PROB-011: Multi-agent architecture | days |
| PROB-002: Auth reuse | 2-3h |
| Self-link prevention | 30min |
| Case-insensitive IDs | 1-2h |
| fpf.rs migration to common::store() | 1h |
| Embed fix (fastembed v5 — upstream) | blocked |
