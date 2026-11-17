# Evidence-Driven PR Protocol

## Overview

This protocol combines **Forgeplan 0.38.0 core** (evidence scoring and activation gates) with optional **public marketplace `fpl-skills` 1.75.1** orchestration and hooks. Core does not intercept every pull request or install this PR hook merely because the CLI is present.

**Evidence** is an `EVID` (EvidencePack) artifact. Prefer `forgeplan link EVID-NNN <TARGET-ID> --relation informs`; the public PR hook recognises incoming EVID edges with `informs` or `based_on`. This is a link-presence check, not a review of the measurement's quality.

## Why Evidence Before PR Matters

Without evidence at PR time:
- Decisions are blind — R_eff = 0 (no proof score)
- Health reports show `blind_spots` — unsubstantiated decisions clutter the artifact graph
- Team cannot evaluate trade-offs or validate claims made in the PR description
- Silent failures accumulate (see PROB-035, PROB-039: most missed bugs trace back to "we tested the happy path, not the real scenario")

With evidence enforced:
- Each decision is anchored to proof
- Quality metrics (R_eff) are meaningful — blind spots are obvious
- Code review can evaluate evidence alongside implementation
- Artifact lifecycle stays coherent (from Shape → Validate → Code → Evidence → Activate)

## The 3-Layer Evidence Enforcement Stack

### Layer 1: Agent Skills (Wave 2 W5+W6)
The public `fpl-skills` plugin provides Markdown instructions in `skills/{audit,sprint,build}/SKILL.md`, executed by the installed agent harness:
- `/audit` drafts an EVID with findings and links known parent artifacts; ad-hoc audits may have no parent.
- `/sprint` drafts evidence at wave close and prompts for collected evidence activation at sprint close.
- `/build` drafts one EVID for a completed build and offers interactive activation; non-interactive use leaves it draft.

These are orchestration requirements, not background core services. They depend on the agent following the skill and on working MCP or CLI access. Skills must report failed publication rather than imply it succeeded. Do not assume automatic activation or a fixed `_next_action` string.

### Layer 2: Pre-PR Hook (FR-014, this document)
When the published `templates/hooks/pre-pr-evidence-check.sh` has been copied and wired into Claude Code **PreToolUse**, it inspects Bash payloads whose first command (after environment assignments) is `gh pr create`:
1. Reads numeric artifact IDs from the branch name and branch-local commits. It selects the nearest available base with a non-empty range; without one, falls back to the last 20 commits.
2. Skips EVID and NOTE. Missing artifact IDs are warned about, not blocked.
3. Looks for incoming EVID `informs` / `based_on` graph edges; a weaker body-text fallback checks the Related/Links/Relations section.
4. Returns 2 for referenced, existing artifacts without evidence; documented bypasses return 0.

This is a local, bypassable agent hook, not GitHub branch protection. It does not inspect every PR body, validate verdict/CL/freshness, or prevent a PR created outside that harness. Slug-only references are not matched by its numeric-ID regex.

### Layer 3: Health Verdict (post-hoc reporting)
`forgeplan health` reports active artifacts without evidence as blind spots and includes an aggregate verdict. It is a diagnostic, not a PR-creation gate.


## When to Create Evidence

### Must Have (Blocking at PR)
- **Features**: PRD/RFC → implement code → capture test results → create EVID → link → PR
- **Architectural decisions**: ADR → decision logic → audit findings → create EVID → link → PR
- **Problems/root-cause analysis**: PROB → investigation → audit/measurement → create EVID → PR
- **API/data model changes**: SPEC → design review → EVID for schema correctness → PR

### Should Have (Recommended, can bypass)
- **Bug fixes**: test that the bug is fixed → optional EVID (many org standards only require for P0)
- **Refactoring**: code review findings → optional EVID (if architectural impact is significant)
- **Documentation updates**: may refer to existing EVID from original feature, no new EVID needed

### Exempt (Auto-bypass, no evidence needed)
- **Documentation-only PRs** (`docs/*` branch)
- **Mechanical sync PRs** (`chore/sync-main-to-dev-*`, `chore/dependabot-*`)
- **Release branch PRs** (`release/v*`)
- **Hotfix branch PRs** (`hotfix/*`)

