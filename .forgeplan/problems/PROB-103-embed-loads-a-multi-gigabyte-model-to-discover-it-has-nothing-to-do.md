---
depth: tactical
id: PROB-103
kind: problem
links:
- target: PROB-102
  relation: based_on
- target: PRD-086
  relation: informs
status: draft
title: embed loads a multi-gigabyte model to discover it has nothing to do
---

---
assigned_number: 103
predicted_number: 103
slug: prob-embed-loads-a-multi-gigabyte-model-to-discover-it-has-nothing-to-do
---

# PROB-103: embed pays the model's price to learn there is no work

## Signal

Reported by a user watching the output, not by any test:

```
$ forgeplan embed
  Loading embedding model...
Done: 2 embedded, 425 already current, 0 failed.

$ forgeplan embed
  Loading embedding model...
```

The model loads on every invocation, including runs where nothing needs
encoding.

Measured on 0.36.0, this workspace, 427 artifacts, all current:

| | |
|---|---|
| before | **8.22 s** to report `0 embedded, 427 already current` |
| after | **0.25 s** (warm, three consecutive runs: 0.83 / 0.25 / 0.25) |

The model was read off disk and never used.

## Root cause

Half of PROB-093. That fix made `embed` incremental — a record is skipped when
it already has a vector and its content hash still matches — which removed the
13m18s of redundant encoding. But the skip decision was made *inside* the loop,
after `Embedder::new()` had already run. The expensive part stayed
unconditional.

So the fix removed the work and left the setup for the work.

## Why nobody noticed

No CI job executes `embed` — see PROB-102, where the whole semantic-search
surface is compiled and linted but never run. The cost was invisible to every
gate and visible only to a person watching a terminal.

The output made it worse: `Loading embedding model...` prints before the
outcome, so the run *looks* like it is doing something. Only the final line
says otherwise, and by then the 8 seconds are spent.

## Fix

Compute the work list first; load the model only if it is non-empty. On an
empty list, print the same `Done:` summary and return.

Also corrects the progress line, which claimed the wrong total:

```
- Embedding 427 artifact(s) ...     ← every record, including the skipped ones
+ Embedding 1 of 427 artifact(s) ...
```

`Loading embedding model...` now appears only when a model is actually about to
be loaded, which is what a reader assumed it meant.

## Verified

- no-op run: 0.25 s warm, and the loading line is absent
- real work: model loads, one changed artifact re-encoded, `1 embedded,
  426 already current, 0 failed`

## Related

| Artifact | Relation |
|---|---|
| PROB-102 | based_on |
| PRD-086 | informs |



