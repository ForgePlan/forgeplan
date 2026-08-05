---
depth: tactical
id: EVID-149
kind: evidence
links:
- target: PRD-008
  relation: informs
status: draft
title: 'id-collision detector + PRD-008 cleanup: scan-import flags two .md with one id; PRD-008 dup stub removed, resolver consistent'
---

## Summary

Closes the file-level id-collision class surfaced by the PRD-008 duplicate. Adds a scan-import detector for two+ `.md` files resolving to one artifact id, and removes the PRD-008 duplicate stub.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Problem

`forgeplan get PRD-008` returned **desynced data** — `status: deprecated` (from one file) with body from another (the draft stub) — because two `.md` carried `id: PRD-008`. The DB-level `create_artifact` dup-id guard only blocks NEW collisions via the proper path; legacy artifacts (pre-Phase-1.5) + hand-authored files (pre-RED-LINE-#11) slip in at the file level, and the title-similarity `DuplicateArtifact` anomaly misses same-id-different-title pairs.

## Fix

- **Detector** (`scan/import.rs`): `find_duplicate_ids()` + `DuplicateIdGroup` in the scan result — groups entries by resolved id, flags any id mapping to >1 file (BTreeMap, sorted, deterministic). `scan-import` CLI prints a loud red warning + fix hint.
- **Cleanup**: removed the broken PRD-008 draft stub (double frontmatter + unfilled template placeholders); kept the content-bearing deprecated file. `reindex` confirms PRD-008 now resolves consistently; detector reports no PRD-008 collision.

## Tests

3 tests (`scan/import.rs`): `find_duplicate_ids_groups_same_id_and_ignores_unique`, `find_duplicate_ids_empty_when_all_unique`, `scan_detects_duplicate_id_files` (e2e two-files-one-id). forgeplan-core **2109 passed / 0 failed**, clippy 0, fmt 0.

## Adjacent findings (NOT fixed here — surfaced for backlog)

`reindex` reported 5 pre-existing data errors unrelated to this cleanup: `ADR-013` has no YAML frontmatter (parse error), and 4 DB rows reference missing files (`EVID-086`, `EVID-036`, `EVID-087`, `SESSION-2026-04-06`). These belong to the v0.33-plan "health debt / 27 problems revision" track.

## Provenance

- Branch: `fix/id-collision-detector` (off `origin/dev`)
- Files: `crates/forgeplan-core/src/scan/import.rs`, `crates/forgeplan-cli/src/commands/scan_import.rs`; removed `.forgeplan/prds/PRD-008-cli-ux-redesign.md`
- **Renumber note (2026-08-06):** this evidence was itself a file-level id-collision — it and the earlier-committed EVID-143 (PROB-073 create-roundtrip profile) both carried `id: EVID-143`. Per the same one-id-one-file rule this evidence documents, it was re-minted as **EVID-149** (content unchanged) so PROB-073 keeps EVID-143. The companion reindex detector that catches this exact class at reindex time (not just scan-import) is issue #394.


