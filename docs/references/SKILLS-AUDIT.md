# Skills & Plugins Audit for PRD Process Engine

## Inventory

| Category | Count |
|----------|-------|
| Slash Commands | 33 |
| Skills | 37 |
| Agent Personas | 50+ |
| SPARC Sub-Modes | 31 |
| MCP Integrations | 6 servers |

---

## Layer Coverage Map

### Layer 1: Discovery & Research ✅ FULL

| Asset | Type | Priority |
|-------|------|----------|
| `/research` | Command | **MUST** |
| `/deep-research` | Command | **MUST** |
| `/recall` | Command | **MUST** |
| `mcp__hindsight__memory_recall` | MCP | **MUST** |
| `/briefing` | Command | SHOULD |
| `sparc/researcher` | SPARC | SHOULD |
| `research-analyst` | Agent | SHOULD |

### Layer 2: Requirements & PRD ⚠️ GAP

| Asset | Type | Priority |
|-------|------|----------|
| `/write-doc rfc` | Command | **MUST** |
| `rfc-template` | Command | **MUST** |
| `sparc/spec-pseudocode` | SPARC | **MUST** |

**GAP**: No `/write-doc prd`, no user story generator, no PRD-specific template.

### Layer 3: Architecture & Design ✅ FULL

| Asset | Type | Priority |
|-------|------|----------|
| `architecture-guardian` | Agent | **MUST** |
| `architect-reviewer` | Agent | **MUST** |
| `sparc/architect` | SPARC | **MUST** |
| `v3-ddd-architecture` | Skill | SHOULD |
| `microservices-architect` | Agent | SHOULD |

### Layer 4: Specification & Contracts ⚠️ GAP

| Asset | Type | Priority |
|-------|------|----------|
| `sparc/spec-pseudocode` | SPARC | **MUST** |
| `gerts-api-tester` | Agent | SHOULD |

**GAP**: No contract-first API design tool. Specs generated FROM code, not BEFORE.

### Layer 5: Decision Making (ADR) ✅ OK

| Asset | Type | Priority |
|-------|------|----------|
| `/write-doc adr` | Command | **MUST** |
| `mcp__hindsight__memory_retain` | MCP | **MUST** |
| `v3/adr-architect` | Agent | SHOULD |

### Layer 6: Planning & Decomposition ✅ FULL

| Asset | Type | Priority |
|-------|------|----------|
| `/sprint` | Command | **MUST** |
| `/wave` | Command | **MUST** |
| `/synthesize` | Command | **MUST** |
| `/do` | Command | **MUST** |
| `goal/goal-planner` | Agent | SHOULD |

### Layer 7: Implementation & Sprint ✅ FULL

| Asset | Type | Priority |
|-------|------|----------|
| `/team-up` | Command | **MUST** |
| `/build` | Command | **MUST** |
| `sparc/code` + `sparc/coder` | SPARC | SHOULD |
| `pair-programming` | Skill | SHOULD |
| `swarm-orchestration` | Skill | SHOULD |

### Layer 8: Quality & Review ✅ FULL

| Asset | Type | Priority |
|-------|------|----------|
| `/audit` | Command | **MUST** |
| `verification-quality` | Skill | **MUST** |
| `github-code-review` | Skill | **MUST** |
| `sparc/reviewer` | SPARC | SHOULD |
| `sparc/security-review` | SPARC | SHOULD |

### Layer 9: Documentation ✅ FULL

| Asset | Type | Priority |
|-------|------|----------|
| `/write-doc` | Command | **MUST** |
| `sparc/docs-writer` | SPARC | **MUST** |
| `/load-doc` + `/sync-docs` | Command | SHOULD |
| `documentation-engineer` | Agent | SHOULD |

### Layer 10: Methodology & Frameworks ✅ OK

| Asset | Type | Priority |
|-------|------|----------|
| `sparc-methodology` | Skill | **MUST** |
| `reasoningbank-intelligence` | Skill | SHOULD |
| `skill-builder` | Skill | SHOULD |

---

## Critical Gaps

| Layer | Gap | Impact | Build Priority |
|-------|-----|--------|----------------|
| **2. PRD** | No PRD template/command | Core feature missing | P0 |
| **2. PRD** | No user story generator | Product requirements incomplete | P1 |
| **4. Spec** | No contract-first design | API specs only from code | P1 |
| **6. Planning** | No Epic decomposition | No PRD → Epic → Story → Task hierarchy | P0 |
| **8. Quality** | No acceptance validator | Can't verify PRD acceptance criteria | P2 |

## Cross-Cutting Assets

| Asset | Layers | Notes |
|-------|--------|-------|
| `/do` | 1,2,6,7,8,9 | **Proto-PRD-engine** — classifies intent, chains commands |
| `/sprint` | 1,6,7 | Discovery → planning → execution |
| `sparc-methodology` | 2,3,4,7,8,10 | 17 modes, full lifecycle |
| `mcp__hindsight__*` | 1,5,9 | Cross-session knowledge persistence |

## Key Insight

> The `/do` command is already a proto-PRD-engine. The main gap is on the **product side** — requirements, user stories, acceptance criteria, epic decomposition. The **engineering side** (architecture, implementation, review, docs) is well-covered with 5+ tools per layer.

## Redundancies (acceptable)

| Area | Assets | Verdict |
|------|--------|---------|
| Code Review | `/audit`, `github-code-review`, `sparc/reviewer`, `code-reviewer`, `core/reviewer` | 5-way — different granularities, keep all |
| Research | `/research`, `/deep-research`, `sparc/researcher`, `research-analyst` | 4-way — clear quick/deep/web/persona split |
| Architecture | `architecture-guardian`, `architect-reviewer`, `sparc/architect`, DDD, microservices | 5-way — gerts-specific vs generic vs pattern-specific |
| Swarm | `swarm-orchestration`, `swarm-advanced`, `v3-swarm-coordination`, ruflo | Heavy — consolidate around Agent Teams |
