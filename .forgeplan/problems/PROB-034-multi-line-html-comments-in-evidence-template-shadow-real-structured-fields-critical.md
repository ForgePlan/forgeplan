---
depth: deep
id: PROB-034
kind: problem
status: active
title: Multi-line HTML comments in evidence template shadow real structured fields (CRITICAL)
---

# PROB-034: Multi-line HTML comments silently break congruence_level parsing

## Signal

Found during /forge E2E verification sprint (v0.17.2 hotfix). On a fresh
workspace with v0.17.1 release binary:

```bash
$ forgeplan new prd "Target"
$ forgeplan new evidence "Test"
$ sed -i.bak 's/^congruence_level: 3$/congruence_level: 0/' \
    .forgeplan/evidence/EVID-001-test.md
$ forgeplan reindex && forgeplan link EVID-001 PRD-001 --relation informs
$ forgeplan score PRD-001 --json | jq '.r_eff, .evidence[0].congruence_level'
1.0000   ← WRONG, should be 0.10
3        ← WRONG, should be 0
```

User explicitly set `congruence_level: 0` in the Structured Fields section,
but the parser returned CL3. This breaks the entire weakest-link R_eff
formula for any evidence created via `forgeplan new evidence`.

## A/B proof on identical workspace

| Binary                           | r_eff   | display CL | Verdict    |
|----------------------------------|---------|------------|------------|
| v0.17.1 (baseline, no fix)       | 1.0000  | CL=3       | ❌ BUG      |
| v0.17.2 (multi-line fix applied) | 0.1000  | CL=0       | ✅ correct  |

Same `.forgeplan/` directory, two different binaries, opposite answers.
This is a 100% reproducible prod bug, not a test-script artifact.

## Root cause

`crates/forgeplan-core/src/scoring/evidence.rs::extract_field` skipped only
lines *starting* with `<!--`, not lines *inside* a multi-line comment block:

```rust
// v0.17.1 (broken):
if trimmed.starts_with('|') || trimmed.starts_with("<!--") {
    continue;
}
```

The evidence template ships with a helpful multi-line comment:

```markdown
<!-- Fill in the Structured Fields section below for R_eff scoring.

     evidence_type: measurement | test | benchmark | audit
     verdict: supports | weakens | refutes
     congruence_level: 0 | 1 | 2 | 3 (CL3=same context, CL0=opposed context)
-->
```

The line `     congruence_level: 0 | 1 | 2 | 3 (CL3=...)` does NOT start
with `<!--`, so `extract_field` matched it, returning the string
`"0 | 1 | 2 | 3 (CL3=..."`. Then:

```rust
let explicit_cl = extract_field(&record.body, "congruence_level")
    .and_then(|s| s.parse::<u8>().ok())   // fails on non-numeric string
    .filter(|&n| n <= 3);                 // → None
```

`parse::<u8>()` failed silently → `explicit_cl = None` → fallback to CL3
default. The **real** `congruence_level: 0` below in the Structured Fields
section was never inspected because `extract_field` returns the **first**
match and short-circuits.

## Blast radius

**CRITICAL — entire scoring system compromised since PRD-035 template.**

- **All fields affected**: `verdict`, `congruence_level`, `evidence_type`,
  `source_tier` — all are declared in the same template comment block.
- **All evidence affected**: every artifact created via `forgeplan new
  evidence` inherits the template comment. Ingested/imported evidence with
  multi-line comments also affected.
- **All R_eff scores affected**: the weakest-link formula reads penalized
  CL values. If all evidence silently defaults to CL3, R_eff is
  artificially inflated across the whole project.
- **Health dashboard lies**: artifacts with "weak evidence" (CL0-1) show as
  "high R_eff" — blind spots stay hidden.
- **PRD-040 confidence intervals broken**: CI is built from per-evidence
  scores that were all wrong.

Every R_eff number reported by v0.17.0 and v0.17.1 in a real workspace is
potentially inflated.

## Fix

`extract_field` now implements a proper multi-line HTML comment state
machine:

```rust
let mut in_multiline_comment = false;
for line in body.lines() {
    let trimmed = line.trim();

    if in_multiline_comment {
        if trimmed.contains("-->") {
            in_multiline_comment = false;
        }
        continue;
    }
    if trimmed.starts_with("<!--") {
        if !trimmed.contains("-->") {
            in_multiline_comment = true;
        }
        continue;
    }

    if trimmed.starts_with('|') { continue; }

    if let Some(rest) = trimmed.strip_prefix(&prefix) {
        let val = rest.trim();
        if !val.is_empty() { return Some(val.to_string()); }
    }
}
```

Two regression tests added:
1. `extract_field_ignores_multiline_html_comments` — replicates the real
   evidence template scenario end-to-end.
2. `extract_field_multiline_comment_nested_fields_all_ignored` — guards
   against multiple `key: value` lines hiding inside a single comment.

## Acceptance Criteria

1. ✅ `extract_field` skips lines inside multi-line `<!-- ... -->` blocks.
2. ✅ `extract_field_ignores_html_comments` (single-line) still passes.
3. ✅ Multi-line regression tests pass.
4. ✅ E2E: `forgeplan new evidence` → edit congruence_level → reindex →
   `forgeplan score --json` returns the correct CL, not CL3 default.
5. ✅ A/B test on same workspace with v0.17.1 vs v0.17.2 binaries shows
   corrected R_eff.
6. ✅ Full `cargo test --workspace` green (1137/1137).

## Impact

**CRITICAL** — trust calculus was silently blind since v0.17.0. Fix is
mandatory for any user relying on R_eff for decision quality.

## Blast Radius

- `forgeplan_core::scoring::evidence::extract_field` (single function, one
  state machine addition)
- All callers (`congruence_level`, `verdict`, `evidence_type`,
  `source_tier`) benefit automatically

## Reversibility

HIGH — 20-line state machine addition, heavily tested. No behavior change
for docs without multi-line comments.

## Related

| Artifact | Relation |
|---|---|
| PRD-040 | informs (scoring intelligence relied on correct CL parsing) |
| PRD-043 | informs (methodology integrity assumed honest R_eff) |
| PROB-031 | sibling (the visible symptom — CLI had its own duplicate parser too) |
| NOTE-048 | informs (real-world verification gaps — this is exactly why) |