## Bypass Mechanism

### When You Need to Bypass

Legitimate cases:
- **Dependency bump without feature change**: `FORGEPLAN_SKIP_EVIDENCE=1 gh pr create`
- **Retroactive evidence**: You merged code, then need to attach EVID for the audit trail (see section below)
- **Emergency hotfix**: `FORGEPLAN_SKIP_EVIDENCE=1 gh pr create --title "[HOTFIX] Production outage" --body "...justification..."`

**⚠️ Important**: Bypass with intent. Always document in the PR body WHY evidence is being skipped. Examples:
```
FORGEPLAN_SKIP_EVIDENCE=1 gh pr create --title "[HOTFIX] Auth token expiry bug" \
  --body "Production outage fix. Evidence: existing EVID-087 covers token refresh logic. New EVID retroactively attached in follow-up PR #NNN."
```

### Bypass Methods

1. **Environment variable**:
   ```bash
   FORGEPLAN_SKIP_EVIDENCE=1 gh pr create --title "..." --body "..."
   ```

2. **Branch prefix** (auto-bypass, no env var needed):
   - `docs/` — documentation-only
   - `chore/sync-` — sync PRs
   - `chore/dependabot-` — dependency updates
   - `release/v` — release branches
   - `hotfix/` — hotfixes

3. **Via Git alias** (optional, for convenience):
   ```bash
   git config --global alias.pr-skip '!FORGEPLAN_SKIP_EVIDENCE=1 gh pr create'
   # Usage: git pr-skip --title "..." --body "..."
   ```

## Retroactive Evidence (How to Capture Work That's Already Merged)

If you merged code without creating EVID first, you can capture evidence retroactively:

### Step 1: Create the EVID artifact
```bash
forgeplan new evidence "Feature X: test coverage 92%, p95 latency 180ms"
```

### Step 2: Fill structured fields
Prepare the **complete body** in `evidence-body.md` (without YAML frontmatter), including actual measurement, result, interpretation and these standalone fields. Save it with `forgeplan update EVID-NNN --body @evidence-body.md` or `forgeplan_update`; do not directly edit the managed projection:
```markdown
## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement
```

(Field semantics: this section; scoring participation: §Evidence Lifecycle and Scoring below.)

Use observed results, not the illustrative numbers in the title. The parser reads the first standalone `key: value` occurrence, ignoring Markdown table rows and HTML comments; keep values unquoted and do not append inline comments.

| Field | Runtime values / effect |
|-------|-------------------------|
| `verdict` | `supports` (1.0), `weakens` (0.5), `refutes` (0.0) before penalties |
| `congruence_level` | Integer 0–3; penalties CL0=0.9, CL1=0.4, CL2=0.1, CL3=0 |
| `source_tier` | Optional `t1`/`t2`/`t3` maps to CL3/CL2/CL1; with explicit CL the lower value wins |
| `evidence_type` | `test`, `measurement`, `benchmark`, `audit`; recursive score modifiers 0, 0, 0.1, 0.2 |

Absent/unrecognised verdict or no usable CL source forces CL0. Unknown `evidence_type` (including marketplace `code_review`) currently falls back to `measurement`; it is not a fifth core type. Use the type that accurately describes the evidence. Typical undeclared supporting measurement scores 0.1, not 1.0; a refuting verdict or other penalties can lower it further.

`link --auto-activate-complete` is explicit opt-in. Its completeness predicate requires valid standalone verdict + explicit CL, not `source_tier` alone, and does not check result quality. Direct EVID activation is not a structured-field completeness guarantee: evidence is outside the five full-lifecycle validation kinds. Review fields and score even after successful activation.

### Step 3: Link to the artifact
```bash
forgeplan link EVID-NNN PRD-MMM --relation informs
```

### Step 4: Activate
```bash
forgeplan activate EVID-NNN
```

