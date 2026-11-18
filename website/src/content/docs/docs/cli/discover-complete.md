---
title: forgeplan discover complete
description: "Mark a discovery session completed and print a health-check hint."
---

`forgeplan discover complete <SESSION_ID>` marks the saved discovery session completed. The CLI prints its finding count and completion time, then suggests `forgeplan health`. It does not group findings, deduplicate them, assign artifact depth or emit `forgeplan new` proposals.

Review findings and choose any follow-on artifacts separately.

## When to use

- **After the agent has finished the agreed source review** — inspect `discover show`; finding counts alone do not certify adequate coverage.
- **At the end of an onboarding sprint** - crystalize everything learned into artifacts.
- **When stopping after a partial survey** — completion records the session's state; it does not certify adequate coverage.

## When NOT to use

- While the agent is still actively submitting findings - wait for it to settle.
- To roll back findings — completion does not delete findings or undo artifacts created during discovery.
- On an already-completed session - idempotent but pointless.

## Usage

```text
forgeplan discover complete <SESSION_ID>
```

## Arguments

```text
  <SESSION_ID>   Discovery session identifier (from discover list / start)
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

```bash
# Replace SESSION_ID with the ID returned by start/list
forgeplan discover complete SESSION_ID

# Full onboarding loop
forgeplan init -y
forgeplan discover start "Example project"
# Agent reads sources and reports findings through MCP
forgeplan discover show SESSION_ID
forgeplan discover complete SESSION_ID
# Separate operator decisions, not automatically emitted proposals:
forgeplan new problem "Auth drift between docs and code"
forgeplan new prd  "Unified config loader"
```

## Reviewing findings after completion

The following is a suggested human/agent analysis, not behavior of this CLI command:

1. Group findings by category and source.
2. Check whether reports about the same module duplicate or contradict each other.
3. Decide which findings justify Problems, ADRs, PRDs or Notes.
4. Choose depth using the applicable routing policy.
5. Create the accepted artifacts explicitly.

CLI completion does not perform these steps or create follow-on artifacts.

## How it fits

`discover complete` is the **hand-off point** from discovery to the normal Forgeplan lifecycle:

```
discover start → agent findings → discover complete → manual review
                                                    ↓
                          forgeplan new problem/prd/adr/note ...
                                                    ↓
                          validate → reason → code/evidence → activate
```

From here, the project flows through the standard Shape → Validate → ADI → Code → Evidence → Activate cycle like any greenfield work.

## See also

- [`forgeplan discover`](/docs/cli/discover/) - parent command + pipeline overview
- [`forgeplan discover show`](/docs/cli/discover-show/) - coverage check before completing
- [`forgeplan discover list`](/docs/cli/discover-list/) - session index
- [`forgeplan new`](/docs/cli/new/) - create the proposed artifacts
- [`forgeplan validate`](/docs/cli/validate/) - next step after new
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan discover complete --help`.

```text
Mark a discovery session as completed

Usage: forgeplan discover complete <SESSION_ID>

Arguments:
  <SESSION_ID>  Session ID to complete

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
