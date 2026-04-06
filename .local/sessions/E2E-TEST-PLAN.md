# E2E Test Plan — Forgeplan CLI

> 56 commands, ~140 test cases, 11 waves
> Created: 2026-04-03
> Status: Draft

## Strategy

**Two runs:**
1. **Clean tempdir** — `forgeplan init -y` on empty project, verify all commands from scratch
2. **Real workspace** — current ForgePlan with 130 artifacts, production-like scenarios

**Priority (HIGH → LOW):**
- Wave 1-3: Core CRUD + Lifecycle — if broken, everything else is meaningless
- Wave 4-6: Graph + Search + Analysis — main value proposition
- Wave 7-8: LLM + FPF + Data — depend on external services
- Wave 9-11: Memory + Infra + Edge cases

## Known Bugs (found pre-E2E)

| # | Bug | Severity | Root Cause | File |
|---|-----|----------|------------|------|
| BUG-1 | `blocked` treats deprecated/superseded as blockers | P1 | `active_ids` filters only `active`, should include deprecated+superseded as "resolved" | `crates/forgeplan-core/src/graph/topological.rs:106-111` |
| BUG-2 | PROB-013 phantom in `tree` — shows as "?" with no title | P2 | Relation in LanceDB references deleted artifact; tree doesn't handle missing artifacts | `crates/forgeplan-cli/src/commands/tree.rs` |
| BUG-3 | Installed release binary silent stdout | P2 | Stale build in `~/.cargo/bin`; fixed by `cargo install --force` | Build toolchain |

---

## Wave 1: Init + Workspace (Clean tempdir)

```bash
TMP=$(mktemp -d) && cd $TMP

forgeplan init -y                    # EXPECT: creates .forgeplan/, success message
forgeplan init -y                    # EXPECT: error — already exists
forgeplan init --force -y            # EXPECT: success — reinit
forgeplan health                     # EXPECT: 0 artifacts, empty dashboard
forgeplan health --compact           # EXPECT: one-line output
forgeplan health --json              # EXPECT: valid JSON
forgeplan status                     # EXPECT: dashboard with zeros
```

**Pass criteria:** all 7 commands return expected output, no panics.

---

## Wave 2: CRUD (new / get / list / update / delete)

### 2a: Create all artifact kinds
```bash
forgeplan new prd "Test PRD"         # EXPECT: PRD-001 created
forgeplan new rfc "Test RFC"         # EXPECT: RFC-001
forgeplan new adr "Test ADR"         # EXPECT: ADR-001
forgeplan new note "Test Note"       # EXPECT: NOTE-001
forgeplan new evidence "Test Evid"   # EXPECT: EVID-001
forgeplan new problem "Test Prob"    # EXPECT: PROB-001
forgeplan new epic "Test Epic"       # EXPECT: EPIC-001
forgeplan new solution "Test Sol"    # EXPECT: SOL-001
forgeplan new spec "Test Spec"       # EXPECT: SPEC-001
forgeplan new refresh "Test Ref"     # EXPECT: REF-001 (or graceful error)
forgeplan new memory "Test Memory"   # EXPECT: mem-xxx or "use remember command" hint
forgeplan new INVALID "Test"         # EXPECT: error — unknown kind
```

### 2b: List
```bash
forgeplan list                       # EXPECT: shows all created artifacts
forgeplan list --type prd            # EXPECT: only PRDs
forgeplan list --status draft        # EXPECT: only drafts
forgeplan list --json                # EXPECT: valid JSON array
forgeplan list --type INVALID        # EXPECT: empty list or error
```

### 2c: Get
```bash
forgeplan get PRD-001                # EXPECT: full content with frontmatter + body
forgeplan get PRD-001 --json         # EXPECT: valid JSON
forgeplan get NONEXISTENT            # EXPECT: error "not found"
```