### Step 5: Create a follow-up PR to merge the EVID branch
```bash
git add .forgeplan/evidence/EVID-NNN-*.md
git commit -m "docs(evidence): retroactive EVID-NNN for PRD-MMM

Refs: PRD-MMM"
gh pr create --title "[Evidence] Add EVID-NNN for PRD-MMM" \
  --body "Retroactively captured evidence from merged feature. See EVID-NNN for test results."
```

## Git Provenance for Code-Claiming Evidence (PRD-082 / #360)

An EvidencePack that claims code changed may declare three extra standalone fields.
Use **all three or none**; a partial claim produces `Incomplete`, then the configured
gate decides whether to warn or block:

```markdown
base_sha: 92154e19
result_sha: 808db24
changed_paths: src/a.rs, tests/b.rs
```

The SHAs and paths above are illustrative. Replace them with commits and repository-relative paths from the actual change. The inline `# comment` syntax of YAML is not supported by this body-field parser.

`forgeplan activate` (CLI and `forgeplan_activate` over MCP) re-derives the claim against
the real git delta — `git diff --merge-base --name-only --no-renames -z base result` —
instead of trusting the executor's self-report. Verify the artifact, not the claim.

| Verdict | Meaning |
|---|---|
| `NotClaimed` | no provenance fields — every pack written before this feature; never a failure |
| `Verified` | the claimed paths appear in the real delta |
| `EmptyDelta` | the two SHAs produce **no** delta — green tests over nothing changed is a null result, not a pass |
| `PathMismatch` | a claimed path is absent from the delta |
| `Incomplete` | only some of the three fields are present |

Gate mode lives in `.forgeplan/config.yaml`:

```yaml
integrity:
  evidence_provenance_gate: warn   # block | warn | off  (default: warn)
```

- **`block`** — activation is refused; the artifact stays `draft`.
- **`warn`** *(default)* — activation proceeds and the discrepancy is surfaced (CLI on stderr,
  MCP inside the success payload).
- **`off`** — the gate is skipped entirely.

`forgeplan activate --force` bypasses the gate, the same escape hatch as the methodology
gates. A git error (unresolvable SHA — hallucinated, or real but absent from a shallow
clone) only ever **warns**, never blocks: the two cannot be told apart locally without a
network fetch (ADR-019). The gate establishes that the claimed change *exists* — it does
not run tests and says nothing about their quality.

## Evidence Lifecycle and Scoring (ADR-020)

Which packs feed `R_eff = min(evidence_scores)` depends on the pack's lifecycle status:

| Evidence status | Participates in min()? | Why |
|---|---|---|
| `draft` | **yes** | activation is not a prerequisite to scoring |
| `active` | **yes** | current testimony; an unexpired `refutes` pack zeroes the score |
| `stale` | **yes** | not terminal; expired `valid_until` separately makes the pack score 0.1 (even for a formerly refuting pack in 0.38.0) |
| `superseded` | **no** | displaced by a successor (`supersede <old> --by <new>`) — history stays in the graph, but it no longer speaks for present reliability |
| `deprecated` | **no** | retired (e.g. a duplicate deprecated with `--reason "superseded by EVID-y"`) |

Every exclusion is logged in `forgeplan score` factors (`Skipped EVID-x (status: superseded)`) and marked in the breakdown (`excluded from min`) — displacement is auditable, never silent. If ALL linked packs are terminal, the artifact degrades to **no active evidence** (R_eff 0.0): recovery requires the replacement pack to be *linked* to the artifact, not just to exist.

Replace obsolete evidence without deleting the old result: link the re-verification pack to the decision and inspect the old pack's saved status. `supersede` accepts an active old pack; a completed, valid draft must first pass `activate`. A saved stale pack instead needs a permitted retirement transition and a recorded replacement relation. Do not activate an unfinished pack or edit an adverse verdict merely to raise a score.

The recursive score also considers linked decision dependencies, so the evidence minimum alone need not be the final number. A leaf EVID without child evidence is scored from its own fields; do not invent evidence-of-evidence merely to obtain a score.

