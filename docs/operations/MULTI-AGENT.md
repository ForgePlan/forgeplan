[English](MULTI-AGENT.md) · [Русский](MULTI-AGENT.ru.md)

# Multi-Agent Workflow (Forgeplan 0.38.0)

Forgeplan computes work plans and records advisory artifact claims. The orchestrator starts and supervises workers through its own agent runtime; `dispatch` does not spawn them. The original coordination tools shipped in v0.24.0; the CLI examples below use the 0.38.0 release.

## Why this exists

Before v0.24.0 the orchestrator (you or an AI agent) had to **manually** keep in mind:
- which of N agents is busy with what
- which PRDs/RFCs touch shared files (or merge-hell follows)
- who blocks whom via dependencies
- which skill set fits which agent (backend/frontend/api/…)

2–3 agents is manageable, 5 is not. Failure modes:
- **Double work** — two agents grab the same PRD
- **File conflict** — two agents edit the same crate → race / merge-hell
- **Serial wasted** — A waits on B while C could have gone in parallel
- **Forgotten blocker** — a PRD is activated while deps are still draft

The four coordination operations help an orchestrator avoid duplicate assignments. They do not enforce source-file ownership or replace a task tracker.

## The four MCP tools

### `forgeplan_dispatch` — the main one

**Purpose:** turn a list of artifacts + live claim set + dependency graph into a ready plan for N agents.

**Contract:**
```jsonc
{
  "name": "forgeplan_dispatch",
  "arguments": {
    "agents": 3,                                    // 1..=64
    "kind": "prd",                                  // optional: filter by kind
    "epic": "EPIC-005",                             // optional: filter by parent_epic
    "status": "draft",                              // default "draft", "any" = all
    "agent_skills": [["backend"], ["frontend"], []],// optional: per-agent skills
    "overlap_threshold": 0.3                        // default 0.3 Jaccard
  }
}
```

**Returns:**
```jsonc
{
  "buckets": [["PRD-901"], ["PRD-902"], ["PRD-903"]],  // one bucket per agent
  "serial_queue": [{"id": "PRD-905", "reason": "missing affected_files frontmatter"}],
  "reasoning": [                                          // NFR-005 — why each decision
    "PRD-901: assigned to agent 0 (no file conflict, skill match)",
    "PRD-905: serialized (conflicts with every bucket or no matching skill)"
  ],
  "candidate_count": 4,
  "claimed_count": 0,
  "blocked_count": 0,
  "skipped_parse_errors": 0,
  "agent_count": 3,
  "overlap_threshold": 0.3,
  "generated_at": "2026-04-19T15:44:56.219+00:00"
}
```

**What the algorithm considers** (in order):

1. **Claims** (`forgeplan_claim`) — already-claimed artifacts are excluded.
2. **Structural dependencies** — unresolved dependencies keep artifacts out of buckets and appear in `serial_queue` with blocker reasons.
3. **File overlap** — Jaccard similarity on the declared path strings. Pairs with overlap ≥ threshold (default 0.3) cannot share a bucket. Paths/globs are not expanded against the filesystem; different strings can refer to overlapping files. Source: frontmatter `affected_files`, otherwise the `## Affected Files` body section.
4. **Domain/skill match** — if `agent_skills` is provided, an artifact goes only to an agent with a matching skill. Skill mismatch → serial queue.
5. **Least-loaded-first greedy** — distributes work evenly; avoids dumping everything onto agent 0.

**Read-only:** does not take `workspace_lock` or reserve assignments. Re-read claims before handing out work. **Important limitation:** conflicts are checked within each bucket, not across buckets. The orchestrator must check all concurrent workers' actual file sets before starting them.

### `forgeplan_claim` — "I am taking this artifact"

```jsonc
{
  "name": "forgeplan_claim",
  "arguments": {
    "id": "PRD-901",
    "agent": "worker-1",          // optional: default = MCP clientInfo name/version
    "ttl_minutes": 30,            // default 30, min 1, max 1440 (24h)
    "note": "implementing FR-003" // optional
  }
}
```

