[English](AGENT-ENFORCEMENT.md) · [Русский](AGENT-ENFORCEMENT.ru.md)

# ForgePlan — Agent Enforcement Guide

Forgeplan **0.38.0** can validate artifacts and reject particular lifecycle operations. It cannot force an arbitrary agent to follow every project instruction. Instructions, skills, host hooks, core gates and CI are separate controls with different failure modes.

**Five layers, not five guarantees:**

| Layer | Control | Boundary |
|---|---|---|
| L1 | `CLAUDE.md` / `AGENTS.md` | Instructions the agent must follow; not a filesystem lock |
| L2 | Installed skills | Workflow instructions; invocation does not prove execution or verification |
| L3 | Host hooks | Only registered events and recognized payloads are checked |
| L4 | CLI/MCP validation and lifecycle | Enforced by the operation called, its configuration and override flags |
| L5 | Required CI checks | Blocks merge only if the repository's branch protection requires the check |

## L1: CLAUDE.md / AGENTS.md Instructions

### What to add to CLAUDE.md

Use a project policy rather than presenting file-count heuristics as built-in gates. This example requires human approval for architectural choices; that is a policy you adopt, not a universal Forgeplan permission check:

```markdown
## Forgeplan methodology

- Read project rules and relevant existing decisions before architectural work.
- Route the task before choosing artifacts. Use `forgeplan route "task" --level 0`
  for explicit offline keyword routing; review its recommendation against risk.
- For a non-trivial architectural choice, present alternatives, trade-offs and
  available evidence. Obtain the project's required human approval before recording it.
- Run the ADI cycle where the chosen workflow requires it: propose hypotheses,
  evaluate them logically, then check them against observable evidence.
- Create and update artifacts through Forgeplan CLI/MCP, not direct file edits.
- Validate drafts before activation; attach evidence describing what actually ran.
- Do not equate a generated document, passed schema validation or task status with
  implementation completion. Report unrun checks and unresolved findings.
```

Tactical work can need no artifact; Standard, Deep and Critical are risk/depth categories, not `<1`, `<5`, `>5` file thresholds. The selected workflow determines whether a PRD, Spec, RFC, ADR or Epic is appropriate. A PRD → RFC → ADR sequence is not itself the artifact lifecycle: artifact status transitions use dedicated commands such as `activate`, `supersede` and `deprecate`.

### What to add to AGENTS.md

```markdown
## Agent roles

| Role | Responsibility |
|---|---|
| Research | Read status, relevant artifacts and evidence; return findings |
| Routing / orchestration | Choose the workflow, assign exact files, start workers |
| Implementation | Change assigned source files according to the approved contract |
| Review / testing | Independently verify implementation; record actual results |
| Artifact recorder / gate | Perform allowed Forgeplan mutations and readiness checks |
| Tracker runner | Maintain tasks and checklists in the configured tracker |

The author of a result is not its independent verifier. Do not assign permissions
just because a role's name appears in a prompt; configure the host's tools too.
```

Public marketplace roles are narrower than an all-powerful orchestrator. `agents-pro:smith` routes methodology; `forgeplan-orchestra:task-runner` drives the configured tracker and delegates source changes and Forgeplan mutations. The tracker can be Orchestra, Jira, Linear, GitHub Issues or another configured system. Orchestra is not Forgeplan's universal control plane.

## L2: Skills & Slash Commands

### Skill: `/forge`

The CLI's `forgeplan setup-skill` installs the bundled `/forge` skill for Claude Code. It is not installation of the full marketplace, agent roster or host safety configuration.

For public marketplace workflows, add the marketplace and install the needed plugins through Claude Code:

```text
/plugin marketplace add ForgePlan/marketplace
/plugin install fpl-skills@ForgePlan-marketplace
/plugin install agents-pro@ForgePlan-marketplace
```

Choose a shipped entrypoint such as `/smith-plan` for routing a concrete task or `/forge-cycle` for the execution workflow. Those are marketplace instructions, not extra CLI commands. Do not assume that every skill installed in a developer's personal environment ships with Forgeplan.

### Proactive Trigger

The public `fpl-skills` prompt router provides recommendations only. It does not auto-start skills, grant consent, create artifacts or enforce an approval gate. Define any automatic execution policy explicitly in the project and host; see [Agent Hooks](AGENT-HOOKS.md).

## L3: Hooks (Pre/Post Automation)

Use the real published plugin manifests or a deliberately configured project hook. The public `forgeplan-workflow` manifest registers a Bash safety hook and a **Write-only, advisory** active-PRD warning. The `fpl-skills` manifest registers session helpers, a Bash safety hook, test/report hints and a prompt router.

These are not comprehensive coverage or methodology enforcement. In particular:

- There is no canonical `.forgeplan/decisions/*.md` directory. Artifacts live in kind-specific directories such as `adrs/`, `prds/`, `rfcs/`, `specs/` and `evidence/`.
- Do not invent flags such as `drift --files`, `drift --quiet`, `status --brief`, or a `.forgeplan/status` file. In 0.38.0, use `forgeplan drift --json`, `forgeplan status`, or `forgeplan health --compact` as appropriate.
- Do not suppress required checks with `2>/dev/null || true`.
- Pattern-based safety scripts do not block every dangerous command. Their input adapter and host registration matter; the public scripts' top-level `.command` parser does not read nested `.tool_input.command`.
- A local pre-commit formatter or evidence hook is not installed merely because the CLI or one marketplace plugin is installed.

