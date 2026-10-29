# Evidence-Driven PR Protocol

## Overview

The project's evidence-before-PR convention asks for linked **evidence** for decisions. The repository's Claude Code hook enforces a limited check when wired into that host; it is not a universal GitHub branch-protection rule and does not verify the quality or truth of the evidence.

**Evidence** is represented as an `EVID` (EvidencePack) artifact with a typed `informs` or `based_on` link to the artifact being evidenced.

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
Wave 2 described marketplace `fpl-skills` v1.5.0+ `/audit`, `/sprint`, `/build` evidence autopublishing. The intended integration is:
- Capture test output, audit findings, benchmark results
- Auto-create `EVID` artifact with structured fields
- Auto-link to the affected PRD/RFC/ADR with `informs` relation
- Emit `_next_action: forgeplan activate <ID>` hint

This is an agent integration contract, not a guarantee for every installed host or skill version; inspect its actual result and linked artifact.

### Layer 2: Pre-PR Hook (FR-014, this document)
When wired into Claude Code PreToolUse, `.claude/hooks/pre-pr-evidence-check.sh` inspects a proposed `gh pr create` command:
1. Parses branch name and last 20 commits for artifact IDs (PRD-NNN, RFC-NNN, etc.)
2. For each detected artifact except EVID and NOTE, queries graph evidence edges, with a heuristic Related/Links/Relations body fallback
3. **Blocks PR creation** (exit code 2) if evidence is missing
4. Provides clear bypass instructions for legitimate exceptions

This gate is host-local. It does not inspect the PR body, admits documented branch/environment bypasses and exits 0 when no usable binary is found. A linked record alone does not prove its claims.

### Layer 3: Health Verdict (post-hoc reporting)
`forgeplan health` detects `blind_spots` — active artifacts without any linked evidence. This is reported in:
- CLI output: `Health: unhealthy (N blind spots found)`
- JSON: `"blind_spots": ["PRD-NNN", "RFC-MMM"]`

This layer does not block work but surfaces gaps for triage and cleanup.

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
forgeplan new evidence "Feature X: measured verification results"
```

### Step 2: Fill structured fields
Prepare the complete prose body in a temporary file, including actual observations and these fields, then apply it with `forgeplan update EVID-NNN --body @/tmp/evid-body.md`. Do not edit canonical `.forgeplan/` artifacts directly:
```markdown
## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement
```

(Field semantics: this section; scoring participation: §Evidence Lifecycle and Scoring below.)

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

An EvidencePack that claims code changed may declare three extra fields in the same
`## Structured Fields` block. **All three or none** — a partial claim produces an
`Incomplete` verdict; whether activation blocks depends on the configured gate:

```markdown
base_sha: 92154e19                        # state BEFORE the change
result_sha: 808db24                       # state AFTER
changed_paths: src/a.rs, tests/b.rs       # comma-separated
```

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
| `draft` | **yes** | a fresh measurement awaiting activation — the score gate runs before activation in the standard flow |
| `active` | **yes** | current testimony; an unexpired `refutes` pack contributes zero |
| `stale` | **yes** | not terminal — flagged for re-evaluation but not displaced; an expired `valid_until` separately decays the pack's score to 0.1 |
| `superseded` | **no** | displaced by a successor (`supersede <old> --by <new>`) — history stays in the graph, but it no longer speaks for present reliability |
| `deprecated` | **no** | retired (e.g. a duplicate deprecated with `--reason "superseded by EVID-y"`) |

Every exclusion is logged in `forgeplan score` factors (`Skipped EVID-x (status: superseded)`) and marked in the breakdown (`excluded from min`) — displacement is auditable, never silent. If ALL linked packs are terminal, the artifact degrades to **no active evidence** (R_eff 0.0): recovery requires the replacement pack to be *linked* to the artifact, not just to exist.

The explicit recursive score additionally includes eligible dependency contributions
and evidence-type modifiers (test/measurement 0, benchmark 0.1, audit 0.2).
Flat captured scores omit those modifiers and recursion; do not equate all score
surfaces. Expiry returns 0.1 even for refuting evidence, before verdict arithmetic.

The honest way to clear a stale weak pack is displacement — link the re-verification evidence, then supersede the old pack. Editing a pack's verdict to raise a score is history falsification; the graph keeps the original pack precisely so you never have to.

## Technical Details

### Hook Behavior

The hook `.claude/hooks/pre-pr-evidence-check.sh`:
- Runs before `gh pr create` (if wired into Claude Code hooks system)
- Scans branch name and last 20 commits for artifact IDs
- Queries `forgeplan graph` or `forgeplan get` to check for evidence links
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
- **Where**: `plugins/fpl-skills/skills/{audit,sprint,build}.py` (marketplace repo)
- **When**: After agent task completes
- **Action**: Auto-create EVID + link + hint for activation

### Layer 2 (Pre-PR Hook)
- **Where**: `.claude/hooks/pre-pr-evidence-check.sh` (this repo)
- **When**: Before `gh pr create` succeeds
- **Trigger**: Claude Code hooks system or Git hook integration (if available)
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
A: The local hook auto-bypasses `docs/*`; that is a branch-name check, not a determination that the diff is documentation-only or a universal branch-protection exemption.

**Q: What if the forgeplan binary is not installed?**
A: The hook soft-fails (exits 0) rather than blocking. You can still create PRs, but you lose the gate. This is intentional for fresh clones without binaries built.

**Q: Can I wire the hook to Git instead of Claude Code?**
A: Not by renaming it. This script expects Claude Code JSON on stdin; Git pre-push supplies another protocol and does not intercept `gh pr create`. A separately reviewed adapter would be needed.

**Q: What if my artifact is really ad-hoc and doesn't need evidence?**
A: Record the rationale, then decide whether a lightweight NOTE, real EVID or an explicitly justified bypass is appropriate. NOTE creation does not automatically assign a 90-day expiry.

## Reference

- **ADR-003**: Markdown as source of truth for artifacts
- **PRD-077**: Wave 2 evidence autopublish and enforcement
- **PROB-035, PROB-039**: Silent failures from happy-path-only testing
- **Hooks**: `.claude/hooks/pre-pr-evidence-check.sh`
- **Health**: `forgeplan health` command
- **Schema**: §Structured Fields above (EvidencePack structure; a dedicated `docs/schemas/EVIDENCE.md` does not exist yet)

