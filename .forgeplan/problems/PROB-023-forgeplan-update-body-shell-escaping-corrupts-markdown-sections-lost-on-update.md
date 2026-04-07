---
depth: tactical
id: PROB-023
kind: problem
status: draft
title: forgeplan update --body shell escaping corrupts markdown — sections lost on update
---

## Problem

When using forgeplan update --body with shell variable containing markdown, content gets corrupted or truncated. Shell escaping of backticks, dollar signs, and quotes in markdown causes loss of sections.

## Signal

- PRD-035 body lost after forgeplan update PRD-035 --body "$BODY" (only frontmatter remained)
- RFC-001 progress bars lost after forgeplan update --body
- Happened twice in Sprint 11-12 sessions

## Root Cause

1. Shell escaping: backticks in markdown code blocks conflict with shell
2. Dollar signs in body interpreted as shell variables
3. Newlines in body may be collapsed by shell argument passing
4. update --body replaces ENTIRE body (by design), but if shell corrupts the input, old content is destroyed

## Proposed Fix

Option A: forgeplan update --body @file.md — read body from file (already supported! line 57 in update.rs)
Option B: forgeplan update --body-append — append to existing body instead of replace
Option C: Always use --body @file for multi-line content, document this clearly

## Recommendation

Option A already works. The issue is that AI agents use --body with inline content instead of @file. Fix: document that --body @filepath is the safe way. MCP tool should use temp file instead of inline body.

## Related

- ADR-003 (files = truth — editing files directly is preferred)
- PROB-022 (lost PRD-035 during discover shaping)