Writes `.forgeplan/claims/PRD-901.yaml` (gitignored). If already held by a different agent — **refuses** with the holder's `agent_id` and `expires_at`. Same-agent calls **renew** the TTL. Expired claims are **transparently overwritten** (AC-3).

**Atomic write** via tempfile + rename — SIGKILL mid-write cannot leave a corrupt YAML.

**A claim is a soft signal, not a write lock.** Other artifact tools do not refuse writes because another agent holds a claim. The separate workspace lock serializes participating CLI/MCP mutations; neither mechanism locks source files, git branches, or a remote clone. Claims are local to the selected `.forgeplan/` directory, not shared automatically across worktrees.

### `forgeplan_release` — "done"

```jsonc
{
  "name": "forgeplan_release",
  "arguments": {
    "id": "PRD-901",
    "agent": "worker-1",  // required if force=false
    "force": false         // true — orchestrator override for a crashed agent
  }
}
```

Without `force`: only the holding agent may release (prevents accidental clobber). Missing claim → no-op (idempotent).

### `forgeplan_claims` — "who is doing what right now"

```jsonc
{
  "name": "forgeplan_claims",
  "arguments": { "active": true }
}
```

Returns:
```jsonc
{
  "count": 2,
  "skipped": 0,  // malformed YAML files — check server logs
  "claims": [
    { "id": "PRD-901", "agent_id": "worker-1", "expires_at": "...", "note": "..." },
    { "id": "PRD-902", "agent_id": "worker-2", "expires_at": "...", "note": null }
  ]
}
```

Sorted by `expires_at` ASC (earliest-expiring first). **Read-only** — does not take the lock.

## Typical flow

### Orchestrator dispatches work to 3 agents

Run from the intended workspace. These CLI commands are real 0.38.0 syntax; IDs are illustrative:

```bash
forgeplan dispatch --agents 3 --epic EPIC-005 --json
forgeplan claims --json
# Review cross-bucket file overlap, then assign exact files to each worker.
forgeplan claim PRD-901 --agent worker-1 --ttl-minutes 30
# The orchestrator starts worker-1 through its own runtime.
# Renew before expiry with the same command and identity.
forgeplan claim PRD-901 --agent worker-1 --ttl-minutes 30
# After work is complete:
forgeplan release PRD-901 --agent worker-1
forgeplan dispatch --agents 3 --json
```

Use the **same explicit `--agent`** for claim, renewal and release. The 0.38.0 implementation defaults differ: `claim` uses `cli-0.38.0`, while `release` uses `cli/0.38.0` (the help text does not expose this mismatch). Always pass a simple identity such as `worker-1`; explicit CLI claim validation rejects `/` and `\` even though MCP `clientInfo` identities can use `name/version`. `--agent` identifies the claim holder; it does not select or start a model.

### An agent crashed — orchestrator reaps the claim

After confirming the holder has stopped:

```bash
forgeplan release PRD-902 --force
forgeplan dispatch --agents 3 --json
```

Alternatively, wait until the recorded `expires_at`. Do not use `--force` to fix an identity mismatch on your own release.

## Current boundaries (0.38.0)

- **CLI parity:** `dispatch`, `claim`, `release`, and `claims` are available. Per-agent `agent_skills` remains MCP-only.
- **No persistent agent profiles:** skills are supplied to each dispatch call, not loaded from `agents/<id>.yaml`.
- **Stdio MCP:** `forgeplan serve` exposes a connection's `clientInfo`; claims are not authenticated access-control grants.
- **Cross-bucket overlap:** the limitation described above remains; a returned plan is not proof that concurrent edits are conflict-free.
- **Separate roles:** Forgeplan holds decisions, evidence and artifact dependencies. A configured tracker holds tasks, assignees and deadlines. Tracker pickup is not `forgeplan claim`; closing a task does not activate an artifact.

MCP coordination calls accept `workspace` for a worktree separate from the server's launch directory. Pass the intended absolute worktree root consistently; a CLI `cwd` change does not redirect an already-running MCP server.

## Frontmatter fields for better dispatch

Two fields the dispatcher reads (both **optional**, but dramatically improve scheduling):

```yaml
---
id: PRD-042
title: Auth rewrite
kind: prd
status: draft
depth: standard
# ← new for dispatch:
affected_files:
  - crates/auth/src/session.rs
  - crates/api/src/auth.rs