### 2d: Update
```bash
forgeplan update PRD-001 --title "Updated Title"      # EXPECT: success
forgeplan update PRD-001 --depth standard              # EXPECT: success
forgeplan update PRD-001 --body "New body content"     # EXPECT: success
forgeplan update PRD-001 --status active               # EXPECT: success or "use activate"?
forgeplan update NONEXISTENT --title "X"               # EXPECT: error
```

### 2e: Delete
```bash
forgeplan delete NOTE-001 --yes      # EXPECT: deleted
forgeplan delete NONEXISTENT --yes   # EXPECT: error
forgeplan get NOTE-001               # EXPECT: error — deleted
```

**Pass criteria:** 22 commands, all return expected output, no panics. All CRUD operations are consistent.

---

## Wave 3: Validation + Scoring + Lifecycle

### 3a: Validation
```bash
forgeplan validate PRD-001           # EXPECT: MUST errors for stub PRD
forgeplan validate PRD-001 --json    # EXPECT: valid JSON with errors list
forgeplan validate PRD-001 --adversarial  # EXPECT: works or "LLM required" error
forgeplan validate                   # EXPECT: validate all artifacts
```

### 3b: Scoring
```bash
forgeplan score PRD-001              # EXPECT: R_eff = 0.00 (no evidence)
forgeplan score PRD-001 --json       # EXPECT: valid JSON
forgeplan score --all                # EXPECT: scores for all active artifacts

forgeplan fgr PRD-001                # EXPECT: F-G-R quality scores
forgeplan fgr PRD-001 --json         # EXPECT: valid JSON
forgeplan fgr                        # EXPECT: all artifacts
```

### 3c: Lifecycle transitions
```bash
forgeplan review PRD-001             # EXPECT: review checklist with issues
forgeplan activate PRD-001           # EXPECT: FAIL — MUST validation errors
forgeplan activate PRD-001 --force   # EXPECT: success — force activate
forgeplan activate PRD-001           # EXPECT: error — already active

forgeplan new prd "Replacement"      # EXPECT: PRD-002
forgeplan supersede PRD-001 --by PRD-002  # EXPECT: PRD-001 → superseded
forgeplan activate PRD-001           # EXPECT: error — superseded is terminal

forgeplan deprecate PRD-002 --reason "test"  # EXPECT: draft → deprecated (or error if only active allowed)

forgeplan stale                      # EXPECT: no stale
forgeplan decay                      # EXPECT: evidence decay report
```

### 3d: Stale / Renew / Reopen
```bash
forgeplan new adr "Stale Test"       # EXPECT: ADR-002
forgeplan activate ADR-002 --force   # EXPECT: active
forgeplan renew ADR-002 --reason "still relevant" --until 2027-01-01  # EXPECT: success (or error if not stale)
forgeplan reopen ADR-002 --reason "needs rethink"  # EXPECT: creates NEW artifact + deprecates ADR-002
```

**Pass criteria:** 20 commands. Lifecycle state machine follows ADR-005 rules. Terminal states are enforced.

---

## Wave 4: Links + Graph

### 4a: Link operations
```bash
forgeplan link RFC-001 PRD-001 --relation based_on    # EXPECT: success
forgeplan link EVID-001 PRD-001 --relation informs    # EXPECT: success
forgeplan link PRD-001 PRD-001 --relation based_on    # EXPECT: BLOCKED — self-link guard (PROB-019)
forgeplan link NONEXISTENT PRD-001                     # EXPECT: error — source not found
forgeplan link PRD-001 NONEXISTENT                     # EXPECT: error — target not found (or creates phantom?)
```

### 4b: Unlink operations
```bash
forgeplan unlink RFC-001 PRD-001 --relation based_on   # EXPECT: success
forgeplan unlink RFC-001 PRD-001 --relation based_on   # EXPECT: error — already unlinked
forgeplan unlink NONEXISTENT PRD-001                    # EXPECT: error
```

