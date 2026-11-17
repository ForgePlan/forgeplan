---
title: forgeplan_release_notes
description: "Auto-generate Keep-a-Changelog-shaped release notes from artifacts that changed between two git refs. Walks `git log` over `.forgeplan/{prds, problems, evidence, rfcs, adrs, specs, epics, solutions}/`, classifies each touched artifact (PRD→Added, PROB→Fixed, EVID-on-security→Security, RFC/ADR→Changed) and emits the structured payload. Quality gate (default): only artifacts with status==active or r_eff_score > 0 are included. Pass `draft: true` to waive the gate. `since` defaults to the latest git tag, `until` defaults to HEAD. Example: `{since: \"v0.30.0\", draft: false}`."
---

Auto-generate Keep-a-Changelog-shaped release notes from artifacts that changed between two git refs. Walks `git log` over `.forgeplan/{prds, problems, evidence, rfcs, adrs, specs, epics, solutions}/`, classifies each touched artifact (PRD→Added, PROB→Fixed, EVID-on-security→Security, RFC/ADR→Changed) and emits the structured payload. Quality gate (default): only artifacts with status==active or r_eff_score > 0 are included. Pass `draft: true` to waive the gate. `since` defaults to the latest git tag, `until` defaults to HEAD. Example: `{since: "v0.30.0", draft: false}`.

**Category**: Other

The tool selects touched artifact files from git history, then loads their **current store records** and groups them into Keep-a-Changelog sections. It does not reconstruct artifact content at `until`, verify that work shipped, or write `CHANGELOG.md`. Review the resulting titles and evidence before using the entries in a release.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## What it does

1. **Resolves the range.** `since` is validated and used as given; without it the tool takes the nearest reachable tag (`git describe --tags --abbrev=0`) and falls back to `HEAD~50` when the repo has no tags at all. `until` defaults to `HEAD`.
2. **Walks added/modified files.** `git log --name-only --diff-filter=AM` covers only `prds`, `problems`, `evidence`, `rfcs`, `adrs`, `specs`, `epics`, `solutions`. Other kind directories are not scanned, and deleted/rename-only entries are not selected by that filter.
3. **Resolves touched filenames** as canonical IDs or pre-merge slugs and loads the current record. Unresolved or no-longer-present records are skipped. A current renamed record is included if its old candidate still resolves.
4. **Applies the quality gate** (see below) and classifies what survives.
5. **Formats the payload** as a structured object with one key per section, plus `since`, `until`, `draft` and `total`.

Git discovery is worktree-aware: if `.git/` is not next to `.forgeplan/`, the tool walks up (bounded by the nearest `.forgeplan/` ancestor or eight hops) and one level down to find it, rejecting symlinked candidates.

## Categories

| Artifact | Section | Notes |
|---|---|---|
| `prd` | `added` | kind classification, not proof of shipment |
| `problem` | `fixed` | kind classification, not a closed-bug check |
| `evidence` linked to a `security` tag | `security` | the evidence itself or any one-hop neighbour carries the tag, case-insensitively |
| `evidence` without a security tag/link | `internal` | included in either mode if it passes the applicable quality gate |
| `rfc`, `adr`, `spec`, `epic`, `solution` | `changed` | architectural and umbrella changes |
| `note`, `memory`, `refresh` | dropped | never emitted, regardless of ref range |

"Linked to security" is a tag check, not a kind check: the artifact carries the `security` tag, or any artifact it links to or is linked from does.

## Quality gate

An artifact has **signal** when its stored status is `active` **or** its stored `r_eff_score` is above zero. This is not a new recursive score or validation run. A positive-score draft can be included; `active` alone also suffices. `draft: true` waives this gate entirely, but does not include excluded kinds, untracked/uncommitted changes or files outside the git range.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `draft` | `boolean` | no | Disable the quality gate (include active artifacts without evidence). Default: false. Schema default: `false`. |
| `since` | `string \| null` | no | Git ref to start from (e.g. `v0.30.0`). Default: latest tag. |
| `until` | `string \| null` | no | Git ref to end at. Default: HEAD. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP text content. Values below are illustrative.

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
- `commit` is a 7-character prefix of the lexicographically smallest touching SHA collected for that filename. The handler does **not** choose the chronologically first or latest commit. It is omitted if no SHA was resolved.
- `evid` is the first incoming relation source whose ID has an `EVID-` prefix, regardless of relation type. It is omitted for evidence artifacts and when none is found; it is not proof that the referenced pack is valid or current.

When the gate filters everything, the payload is still valid and the hint tells you so:

```json
{
  "_next_action": "No artifacts changed between v0.1.0 and HEAD. Widen range or pass draft=true.",
  "added": [],
  "fixed": [],
  "security": [],
  "changed": [],
  "internal": [],
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
- Invalid refs surface as plain-text tool errors (`isError: true`), not empty results. A leading-dash ref is refused by validation; nonexistent refs fail in git. Illustrative failure:

```text
release-notes failed: git log failed: fatal: bad revision 'v9.9.9..HEAD'
```

- This handler emits no `resolved_via` / `resolved_workspace` trace. Workspace-resolution failures are tool errors. This is this handler's contract, not a rule that every read tool omits traces.

## CLI equivalent

- [`forgeplan release-notes`](/docs/cli/release-notes/) - the same walk with a `--output` switch between `markdown` (the default, directly pasteable), `md`, `text` and `json`.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_journal`](/docs/mcp/forgeplan_journal/) - the chronological decision timeline behind these entries
- [`forgeplan_drift`](/docs/mcp/forgeplan_drift/) - find decisions whose files changed after they were written
- [`forgeplan_progress`](/docs/mcp/forgeplan_progress/) - checkbox state per artifact, useful when a release note needs a completion figure
- [`forgeplan release-notes`](/docs/cli/release-notes/) - run it from the terminal while preparing a tag