`forgeplan stale` reports expired dates without persisting `status: stale`. Use `renew` for reviewed stale records. In 0.38 its generic transition check also accepts draft without activation gates, and its date check validates format rather than future validity ([#547](https://github.com/ForgePlan/forgeplan/issues/547)). Do not use that defect to activate drafts. Expiry, saved lifecycle status and advisory phase are separate.

## Technical Details

### Hook Behavior

The hook `.claude/hooks/pre-pr-evidence-check.sh`:
- Requires Claude Code PreToolUse wiring and its JSON stdin payload
- Scans numeric IDs in the branch name and selected branch-local commit range (last 20 commits only as fallback)
- Queries `forgeplan get` and `forgeplan graph --json`; falls back to a scoped body-text heuristic
- Exit codes:
  - **0** = proceed (evidence found or bypassed)
  - **2** = evidence missing (blocks PR)
- Soft fallback: if `forgeplan` binary not on PATH, exits 0 (doesn't block)

### Artifact ID Detection

The hook looks for these patterns:
- Branch name: `feat/PRD-077-something` → detects `PRD-077`
- Commit message: `feat(prd): implement auth\n\nRefs: PRD-077, FR-001..003` → detects `PRD-077`
- Also handles: RFC, ADR, EPIC, SPEC, PROB, EVID, NOTE

### Evidence Relation Types

The hook checks for:
- **`informs`**: EVID provides supporting data for the artifact (common direction)
- **`based_on`**: artifact is grounded in EVID findings

Both relations satisfy the evidence requirement.

## Integration with CI/CD

### Layer 1 (Agent Skills)
- **Where**: `plugins/fpl-skills/skills/{audit,sprint,build}/SKILL.md` (public marketplace)
- **When**: Audit/build completion or sprint wave close, as specified by each skill
- **Action**: Draft EVID, link attributable parents, report publication and ask/defer activation as specified

### Layer 2 (Pre-PR Hook)
- **Where**: `.claude/hooks/pre-pr-evidence-check.sh` (this repo)
- **When**: Before `gh pr create` succeeds
- **Trigger**: Claude Code PreToolUse Bash hook with JSON payload
- **Action**: Block PR if evidence missing; provide bypass instructions

### Layer 3 (Health Reporting)
- **Where**: `forgeplan health` command, CI job (if wired)
- **When**: Post-merge or on-demand during development
- **Action**: Report blind spots for triage

## Health Report Integration

When you run `forgeplan health`, the report includes:

```
Artifacts:
  ...
  
Blind Spots (artifacts without evidence):
  - PRD-077 (3 days old, Standard depth)
  - RFC-009 (1 week old, Deep depth)
```

This helps the team identify decisions that need evidence capture, either retroactively or in future work.

## FAQ

**Q: What if I'm in a rush and just need to merge?**
A: Use `FORGEPLAN_SKIP_EVIDENCE=1 gh pr create`. But document in the PR body why you're skipping. Retroactively attach EVID in a follow-up PR if the bypass reason justifies it.

**Q: Does documentation-only PRs need evidence?**
A: The hook skips any `docs/*` branch by name; it does not prove the diff is documentation-only. This exemption is not a GitHub branch-protection rule.

**Q: What if the forgeplan binary is not installed?**
A: The hook soft-fails (exits 0) rather than blocking. You can still create PRs, but you lose the gate. This is intentional for fresh clones without binaries built.

**Q: Can I wire the hook to Git instead of Claude Code?**
A: Not by copying it unchanged. The script expects Claude Code JSON on stdin and a `gh pr create` command. Git `pre-push` uses a different input protocol and does not intercept PR creation.

**Q: What if my artifact is really ad-hoc and doesn't need evidence?**
A: Capture a Note if that matches the work, or document a justified bypass. A Note is not automatically given an expiry by `forgeplan new`; do not fabricate an EVID measurement to satisfy a link check.

## Reference

- **ADR-003**: Markdown as source of truth for artifacts
- **PRD-077**: Wave 2 evidence autopublish and enforcement
- **PROB-035, PROB-039**: Silent failures from happy-path-only testing
- **Hooks**: `.claude/hooks/pre-pr-evidence-check.sh`
- **Health**: `forgeplan health` command
- **Schema**: §Structured Fields above (EvidencePack structure; a dedicated `docs/schemas/EVIDENCE.md` does not exist yet)

