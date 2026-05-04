[English](FORGEPLAN-GUIDE.md) · [Русский](FORGEPLAN-GUIDE.ru.md)

# Forgeplan — Complete Practical Guide

> One document: methodology + commands + examples + pitfalls.
> For humans and AI agents alike.

---

## What Is Forgeplan

Forgeplan forces you to **think before coding**. Instead of "open IDE -> write code -> deploy", you get "determine depth -> create artifact -> validate quality -> confirm with evidence -> code".

**Not Jira.** Not project management. Not a task tracker. Forgeplan is a **structured knowledge base** for engineering decisions.

**Primary consumer**: AI agent (Claude Code, Cursor) via MCP server. CLI is for human inspection.

---

## Installation

### 1. AI Skill (for any AI agent)

```bash
# Install /forge skill for Claude Code, Cursor, Codex, Gemini, and 40+ agents
npx skills add ForgePlan/forgeplan --skill forge
```

The skill will be installed into selected agents. After that, in your AI chat:
```
/forge "Add OAuth2 authentication"
```

### 2. CLI Binary

```bash
# macOS (Homebrew)
brew install forgeplan/tap/forgeplan

# From source (Rust)
cargo install forgeplan

# Or download a binary from GitHub Releases
# https://github.com/ForgePlan/forgeplan/releases
```

### 3. MCP Server (for AI agents)

Add to your project's `.mcp.json`:

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve"]
    }
  }
}
```

---

## Quick Start (5 minutes)

```bash
# 1. Initialize workspace
forgeplan init

# 2. Determine what to do
forgeplan route "Add OAuth2 authentication"
# -> Depth: Deep, Pipeline: PRD -> Spec -> RFC -> ADR

# 3. Create first artifact
forgeplan new prd "OAuth2 Authentication"

