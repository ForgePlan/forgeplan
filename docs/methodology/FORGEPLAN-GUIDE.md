[English](FORGEPLAN-GUIDE.md) · [Русский](FORGEPLAN-GUIDE.ru.md)

# Forgeplan — Complete Practical Guide

> One document: methodology + commands + examples + pitfalls.
> For humans and AI agents alike.

---

## What Is Forgeplan

Forgeplan supports **thinking before coding**: determine depth, shape and validate the decision, implement it, collect evidence, then review and activate. CLI/MCP automate artifact operations; agents and people perform the engineering work.

**Not Jira.** Not project management. Not a task tracker. Forgeplan is a **structured knowledge base** for engineering decisions.

AI agents can use the MCP server; people and agents can also use the CLI. Neither interface independently verifies implementation correctness.

---

## Installation

### 1. AI Skill (for any AI agent)

```bash
# Install the bundled /forge instructions for Claude Code (requires the binary below)
forgeplan setup-skill
```

This writes `~/.claude/skills/forge/SKILL.md`; it does not install marketplace agents, `/forge-cycle`, MCP configuration or safety hooks. For the full workflow, install the published `forgeplan-workflow` plugin and the specialist packs your project uses.
```
/forge "Add OAuth2 authentication"
```

### 2. CLI Binary

```bash
# macOS (Homebrew)
brew install forgeplan/tap/forgeplan
# Homebrew 6.0+: if refused with "untrusted tap", run `brew trust forgeplan/tap` once, then re-run.

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
forgeplan route "Add OAuth2 authentication" --level 0
# -> Depth: Deep, Pipeline: PRD -> Spec -> RFC -> ADR

# 3. Create first artifact
forgeplan new prd "OAuth2 Authentication"

# 4. Check project state
forgeplan health
```

> **Alias**: release installers provide `fpl`. After a source install, `forgeplan setup --skip-model --skip-hooks` creates it beside the executable without downloading the model or installing repository hooks.

---

## Methodology: 3 Questions Instead of Bureaucracy

### Question 1: "What depth?"

Ask yourself one question: **"Is this reversible within a day?"**

| Answer | Depth | What to create | Example |
|--------|-------|---------------|---------|
| Yes, trivial | **Tactical** | Nothing or Note | Fix typo, update config |
| No, there are choices | **Standard** | PRD -> RFC | New feature, 1-3 days |
| No, affects many | **Deep** | PRD -> Spec -> RFC -> ADR | New module, 1-2 weeks |
| Strategy, cross-team | **Critical (marketplace policy)** | Epic + coordinated Deep workstreams | New subsystem |

Or use automatic routing:

```bash
forgeplan route "task description"
```

`--level 0` forces offline keyword routing. Without it, CLI 0.38 tries configured LLM routing and falls back to keywords; auto mode can escalate a Deep LLM result to ADI reasoning. MCP `forgeplan_route` has **no `level` parameter** in 0.38. The CLI help's “keywords (default)” wording is narrower than the implementation.

The **0.38 core pipeline** is Tactical = none, Standard = PRD → RFC, Deep = PRD → Spec → RFC → ADR. Core parses `critical` as Deep; it does not produce a distinct Critical route. The **published marketplace methodology** adds SOL (compare approaches) before design and REFRESH (record reconciliation) at Deep/Critical. Keep these workflow requirements separate from the binary's pipeline output. See [versioned sources](#versioned-sources).

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

Fix MUST findings and all additional activation-gate failures. For PRD/Epic/Spec/RFC/ADR, review also checks body length, unfilled-template markers and an evidence link. A machine PASS is not an independent review or proof that code works.

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
| `forgeplan update <id>` | Update metadata/body | `forgeplan update PRD-001 --body @prd.md` (lifecycle statuses go through `activate` / `supersede` / `deprecate`) |
| `forgeplan delete <id>` | Delete artifact | `forgeplan delete PRD-001 --yes` |
| `forgeplan list` | List artifacts | `forgeplan list --type prd --status active` |

