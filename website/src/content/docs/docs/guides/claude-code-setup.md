---
title: Claude Code Setup Guide
description: Configure Claude Code for maximum productivity with Forgeplan
---

## What is CLAUDE.md?

`CLAUDE.md` is Claude Code's project memory - a file at your repo root that tells Claude about your project, conventions, and workflows. Claude reads it at every session start.

## Recommended CLAUDE.md structure

Use this outline, adapting commands and policies to the repository. These
sections do not install hooks, MCP servers or memory by themselves:

```markdown
# CLAUDE.md

## Quick Start
- Dev environment setup
- Key commands
- Where to find things

## Methodology (Forgeplan)
- Route → Shape → Validate → Code → Evidence → Activate
- Depth calibration table
- Artifact creation flow

## Git Workflow
- Branching strategy (main ← dev ← feat/*)
- Commit format (conventional commits + Refs)
- PR pipeline: Code → Audit → Fix → Test → PR

## Enforcement Hooks
- Installed hook paths and their actual trigger/matcher
- What each hook checks, and any bypasses or unsupported operations
- Project test and formatting commands

## Memory (Hindsight)
- Explicit .hindsight.json: url, banks, defaultBank
- Start with memory_get_current_bank and memory_status
- Recall existing knowledge; enable writes separately when authorized

## Hard Requirements
- Language-specific rules
- Architecture constraints
- Testing standards
```

## Forgeplan Section

Add this to any project's CLAUDE.md to integrate Forgeplan:

```markdown
## Forgeplan

### Session start
forgeplan health   # blind spots, orphans - fix FIRST

### Before any task
forgeplan route "task description"   # determines depth

### Full cycle (Standard+)
1. forgeplan new prd "Title"         # create artifact
2. Fill MUST sections via forgeplan update --body @file
3. forgeplan validate PRD-XXX        # structural/content gates
4. forgeplan reason PRD-XXX --save   # LLM required; save ADI Note
5. Implement and test the accepted scope
6. Independent review + observed evidence; set verdict/congruence/type
7. forgeplan link EVID-XXX PRD-XXX --relation informs
8. forgeplan score PRD-XXX           # inspect weakest current evidence
9. forgeplan review PRD-XXX
10. forgeplan activate PRD-XXX       # main orchestrator, not the author

### Tactical depth
No planning artifact needed; still apply the project's code/review/test rules.
```

`reason` needs a configured, authenticated LLM provider; installing `/forge`
does not supply one. Core artifact operations and validation need no LLM.
The CLI's FPF corpus injection is requested with `--fpf`; the published
harness calls for FPF/ADI on Standard+ work. See
[Configuration](/docs/getting-started/configuration/) for provider setup.

### Choose a workflow and separate the roles