# 4. Check project state
forgeplan health
```

> **Alias**: `fpl` = `forgeplan`. Create a symlink: `ln -s $(which forgeplan) /usr/local/bin/fpl`

---

## Methodology: 3 Questions Instead of Bureaucracy

### Question 1: "What depth?"

Ask yourself one question: **"Is this reversible within a day?"**

| Answer | Depth | What to create | Example |
|--------|-------|---------------|---------|
| Yes, trivial | **Tactical** | Nothing or Note | Fix typo, update config |
| No, there are choices | **Standard** | PRD -> RFC | New feature, 1-3 days |
| No, affects many | **Deep** | PRD -> Spec -> RFC -> ADR | New module, 1-2 weeks |
| Strategy, cross-team | **Critical** | Epic -> PRD[] -> RFC[] -> ADR[] | New subsystem |

Or use automatic routing:

```bash
forgeplan route "task description"
```

The engine analyzes keywords (security -> Deep+, breaking change -> Deep+, cross-team -> Standard+) and provides an instant recommendation without LLM.

### Question 2: "Which artifact?"

| Artifact | Answers the question | When NOT needed |
|----------|---------------------|-----------------|
| **PRD** | WHAT and why? | Bug fix, refactoring |
| **RFC** | HOW to build? | Architecture is obvious, < 1 day |
| **ADR** | WHY this decision? | Decision is trivial and reversible |
| **Spec** | HOW EXACTLY does it work? | No API / data model changes |
| **Epic** | How to group? | Task = one PRD |

### Question 3: "Is the artifact ready?"

```bash
forgeplan review PRD-001
# -> MUST: Missing Problem section
# -> SHOULD: density < 50 words
# -> Ready to activate? NO
```

If MUST is empty -- activate. If not -- refine.

### The Main Rule

**Pipeline = guideline, NOT bureaucracy.** Don't create all 10 types for every task. Tactical depth = just do it. Standard = PRD + RFC. Only Deep+ requires the full pipeline.

---

## All Commands (by category)

### Creating and Managing Artifacts

| Command | What it does | Example |
|---------|-------------|---------|
| `forgeplan init` | Create .forgeplan/ workspace | `forgeplan init` |
| `forgeplan new <kind> "<title>"` | Create artifact from template | `forgeplan new prd "Auth System"` |
| `forgeplan get <id>` | Read artifact | `forgeplan get PRD-001` |
| `forgeplan update <id>` | Update metadata/body | `forgeplan update PRD-001 --status active` |
| `forgeplan delete <id>` | Delete artifact | `forgeplan delete PRD-001 --yes` |
| `forgeplan list` | List artifacts | `forgeplan list --type prd --status active` |

**Artifact kinds:** `prd`, `epic`, `spec`, `rfc`, `adr`, `note`, `problem`, `solution`, `evidence`, `refresh`

### Links and Graph

| Command | What it does | Example |
|---------|-------------|---------|
| `forgeplan link <src> <tgt>` | Link artifacts | `forgeplan link RFC-001 PRD-001 --relation based_on` |
| `forgeplan graph` | Mermaid dependency graph | `forgeplan graph` |

**Link types (--relation):** `informs`, `based_on`, `supersedes`, `contradicts`, `refines`

### Quality and Validation

| Command | What it does | Example |
|---------|-------------|---------|
| `forgeplan validate [id]` | Check completeness | `forgeplan validate PRD-001` |
| `forgeplan score [id]` | R_eff quality score | `forgeplan score PRD-001` |
| `forgeplan fgr [id]` | F-G-R scores (Formality, Granularity, Reliability) | `forgeplan fgr` |
| `forgeplan estimate <id>` | Effort estimate by grade (Jun/Mid/Sen/PS/AI) | `forgeplan estimate PRD-022` |
| `forgeplan estimate <id> --grade mid` | Highlight specific grade | `forgeplan estimate PRD-022 --grade junior` |
| `forgeplan estimate <id> --my-grade` | Grade from config grade_profile | `forgeplan estimate PRD-022 --my-grade` |

### Lifecycle

| Command | What it does | Example |
|---------|-------------|---------|
| `forgeplan review <id>` | Checklist: ready to activate? | `forgeplan review PRD-001` |
| `forgeplan activate <id>` | Draft -> Active (validation gate) | `forgeplan activate PRD-001` |
| `forgeplan supersede <id> --by <new>` | Active -> Superseded + chain warnings | `forgeplan supersede PRD-001 --by PRD-002` |
| `forgeplan deprecate <id> --reason "..."` | Active -> Deprecated | `forgeplan deprecate PRD-001 --reason "Cancelled"` |

**Rule:** Notes and Problems do not require a validation gate. PRD, RFC, ADR, Epic, Spec -- MUST rules must pass.

### Dashboards and Analytics

| Command | What it does | Example |
|---------|-------------|---------|
| `forgeplan health` | Full project health | `forgeplan health --compact` |
| `forgeplan status` | Brief dashboard | `forgeplan status` |
| `forgeplan blindspots` | Artifacts without evidence, orphans | `forgeplan blindspots` |
| `forgeplan journal` | Decision timeline with R_eff | `forgeplan journal --risk` |
| `forgeplan fpf` | FPF dashboard: contexts + F-G-R + actions | `forgeplan fpf` |
| `forgeplan stale` | Artifacts with expired valid_until | `forgeplan stale` |
| `forgeplan decay` | Impact of expired evidence on R_eff | `forgeplan decay` |
| `forgeplan progress [id]` | Checkbox progress bars | `forgeplan progress` |

### Routing and Calibration

| Command | What it does | Example |
|---------|-------------|---------|
| `forgeplan route "<description>"` | Rule-based depth + pipeline (no LLM) | `forgeplan route "Add OAuth2"` |
| `forgeplan route "<desc>" --explain` | + LLM explanation | `forgeplan route "Add OAuth2" --explain` |
| `forgeplan calibrate [id]` | Suggest depth for existing artifact | `forgeplan calibrate PRD-001` |

### AI-powered (require LLM config)

| Command | What it does | Example |
|---------|-------------|---------|
| `forgeplan generate <kind> "<desc>"` | AI artifact generation | `forgeplan generate prd "Payment system"` |
| `forgeplan reason <id>` | ADI reasoning cycle | `forgeplan reason PRD-001 --json` |
| `forgeplan decompose <id>` | PRD -> RFC tasks via AI | `forgeplan decompose PRD-001` |
| `forgeplan capture "<decision>"` | Record a decision as Note/ADR | `forgeplan capture "Use Redis for cache"` |
| `forgeplan search <query> --semantic` | Semantic vector search | `forgeplan search "auth" --semantic` |

### MCP Server

```bash
forgeplan serve  # start MCP server (stdio transport)
```

63 MCP tools -- all commands above are available via MCP protocol.

---

## Estimate Engine -- Effort Estimation

### Why

Turns documentation (FR in PRD, Phases in RFC) into effort estimates. No separate Excel spreadsheet needed -- the estimate lives alongside artifacts.

### Basic Command

```bash
forgeplan estimate PRD-022
```

Output table:

```
Estimate for PRD-022: AI Estimation Engine
Confidence: 40%

  ID      Description                  Cmpl   Jun    Mid  Senior    PS     AI
  ---------------------------------------------------------------------------
  FR-001  User can run estimate          3    16h    12h    8.0h  5.6h   1.0h
  FR-002  System extracts work items     3    16h    12h    8.0h  5.6h   1.0h
  FR-003  Fibonacci complexity           2    10h   7.5h    5.0h  3.5h   0.7h
  ---------------------------------------------------------------------------
  TOTAL                                  8    42h    32h     21h   15h   2.7h
                                              5.3d   3.9d   2.6d  1.8d  0.3d
