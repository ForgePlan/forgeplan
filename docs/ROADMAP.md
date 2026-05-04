# Forgeplan Roadmap — Gap Analysis & Priorities

> Generated 2026-04-11 after v0.18.0 release. Based on /fpf DECOMPOSE
> analysis of 193 artifacts, 3 completed EPICs, and 16 draft IDEAS.

## Current State

- **v0.18.0** released (Production BM25 + Russian morphology + quality gates)
- **193 artifacts**, 112 active, health: "Project looks healthy!"
- **1940 tests**, 76 CLI commands, 63 MCP tools
- **3 EPICs complete**: EPIC-001 (foundation), EPIC-002 (v2.0 vision), EPIC-003 (search+intelligence)

---

## Categories

### 1. Architecture (Core Engine) — 85%

| Done | Gap |
|---|---|
| LanceDB embedded (ADR-003 files-first) | Code-fence awareness in extract_field (PROB-035 remainder) |
| 10 artifact types + lifecycle state machine | DSL scripting for custom rules (NOTE-039 — Lua/Rhai) |
| R_eff weakest-link + CL penalty + fail-closed | Pluggable storage drivers (RFC-003 planned, not wired) |
| FPF engine v2 (rule engine + bounded contexts) | Delta-specs (OpenSpec pattern, deferred from PRD-015) |
| Graph (petgraph, topological sort, blocked/order) | |
| Production BM25 + Russian stemming (v0.18.0) | |
| Semantic search BGE-M3 (feature-gated) | |
| Trust calculus hardening (F1/F2 fail-closed, v0.17.2) | |

**Assessment:** Core is solid. Remaining gaps are extensibility and hardening.

### 2. UX / Usability — 70%

| Done | Gap |
|---|---|
| 56 CLI commands with clap derive | **Desktop App** (Tauri + React) — Phase 5, not started |
| `--json` output on most commands | `forgeplan doctor` — workspace diagnostics (NOTE-029) |
| Styled terminal (colors, progress bars, unicode) | `forgeplan links` — visual relationship graph (NOTE-029) |
| `health` + `tree` + `blocked` dashboards | `forgeplan diff` — artifact comparison (NOTE-030) |
| Error hints with suggested next commands | `forgeplan watch` v2 — hot-reload (NOTE-030) |
| Duplicate guard + stub detection (PRD-043) | VS Code extension (NOTE-030) |
| 63 MCP tools for AI agents | **Website** — landing + docs portal (PRD-024) |

**Assessment:** CLI is mature, MCP is excellent. No GUI. Website and Desktop are the main gaps for user adoption.

### 3. Performance — 80%

| Done | Gap |
|---|---|
| O(N) batch BM25 search (v0.18.0) | Lazy loading for large workspaces (1000+ artifacts) |
| 43 MB binary (strip+lto+codegen-units optimized) | Incremental reindex (currently full scan) |
| 0.23s search on 193 artifacts | Background embedding (BGE-M3 blocks ~60s on first run) |
| LanceDB columnar + Arrow (fast reads) | |

**Assessment:** Fast for current scale (100-500 artifacts). Gaps matter only for scale-up to enterprise.

### 4. Distribution — 65%

| Done | Gap |
|---|---|
| `brew install forgeplan` (Homebrew tap) | **crates.io** (`cargo install forgeplan`) |
| cargo-dist (macOS arm/x86, Linux, Windows) | **npm/npx wrapper** (JS ecosystem) |
| GitHub Releases with prebuilt binaries | **Docker image** |
| install.sh script | **Linux native** (apt, snap, flatpak) |
| CI pipeline (fmt + clippy + tests + health gate) | **Auto-update** notification mechanism |

**Assessment:** macOS excellent. Linux/Windows via GH releases but not via native package managers.

### 5. Documentation — 60%

