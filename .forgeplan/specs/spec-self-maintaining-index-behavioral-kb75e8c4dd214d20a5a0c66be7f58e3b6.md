---
assigned_number: 13
author: Eli Rum
created: 2026-11-18
depth: standard
id: SPEC-013
links:
- target: prd-self-maintaining-derived-index-lazy-sync-kff438581336d5e91e7a2662055c61575
  relation: refines
slug: spec-self-maintaining-index-behavioral-kb75e8c4dd214d20a5a0c66be7f58e3b6
status: Draft
title: Self-maintaining index behavioral contracts
updated: 2026-11-18
---

# SPEC-013: Self-maintaining index behavioral contracts

## Requirements

Behavioral contract for the derived-index maintenance surface. Frozen scenarios for TDD; each `#### Scenario` is a failing test until GREEN.

### Requirement: Lazy sync on read

The derived index shall not serve a stale artifact body when the `.md` file on disk is newer than the index's `updated_at` for that artifact.

#### Scenario: edited file visible to `get`

```gherkin
Given a workspace with PRD-001 whose body is "alpha"
And the LanceDB row for PRD-001 has updated_at T0
When I write "alpha-edited" into .forgeplan/prds/PRD-001.md with mtime T1 > T0
And I run `forgeplan get PRD-001`
Then the output contains "alpha-edited"
And stderr prints `reconciled: 1 file(s)`
And the LanceDB row's updated_at is T1
```

#### Scenario: new file visible to `list`

```gherkin
Given a workspace where `.forgeplan/notes/` is empty in the index
When I create `.forgeplan/notes/note-fresh-<hash>.md` with frontmatter for NOTE-001
And I run `forgeplan list --kind note`
Then the output contains NOTE-001
```

#### Scenario: deleted file removed from index

```gherkin
Given a workspace where NOTE-001 exists in index and on disk
When I delete `.forgeplan/notes/note-*.md`
And I run `forgeplan list --kind note`
Then NOTE-001 is absent from the output
```

#### Scenario: unchanged file is a no-op

```gherkin
Given PRD-001 file and index row both at mtime T0
When I run `forgeplan get PRD-001`
Then stderr does NOT print a reconciliation line
And the index row is untouched
```

#### Scenario: many files, bounded cost

```gherkin
Given a workspace with 500 artifacts
And exactly 3 files have been edited since the last index
When I run `forgeplan list`
Then lazy sync completes in under 50ms (p95)
And stderr reports `reconciled: 3 file(s)`
```

### Requirement: Embedding queue drains without manual `embed`

After any mutation, pending embedding jobs shall be durable and shall drain on the next `index`/`embed`/`search --semantic` — not require a separate `embed` call the operator must remember.

#### Scenario: mutation enqueues a pending job

```gherkin
Given a workspace with semantic-search enabled and the model available
When I run `forgeplan new note "T" --allow-duplicate`
Then the durable queue contains one pending job for the new artifact
And the queue file survives a SIGKILL
```

#### Scenario: `index --embed-profile` drains pending jobs

```gherkin
Given a workspace with 3 pending embedding jobs
When I run `forgeplan index --embed-profile <id>`
Then after the pass the queue is empty
And `forgeplan search --semantic "<title>"` finds the mutated artifact
```

#### Scenario: drain skipped when model absent

```gherkin
Given a workspace with pending jobs and no warmed model
When I run `forgeplan index --embed-profile <id>`
Then the pass completes (index maintenance still runs)
And the queue still holds the pending jobs
And stderr reports `embedding drain skipped: model not present`
```

### Requirement: Model auto-download on first semantic use

The first call that needs the embedding model shall download it with visible progress instead of requiring `setup --warm-model`.

#### Scenario: cold `search --semantic` bootstraps the model

```gherkin
Given a fresh install where ~/.forgeplan/models/ is empty
And the binary has semantic-search feature
When I run `forgeplan search --semantic "auth"`
Then a progress line shows the model downloading (~2.1 GB)
And the search completes with results
And a second `search --semantic` finishes in under 1 second without downloading
```

#### Scenario: model download interrupted is resumable

```gherkin
Given a model download in progress
When I send SIGINT
Then the partial cache is not corrupted
And a second run resumes the download
```

#### Scenario: build without semantic-search gives honest error

```gherkin
Given a binary built WITHOUT the semantic-search feature
When I run `forgeplan search --semantic "x"`
Then the command exits non-zero
And stderr contains "semantic-search feature" and a Fix: hint pointing at the right install
```

### Requirement: JSON envelope on error path

Every `--json` command shall emit a parseable envelope on stdout in both success and error paths.

#### Scenario: `get --json` on a bad selector is parseable

```gherkin
Given any workspace
When I run `forgeplan get NOPE-999 --json`
Then stdout is a single JSON object
And `.ok` is false
And `.error` contains "NOPE-999"
And exit code is non-zero
```

#### Scenario: `--json` envelope covers all machine-facing commands

```gherkin
Given each of [get, list, search, graph, export, release-notes, health] with a `--json` flag
When the command errors before serialization
Then stdout is still a JSON envelope, not plain `Error:` text
```

### Requirement: Corrupt config fails the index pass

`maintain_index` shall not run against a workspace whose `config.yaml` cannot be parsed — the config drives the embedding profile, and silent defaulting would index under wrong assumptions.

#### Scenario: invalid YAML in config.yaml aborts

```gherkin
Given a workspace where `.forgeplan/config.yaml` contains `{{{{not valid yaml: [unclosed`
When I run `forgeplan index`
Then the command exits non-zero
And stderr names `.forgeplan/config.yaml`
And no reconcile/publish pass runs
```

#### Scenario: valid config runs normally

```gherkin
Given a workspace with a well-formed `config.yaml`
When I run `forgeplan index`
Then the pass completes and reports `verified: nothing staged` or reconciles
```

## Related

- PRD-092 — parent product spec (lazy sync + queue + bootstrap)
- RFC-004 — Files-First Architecture (Phase 1 lazy sync, Phase 2 watcher)
- ADR-003 — Markdown = source of truth
- PROB-067 / SPEC-005 / RFC-009 / ADR-012 — Constellation identity work (same store)
- GitHub #568 — `--json` error-path envelope bug; #569 — corrupt-config index bug