For working event JSON and exact published script paths, use [AGENT-HOOKS.md](AGENT-HOOKS.md), not snippets based on undocumented environment variables.

## L4: MCP Server

### MCP Tools Design

Run `forgeplan serve` from the intended workspace. For Claude Code project `.mcp.json`:

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

There is no `serve --mcp` flag. The server exposes 73 tools in 0.38.0; a client-specific `tools` allowlist in this example would not be a server-side authorization policy.

### MCP Tool Definitions

Tool names below are bare MCP names. The host adds its own prefix. Inputs shown are illustrative JSON arguments:

| Tool | Input | Purpose |
|---|---|---|
| `forgeplan_status` | `{}` | Artifact counts by kind/status |
| `forgeplan_get` | `{"id":"ADR-007"}` | Read the decision and its body |
| `forgeplan_new` | `{"kind":"adr","title":"Session storage"}` | Create an artifact, not approve its decision |
| `forgeplan_validate` | `{"id":"PRD-001"}` | Validate required sections for that kind/depth |
| `forgeplan_score` | `{"id":"ADR-007"}` | Score linked current evidence |
| `forgeplan_drift` | `{}` | Workspace drift report; no `files` argument |
| `forgeplan_coverage` | `{}` | Decision coverage by code module |
| `forgeplan_guard` | `{"target_session_phase":"coding"}` | Check a methodology session transition |

There is **no `forgeplan_context` MCP tool in 0.38.0**. The CLI `forgeplan context ADR-007 --json` takes an artifact ID, not a source path or file list. Use coverage/search to locate decisions, then read the actual artifacts.

### CLAUDE.md with MCP

Specify the operations each role may call, not just when to call them. Require the worker to read the tool response, including errors and `_next_action`, before claiming completion. A hint is guidance, not permission to exceed the assignment.

Claims are advisory intent. `claim --agent worker-1` does not prevent another worker's `update`; `release` must use the same identity. Workspace write serialization is a separate mechanism. See [Multi-Agent Workflow](MULTI-AGENT.md).

Artifact phase markers (`shape/validate/adi/code/test/audit/evidence/done`) are advisory and distinct from methodology session phases (`idle/routing/shaping/coding/evidence/pr`). A phase advance does not run tests or activate an artifact. For lifecycle kinds, normal activation checks body length, stub content, evidence linkage and MUST validation; lightweight kinds skip those gates. `force` can bypass validation/methodology gates. Link presence is not independent proof of evidence quality or review sign-off: project acceptance still requires actual verification.

## L5: CI/CD Validation

### GitHub Action

The following is a **project workflow example**, not a bundled branch-protection setup. The runner must already have the pinned 0.38.0 binary, the tracked `.forgeplan/` projection and a valid derived index. For a fresh clone, initialize/rebuild the index according to the installation guide before these checks; do not use `scan-import` as index recovery.

```yaml
name: Forgeplan Compliance
on: [pull_request]
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      # Provision Forgeplan 0.38.0 and its workspace index before these steps.
      - name: Validate artifacts
        run: forgeplan validate --ci
      - name: Check project health
        run: forgeplan health --strict
      - name: Report drift
        run: forgeplan drift --json
```

`validate --ci` exits 1 on MUST failures. `health --strict` supplies the strict health exit signal. Empty workspaces and advisory-only phase signals can still exit 0. `drift --json` is a report, not a `--fail-on-drift` merge gate. Do not use nonexistent flags `validate --all --fail-on-incomplete`, `coverage --percent`, or `score --stale-count`.

Review the policy against real repository requirements and make the selected CI job required in branch protection. A successful YAML workflow alone does not prove semantic correctness, evidence quality or coverage of every modified module.

## Combined Approach (recommended)

### Adoption Order

1. Define project rules, human-approval boundaries and the tracker configuration.
2. Install only the skills, agents and host hooks needed for those rules.
3. Configure CLI/MCP workspace selection and role-specific tool grants.
4. Require CI checks for properties those checks actually enforce.
5. Before declaring completion, compare the acceptance criteria with independent verification evidence; update the tracker separately.

### Example Full CLAUDE.md Section

```markdown
## Forgeplan integration

- Read relevant decisions before editing and keep exact worker file ownership.
- Follow the chosen workflow; architectural approval is a project policy.
- Use Forgeplan CLI/MCP for artifacts, the configured tracker for task status.
- Validate drafts before activation. Lifecycle overrides require explicit authority.
- The implementation agent is not its independent reviewer.
- Report checks actually run, unresolved findings and checks not run.
```

## Release sources

- [Core CLI 0.38.0 command definitions](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/main.rs), [health exit policy](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/commands/health.rs#L24-L58).
- [Public workflow hooks](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-workflow/hooks/hooks.json), [fpl-skills hooks](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/fpl-skills/hooks/hooks.json).
- [Public tracker-runner separation](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-orchestra/agents/task-runner.md#L184-L218) and [marketplace installation](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/README.md#L42-L48).
