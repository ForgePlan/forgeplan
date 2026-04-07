# ForgePlan v0.17 — Sprint Roadmap

> Created: 2026-04-07
> Owner: gogocat
> Status: Planning → Execution
> Epic: EPIC-003 (Sprint 13: Search, Discovery, Intelligence)

## Branch Strategy

```
main ◄───────────────────────────── release/v0.17.0 (final)
dev  ◄───────────────────────────── release/v0.17.0 (sync back)
                                          ▲
                                          │ merge after full /forge-cycle audit
                                          │
                          ┌───────────────┼───────────────┬──────────────┐
                          │               │               │              │
              feat/sprint-13.0  feat/sprint-13.1  feat/sprint-13.2  feat/sprint-13.3
                  security        prd-043 INT       prd-039 search    prd-035 tags
                          │               │               │              │
                          │               │               │              │
                          ▼               ▼               ▼              ▼
                   release/v0.17.0 (integration branch — created from dev)
                          ▲
                          │
              ┌───────────┼───────────────┬───────────────┐
              │           │               │               │
        feat/sprint-13.4 (discover-mcp)  feat/sprint-13.5 (scoring)
        feat/sprint-13.6 (fpf-rules)     feat/sprint-13.7 (kb-vector)
```

### Rules

1. **release/v0.17.0** — integration branch создаётся ИЗ dev в самом начале
2. **Каждый sprint** → отдельная feature branch ИЗ release/v0.17.0 (НЕ из dev)
3. **PR feature → release/v0.17.0** — обычный squash merge
4. **После всех sprint'ов** → полный `/forge-cycle` audit на release branch (независимый от локальных проверок)
5. **release/v0.17.0 → main** — merge commit (сохраняет историю sprint'ов)
6. **main → dev** — sync back через merge commit
7. **Tag v0.17.0** — на main после merge

### Команды

```bash
# Setup (один раз)
git checkout dev && git pull origin dev
git checkout -b release/v0.17.0
git push -u origin release/v0.17.0

# Per sprint (повторяется N раз)
git checkout release/v0.17.0 && git pull
git checkout -b feat/sprint-XX
# ... код, test, audit ...
git push -u origin feat/sprint-XX
gh pr create --base release/v0.17.0
gh pr merge --squash --delete-branch=false  # сохраняем ветку

# Финал (после всех sprint'ов)
git checkout release/v0.17.0 && git pull
# Полный /forge-cycle на release branch:
cargo test --workspace
cargo fmt --check
cargo check --workspace
forgeplan health --ci
forgeplan validate --ci
/audit                    # минимум 4 агента
# Если всё PASS:
gh pr create --base main --title "Release v0.17.0"
gh pr merge --merge       # merge commit, НЕ squash
git checkout main && git pull
git tag -a v0.17.0 -m "Release v0.17.0: Search, Discovery, Intelligence"
git push origin v0.17.0
git checkout dev && git merge main && git push origin dev
```

---

## Sprint Sequence

> **Linear order:** 13.0 → 13.1 → 13.2 → 13.3 → 13.4 → 13.5 → 13.6 → 13.7 → release.
> Sprint 13.1 (Methodology Integrity) — **FOUNDATION first**, prevents future stubs/duplicates in remaining sprints.

### Sprint 13.0 — Security Hotfix 🔴

| Field | Value |
|-------|-------|
| Branch | `feat/sprint-13.0-security` (from release/v0.17.0) |
| Type | Hotfix (Tactical, no artifact) |
| TeamCreate | ❌ no (4 dep bumps) |
| Time | 1-2h |
| LOC | ~10 (lockfiles) |
| Artifact | — |

**Scope:**
- Vite #6 HIGH: WebSocket arbitrary file read
- Vite #4 HIGH: server.fs.deny bypass
- Vite #5 MED: path traversal .map handling
- lru #3 LOW: Stacked Borrows

**Commands:**
```bash
git checkout release/v0.17.0 && git pull
git checkout -b feat/sprint-13.0-security
cd website && npm update vite && npm audit
cd .. && cargo update -p lru && cargo audit
cargo test --workspace
git commit -am "chore(deps): bump vite + lru — fix CVEs (Dependabot #3-6)"
git push -u origin feat/sprint-13.0-security
gh pr create --base release/v0.17.0
```

---

### Sprint 13.1 — Methodology Integrity (FOUNDATION) 🛡

| Field | Value |
|-------|-------|
| Branch | `feat/sprint-13.1-prd-043-integrity` |
| Type | Feature (Standard) |
| TeamCreate | ✅ `sprint-prd-043-integrity` |
| Time | 1d |
| LOC | ~160 |
| Artifact | PRD-043 (READY) |
| Why first | Prevents stubs/duplicates in all remaining sprints |

**Scope:**
- Duplicate guard в `forgeplan new` (search-before-create)
- Stub detection rule в validation (blocks `forgeplan activate` on template body)
- Health check для существующих duplicates

**Waves:**
- W1: `duplicate-guard` (cli/commands/new.rs) + `stub-rule` (validation/rules.rs)
- W2: `health-duplicates` (health/mod.rs) + `mcp-integration` (mcp/server.rs)
- W3: Tests + Audit

---

### Sprint 13.2 — Smart Search v2 🟢

| Field | Value |
|-------|-------|
| Branch | `feat/sprint-13.2-prd-039-search` |
| Type | Feature (Standard) |
| TeamCreate | ✅ `sprint-prd-039-search` |
| Time | 1-1.5d |
| LOC | ~430 |
| Artifact | PRD-039 (READY) |

**Waves:**
- W1 Foundation: `bm25-impl` + `filter-dsl` (parallel, ~200 LOC)
- W2 Integration: `smart-search-integrator` (~150 LOC)
- W3 CLI/MCP: `cli-flags` + `mcp-tools` (parallel, ~80 LOC)
- W4 Tests + Audit: `test-writer` + `code-reviewer`

**File ownership:**
| Agent | Files | Wave |
|-------|-------|------|
| bm25-impl | search/bm25.rs (NEW), search/mod.rs | W1 |
| filter-dsl | search/filter.rs (NEW) | W1 |
| smart-search-integrator | search/smart.rs (MODIFY) | W2 |
| cli-flags | cli/commands/search.rs | W3 |
| mcp-tools | mcp/server.rs (search tool) | W3 |

---

### Sprint 13.3 — Discovery Phase 1: Tags + Source Tier 🟡

| Field | Value |
|-------|-------|
| Branch | `feat/sprint-13.3-prd-035-tags` |
| Type | Feature (Standard) |
| TeamCreate | ✅ `sprint-prd-035-tags` |
| Time | 1.5d |
| LOC | ~250 |
| Artifact | PRD-035 (part 1: FR-001-003 + FR-008) |

**Waves:**
- W1 Schema: `frontmatter-tags` + `db-schema-migrate`
- W2 CLI: `tag-commands` + `list-tag-filter`
- W3 Source Tier: `evidence-tier-mapping`
- W4 Tests + Audit

---

### Sprint 13.4 — Discovery Phase 1: MCP Tools + CLI 🟡

| Field | Value |
|-------|-------|
| Branch | `feat/sprint-13.4-prd-035-discover` |
| Depends on | Sprint 13.3 merged |
| Type | Feature (Deep) |
| TeamCreate | ✅ `sprint-prd-035-discover` |
| Time | 2d |
| LOC | ~500 |
| Artifact | PRD-035 (part 2: FR-004-007) |

**Waves:**
- W1 Module: `discover-module-core` (NEW core/discover/)
- W2 MCP Tools: `mcp-discover-tools` (3 tools)
- W3 CLI: `cli-discover`
- W4 Tests + Audit

---

### Sprint 13.5 — Scoring & Routing Intelligence 🟡

| Field | Value |
|-------|-------|
| Branch | `feat/sprint-13.5-prd-040-scoring` |
| Note | unchanged number — was already 13.5 |
| Type | Feature (Standard) |
| TeamCreate | ✅ `sprint-prd-040-scoring` |
| Time | 0.5-1d |
| LOC | ~130 |
| Artifact | PRD-040 (READY) |

**Waves:**
- W1 Skills + CI: `routing-skills` + `reff-ci`
- W2 Integration: `cli-display` + `health-update`
- W3 Tests + Audit

---

### Sprint 13.6 — RFC-001 Phase 3: FPF Rules CLI/MCP 🟡

| Field | Value |
|-------|-------|
| Branch | `feat/sprint-13.6-prd-041-fpf-rules` |
| Type | Feature (Standard) |
| TeamCreate | ✅ `sprint-prd-041-fpf-rules` |
| Time | 1d |
| LOC | ~150 |
| Artifact | PRD-041 (READY) |

**Note:** Rule engine API уже full-public (`run_rules`, `default_rules`, `Rule`, `EnrichedData`). Только CLI/MCP wiring needed.

**Waves:**
- W1 CLI: `fpf-rules-cmd` + `fpf-check-cmd`
- W2 MCP: `mcp-fpf-tools`
- W3 Tests + Audit

---

### Sprint 13.7 — FPF KB Vector Search + PRD-018 Cleanup ⚪

| Field | Value |
|-------|-------|
| Branch | `feat/sprint-13.7-prd-042-kb-search` |
| Type | Feature (Standard) |
| TeamCreate | ✅ `sprint-prd-042-kb-search` |
| Time | 1d |
| LOC | ~200 |
| Artifact | PRD-042 (READY) — supersedes PRD-018 |

**Scope:**
- Schema migration: add embedding column to fpf_spec
- `db/store::search_fpf` extended with hybrid keyword + vector path
- `fpf/knowledge::ingest_fpf_directory` encodes embeddings when feature enabled
- `forgeplan supersede PRD-018 --by PRD-042` (cleanup false-active stub)

**Waves:**
- W1: `schema-migration` (db/schema.rs + db/migrate.rs)
- W2: `hybrid-search-impl` (db/store.rs::search_fpf) + `ingest-embeddings` (fpf/knowledge.rs)
- W3: Tests + Audit + supersede PRD-018

---

## Final Release Cycle (after all sprints)

После merge всех 8 sprint'ов в `release/v0.17.0`:

```
1. ✅ /forge-cycle full audit on release branch
   - cargo test --workspace (all PASS)
   - cargo fmt --check (0 diffs)
   - cargo check (0 warnings)
   - forgeplan health --ci (PASS)
   - forgeplan validate --ci (PASS)
   - /audit с минимум 4 агентами:
     * code-reviewer (security + quality)
     * Rust expert (idiomatic patterns)
     * architect-reviewer (system integrity)
     * test-coverage analyzer
2. Fix all HIGH/CRITICAL findings on release branch
3. Re-run cargo test + audit until clean
4. PR release/v0.17.0 → main (merge commit)
5. Tag v0.17.0
6. Sync main → dev
7. cargo-dist release (5 platforms)
8. memory_retain final report in Hindsight
9. Update SPRINTS.md, TODO.md, CHANGELOG.md
```

---

## Effort Summary

| Sprint | Branch | Time | LOC | Artifact | Risk |
|--------|--------|------|-----|----------|------|
| 13.0 Security | feat/sprint-13.0-security | 1-2h | ~10 | — | 🔴 HIGH |
| 13.1 Search v2 | feat/sprint-13.1-prd-039-search | 1-1.5d | ~430 | PRD-039 | 🟢 LOW |
| 13.4a Tags | feat/sprint-13.4a-prd-035-tags | 1.5d | ~250 | PRD-035p1 | 🟡 MED |
| 13.4b Discover | feat/sprint-13.4b-prd-035-discover | 2d | ~500 | PRD-035p2 | 🟡 MED |
| 13.5 Scoring | feat/sprint-13.5-prd-040-scoring | 0.5-1d | ~130 | PRD-040 | 🟢 LOW |
| 13.2 FPF Rules | feat/sprint-13.2-prd-041-fpf-rules | 1d | ~150 | PRD-041 | 🟡 MED |
| 13.3 KB Search | feat/sprint-13.3-prd-042-kb-search | 0.5-1d | ~150 | PRD-042 | ⚪ LOW |
| **Final audit** | release/v0.17.0 | 0.5d | — | — | — |
| **TOTAL** | | **~8-11d** | **~1620** | **5 PRDs + 1 EPIC** | |

---

## Pre-flight checklist (before starting Sprint 13.0)

- [x] Roadmap saved (this file)
- [x] EPIC-003 created, shaped, validate PASS
- [x] PRD-041 (FPF Rules) created, shaped, validate PASS
- [x] PRD-042 (KB Vector Search) created, shaped, validate PASS
- [x] PROB-024 created (duplicate guard problem)
- [x] PRD-043 created (Methodology Integrity solution — Sprint 13.1 FOUNDATION)
- [x] All artifacts linked to EPIC-003
- [x] Audit findings on shaping batch addressed (5 issues fixed)
- [ ] release/v0.17.0 branch created from dev (after PR #143 merged)
- [ ] Pre-flight done → start Sprint 13.0

## Audit log (2026-04-07)

PR #143 originally created without audit. After audit caught:

| # | Severity | Finding | Fix |
|---|----------|---------|-----|
| 1 | CRITICAL | PRD-042 dup of active PRD-018 stub | Will supersede PRD-018 in Sprint 13.7 |
| 2 | HIGH | PRD-042 wrong Affected Files (knowledge.rs vs db/store.rs) | Fixed |
| 3 | MED | PRD-041 "possibly expose" — API already public | Fixed |
| 4 | MED | Sprint numbering chaotic (13.0, 13.1, 13.4a...) | Renumbered linearly |
| 5 | LOW | PRD-042 Problem mischaracterized search_fpf as primitive grep | Fixed |
| BONUS | NEW | Discovered class of issue → PROB-024 + PRD-043 | Added to roadmap as Sprint 13.1 FOUNDATION |
