---
depth: tactical
id: EVID-050
kind: evidence
links:
- target: PRD-023
  relation: informs
status: active
title: Distribution pipeline — cargo-dist validated, 5 targets, audit-fixed
---

# EVID-050: Distribution pipeline — cargo-dist validated, 5 targets, audit-fixed

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-04-04 |
| Valid Until | 2026-07-04 |
| Target | PRD-023 |

## Structured Fields

evidence_type: audit
verdict: supports
congruence_level: 3

## Measurement

cargo-dist v0.31.0 initialized on Forgeplan workspace. `dist plan` executed successfully, producing build plan for 5 targets across 2 binaries. Two parallel audits conducted: workflow security (4C+3H findings) and PRD quality (3H+3M findings).

## Result

- `dist plan`: 5 targets (aarch64-apple-darwin, aarch64-unknown-linux-gnu, x86_64-apple-darwin, x86_64-unknown-linux-gnu, x86_64-pc-windows-msvc)
- 2 binaries: forgeplan-cli + forgeplan-mcp
- Installers: shell (install.sh) + homebrew (forgeplan.rb)
- Checksums: SHA256 per artifact + global sha256.sum
- Audit findings fixed: 4 CRITICAL (action versions @v6/@v7 → @v4), 3 HIGH (redundant config, gitignore, PRD tech leakage)
- 753 tests pass, 0 fmt diffs, 0 warnings after fixes
- PRD-023 validate: PASS (0 errors)

## Interpretation

cargo-dist (H1 from ADI) is validated as the correct approach. It generates production-grade CI pipeline covering all FR-001 through FR-008 requirements. The audit revealed cargo-dist v0.31.0 generates non-existent action versions — this is a known tool bug, manually patched. The approach saves ~200 lines of custom YAML.

## Congruence Level Justification

CL3: Direct measurement on the actual Forgeplan workspace. dist plan, dist generate, and audit executed in the real project context, not a test fixture.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-023 | informs |