```

### Flags

```bash
forgeplan estimate PRD-022 --grade middle   # highlight specific grade
forgeplan estimate PRD-022 --my-grade       # grade from config.yaml (domain-aware)
forgeplan estimate PRD-022 --json           # machine-readable output
```

### Calculation Model

**Base = Senior** (baseline x1.0). All grades are multipliers of Senior:

| Grade | Multiplier | Example (Medium=8h Senior) |
|-------|-----------|---------------------------|
| Junior | x2.0 | 16h |
| Middle | x1.5 | 12h |
| **Senior** | **x1.0** | **8h** (baseline) |
| Principal | x0.7 | 5.6h |
| AI | task-type | 1.0h (PureCoding) |

**AI is calculated differently** -- it accounts for task type:

| Task Type | AI Multiplier | Example (8h base) | With review (+30%) |
|-----------|-------------|-------------------|-----------------|
| PureCoding | x0.10 | 0.8h | **1.04h** |
| CodingInfra | x0.25 | 2.0h | 2.6h |
| DesignCoding | x0.30 | 2.4h | 3.1h |
| PureInfra | x0.50 | 4.0h | 5.2h |
| Coordination | x1.00 | 8.0h | 10.4h |

**Fibonacci complexity** (1, 2, 3, 5, 8, 13) -> base Senior hours (3h, 5h, 8h, 13h, 21h, 34h).

**Confidence** depends on artifact completeness:
- Has FR in PRD: +30%
- Has Implementation Phases in RFC: +25%
- Has Spec: +15%
- Has evidence from past tasks: +20%

### Configuration in config.yaml

Uncomment and customize:

```yaml
# .forgeplan/config.yaml
estimate:
  grade_profile:
    backend: middle        # your grade in backend
    frontend: junior       # your grade in frontend
    devops: senior         # your grade in devops
    ai_ml: principal       # your grade in AI/ML
    default: senior        # fallback for unfamiliar domains
  grade_multipliers:       # override defaults if needed
    junior: 2.0
    middle: 1.5
    senior: 1.0
    principal: 0.7
    ai: 0.4
  ai_task_multipliers:     # AI speed by task type
    pure_coding: 0.10      # AI does coding ~10x faster
    coding_infra: 0.25     # code + infrastructure
    design_coding: 0.30    # design + implementation
    pure_infra: 0.50       # pure infra (K8s, CI/CD)
    coordination: 1.00     # meetings -- AI doesn't help
  review_overhead: 0.30    # +30% to AI time for human review
  safety_margin: 0.50      # warn if sprint > 50%
