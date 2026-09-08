---
depth: tactical
id: PROB-100
kind: problem
links:
- target: PROB-059
  relation: refines
status: draft
title: body-links-drift and forgeplan_get both read a record that carries no links
---

---
assigned_number: 100
predicted_number: 100
slug: prob-body-links-drift-and-forgeplan-get-both-read-a-record-that-carries-no-links
---

# PROB-100: two tools, one blind spot

## Problem

`ArtifactRecord` has no link fields. Everything that reads a record therefore
sees an artifact with five edges exactly as it sees an orphan, and two shipped
tools were built on top of that blindness without anyone noticing.

**The validator warned about links that exist.** `frontmatter_map()` rebuilds a
frontmatter from the record's columns. The record has no link columns, so the map
never carries `links`, so `extract_frontmatter_link_targets` returned an empty
set for every artifact. `body-links-drift` compared the body's
`## Related Artifacts` table against that empty set and warned on everything it
found there — including targets that were correctly linked and visible in
`forgeplan graph` the same minute.

**`forgeplan get` reported an orphan for a linked artifact.** Both edge sets were
fetched, reduced to a boolean to choose a hint, and discarded one line before the
output. The response carried about twenty fields and no links at all.

## Why it survived

Neither tool failed. The warning was plausible, the response looked complete, and
both were confidently wrong in a way that reads as correct.

The warning also could not be closed. Linking the named target did not silence
it, because the comparison never saw links; the only way to make it stop was to
delete the table it was complaining about. A SHOULD-level warning that fires on
essentially every well-linked artifact and cannot be resolved by doing the right
thing teaches its readers to skip validator warnings — which is more expensive
than the drift it was built to catch.

The regex made it worse. `[A-Z]+-[0-9]+` matched any token of that shape, so
requirement numbers (`FR-1`) and invariant numbers (`I-3`) were named as missing
link targets. Those can never be linked, and the remediation the rule printed —
`forgeplan link <this-id> FR-1` — could not be executed. That is a PRD-071
violation of the same shape as #348 and #351: a hint an agent is obliged to run
that cannot work.

## Measurement

Workspace: this repository, `forgeplan 0.35.0` built from `ca5a7c2`.

`forgeplan validate SPEC-003`, before:

```
mentions ADR-009, EPIC-007, FR-1, FR-2, FR-3, FR-5, PRD-065, SPEC-004
```

Eight ids, seven false. `ADR-009`, `PRD-065` and `SPEC-004` are real edges —
`forgeplan graph` shows all three. `FR-1` through `FR-5` are requirement numbers.

After: one id, `EPIC-007`, and it is true — the graph has no `SPEC-003 → EPIC-007`
edge.

`forgeplan get SPEC-003 --json`, before: 14 keys, none about links. After: three
outbound edges matching `graph` exactly, plus one inbound (`EVID-089 informs`)
that `graph | grep "SPEC-003 -->"` cannot show at all.

A third disagreement surfaced while measuring, not reported in either issue:
`SPEC-003` has **three** different frontmatters — the file on disk (with links),
the copy embedded in the stored `body` (stale, no links, still carrying a
`created:` field the file dropped), and the reconstruction from record columns
(no links, ever).

## Relation to PROB-059

PROB-059 introduced this rule and is still `active`. The drift it describes is
real and remains worth detecting. This records that its detector never worked:
the rule shipped in a state where its condition could not be satisfied.

## Fix

Links are read from the relations table — the same source `forgeplan graph`
renders and `forgeplan link` writes — through
`LanceStore::frontmatter_map_with_links`, used at all three sites that run the
full rule set. The extractor keeps only prefixes that map to a real artifact
kind. `get` emits both edge sets unconditionally, as empty arrays when there are
none, so a caller can tell "no links" from "not reported".

## Related

| Artifact | Type | Relation |
|---|---|---|
| PROB-059 | Problem | the rule this one reports as non-functional |

GitHub: #446, #447.