**Artifact kinds:** `prd`, `epic`, `spec`, `rfc`, `adr`, `note`, `problem`, `solution`, `evidence`, `refresh`

These are the common decision-workflow kinds; core 0.38 also supports local `memory` and six brownfield kinds (`use_case`, `glossary`, `invariant`, `scenario`, `hypothesis`, `domain_model`). Canonical Markdown lives under `.forgeplan/`; LanceDB is the rebuildable query index. Prefer CLI/MCP writes; `reindex` repairs the index from Markdown, while `scan-import` imports other documents.

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

**Rule:** full activation gates apply to PRD, RFC, ADR, Epic and Spec. Lightweight kinds such as Note and Problem skip those gates; their base validation is not a quality verdict.

### Dashboards and Analytics

| Command | What it does | Example |
|---------|-------------|---------|
| `forgeplan health` | Full project health | `forgeplan health --compact` |
| `forgeplan status` | Brief dashboard | `forgeplan status` |
| `forgeplan blindspots` | Artifacts without evidence, orphans | `forgeplan blindspots` |
| `forgeplan journal` | Decision timeline with R_eff | `forgeplan journal --risk` |
| `forgeplan fpf dashboard` | FPF dashboard: contexts + F-G-R + actions | `forgeplan fpf dashboard` |
| `forgeplan stale` | Artifacts with expired valid_until | `forgeplan stale` |
| `forgeplan decay` | Impact of expired evidence on R_eff | `forgeplan decay` |
| `forgeplan progress [id]` | Checkbox progress bars | `forgeplan progress` |

### Routing and Calibration

| Command | What it does | Example |
|---------|-------------|---------|
| `forgeplan route "<description>" --level 0` | Offline keyword depth + core pipeline | `forgeplan route "Add OAuth2" --level 0` |
| `forgeplan route "<desc>" --level 1` | Try LLM classification; fall back if unavailable | `forgeplan route "Add OAuth2" --level 1` |
| `forgeplan calibrate [id]` | Suggest depth for existing artifact | `forgeplan calibrate PRD-001` |

### AI generation and local semantic search

| Command | What it does | Example |
|---------|-------------|---------|
| `forgeplan generate <kind> "<desc>"` | AI artifact generation | `forgeplan generate prd "Payment system"` |
| `forgeplan reason <id>` | ADI reasoning cycle | `forgeplan reason PRD-001 --json` |
| `forgeplan decompose <id>` | PRD -> RFC tasks via AI | `forgeplan decompose PRD-001` |
| `forgeplan capture "<decision>"` | Record a decision as Note/ADR | `forgeplan capture "Use Redis for cache"` |
| `forgeplan search <query> --semantic` | Local embedding search, not an LLM call; needs the model/index | `forgeplan search "auth" --semantic` |

### MCP Server

```bash
forgeplan serve  # start MCP server (stdio transport)
```

The 0.38 server exposes 73 tools. CLI and MCP overlap but are not identical; use each interface's reference for parameters and availability.

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

Record the verdict, context match and evidence type explicitly. This example is illustrative, not a published benchmark:

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

In 0.38, missing/unrecognized `verdict`, or missing both `congruence_level` and valid `source_tier`, forces CL0. For an unexpired supports pack, the recursive score is then at most 0.1: test/measurement = 0.1, benchmark/audit = 0 after their type penalty. Weakens/refutes can also score 0. Missing `evidence_type` alone defaults to `measurement`, not CL0. Keep all three fields explicit, without inline `#` comments.

### Link Evidence to an Artifact

```bash
forgeplan link EVID-001 ADR-002 --relation informs
forgeplan score ADR-002
# With only the fresh supports/CL3/benchmark pack above and no weaker dependency: R_eff = 0.90
```

### Congruence Levels (CL)

