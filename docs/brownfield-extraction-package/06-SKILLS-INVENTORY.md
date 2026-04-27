# Skills Inventory (at-a-glance)

> Quick reference for the 12 skills. Full design docs in `skills/`.

## Priority tiers

| Tier | Skills | Rationale |
|---|---|---|
| **P0 — Foundation** | C1, C2 | Everything else depends on these |
| **P1 — Factum** | C4, C5 | Low-risk, high-value, code-only |
| **P2 — Intent** | C3 | LLM-heavy, generates hypotheses |
| **P3 — Synthesis** | C6, C7, C8, C9 | Depends on P0-P2 data |
| **P4 — Output** | C10, C11, C12 | Deliverables, requires P0-P3 complete |

## Quick table

| # | Skill | Input | Output | Implementation effort | Autoresearch hook |
|---|---|---|---|---|---|
| C1 | `ubiquitous-language` | Codebase | `glossary` artifacts | Small (pattern extraction) | `/autoresearch:learn --mode glossary` |
| C2 | `use-case-miner` | Entry points | `use-case` artifacts | Medium (trace flow) | `/autoresearch:learn --mode use-case` |
| C3 | `intent-inferrer` | Code patterns | `hypothesis` artifacts | Large (LLM reasoning) | `/autoresearch:reason --mode intent` |
| C4 | `invariant-detector` | Guards | `invariant` artifacts | Small (AST scan) | `/autoresearch:learn --mode invariant` |
| C5 | `causal-linker` | Actions + events | Graph edges | Medium (multi-file trace) | `/autoresearch:predict --persona causality-analyst` |
| C6 | `hypothesis-triangulator` | `hypothesis` + git + docs | Confidence updates | Large (triangulation logic) | New skill |
| C7 | `interview-packager` | Parked hypotheses | Interview markdown | Small (clustering) | New skill |
| C8 | `scenario-writer` | Verified use-cases | `scenario` artifacts | Medium (Gherkin generation) | `/autoresearch:scenario --template gherkin` |
| C9 | `kg-curator` | All artifacts | Graph + reports | Large (graph reasoning) | New skill |
| C10 | `canonical-reproducer` | All verified | Standalone docs | Medium (templating) | `/autoresearch:learn --mode canonical` |
| C11 | `reproducibility-validator` | Canonical + code | Validation report | Large (cross-check) | `/autoresearch:predict --persona reproducibility-judge` |
| C12 | `rag-packager` | All verified | RAG bundle | Small (format conversion) | New skill |

## Skill interfaces (common)

Every skill conforms to this shape:

```yaml
skill_name: "<name>"
version: 1.0.0
description: "<one line>"
input:
  required: [...]
  optional: [...]
output:
  artifacts_kind: [...]
  side_effects: [...]
modes:
  - name: "<mode>"
    purpose: "..."
metric:
  name: "coverage" | "quality" | "reproducibility"
  formula: "..."
  target: 0.85
dependencies:
  artifacts: [...]
  skills: [...]
  external: [git, forgeplan-mcp, autoresearch]
```

## Invocation pattern (from orchestrator)

Each skill is invoked as a sub-agent with:
- **Prompt template** (in skill design doc).
- **Read-only scope** (file list).
- **Write target** (forgeplan artifact kind + workspace path).
- **Metric & guard** (validation command).
- **Iteration budget** (max loop count).
- **Escalation rule** (when to ask user).

## Order of implementation (for forgeplan agent)

Wave 1 (foundation — can build in parallel):
1. Update forgeplan with 6 new artifact kinds (`04-FORGEPLAN-EXTENSIONS.md`).
2. Create templates (`templates/`).
3. Build C1 (`skills/01-ubiquitous-language.md`).
4. Build C4 (`skills/04-invariant-detector.md`).

Wave 2:
5. Build C2 (`skills/02-use-case-miner.md`).
6. Build C5 (`skills/05-causal-linker.md`).

Wave 3:
7. Build C3 (`skills/03-intent-inferrer.md`).
8. Build C6 (`skills/06-hypothesis-triangulator.md`).

Wave 4:
9. Build C7 (`skills/07-interview-packager.md`).
10. Build C8 (`skills/08-scenario-writer.md`).
11. Build C9 (`skills/09-kg-curator.md`).

Wave 5:
12. Build C10 (`skills/10-canonical-reproducer.md`).
13. Build C11 (`skills/11-reproducibility-validator.md`).
14. Build C12 (`skills/12-rag-packager.md`).
15. Build orchestrator (`orchestration/extract-business-logic.md`).

See `ROADMAP.md` for timing and `TASKS.md` for checklist.