### 4c: Graph visualization
```bash
forgeplan graph                      # EXPECT: mermaid diagram
forgeplan graph --json               # EXPECT: valid JSON
forgeplan tree                       # EXPECT: ASCII tree
forgeplan tree PRD-001               # EXPECT: subtree from PRD-001
forgeplan tree --depth 2             # EXPECT: limited depth
forgeplan tree --json                # EXPECT: valid JSON
forgeplan order                      # EXPECT: topological sort
forgeplan order --json               # EXPECT: valid JSON
forgeplan blocked                    # EXPECT: blocked analysis (check BUG-1!)
forgeplan blocked PRD-001            # EXPECT: specific artifact
forgeplan blocked --json             # EXPECT: valid JSON
```

**Pass criteria:** 16 commands. Self-link guard works. No phantom links. Graph commands handle empty/sparse graphs.

---

## Wave 5: Search + Analysis

### 5a: Search modes
```bash
forgeplan search "test"              # EXPECT: smart search results
forgeplan search "test" --keyword    # EXPECT: keyword-only results
forgeplan search "test" --semantic   # EXPECT: semantic results (may need embeddings)
forgeplan search "test" --type prd   # EXPECT: filtered by kind
forgeplan search "test" -n 5         # EXPECT: max 5 results
forgeplan search "test" --json       # EXPECT: valid JSON
forgeplan search ""                  # EXPECT: error or empty results
forgeplan search "nonexistent_gibberish_xyz_12345"  # EXPECT: 0 results, no error
```

### 5b: Analysis commands
```bash
forgeplan blindspots                 # EXPECT: blind spots report
forgeplan gaps                       # EXPECT: pipeline compliance
forgeplan journal                    # EXPECT: decision journal
forgeplan journal --type adr         # EXPECT: filtered
forgeplan journal --risk             # EXPECT: at-risk only
forgeplan progress                   # EXPECT: checkbox progress all
forgeplan progress PRD-001           # EXPECT: specific artifact
forgeplan progress --json            # EXPECT: valid JSON
```

**Pass criteria:** 16 commands. Smart search returns ranked results. Analysis commands handle empty data gracefully.

---

## Wave 6: Codebase Awareness

```bash
forgeplan scan                       # EXPECT: scan current dir modules
forgeplan scan --path /tmp           # EXPECT: path traversal guard (canonicalize, boundary check)
forgeplan scan --path ./nonexistent  # EXPECT: error — path not found
forgeplan coverage                   # EXPECT: decision coverage report
forgeplan coverage --backfill        # EXPECT: backfill mode
forgeplan drift                      # EXPECT: drifted decisions
forgeplan drift --json               # EXPECT: valid JSON
forgeplan calibrate                  # EXPECT: depth calibration for all
forgeplan calibrate PRD-001          # EXPECT: specific artifact
```

**Pass criteria:** 9 commands. Path traversal is blocked. Coverage/drift work with or without git history.

---

## Wave 7: Routing + Estimation

### 7a: Smart routing
```bash
forgeplan route "fix a typo"                          # EXPECT: Tactical
forgeplan route "implement new auth system"            # EXPECT: Standard or higher
forgeplan route "redesign entire data model"           # EXPECT: Deep
forgeplan route "fix a typo" --level 0                 # EXPECT: explicit L0 keywords
forgeplan route "fix a typo" --level 1                 # EXPECT: L1 LLM (needs API key)
forgeplan route ""                                     # EXPECT: error or default
```

### 7b: Estimation
```bash
forgeplan estimate PRD-001                             # EXPECT: rule-based effort
forgeplan estimate PRD-001 --grade senior              # EXPECT: senior grade
forgeplan estimate PRD-001 --my-grade                  # EXPECT: config profile
forgeplan estimate PRD-001 --json                      # EXPECT: valid JSON
forgeplan estimate PRD-001 --complexity "FR-001=8"     # EXPECT: manual override
forgeplan estimate NONEXISTENT                         # EXPECT: error
forgeplan calibrate-estimate PRD-001 --actual-hours 10 # EXPECT: calibration result
forgeplan calibrate-estimate PRD-001 --actual-hours 10 --grade senior  # EXPECT: grade-specific
```

