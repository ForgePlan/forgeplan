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
# Homebrew 6.0+: if refused with "untrusted tap", run `brew trust forgeplan/tap` once, then re-run.

# From a checked-out source tree (not a crates.io install recipe)
git clone https://github.com/ForgePlan/forgeplan.git && cd forgeplan
cargo install --path crates/forgeplan-cli --features semantic-search

# Or download a binary from GitHub Releases
# https://github.com/ForgePlan/forgeplan/releases
```

Release binaries include semantic search; ordinary Cargo builds leave the
feature off by default. Weights are separate: `forgeplan setup` prepares the
BGE-M3 cache. Offline inference requires a complete snapshot. Cache precedence
is `FORGEPLAN_MODEL_CACHE` → `~/.forgeplan/models` → platform/legacy fallback when
home is unavailable; `HF_HOME` does not override tract's cache.
For a genuinely new workspace, `init -y --with-model` explicitly requests a
download. An already initialized workspace can return before that step.

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

Use a new directory outside any existing workspace. Discovery walks up parents:
`init -y` may be a no-op against a parent's `.forgeplan/`. Check the reported path
before writing. For clones with tracked artifacts, see the [documentation index](../README.md)
for local config and reindex handling.

```bash
# 1. Initialize workspace
forgeplan init

# 2. Determine what to do
forgeplan route "Add OAuth2 authentication" --level 0
# Inspect the recommendation; it is not an execution or acceptance result.

# 3. Create first artifact
forgeplan new prd "OAuth2 Authentication"

# 4. Check project state
forgeplan health
```

> **Alias**: `fpl` = `forgeplan`. Source installations can create it with `forgeplan setup --skip-model`.

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
forgeplan route "task description" --level 0
```

`--level 0` forces keyword routing without an LLM. Plain `route` may attempt configured LLM routing and further analysis; do not use omission of `--explain` as an offline guarantee.

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

If MUST findings remain, refine the artifact. Zero MUST findings is necessary
for ordinary gated activation but not sufficient: activation also checks length,
evidence and stubs. Review is not a status transition.

### The Main Rule

**Pipeline = guideline, NOT bureaucracy.** Don't create all 10 types for every task. Tactical depth = just do it. Standard = PRD + RFC. Only Deep+ requires the full pipeline.

---

## All Commands (by category)

### Workspace layout and interface availability

The command examples below do not establish universal CLI/MCP parity.
Constellation notes target the scoped `0.39.0+constellation.s1.1` build,
not published `0.38.0`.

| Operation | Compatible legacy layout | Migrated Constellation layout |
|---|---|---|
| CLI `validate` | Exercised successfully | Exercised successfully |
| Full MCP `forgeplan_validate` | Validation interface | Exercised successfully on migrated artifacts |
| Other registered full tools | Check their contract | Do not infer support from registration |