domain: backend   # frontend | backend | api | infra | docs | testing | general
---
```

- **Without `affected_files`:** the dispatcher applies the R-2 safety bias and sends the artifact to the serial queue (treat as shared ground).
- **With `affected_files`:** the dispatcher computes Jaccard overlap against other candidates and decides if they are parallel-compatible.
- **With `domain`:** matching uses ASCII domain names and case-insensitive skill comparison. Invalid domain characters are normalized to no domain, not rejected as an artifact write. An empty agent skill list accepts any domain.

**Fallback for legacy artifacts** (no FM key): the dispatcher reads the `## Affected Files` markdown section in the body — backward-compatible.

## Identity stamping (Inc 2)

**Best-effort** stamping on MCP `new` and `update`, when the server has captured valid `clientInfo`:

```yaml
# In .forgeplan/prds/PRD-042-title.md after forgeplan_update:
---
...
last_modified_by: claude-code/1.0
last_modified_at: 2026-04-19T15:44:56+00:00
---
```

Used for:
- Retro-audit ("who touched this last?")
- Activity log (`.forgeplan/logs/tools-YYYY-MM-DD.jsonl` — every MCP invocation carries `client_info`)
- `forgeplan_get` `_next_action` hint (when a claim and identity are known, the "held by ..." hint appears automatically)

**Unicode protection:** control characters, bidi overrides, ZWJ, path separators are **rejected** in `AgentIdentity::new` — invisible characters cannot leak into markdown via `clientInfo`.

## Input bounds (security)

| Param | Cap | Reason |
|---|---|---|
| `agents` | 1..=64 | PRD target is 2–5; unbounded → OOM (CWE-770) |
| `agent_skills[i].length` | ≤ 32 | O(N²) Jaccard hot path |
| `affected_files.length` | ≤ 512 | same |
| `affected_files[i].length` | ≤ 512 bytes | same |
| Claim file size | ≤ 64 KB | billion-laughs, R1 parity |
| Claim TTL | 1..=1440 min (24h) | shorter churns, longer risks stuck agent |

Bounds are operation-specific: dispatch rejects invalid agent counts/thresholds at the boundary, while affected-file parsing truncates the list to 512 entries and drops overlong paths. Do not treat these limits as a file-ownership check.

## Further reading

- **PRD-057** (`.forgeplan/prds/PRD-057-*.md`) — original requirements and Growth Vision, not a statement that every roadmap item shipped
- **EVID-077** (`.forgeplan/evidence/EVID-077-*.md`) — historical v0.24.0 verification (R_eff=1.00, CL3), not a fresh 0.38.0 test result
- **CHANGELOG.md** `[0.24.0]` — historical release notes
- [AGENT-HOOKS.md](AGENT-HOOKS.md) — public hook registration, advisory checks and host boundaries
- [UNIFIED-WORKFLOW.md](../methodology/UNIFIED-WORKFLOW.md) — how Forgeplan × Orchestra × Hindsight fit together

## Release sources

- [Core 0.38.0 claims](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/claim/mod.rs#L1-L10) — advisory intent versus workspace locking.
- [Dispatcher and overlap checks](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/dispatch/mod.rs#L383-L508) and [CLI parity](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/commands/dispatch.rs#L114-L185).
- [Public task-runner role](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-orchestra/agents/task-runner.md#L184-L218) — tracker work is separate from source changes and Forgeplan mutations.
- [CLI claim default](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/commands/claim.rs#L17-L28) versus [release default](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/commands/release.rs#L13-L15) — why explicit matching identities are necessary.
