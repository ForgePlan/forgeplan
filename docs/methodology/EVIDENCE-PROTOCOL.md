# Evidence-Driven PR Protocol

## Overview

Forgeplan enforces a requirement that every artifact (PRD, RFC, ADR, EPIC, SPEC, PROB) mentioned in a pull request must have linked **evidence** before the PR is created. This ensures that architectural and feature decisions are backed by proof: tests, benchmarks, audits, or measurements.

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
The marketplace plugin `fpl-skills` v1.5.0+ includes upgraded `/audit`, `/sprint`, `/build` skills that **autopublish EVID** after agent work. These skills:
- Capture test output, audit findings, benchmark results
- Auto-create `EVID` artifact with structured fields
- Auto-link to the affected PRD/RFC/ADR with `informs` relation
- Emit `_next_action: forgeplan activate <ID>` hint

This layer catches agent-spawned workflows and ensures evidence is created during work, not afterwards.

### Layer 2: Pre-PR Hook (FR-014, this document)
Before `gh pr create` succeeds, `.claude/hooks/pre-pr-evidence-check.sh` runs:
1. Parses branch name and last 20 commits for artifact IDs (PRD-NNN, RFC-NNN, etc.)
2. For each non-EVID artifact found, queries forgeplan graph for `informs` or `based_on` evidence links
3. **Blocks PR creation** (exit code 2) if evidence is missing
4. Provides clear bypass instructions for legitimate exceptions

This layer is a hard gate at the human PR boundary — you cannot push past this without evidence or explicit bypass.

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
forgeplan new evidence "Feature X: test coverage 92%, p95 latency 180ms"
```

### Step 2: Fill structured fields
Edit `.forgeplan/evidence/EVID-NNN-*.md` and add:
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
`## Structured Fields` block. **All three or none** — a partial claim is rejected as
`Incomplete`:

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
| `active` | **yes** | current testimony; an active `refutes` pack zeroes the score |
| `stale` | **yes** | not terminal — flagged for re-evaluation but not displaced; an expired `valid_until` separately decays the pack's score to 0.1 |
| `superseded` | **no** | displaced by a successor (`supersede <old> --by <new>`) — history stays in the graph, but it no longer speaks for present reliability |
| `deprecated` | **no** | retired (e.g. a duplicate deprecated with `--reason "superseded by EVID-y"`) |

Every exclusion is logged in `forgeplan score` factors (`Skipped EVID-x (status: superseded)`) and marked in the breakdown (`excluded from min`) — displacement is auditable, never silent. If ALL linked packs are terminal, the artifact degrades to **no active evidence** (R_eff 0.0): recovery requires the replacement pack to be *linked* to the artifact, not just to exist.

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
A: No — `docs/*` branches auto-bypass. Branch protection assumes docs changes don't need architectural evidence.

**Q: What if the forgeplan binary is not installed?**
A: The hook soft-fails (exits 0) rather than blocking. You can still create PRs, but you lose the gate. This is intentional for fresh clones without binaries built.

**Q: Can I wire the hook to Git instead of Claude Code?**
A: Yes — copy `.claude/hooks/pre-pr-evidence-check.sh` to `.git/hooks/pre-push` (or custom hook) and rename to `pre-push`. Make sure your hook invokes it before `git push`.

**Q: What if my artifact is really ad-hoc and doesn't need evidence?**
A: File a PROB or decision note explaining why, then decide: (1) reclassify as `NOTE` (ephemeral, auto-expires), (2) retroactively create minimal EVID, or (3) use bypass + document.

## Reference

- **ADR-003**: Markdown as source of truth for artifacts
- **PRD-077**: Wave 2 evidence autopublish and enforcement
- **PROB-035, PROB-039**: Silent failures from happy-path-only testing
- **Hooks**: `.claude/hooks/pre-pr-evidence-check.sh`
- **Health**: `forgeplan health` command
- **Schema**: §Structured Fields above (EvidencePack structure; a dedicated `docs/schemas/EVIDENCE.md` does not exist yet)