```

After configuration, `--my-grade` will automatically use the correct grade:

```bash
forgeplan estimate PRD-022 --my-grade
# -> "Using grade: Middle (domain: backend, from config grade_profile)"
```

### Multi-grade Profile: Why

You can be a **Senior in DevOps** and a **Junior in Frontend** at the same time. A K8s task takes 5h (Senior), while an equally complex React task takes 10h (Junior). Forgeplan accounts for this via `grade_profile`.

### Workflow with Estimate

```bash
# 1. Created PRD with FR
forgeplan new prd "Auth System"
# -> filled in FR-001..FR-005

# 2. Estimated effort
forgeplan estimate PRD-022
# -> Senior: 52h (6.5 days), AI: 6.9h (0.9 days)

# 3. Created RFC, refined estimate
forgeplan estimate RFC-005
# -> 12 phase steps, confidence +25%

# 4. Plan sprint with 40-50% safety margin
# Senior capacity = 80h/sprint -> take tasks for 40h max
```

---

## Evidence and R_eff -- How to Confirm Decisions

### Why

Without evidence, R_eff = 0.0 for all artifacts. The health dashboard screams "At Risk". Decisions are made on words, not facts.

### How to Create an EvidencePack

```bash
forgeplan new evidence "Benchmark: LanceDB vs SQLite insert performance"
```

### IMPORTANT: Structured Fields

An EvidencePack **must** contain structured fields in the body:

```markdown
## Measurements

Tested inserting 1000 records:
- LanceDB: 5ms average
- SQLite + faiss: 12ms average

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: benchmark
```

| Field | Values | Description |
|-------|--------|-------------|
| `verdict` | `supports` / `weakens` / `refutes` | Confirms, weakens, or disproves |
| `congruence_level` | `0`-`3` | CL3 = same context (best). CL0 = opposed context (penalty 0.9) |
| `evidence_type` | `measurement` / `test` / `benchmark` / `audit` | Type of evidence |

**Without these fields**, the R_eff parser cannot find data and defaults to CL0 -> R_eff = 0.1 instead of 1.0.

### Link Evidence to an Artifact

```bash
forgeplan link EVID-001 ADR-002 --relation informs
forgeplan score ADR-002
# -> R_eff = 1.00 (was 0.00)
```

### Congruence Levels (CL)

| CL | Penalty | When |
|----|---------|------|
| CL3 | 0.0 | Evidence collected on the target system (benchmark on our code) |
| CL2 | 0.1 | Similar context (benchmark from another project on the same stack) |
| CL1 | 0.4 | Different context (article, documentation, someone else's experience) |
| CL0 | 0.9 | Opposed context (evidence from a different domain) |

### R_eff = min(evidence_scores)

R_eff = weakest link. If there are 3 evidence items and one is weak -- R_eff = the weak one. NOT average.

---

## Validation -- How to Check Quality

### Rules by Depth

| Depth | PRD rules | RFC rules | ADR rules |
|-------|-----------|-----------|-----------|
| Tactical | 3 base rules | 3 base | 3 base |
| Standard | 9 rules (+ audience, density, leakage) | 5 rules (+ options, phases) | 3 rules |
| Deep | 16 rules (+ timeline, stakeholders, risks, acceptance) | 6 rules (+ risks) | 5 rules (+ invariants, rollback) |

### Validator Aliases

The validator accepts synonyms:

| Expected | Also accepted |
|----------|--------------|
| `## Problem` | `## Motivation`, `## Problem Statement`, `## Background` |
| `## Goals` | `## Success Criteria`, `## Objectives` |
| `## Non-Goals` | `## Out of Scope`, `## Product Scope` |
| `## Related` | `## Related Artifacts`, `## Dependencies` |
| `## Target Users` | `## Target Audience`, `## Users`, `## Audience` |

### What Validation Checks

- **MUST** -- blocks activation. Required sections, frontmatter fields.
- **SHOULD** -- warning. Text density, absence of tech leakage in FR.
- **COULD** -- suggestion. FR format `[Actor] can [capability]`.

---

## Lifecycle -- From Draft to Active

```
Draft --review--> Draft (if MUST failures)
Draft --activate--> Active (if MUST passed)
Active --supersede--> Superseded (link to replacement)
Active --deprecate--> Deprecated (with reason)
```

