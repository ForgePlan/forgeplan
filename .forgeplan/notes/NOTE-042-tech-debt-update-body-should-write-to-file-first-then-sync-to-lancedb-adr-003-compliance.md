---
depth: tactical
id: NOTE-042
kind: note
links:
- target: PRD-019
  relation: informs
- target: PROB-023
  relation: informs
- target: ADR-003
  relation: informs
status: draft
title: 'TECH DEBT: update --body should write to FILE first then sync to LanceDB (ADR-003 compliance)'
---

## Tech Debt: File-First Update Flow

### Current (wrong)
--body content → store.update_body (LanceDB) → render_projection (file from LanceDB)

### Correct per ADR-003
--body content → write directly to .md file → sync_file_to_store (LanceDB from file)

### Why it matters
ADR-003: files = source of truth, LanceDB = derived cache.
Current flow treats LanceDB as primary and file as derived — inverted.
If LanceDB corrupts or loses data, file should be authoritative. But with current flow, file gets overwritten from LanceDB.

### Fix
1. In update.rs: write body_content directly into the markdown file (preserve frontmatter, replace body section)
2. Call sync_file_to_store to update LanceDB from file
3. Skip render_projection_with_body entirely for body updates

### Priority: P1 — not blocking but violates core architecture principle