**Pass criteria:** 14 commands. Routing returns valid depth. Estimation handles missing FR gracefully.

---

## Wave 8: LLM Commands (require GEMINI_API_KEY or similar)

> Skip if no API key available. Mark as SKIP, not FAIL.

```bash
forgeplan generate prd "Auth system with SSO"         # EXPECT: AI-generated PRD
forgeplan reason PRD-001                               # EXPECT: ADI analysis
forgeplan reason PRD-001 --fpf                         # EXPECT: with FPF patterns
forgeplan reason PRD-001 --save                        # EXPECT: save as Note
forgeplan reason PRD-001 --json                        # EXPECT: JSON output
forgeplan decompose PRD-001                            # EXPECT: PRD → RFC tasks
forgeplan context PRD-001                              # EXPECT: full context bundle
forgeplan context PRD-001 --json                       # EXPECT: JSON
forgeplan capture "Use PostgreSQL for persistence"     # EXPECT: creates Note/ADR
forgeplan capture "Use Redis" --context "for caching"  # EXPECT: with context
```

**Pass criteria:** 10 commands. Without API key: graceful error messages (not panics). With key: valid AI output.

---

## Wave 9: FPF Knowledge Base

```bash
forgeplan fpf status                 # EXPECT: KB status (ingested count, source)
forgeplan fpf ingest                 # EXPECT: ingest FPF spec (204 sections)
forgeplan fpf list                   # EXPECT: list all sections
forgeplan fpf search "trust"         # EXPECT: finds B.3 Trust Calculus
forgeplan fpf search "trust" --limit 3  # EXPECT: max 3 results
forgeplan fpf section "B.3"          # EXPECT: full section content
forgeplan fpf section "B.3" --summary   # EXPECT: summary only
forgeplan fpf section "NONEXISTENT"  # EXPECT: error — section not found
forgeplan fpf dashboard              # EXPECT: dashboard with scores
```

**Pass criteria:** 9 commands. FPF ingest is idempotent. Search finds relevant sections.

---

## Wave 10: Memory + Data Safety

### 10a: Memory commands
```bash
forgeplan remember "Always use PostgreSQL" --category convention   # EXPECT: mem-xxx created
forgeplan remember --list             # EXPECT: lists all memories
forgeplan recall "PostgreSQL"         # EXPECT: finds the memory
forgeplan recall --category convention # EXPECT: filtered
forgeplan recall --json               # EXPECT: valid JSON
forgeplan remember --forget mem-xxx   # EXPECT: deleted (use actual ID)
forgeplan promote mem-xxx --kind prd  # EXPECT: promoted to PRD (use actual ID or new memory)
```

### 10b: Export / Import
```bash
forgeplan export                      # EXPECT: exports to default path
forgeplan export --output /tmp/test-export.json  # EXPECT: exports to specified path
forgeplan import /tmp/test-export.json           # EXPECT: imports
forgeplan import /tmp/test-export.json --force   # EXPECT: overwrites
forgeplan import NONEXISTENT.json                # EXPECT: error — file not found
```

### 10c: Scan-import
```bash
forgeplan scan-import --dry-run       # EXPECT: preview only
forgeplan scan-import                 # EXPECT: scan and import docs
forgeplan scan-import --path ./docs   # EXPECT: specific directory
```

**Pass criteria:** 15 commands. Export → Import roundtrip preserves data. Memory lifecycle works.

---

## Wave 11: Infrastructure + Edge Cases