See the [user guide](https://forgeplan.dev/docs/guides/constellation/),
[interface reference](https://forgeplan.dev/docs/reference/constellation/) and
[portable pilot (RU)](../operations/CONSTELLATION-PILOT.ru.md).

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
| `forgeplan estimate <id> --grade middle` | Highlight specific grade | `forgeplan estimate PRD-022 --grade junior` |
| `forgeplan estimate <id> --my-grade` | Grade from config grade_profile | `forgeplan estimate PRD-022 --my-grade` |

### Lifecycle

| Command | What it does | Example |
|---------|-------------|---------|
| `forgeplan review <id>` | Checklist: ready to activate? | `forgeplan review PRD-001` |
| `forgeplan activate <id>` | Draft -> Active (validation gate) | `forgeplan activate PRD-001` |
| `forgeplan supersede <id> --by <new>` | Active -> Superseded + chain warnings | `forgeplan supersede PRD-001 --by PRD-002` |
| `forgeplan deprecate <id> --reason "..."` | Active -> Deprecated | `forgeplan deprecate PRD-001 --reason "Cancelled"` |

**Rule:** PRD, RFC, ADR, Epic and Spec have validation gates. Notes and Problems
do not use the same schema gate; additional lifecycle checks still depend on
kind and depth. Do not equate successful `validate` with activation.

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
| `forgeplan route "<description>" --level 0` | Keyword-only depth + pipeline, no LLM | `forgeplan route "Add OAuth2" --level 0` |
| `forgeplan route "<desc>" --explain` | + LLM explanation | `forgeplan route "Add OAuth2" --explain` |
| `forgeplan calibrate [id]` | Suggest depth for existing artifact | `forgeplan calibrate PRD-001` |

### AI-powered (require LLM config)

| Command | What it does | Example |
|---------|-------------|---------|
| `forgeplan generate <kind> "<desc>"` | AI artifact generation | `forgeplan generate prd "Payment system"` |
| `forgeplan reason <id>` | ADI reasoning cycle | `forgeplan reason PRD-001 --json` |
| `forgeplan decompose <id>` | PRD -> RFC tasks via AI | `forgeplan decompose PRD-001` |
| `forgeplan capture "<decision>"` | Record a decision as Note/ADR | `forgeplan capture "Use Redis for cache"` |

### Local semantic search (no LLM provider required)

`forgeplan search "auth" --semantic` needs a `semantic-search` build and a
complete compatible cached model, not `llm` config. On migrated layout it also
needs a published embedding profile and eligible vectors; forced semantic errors
are not a promise of keyword fallback. Run `index`, then `embed` explicitly.
Automatic drain additionally needs active `config.embedding` (`model: bge-m3`)
and live maintenance; manual `embed` does not save this setting.

### MCP Server

```bash
forgeplan serve  # start MCP server (stdio transport)
```

The scoped Constellation build registers 76 full-profile tools and a separate
9-tool `constellation-read` profile. `full` is the default and retains writing.
Startup maintenance selection and per-request workspace selection differ.
`--workspace` supplies the server default, but an explicit `FORGEPLAN_WORKSPACE`
binding takes precedence for startup maintenance. Request resolution prefers
explicit workspace, then environment, server default and cwd. Read-profile
handles do not launch maintenance or accept arbitrary paths.

---

## Estimate Engine -- Effort Estimation

### Why

Turns documentation (FR in PRD, Phases in RFC) into effort estimates. No separate Excel spreadsheet needed -- the estimate lives alongside artifacts.

### Basic Command

```bash
forgeplan estimate PRD-022
```

Illustrative estimate output (heuristic hours, not measured task durations):

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

**Confidence** is a completeness heuristic, not measured forecast accuracy. It starts at 10%, then adds:
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

Author evidence with structured fields and real observations. The following numbers are **illustrative**, not a performed benchmark; do not copy them as evidence:

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

Missing/unknown verdict or missing CL declaration forces CL0; a plain undeclared pack scores about 0.1. `evidence_type` separately defaults to measurement. Actual R_eff also depends on expiry, other evidence and dependencies; the next single-benchmark example is not guaranteed to score 1.00 in the recursive report (benchmark modifier 0.1).

### Link Evidence to an Artifact

```bash
forgeplan link EVID-001 ADR-002 --relation informs
forgeplan score ADR-002
# Inspect the actual report: its score, weakest link, expiry and dependency factors.
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

---

## Validation -- How to Check Quality

### Rules by Depth

| Depth | PRD rules | RFC rules | ADR rules |
|-------|-----------|-----------|-----------|
| Tactical | Core PRD sections plus shared checks | Summary, motivation, direction | Context, decision, consequences |
| Standard | + audience; density/leakage SHOULD | + alternatives and phases | Core sections |
| Deep | + timeline, stakeholders, risks, acceptance, metrics | + risks | + invariants and rollback |

This is a section summary, not a frozen rule count. Shared rules and additional
measurability/NFR/traceability checks are registered in `validation/rules.rs`.

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

- **MUST** — required implemented schema checks; ordinarily block gated activation.
- **SHOULD** — warnings; density severity depends on depth (50 words: Standard SHOULD, Deep MUST).
- **COULD** — advice, including `adversarial-too-clean` in optional adversarial checks.

Authoring recommendations are broader than executable checks: section presence
does not prove measurability, causal reasoning or consistency. Metadata checks
for `id`/`status` presence do not alone verify all lifecycle values.

---

## Lifecycle -- From Draft to Active

```text
draft --activate--> active
active --> superseded | deprecated | stale
stale --renew--> active
stale --reopen--> deprecated + new draft
stale --deprecate--> deprecated
```

`superseded` and `deprecated` are terminal. `review` leaves status unchanged.
Approved, Implemented and Closed are process descriptions, not persisted status
values. Session phases and artifact phase markers are separate. Activation
includes length/evidence/stub gates in addition to validation.

### Typical Flow

Illustrative legacy-layout sequence, not an observed transcript. Before the
last step, complete all depth-specific content and real evidence required by
activation. On migrated layout use the supported interface listed above.

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
# Inspect findings and activation checklist; no fabricated PASS output.

# 6. Activated
forgeplan activate PRD-001
# Succeeds only if all applicable activation gates pass.
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

The selected build inventories 76 full-profile tools and 9 separate read-profile tools. These six are a teaching selection, not measured 90% workflow coverage or acceptance of every registered tool.

---

## Forge Mode -- Permission Model for AI Agents

**Forge Mode** is a local permission convention, not the FPF B.3 upstream trust algorithm. The configuration below is illustrative; actual permissions and hook loading must be checked in the host. It neither installs controls nor overrides assignment-specific restrictions.

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

The native `/forge-cycle PRD-XXX` prompt describes nine numbered phases (0–8):

```
Phase 0: OBSERVE    -> forgeplan health + stale + fpf dashboard (what's happening?)
Phase 1: ROUTE      -> forgeplan route (what depth?)
Phase 2: SPRINT     -> /sprint (wave plan)
Phase 3: BUILD      -> /team-up (implementation with Rust skills)
Phase 4: AUDIT      -> /audit (adversarial review, MUST find issues)
Phase 5: FIXES      -> /team-up (fix audit findings)
Phase 6: EVIDENCE   -> forgeplan new evidence + score + activate
Phase 7: COMMIT     -> git commit + PR + hindsight
Phase 8: NEXT       -> forgeplan health -> next feature
```

The native AUDIT issue quota remains in that prompt; it is not evidence that
defects exist, and this documentation does not amend that policy or satisfy G3.

**Local auto-resolve guidance** uses ADI + weakest-link + reversibility. Native Build stop conditions also include equally weak options and cross-cutting/other-PRD impact, not only irreversible choices. Three alternatives are a local prompt heuristic, not an upstream FPF universal threshold.

---

## Pitfalls (from real dogfood experience)

### 1. EvidencePack without structured fields -> no usable evidence signal

The parser looks for `verdict:`, `congruence_level:`, `evidence_type:` in the body as plain text. Missing fields make the pack's congruence level uncertain; the exact R_eff returned depends on whether it is scored as a leaf, through another artifact, or with no eligible evidence at all.

**Solution:** Always add a `## Structured Fields` section.

### 2. All artifacts stuck in Draft forever

If you never run `forgeplan review` -> `forgeplan activate`, all artifacts remain in Draft forever. The health dashboard will show "ALL DRAFT".

**Solution:** Review the completed artifact, resolve applicable gates and activate only when it is ready.

### 3. Validator requires sections missing from body

Body contracts depend on the interface. The current captured `ArtifactRecord`
used by ordinary reads retains the full Markdown file including frontmatter;
CLI `get --json` exposes that full `body`. Full MCP pinned
`forgeplan_constellation_get` returns prose in nested `artifact.body`.
Do not treat those payloads as interchangeable. The validator still needs the
required sections: a template filled only with Summary + FR is incomplete.

**Solution:** Fill all MUST sections for your depth. Or use aliases (Motivation instead of Problem, Out of Scope instead of Non-Goals).

### 4. `forgeplan update --body` accepts @filepath

```bash
forgeplan update PRD-001 --body @/tmp/new-body.md
```

No need to copy content into the command line.

### 5. 10 artifact types, but you really only need 6

The ten types here are an introductory selection, not the complete core taxonomy. Evidence is needed whenever the applicable gate requires it, not only in mature projects. The Rust core additionally supports Memory and six brownfield extraction kinds; individual CLI/MCP schemas expose their own subsets.

### 6. PRD stubs: "created an ID, forgot to fill it in"

**Anti-pattern:** `forgeplan new prd "Title"` -> immediately write code -> PRD stays a stub forever.

Result: validation can report missing sections/placeholders; the exact findings depend on depth and content. A stub is not activation-ready.

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
| [HOW-TO-USE.md](HOW-TO-USE.md) | Methodology rules with examples |
| [DEPTH-CALIBRATION.md](DEPTH-CALIBRATION.md) | Depth levels and escalation |
| [QUALITY-GATES.md](QUALITY-GATES.md) | Verification and adversarial review |
| [ARTIFACT-MODEL.md](ARTIFACT-MODEL.md) | Artifact hierarchy |
| [PRD-RFC-ADR-FLOW.md](PRD-RFC-ADR-FLOW.md) | Which document to create |
| [GLOSSARY.md](GLOSSARY.md) | Terms and upstream/product distinctions |
| `CLAUDE.md` | Instructions for AI agent + CLI quick reference |
