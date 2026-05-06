---
depth: standard
id: PROB-059
kind: problem
status: active
title: body↔links drift detector — strict parser warning on Related Artifacts table missing frontmatter links
---

# PROB-059: body↔links drift detector

## Signal

Forgeplan stores artifact relations в two places:
1. **Source of truth** — frontmatter `links:` array (written by `forgeplan link`)
2. **Human-readable** — `## Related Artifacts` table в body markdown

These drift apart routinely:

- Agent runs `forgeplan new prd "..."` → empty file
- Agent fills body via Write/cat (full rewrite). Body includes `## Related Artifacts` table mentioning PRD-005, RFC-001. Frontmatter `links:` empty.
- `forgeplan validate` returns PASS (body table is informational; `links:` is what's checked).
- Agent forgets `forgeplan link X Y --relation ...`.
- Result: artifact looks linked when reading markdown, but `forgeplan graph`, `forgeplan order`, `forgeplan score` propagation, и Lance semantic search all see isolated node.

**Confirmed на этой session**: PRD-074 (7 body mentions / 1 link), PRD-075 (6 body mentions / 1 link), все EVID-104..111 follow same pattern. 20+ silently-missing edges from this session alone.

Reverse drift: `forgeplan link` populates `links:` → agent rewrites body via Write без re-reading frontmatter → stomps `links:` → next reindex deletes the edge. Both cases are agent-friendly landmines.

## Constraints

- Validate exit-code unchanged (still 0 для warnings; non-zero для `--strict` follow-up if implemented).
- Code-block / inline-code mentions ignored (don't false-flag `\`forgeplan link X Y\`` examples в docs).
- Self-id mentions ignored.
- Free-text "see also PRD-005" mentions outside `## Related Artifacts` section ignored — only formal table rows count as "this artifact claims a relation".

## Optimization Targets

- **High signal-to-noise**: only flag drift где user explicitly authored a relation claim в the table. Strict parser, не loose regex.
- **Low false-positive rate**: code blocks, HTML comments, и body sections other than `## Related Artifacts` excluded.
- **Actionable error message**: warning includes the `forgeplan link` command к run.

## Acceptance Criteria

- [x] **AC-1** New `body-links-drift` SHOULD-level rule в `validation::base_rules()` applying к all artifact kinds.
- [x] **AC-2** Strict parser targets `^##+\s+Related Artifacts$` heading + table rows. HTML comments + fenced code blocks + inline backticks stripped first via shared `strip_non_prose_for_leakage` helper (PROB-038 origin).
- [x] **AC-3** Warning message includes the missing IDs and the `forgeplan link` command template.
- [x] **AC-4** Self-id mentions ignored.
- [x] **AC-5** +6 unit tests covering: happy path table extraction, free-text mentions ignored, HTML-comment exclusion, no-section returns empty, frontmatter targets parsing, no-links case.
- [x] **AC-6** Real prose leakage (e.g. tech-leakage existing rule) NOT regressed.

## Deferred к follow-up

- **`forgeplan reconcile <id>` interactive command** — out of scope for v1. Adds dialoguer UX, edge cases (TTY detection, --no-input fallback). Can be added once user feedback shows demand.
- **`forgeplan reconcile --all --apply`** — non-interactive bulk fix. Useful для workspace-wide cleanup AFTER warning rule lands и users have visibility. Track in PROB-059 follow-up sprint.
- **Template hint** — adding `# DO NOT edit links: by hand` comment к `forgeplan new` template frontmatter. Trivial change, can ride along с reconcile work.
- **`--strict` flag для CI** — promotes drift warning to error exit. Add when первый CI consumer asks.

## Blast Radius

- Validate output для **all** artifact kinds (base rule, не PRD-only). Existing artifacts с drift will start showing SHOULD warning until cleaned up.
- Workspace-wide audit: this session's PRD-074/075 + EVID-104..111 will all surface drift на first validate after merge. Expected — not a regression.
- No CLI/MCP/wire format changes.

## Reversibility

**HIGH** — pure additive validator rule. `git revert <commit>` removes the rule cleanly. No schema migration, no LanceDB changes, no breaking API.

## Related Artifacts

| Artifact | Relation |
|---|---|
| ADR-003 | informs (markdown is source of truth — body table и links: array both live in markdown) |
| PROB-038 | informs (related strip-non-prose helper from same audit batch — DRY against same helper) |
| PRD-073 | informs (file-first invariant motivates the warning — body and links must agree on disk) |



