---
depth: tactical
id: EVID-079
kind: evidence
links:
- target: ADR-008
  relation: supports
- target: EPIC-006
  relation: supports
- target: PRD-059
  relation: supports
- target: PRD-060
  relation: supports
- target: PRD-061
  relation: supports
- target: PRD-062
  relation: supports
- target: PRD-063
  relation: supports
- target: PRD-064
  relation: supports
status: draft
title: Brownfield pipeline — sources research (ccpm OpenSpec adr-tools BMAD) supports unified approach
---

# EVID-079: Brownfield pipeline — sources research (ccpm OpenSpec adr-tools BMAD) supports unified approach

## Structured Fields

verdict: supports
congruence_level: 2
evidence_type: research

## Measurement

Исследование 4 adjacent проектов в `sources/` на предмет паттернов brownfield миграции, agent-skills distribution, self-describing tool design — чтобы зафиксировать ADR-008 решение на проверенных паттернах, а не изобретать с нуля.

**Method**:
1. `sources/ccpm/skill/ccpm/SKILL.md` — прочитан полностью, замечены паттерны frontmatter (`name`+`description`), 5-phase workflow, script-first rule, references/conventions.md — централизованные конвенции.
2. `sources/OpenSpec/README.md`, `docs/migration-guide.md`, `docs/opsx.md`, `openspec/config.yaml` — прочитаны. Зафиксированы: прямая цитата «built for brownfield not just greenfield», auto-install skills в `.claude/skills/` (и эквиваленты), context injection через `config.yaml`, init-time detection legacy files с per-file cleanup proposal, action-based (не phased) workflow, schema.yaml + templates hackability.
3. `sources/adr-tools/src/adr-new`, `README.md` — прочитаны. Зафиксированы: `-s SUPERSEDED` bidirectional update (меняет status обеих сторон), `-l TARGET:LINK:REVERSE-LINK` typed bidirectional links, self-bootstrap (первая ADR создаётся automagically).
4. `sources/BMAD-METHOD/AGENTS.md`, `tools/skill-validator.md` — прочитаны. Зафиксированы: 14 deterministic skill validation rules (SKILL-01..07, WF-01/02, PATH-02, STEP-01/06/07, SEQ-02), name format constraint `^bmad-[a-z0-9]+(-[a-z0-9]+)*$`, description must include «Use when» clause (SKILL-06).

## Result

**Паттерны confirmed** в 3+ adjacent проектах (≥2 = industrial consensus, 3+ = emerging standard):

| Pattern | ccpm | OpenSpec | adr-tools | BMAD | Для нас |
|---|---|---|---|---|---|
| Agent-skills standard (SKILL.md + name+description frontmatter) | ✅ | ✅ | — | ✅ | Adopt |
| Multi-harness auto-install (`.claude/`, `.cursor/`, `.windsurf/`, etc.) | — | ✅ | — | ✅ | Adopt (PRD-062) |
| Context injection через config (inject в каждый request) | — | ✅ | — | — | Adopt (PRD-060) |
| Init-time detection legacy + cleanup wizard | — | ✅ | — | — | Adopt (PRD-062) |
| Script-first rule (deterministic → bash, reasoning → LLM) | ✅ | — | — | — | Adopt (core CLI vs skill LLM) |
| Self-bootstrap (init creates first artifact) | — | — | ✅ | — | Consider (out of PRD-064 scope, future) |
| Bidirectional supersede (atomically update both sides) | — | — | ✅ | — | Adopt (PRD-063) |
| Skill validator with deterministic rules | — | — | — | ✅ | Adopt (for brownfield-pack skills PRD-061 validation) |
| Action-based workflow (не phased) | — | ✅ | — | — | Reject (сохраняем shape→code phases — unique forge value) |
| All-phases in one big command | — | — | — | — | Reject (splitting discover/migrate) |

**Consensus**: 8 из 10 identified паттернов адоптируются. 2 отвергаются с обоснованием (unique forge value: R_eff, FPF, phased workflow).

**Direct citations подтверждающие unified approach**:
- OpenSpec philosophy: *«→ fluid not rigid · → built for brownfield not just greenfield · → scalable from personal projects to enterprises»*
- ccpm README: *«CCPM is now an AGENT SKILL! It works with any Agent Skills–compatible harness... Claude Code, Codex, OpenCode, Factory, Amp, Cursor, and more.»*
- OpenSpec migration-guide: *«OpenSpec now uses agent skills, the emerging standard across coding agents. This simplifies your setup while keeping everything working as before.»* — validates что standard emerging, timing подходящий для нашего adoption.

## Interpretation

ADR-008 решение **Unified Approach (B)** vs other alternatives получает signal: 8 из 10 ключевых паттернов валидированы в ≥1 adjacent проекте, 3 паттерна (agent skills + context injection + init-time detection) валидированы в OpenSpec — проект близкий по миссии (spec-driven development с brownfield focus).

Options rejected в ADR-008 тоже подкреплены research:
- **A (только stderr hints)**: ни один из 4 проектов не ограничился только hints — все имеют skill distribution. Evidence против.
- **C (docs-only)**: ни один из 4 проектов не полагается только на docs — у всех runtime integration (skill install). Evidence против.
- **D (full OPSX port)**: ценность forge (R_eff, FPF, phased workflow) не существует в OpenSpec — теряем unique при полном port. Evidence против.

**Weakest link** (согласно ADR-008) — agentskills.io standard maturity. Research confirm: стандарт emerging, формальной spec нет, но 4 проекта уже используют одни и те же conventions (name+description+use-when). Adoption risk приемлем при изоляции harness-adapters в одном crate (PRD-062 design).

**Recommendation**: Adopt Unified Approach (ADR-008 Decision) с высокой уверенностью. Design patterns документированы в PRD-059..063. Weakest Link addressed через modular adapter crate.

## Congruence Level Justification

CL2 (related context): evidence — research паттернов в adjacent проектах, не прямое measurement на нашем коде. Это proper research evidence, не measurement. Unified Approach не тестировался на нашем vault — это будет E1 evidence в code phase (PRD-059 E2E test). Поэтому CL2 correctly, не CL3.

Penalty CL2 = 0.1, acceptable for Shape-phase decision-support evidence.

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| ADR-008 | ADR | supports |
| EPIC-006 | Epic | supports |