| Done | Gap |
|---|---|
| CLAUDE.md (full AI agent guide) | **Public website** (landing + docs portal) |
| docs/methodology/ (10 files) | **Public README.md** (current one is internal-focused) |
| FORGEPLAN-GUIDE.md (full CLI + methodology ref) | **MCP tools API reference** |
| CHANGELOG (v0.17.0 → v0.18.0) | **Video/tutorial** walkthrough |
| FPF KB (204 sections, searchable) | **man pages** |
| 8-point verification checklist (v0.18.0) | |

**Assessment:** Internal docs are excellent. Public-facing documentation is the biggest gap for adoption.

### 6. Integrations — 55%

| Done | Gap |
|---|---|
| MCP server (63 tools, stdio transport) | **Linear/Jira sync** (NOTE-028) |
| LLM integration (Gemini, configurable provider) | **GitHub Issues bridge** |
| git-sync (frontmatter to LanceDB) | **Slack/Teams notifications** |
| Claude Code hooks (safety, forge-mode) | **CI/CD pipeline gates** (NOTE-026) |
| Orchestra integration (task management) | **VS Code / JetBrains extension** |

**Assessment:** MCP-first approach is strong. Ecosystem integrations are the gap.

---

## Priority Matrix

| Category | Maturity | Biggest Gap | User Impact | Effort |
|---|---|---|---|---|
| Architecture | 85% | Code-fence, DSL | Low | Small |
| **UX** | **70%** | **Desktop + Website** | **HIGH** | Large |
| Performance | 80% | Incremental reindex | Medium | Medium |
| **Distribution** | **65%** | **crates.io + Docker** | **HIGH** | Small |
| **Documentation** | **60%** | **Public website + README** | **HIGH** | Medium |
| **Integrations** | **55%** | **CI/CD gates + trackers** | **MEDIUM** | Medium |

---

## Recommended Next Sprints

### Sprint A: Public Presence (3-5 days)
> Close the "people can't find us" gap.

- [ ] Website landing page (PRD-024, Astro + Starlight)
- [ ] Public README.md rewrite (user-facing, not internal)
- [ ] crates.io publish (`cargo install forgeplan`)
- [ ] Docker image (Dockerfile + GH Actions publish)

### Sprint B: CI/CD Integration (1-2 days)
> Make Forgeplan part of the dev workflow, not a separate tool.

- [ ] `forgeplan validate --ci` exit code for pipeline gates
- [ ] `forgeplan health --ci` with structured JSON output
- [ ] GitHub Action reusable workflow (`forgeplan/action`)
- [ ] CI/CD setup guide in docs

### Sprint C: Desktop App (2-4 weeks)
> For users who don't live in the terminal.

- [ ] EPIC-004: Tauri 2.0 + React UI
- [ ] Shared Rust core (forgeplan-core)
- [ ] Dashboard view (health, tree, search)
- [ ] Artifact editor with live validation

### Sprint D: Ecosystem (1-2 weeks)
> Connect Forgeplan to existing tools.

- [ ] VS Code extension (tree view + search + score)
- [ ] GitHub Issues bridge (artifact ↔ issue sync)
- [ ] Linear/Jira export adapter
- [ ] Slack notification hooks

---

## Backlog (IDEAS — no commitment)

| ID | Idea | Category |
|---|---|---|
| NOTE-025 | Agent Memory Engine | Integrations |
| NOTE-026 | CI/CD Architecture Linter | Integrations |
| NOTE-027 | Ruflo/Gastown Integration | Integrations |
| NOTE-028 | Task Tracker Bridges | Integrations |
| NOTE-029 | CLI UX Polish (doctor, links) | UX |
| NOTE-030 | Tier 2-3 features (diff, watch, dashboard) | UX |
| NOTE-039 | DSL scripting (Lua/Rhai) | Architecture |
| NOTE-042 | TECH DEBT: update --body file-first | Architecture |
| PROB-022 | Brownfield onboarding improvements | UX |
| PRD-025 | Nx Monorepo Migration | Architecture |