### 11a: Infrastructure commands
```bash
forgeplan log                         # EXPECT: audit trail
forgeplan log --limit 5               # EXPECT: max 5 entries
forgeplan log PRD-001                 # EXPECT: specific artifact log
forgeplan log --source cli            # EXPECT: filtered by source
forgeplan log --json                  # EXPECT: valid JSON

forgeplan reindex                     # EXPECT: rebuild index from .md files
forgeplan embed                       # EXPECT: generate embeddings (may be slow)
forgeplan migrate                     # EXPECT: schema migration (no-op if current)
forgeplan git-sync                    # EXPECT: sync from git (may need recent pull)

forgeplan watch &                     # EXPECT: starts watcher
sleep 2 && kill %1                    # EXPECT: clean shutdown

forgeplan serve &                     # EXPECT: starts MCP server on stdio
sleep 2 && kill %1                    # EXPECT: clean shutdown

forgeplan setup-skill                 # EXPECT: installs Claude Code skill
```

### 11b: CLI meta
```bash
forgeplan --version                   # EXPECT: "forgeplan X.Y.Z"
forgeplan help                        # EXPECT: help text with all commands
forgeplan NONEXISTENT_COMMAND         # EXPECT: error with suggestion
```

### 11c: Edge cases — no workspace
```bash
cd $(mktemp -d)
forgeplan list                        # EXPECT: error — no .forgeplan/ found
forgeplan health                      # EXPECT: error — no .forgeplan/ found
forgeplan get PRD-001                 # EXPECT: error — no workspace
```

### 11d: Edge cases — corrupt data
```bash
# Setup: init workspace, create artifact, corrupt markdown
forgeplan init -y
forgeplan new note "Corrupt Test"
echo "GARBAGE" > .forgeplan/notes/note-corrupt-test.md
forgeplan reindex                     # EXPECT: handles gracefully, warns about corrupt file
forgeplan list                        # EXPECT: shows what it can, warns about corrupt
```

### 11e: Edge cases — stress
```bash
# Create 50 artifacts rapidly
for i in $(seq 1 50); do forgeplan new note "Stress $i"; done
forgeplan list | wc -l                # EXPECT: 50+ lines
forgeplan search "Stress" | wc -l    # EXPECT: results
forgeplan health                      # EXPECT: completes in < 5s
forgeplan tree                        # EXPECT: renders all
```

**Pass criteria:** 20+ commands. No panics on corrupt data. Graceful errors without workspace. Performance acceptable under stress.

---

## Summary

| Wave | Commands | Focus | Priority |
|------|----------|-------|----------|
| 1 | 7 | Init + Workspace | HIGH |
| 2 | 22 | CRUD | HIGH |
| 3 | 20 | Validation + Scoring + Lifecycle | HIGH |
| 4 | 16 | Links + Graph | HIGH |
| 5 | 16 | Search + Analysis | HIGH |
| 6 | 9 | Codebase Awareness | HIGH |
| 7 | 14 | Routing + Estimation | MEDIUM |
| 8 | 10 | LLM Commands | MEDIUM (skip if no key) |
| 9 | 9 | FPF Knowledge Base | MEDIUM |
| 10 | 15 | Memory + Data Safety | MEDIUM |
| 11 | 20+ | Infrastructure + Edge Cases | MEDIUM |
| **Total** | **~158** | | |

## Execution Log

> Fill in during test run. Format: `[PASS|FAIL|SKIP] command — notes`

### Run 1: Clean tempdir (date: ___)
<!-- paste results here -->

### Run 2: Real workspace (date: ___)
<!-- paste results here -->

---

## Execution Log

### Run 1: Sprint 8 (2026-04-03)
- Waves 1-7: 83 commands, 0 failures (clean tempdir + real workspace)
- Known bugs found: 10 (all fixed in PR #95)

### Run 2: Sprint 9 (2026-04-04)
- Wave 8 (LLM): 10/10 pass (gemini-3-flash-preview)
- Wave 9 (FPF KB): 9/9 pass (204 sections, search, dashboard)
- Wave 10 (Memory+Data): 11/11 pass (remember/recall/forget, export/import, scan)
- Wave 11 (Infra+Edge): 8/8 infra + 3/3 meta + 3/3 no-workspace + 1/1 corrupt + 1/1 stress(50 artifacts in 3s)

### Total: 139 commands tested, 0 failures