| CL | Penalty | When |
|----|---------|------|
| CL3 | 0.0 | Evidence collected on the target system (benchmark on our code) |
| CL2 | 0.1 | Similar context (benchmark from another project on the same stack) |
| CL1 | 0.4 | Different context (article, documentation, someone else's experience) |
| CL0 | 0.9 | Opposed context (evidence from a different domain) |

### R_eff = min(evidence_scores)

R_eff = weakest link. If there are 3 evidence items and one is weak -- R_eff = the weak one. NOT average. Only *current* evidence participates: superseded/deprecated packs are excluded from the min (ADR-020) — displace a stale weak pack with `supersede <old> --by <new>` instead of editing its verdict.

The user-facing `forgeplan score` uses the **recursive** engine. A fresh pack contributes `max(0, verdict_score − CL_penalty − type_penalty)`: test/measurement subtract 0, benchmark 0.1, audit 0.2. Expired eligible packs contribute 0.1 before those deductions. The artifact's own evidence minimum is then compared with recursive dependency scores; `based_on`/`informs` dependencies subtract another 0.1, while `refines` subtracts 0. Terminal evidence and draft/terminal dependencies are excluded; Note/Memory dependencies are skipped. Thus the benchmark above gives `1 − 0 − 0.1 = 0.90`, not 1.00.

The flat compatibility helper `r_eff()` omits the type penalty and dependency traversal; do not use its formula as the CLI result. Sources: [recursive type scoring](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/scoring/reff.rs#L232-L254), [dependency minimum](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/scoring/reff.rs#L424-L499) and [CLI calculation](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/commands/score.rs#L130-L142).

---

## Validation -- How to Check Quality

### Rules by Depth

| Kind / depth | Required body content in 0.38 |
|-------------|------------------------------|
| PRD, all depths | Problem, Goals, Non-Goals, Functional Requirements, Related Artifacts |
| PRD, Standard+ | Also Target Users; problem density and implementation leakage are SHOULD at Standard, MUST at Deep |
| PRD, Deep | Also Timeline, Stakeholders, Acceptance Criteria, Risk Assessment, Success Metrics; domain-specific checks can add requirements |
| RFC | Summary, Motivation, Proposed Direction; Risks at Deep. Options and Implementation Phases are SHOULD |
| ADR | Context, Decision, Consequences; Invariants and Rollback at Deep (SHOULD otherwise) |
| Epic | Vision, Outcomes, Children, Phases, Progress |
| Spec | Summary, Related Artifacts, and a structural or behavioral contract |

All kinds also receive base rules. In 0.38, NFR presence/measurability and `### Requirement` without a nested `#### Scenario` are SHOULD findings. This is a minimum, not a substitute for substantive review.

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
Active --supersede--> Superseded (link to replacement; terminal)
Active --deprecate--> Deprecated (with reason; terminal)
Active --stale--> Stale --renew--> Active
Stale --reopen--> Deprecated (old) + new Draft
```

`valid_until` expiry is detected by `stale` and scoring; do not assume a background job rewrites status. Use lifecycle verbs, not `update --status active|superseded|deprecated`. Advisory `phase` and session state are separate from status.

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

# 6. After real verification, linked evidence and required independent review
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
# Install only the bundled Claude Code /forge skill
forgeplan setup-skill
```

Other runtimes need their own instruction/MCP setup. Marketplace commands are prompt contracts executed by the orchestrator, not extra CLI subcommands.

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

These six tools orient and manage lifecycle. Real work also needs body updates, links, evidence and independent verification; tool count does not measure workflow coverage.

---

## Forge Mode -- Permission Model for AI Agents

Forge Mode is a project/host permission policy, not a safety boundary installed by the Forgeplan binary. Classify operations by their effect, not by a blanket `forgeplan:*` or `git:*` allowlist.

| Operation | Responsibility |
|-----------|----------------|
| Read artifacts / inspect status | Agent follows the host's read permissions |
| Edit, commit, run checks | Project policy determines authorization and commands |
| Delete, override gates, publish or push | Obtain the authorization required by the host/project |

`setup-skill` installs instructions only. `setup` can install index-sync git hooks; those are not a general command safety hook. Do not treat `acceptEdits`, a blacklist or an autonomous prompt as permission to bypass independent review or user consent.

### /forge-cycle -- Full FPF-aligned Dev Cycle

`/forge-cycle` belongs to the published `forgeplan-workflow` plugin, not CLI 0.38. Its orchestrator executes the operations represented by `/sprint`, `/team-up` and `/audit`; the template does not auto-invoke nested slash commands.

The published contract includes:

1. Read health/session state and identify the task; optionally read configured memory.
2. Shape a raw idea as a Brief **Note**, then route and create/fill actual artifacts.
3. Standard+: ADI evidence, recorded depth and pre-build `/gate-check`; Deep+ also decomposition and effort estimation.
4. Implement with claims/file ownership, run project checks, obtain independent reviewer evidence.
5. Link implementation evidence; Deep+ post-build `/gate-check`.
6. Standard+: guardian checks the full evidence chain; the orchestrator reviews and activates, not the author or guardian.
7. Commit/PR only within granted permissions; record shipping after merge. Deep+ `/wrap` records actual reconciliation in REFRESH.

`/gate-check` returns a threshold verdict; it does not read code or replace guardian. User decisions are also needed for unresolved requirements, depth changes, overrides and permissions—not only irreversible choices. See [HOW-TO-USE](HOW-TO-USE.md) for responsibilities and concurrency.

---

## Pitfalls (from real dogfood experience)

### 1. EvidencePack without structured fields -> R_eff = 0.1

The parser reads standalone structured fields, not table rows or commented examples. Missing verdict or context declaration fails closed; missing type alone defaults to measurement.

**Solution:** Always add a `## Structured Fields` section.

### 2. All artifacts stuck in Draft forever

If you never run `forgeplan review` -> `forgeplan activate`, all artifacts remain in Draft forever. The health dashboard will show "ALL DRAFT".

**Solution:** Review after filling the artifact; activate only after its evidence, implementation and applicable review gates are ready.

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

That dogfood observation is historical, not current guidance to omit evidence. Evidence supports ordinary Standard decisions too; the published Deep/Critical methodology adds SOL and REFRESH.

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

**Solution:** In an implementation cycle, activate only after the promised implementation and evidence are ready. Runtime checks structure/links, not whether the code was actually exercised; an independently reviewed decision-only scope must be labeled as such.

---

## References

| Document | Description |
|----------|-------------|
| [HOW-TO-USE](HOW-TO-USE.md) | Task-oriented methodology instructions |
| [DEPTH-CALIBRATION](DEPTH-CALIBRATION.md) | Depth and escalation |
| [QUALITY-GATES](QUALITY-GATES.md) | Machine gates versus independent review |
| [ARTIFACT-MODEL](ARTIFACT-MODEL.md) | Artifact hierarchy |
| [PRD-RFC-ADR-FLOW](PRD-RFC-ADR-FLOW.md) | Which document to create |
| [GLOSSARY](GLOSSARY.md) | Terminology |

## Versioned sources

Checked against **Forgeplan 0.38.0** (`d56466b10ddf788c62a6e8acffd0e0f578d0c1d1`) and public marketplace **1.187.1** (`b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7`), not development drafts:

- [CLI routing](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/commands/route.rs#L15-L38) and [core pipeline](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/routing/pipeline.rs#L7-L19).
- [Validation rules](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/validation/rules.rs#L353-L550), [activation gates](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/lifecycle/mod.rs#L139-L220), [evidence parsing](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/scoring/evidence.rs#L57-L204).
- [Published methodology](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-workflow/skills/forgeplan-methodology/SKILL.md#L71-L103), [cycle contract](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-workflow/commands/forge-cycle.md#L549-L573) and [guardian handoff](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-workflow/commands/forge-cycle.md#L1096-L1168).