### Typical Flow

```bash
# 1. Created artifact
forgeplan new prd "Payment Processing"

# 2. Filled body (Problem, Goals, Non-Goals, FR, Related, Target Users)

# 3. Checked
forgeplan review PRD-001
# -> MUST fix: Missing Problem section

# 4. Refined body
forgeplan update PRD-001 --body @/tmp/prd-001-body.md

# 5. Repeated review
forgeplan review PRD-001
# -> Review PASSED -- ready to activate

# 6. Activated
forgeplan activate PRD-001
# -> draft -> active
```

### build-on-draft Warning

If an RFC references a PRD that is still in Draft -- review will show a warning:
```
Warning: build-on-draft: depends on PRD-001 which is still Draft
```

This does not block activation but signals an immature dependency.

---

## Integration with AI Agents

### Option 1: Skill + MCP (recommended)

```bash
# Install skill for all supported agents
npx skills add ForgePlan/forgeplan --skill forge
```

Supports 40+ agents: Claude Code, Cursor, Codex, Gemini CLI, GitHub Copilot, Cline, Continue, Windsurf, and more.

After installation:
```
/forge "Add OAuth2 authentication"
```

### Option 2: MCP Server Directly

In your project's `.mcp.json` (Claude Code, Cursor):

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve"]
    }
  }
}
```

### Option 3: Rules Files (for agents without MCP)

| Agent | File | What to add |
|-------|------|------------|
| Claude Code | `CLAUDE.md` | "How to use Forgeplan CLI" section (see this project) |
| Cursor | `.cursorrules` | Same rules in Cursor format |
| Codex | `AGENTS.md` | Instructions for Codex |
| Gemini CLI | `.gemini/rules` | Rules for Gemini |

### Core Workflow (6 tools)

```
1. forgeplan_health     -> session start: what's happening in the project?
2. forgeplan_route      -> "what to create?" depth + pipeline
3. forgeplan_new        -> create artifact
4. forgeplan_validate   -> check quality
5. forgeplan_review     -> ready to activate?
6. forgeplan_activate   -> draft -> active
```

63 MCP tools total. 6 core tools cover 90% of the workflow.

---

## Forge Mode -- Permission Model for AI Agents

When working with AI agents (Claude Code, Cursor) in autonomous mode, use **Forge Mode** -- a permission model with 3 trust zones (FPF B.3 Trust Calculus):

| Zone | What | Mode | Examples |
|------|------|------|---------|
| **Green** | Read-only + build + test + forgeplan | Auto-allowed | `cargo test`, `forgeplan health`, `git status` |
| **Yellow** | Create/edit files, git add/commit | Auto-allowed (acceptEdits) | Write, Edit, `git add`, `git commit` |
| **Red** | Irreversible actions | **BLOCKED** | `git push --force`, `rm -rf /`, `cargo publish` |

### Configuration (Claude Code)

1. **Whitelist** in `settings.local.json` -- wildcard patterns:
```json
{
  "permissions": {
    "allow": [
      "Bash(cargo:*)", "Bash(forgeplan:*)", "Bash(git:*)",
      "Bash(ls:*)", "Bash(find:*)", "Bash(grep:*)",
      "mcp__hindsight__memory_recall", "mcp__hindsight__memory_retain"
    ]
  }
}
```

2. **Safety hook** in `.claude/hooks/forge-safety-hook.sh` -- PreToolUse blacklist:
```bash
# Blocked even in yolo mode:
# git push --force, git reset --hard, rm -rf /, cargo publish
```

3. **Claude Code mode**: `acceptEdits` (files auto, bash via whitelist)

### /forge-cycle -- Full FPF-aligned Dev Cycle

The `/forge-cycle PRD-XXX` command launches an 8-phase cycle:

```
Phase 0: OBSERVE    -> forgeplan health + stale + fpf (what's happening?)
Phase 1: ROUTE      -> forgeplan route (what depth?)
Phase 2: SPRINT     -> /sprint (wave plan)
Phase 3: BUILD      -> /team-up (implementation with Rust skills)
Phase 4: AUDIT      -> /audit (adversarial review, MUST find issues)
Phase 5: FIXES      -> /team-up (fix audit findings)
Phase 6: EVIDENCE   -> forgeplan new evidence + score + activate
Phase 7: COMMIT     -> git commit + PR + hindsight
Phase 8: NEXT       -> forgeplan health -> next feature
```

**FPF auto-resolve**: on conflicts/choices, the agent automatically applies the ADI cycle (Abduction -> Deduction -> Induction) + WLNK + Reversibility check. It asks the user only for irreversible decisions.

---

## Pitfalls (from real dogfood experience)

### 1. EvidencePack without structured fields -> R_eff = 0.1

The parser looks for `verdict:`, `congruence_level:`, `evidence_type:` in the body as plain text. Without them -- CL0 by default.

**Solution:** Always add a `## Structured Fields` section.

### 2. All artifacts stuck in Draft forever

If you never run `forgeplan review` -> `forgeplan activate`, all artifacts remain in Draft forever. The health dashboard will show "ALL DRAFT".

**Solution:** After filling an artifact -- immediately review + activate.

### 3. Validator requires sections missing from body

The body in LanceDB is stored WITHOUT frontmatter. The validator gets frontmatter from record fields (id, status, kind) and looks for sections in the body. If you only filled Summary + FR when creating via `forgeplan new` -- the validator will say "Missing Problem, Goals, Non-Goals".

**Solution:** Fill all MUST sections for your depth. Or use aliases (Motivation instead of Problem, Out of Scope instead of Non-Goals).

### 4. `forgeplan update --body` accepts @filepath

```bash
forgeplan update PRD-001 --body @/tmp/new-body.md
```

No need to copy content into the command line.

### 5. 10 artifact types, but you really only need 6

From dogfood experience: PRD, RFC, ADR, Note, Problem, Epic -- are actually used. EvidencePack, Spec, SolutionPortfolio, RefreshReport -- are for mature projects with many artifacts.

### 6. PRD stubs: "created an ID, forgot to fill it in"

**Anti-pattern:** `forgeplan new prd "Title"` -> immediately write code -> PRD stays a stub forever.

Result: `forgeplan validate` shows 5 MUST errors, PRD cannot be activated, no decision justification.

**Solution:** Shape -> Validate -> Code. After `forgeplan new` -- IMMEDIATELY fill MUST sections (Problem, Goals, Non-Goals, Target Users, Related). Run `forgeplan validate` and make sure it passes. Only then code.

### 7. Code is done, but no Evidence -> R_eff = 0.0

**Anti-pattern:** fully implemented PRD (200+ tests), but no EvidencePack created. Health screams "blind spot", R_eff = 0.0.

**Solution:** Code -> Evidence -> Activate. After implementation:
```bash
forgeplan new evidence "What was confirmed: tests, LOC, dogfood"
# Add structured fields to body
forgeplan link EVID-XXX PRD-XXX --relation informs
forgeplan score PRD-XXX   # -> R_eff > 0
forgeplan activate PRD-XXX
```

### 8. Active without code = false status

**Anti-pattern:** activated a PRD before implementation began. Health shows no problems, but the artifact is an empty promise.

**Solution:** Activate ONLY when code is written + evidence is created. If the PRD describes future work -- leave it in draft.

---

## References

| Document | Description |
|----------|-------------|
| `docs/guides/HOW-TO-USE.md` | 10 methodology rules with examples |
| `docs/guides/DEPTH-CALIBRATION.md` | Details on the 4 depth levels + escalation |
| `docs/guides/QUALITY-GATES.md` | Verification Gate + Adversarial Review |
| `docs/guides/ARTIFACT-MODEL.md` | Artifact hierarchy: Epic -> PRD -> Spec -> RFC -> ADR |
| `docs/guides/PRD-RFC-ADR-FLOW.md` | Decision tree: which document to create |
| `docs/guides/GLOSSARY.md` | 31 terms |
| `CLAUDE.md` | Instructions for AI agent + CLI quick reference |
