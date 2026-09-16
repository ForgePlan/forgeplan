---
title: forgeplan_release_notes
description: "Auto-generate Keep-a-Changelog-shaped release notes from artifacts that changed between two git refs. Walks `git log` over `.forgeplan/{prds, problems, evidence, rfcs, adrs, specs, epics, solutions}/`, classifies each touched artifact (PRD→Added, PROB→Fixed, EVID-on-security→Security, RFC/ADR→Changed) and emits the structured payload. Quality gate (default): only artifacts with status==active or r_eff_score > 0 are included. Pass `draft: true` to waive the gate. `since` defaults to the latest git tag, `until` defaults to HEAD. Example: `{since: \"v0.30.0\", draft: false}`."
---

Auto-generate Keep-a-Changelog-shaped release notes from artifacts that changed between two git refs. Walks `git log` over `.forgeplan/{prds, problems, evidence, rfcs, adrs, specs, epics, solutions}/`, classifies each touched artifact (PRD→Added, PROB→Fixed, EVID-on-security→Security, RFC/ADR→Changed) and emits the structured payload. Quality gate (default): only artifacts with status==active or r_eff_score > 0 are included. Pass `draft: true` to waive the gate. `since` defaults to the latest git tag, `until` defaults to HEAD. Example: `{since: "v0.30.0", draft: false}`.

**Category**: Other

The changelog writes itself from the artifact graph: the tool diffs two git refs to find which artifacts were touched, loads each one back out of the store, drops the ones that carry no signal, and returns the survivors bucketed into Keep-a-Changelog sections. The agent's job is then only to paste the entries under `[Unreleased]` in `CHANGELOG.md`.

## What it does

1. **Resolves the range.** `since` is validated and used as given; without it the tool takes the nearest reachable tag (`git describe --tags --abbrev=0`) and falls back to `HEAD~50` when the repo has no tags at all. `until` defaults to `HEAD`.
2. **Walks the history.** `git log --name-only` runs over the allow-listed kind directories only: `prds`, `problems`, `evidence`, `rfcs`, `adrs`, `specs`, `epics`, `solutions`. Notes, Memory and Refresh live in directories the changelog deliberately ignores.
3. **Loads each touched artifact** from the store by canonical id. An artifact that was renamed or deleted since the commit is silently dropped rather than reported as an error.
4. **Applies the quality gate** (see below) and classifies what survives.
5. **Formats the payload** as a structured object with one key per section, plus `since`, `until`, `draft` and `total`.

Git discovery is worktree-aware: if `.git/` is not next to `.forgeplan/`, the tool walks up (bounded by the nearest `.forgeplan/` ancestor or eight hops) and one level down to find it, rejecting symlinked candidates.

## Categories

| Artifact | Section | Notes |
|---|---|---|
| `prd` | `added` | a shipped feature |
| `problem` | `fixed` | a closed bug |
| `evidence` linked to a `security` tag | `security` | the evidence itself, or any neighbour carrying the `security` tag |
| `evidence` with no security link | `internal` | kept out of the user-facing sections but still visible in draft mode |
| `rfc`, `adr`, `spec`, `epic`, `solution` | `changed` | architectural and umbrella changes |
| `note`, `memory`, `refresh` | dropped | never emitted, regardless of ref range |

"Linked to security" is a tag check, not a kind check: the artifact carries the `security` tag, or any artifact it links to or is linked from does.

## Quality gate

An artifact has **signal** when its status is `active` or its R_eff score is above zero. Without `draft: true`, everything without signal is dropped, which is what keeps half-finished drafts out of a changelog. Passing `draft: true` waives the gate entirely and lets you see unevidenced work in flight.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `since` | `string` | no (default: nearest reachable git tag, else `HEAD~50`) | Git ref to start from, for example `v0.30.0`. Validated before it reaches git. |
| `until` | `string` | no (default: `HEAD`) | Git ref to end at. Validated the same way. |
| `draft` | `boolean` | no (default: `false`) | Disable the quality gate: include artifacts without evidence and drafts with R_eff of zero. |
| `workspace` | `string` | no | Explicit workspace directory. When set, the server resolves `.forgeplan/` from this path instead of its startup CWD. PRD-078 / ADR-015. |

_Schema source: `crates/forgeplan-mcp/src/types.rs::ReleaseNotesParams`_

Both refs are checked before they are handed to `git`: an empty string, a leading `-`, a `..` range, a leading non-alphanumeric or any character outside `A-Za-z0-9 _ / . -` is refused. That is deliberately stricter than git's own ref rules, because a ref that starts with a dash would otherwise be parsed as a git CLI option.

## Returns

```json
{
  "_next_action": "1 entries. Paste under [Unreleased] in CHANGELOG.md.",
  "added": [
    {
      "commit": "2d7a6ee",
      "id": "PRD-001",
      "kind": "prd",
      "title": "Ship the payment gateway"
    }
  ],
  "changed": [],
  "draft": true,
  "fixed": [],
  "internal": [],
  "security": [],
  "since": "v0.1.0",
  "total": 1,
  "until": "HEAD"
}
```

Every section key is always present, empty or not, so a consumer never has to guess whether a section is missing or empty. Within an entry:

- `id`, `title`, `kind` come straight from the artifact record.
- `commit` is the short (7-character) SHA of the first commit in the range that touched the file. It is omitted when no commit could be resolved.
- `evid` is the id of a linked EvidencePack, and is only attached to non-evidence artifacts. Evidence entries never carry it.

When the gate filters everything, the payload is still valid and the hint tells you so:

```json
{
  "_next_action": "No artifacts changed between v0.1.0 and HEAD. Widen range or pass draft=true.",
  "total": 0,
  "since": "v0.1.0",
  "until": "HEAD",
  "draft": false
}
```

## Examples

Notes since the last release, gated:

```json
{ "since": "v0.30.0" }
```

Same range including unevidenced work:

```json
{ "since": "v0.30.0", "draft": true }
```

Everything merged since the last tag, letting the tool pick the tag:

```json
{}
```

## Notes

- The tool is read-only: it runs `git log`, reads the artifact store, and writes nothing. It does not update `CHANGELOG.md` for you.
- A bad ref surfaces as a tool error rather than an empty result. A leading-dash ref is refused by the validator, and a ref that does not exist in the repo comes back from git itself:

```text
release-notes failed: git log failed: fatal: bad revision 'v9.9.9..HEAD'
```

- There is no workspace trace in the response: this is a read path, so unlike the mutating tools it does not report `resolved_via` / `resolved_workspace`.

## CLI equivalent

- [`forgeplan release-notes`](/docs/cli/release-notes/) - the same walk with a `--output` switch between `markdown` (the default, directly pasteable), `md`, `text` and `json`.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_journal`](/docs/mcp/forgeplan_journal/) - the chronological decision timeline behind these entries
- [`forgeplan_drift`](/docs/mcp/forgeplan_drift/) - find decisions whose files changed after they were written
- [`forgeplan_progress`](/docs/mcp/forgeplan_progress/) - checkbox state per artifact, useful when a release note needs a completion figure
- [`forgeplan release-notes`](/docs/cli/release-notes/) - run it from the terminal while preparing a tag