`forgeplan setup-skill` writes only the bundled Claude Code `/forge` skill.
The full marketplace bundle is described in
[Installation](/docs/getting-started/installation/#full-harness-bundle-claude-code).
For the public plugin workflows:

| Workflow | Context and entry condition |
|---|---|
| `/bmad` (`agents-bmad`) | A new product/service from idea to delivery; run `/bmad-init` before the first cycle |
| `/sparc` (`agents-sparc`) | One well-scoped feature in an existing active system; no separate SPARC initialization hook |
| `/tdd` (`agents-tdd`) | Implementation after an active PRD and active SPEC with `#### Scenario` blocks; run `/tdd-init` first |

Bootstrap's use of `agents-sparc:specification` to create the first PRD does
not mean that the SPARC pipeline is the greenfield workflow.

| Profile | Responsibility |
|---|---|
| A | Create draft artifacts; do not self-activate |
| B / B-gate | Independently review and record evidence / guardian readiness verdict |
| C | Read-only research |
| C-coder | Change source, not Forgeplan artifacts |
| D | Maintain existing metadata and links using lifecycle commands where required |

Keep the generator and verifier in separate contexts. The verifier's model
tier must not be below the generator's. `opus`/`sonnet`/`haiku` frontmatter
values are Claude Code bindings, not portable model guarantees. Other
runtimes need their own tier mapping and enforcement; Claude Code denylists
do not become server authorization. The main orchestrator activates after
review, while authors and guardian do not activate their own output.

## Enforcement Hooks

The public `fpl-skills` bundle includes plugin-level hooks and optional
project-local templates under `templates/hooks/`:

| Template | What it checks |
|---|---|
| `safety-hook.sh` | Selected destructive git/rm command patterns |
| `pre-pr-evidence-check.sh` | Linked evidence for artifact IDs found in branch/commit context before `gh pr create`; includes documented exceptions and a bypass |
| `forge-safety-hook.sh` | Selected destructive Forgeplan CLI operations |

`/smith-bootstrap` offers to copy these templates into an empty
`.claude/hooks/` with user approval and merge the settings. `setup-skill`
does not install them. They are pattern-based defense in depth, not a
general sandbox, a full test runner, or proof of readiness.

### Setting Up Hooks

After placing the published `safety-hook.sh` template at
`.claude/hooks/safety-hook.sh`, register it with a command-hook object. Merge
this entry into existing settings rather than replacing other hooks:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "bash .claude/hooks/safety-hook.sh"
          }
        ]
      }
    ]
  }
}
```

The `forgeplan setup` git hooks are different: `post-merge` and
`post-checkout` run `git-sync` to refresh the derived index. They neither
install these Claude hooks nor regenerate embeddings.

## MCP Server Configuration

From the project directory, prefer:

```bash
forgeplan mcp install --client claude --scope project
```

It writes the detected absolute binary path and preserves existing server
entries and `env`. For manual setup, ensure `forgeplan` is on the MCP
client's PATH:

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve"]
    }
  }
}
```

The published 0.38.0 stdio server exposes 73 tools. Restart the client and
confirm that `forgeplan_health` reads the intended workspace. Project-scoped
config does not itself pin the server's working directory. See
[MCP Setup](/docs/guides/mcp-setup/) for paths and workspace boundaries.

## Memory integration (Hindsight)

Hindsight is a separate optional plugin and service, not one of Forgeplan's
73 tools. Public `fpl-hsmem` 3.7.0 requires Hindsight API **>=0.10.2 and
<0.11.0** and an explicit project `.hindsight.json` declaring `url`, `banks`
and `defaultBank`. The selected bank must already exist on the server.
There is no implied URL or folder-derived bank.

Start with the plugin's read-only pilot policy and call
`memory_get_current_bank`, then `memory_status`. Use `memory_recall` to find
relevant facts. Manual retention, reflection, bootstrap and automatic hooks
are separate write/automation decisions, not prerequisites for Forgeplan.
`autoRecall` and `autoRetain` default off. BankGate's allowlist is client
safety, not server authorization. Keep credentials out of committed settings.

## Recommended Permissions

Use a narrow project-specific policy. This example permits inspection only;
it does not preapprove every `forgeplan`, package-manager or git operation:

```json
{
  "permissions": {
    "allow": [
      "Bash(forgeplan --version)",
      "Bash(forgeplan health)",
      "Bash(git status:*)",
      "Bash(git diff:*)",
      "Bash(git log:*)",
      "Read",
      "Glob",
      "Grep"
    ],
    "deny": [
      "Bash(git push --force*)",
      "Bash(rm -rf /*)",
      "Bash(cargo publish*)"
    ]
  }
}
```

## Multi-Project Setup

For monorepo or multi-project setup, each subdirectory can have its own CLAUDE.md:

```
project/
├── CLAUDE.md          ← root config (git, methodology)
├── packages/
│   ├── core/
│   │   └── CLAUDE.md  ← package-specific rules
│   └── web/
│       └── CLAUDE.md  ← frontend-specific rules
└── .claude/
    ├── hooks/         ← shared hooks
    └── settings.json  ← permissions
```

## Best Practices

1. **Keep CLAUDE.md under 500 lines** - Claude reads it every session. Too long = wasted context.
2. **Put details in docs/, not CLAUDE.md** - reference `docs/guides/X.md` for deep content.
3. **Update after decisions** - new convention? Add it to CLAUDE.md immediately.
4. **Use hooks for concrete checks** — document their matchers and limits; neither prompts nor pattern hooks are a universal security boundary.
5. **Forgeplan health first** - always start session with `forgeplan health` to catch blind spots.
