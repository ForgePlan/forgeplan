[English](MULTI-AGENT.md) · [Русский](MULTI-AGENT.ru.md)

# Multi-Agent Workflow (v0.24.0+)

Since **v0.24.0** (PRD-057), Forgeplan can **dispatch work** between 2–5 sub-agents sharing a single workspace. One MCP call returns a ready-to-execute plan: who works on what, what runs in parallel, what waits in the serial queue, and why.

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

v0.24.0 closes this through four MCP tools.

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
  "serial_queue": ["PRD-905"],                           // waits their turn
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
2. **Structural dependencies** — blocked artifacts (via `graph::topological::kahn_sort`, the same Kahn sort used by `forgeplan_blocked`) are excluded with explanation.
3. **File overlap** — Jaccard similarity on `affected_files`. Pairs with overlap ≥ threshold (default 0.3) are treated as conflicting. `affected_files` source: frontmatter key if present; otherwise the `## Affected Files` markdown section (fallback for legacy artifacts).
4. **Domain/skill match** — if `agent_skills` is provided, an artifact goes only to an agent with a matching skill. Skill mismatch → serial queue.
5. **Least-loaded-first greedy** — distributes work evenly; avoids dumping everything onto agent 0.

**Read-only:** does not mutate the workspace, does not take `workspace_lock`. Safe to poll at 1 Hz — does not serialize writers.

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

```
1. orchestrator → forgeplan_dispatch --agents 3 --epic EPIC-005
   ← { buckets: [[PRD-A], [PRD-B], [PRD-C]], serial: [PRD-D], ... }

2. orchestrator → worker-1 "work on PRD-A"
   orchestrator → worker-2 "work on PRD-B"
   orchestrator → worker-3 "work on PRD-C"

3. worker-1 → forgeplan_claim PRD-A --ttl 30
   worker-2 → forgeplan_claim PRD-B --ttl 30
   worker-3 → forgeplan_claim PRD-C --ttl 30

4. Agents work in parallel. Every ~N minutes:
   worker-1 → forgeplan_claim PRD-A (same agent, renews TTL)

5. worker-1 done → forgeplan_release PRD-A
   orchestrator → forgeplan_dispatch --agents 3 (re-plan — PRD-D may be parallelizable now)
```

### An agent crashed — orchestrator reaps the claim

```
worker-2 crashed; claim on PRD-B still valid for 20 more minutes.
orchestrator → forgeplan_release PRD-B --force
orchestrator → forgeplan_dispatch --agents 3 (PRD-B available again)
```

Alternative: just wait for TTL expiry — the claim will self-expire after 30 minutes.

## What v0.24.0 does NOT ship

(Deferred to v0.25+ per PRD-057 Growth Vision)

- **CLI parity** — no `forgeplan dispatch/claim/release/claims` CLI commands; MCP only. To use from CLI, pipe to `forgeplan serve` stdio MCP.
- **`agents/<id>.yaml` profiles** — skills are passed per-call, not persisted. Roadmapped for v0.27.
- **HTTP/SSE transport** — identity capture works on stdio only (one client per connection). Multi-connection HTTP needs per-request identity extraction.
- **Inter-bucket overlap check** — the dispatcher checks file conflicts within a bucket but not between agents. Two overlapping PRDs may land in different buckets (they will merge-conflict on the mainline — user-level mitigation: don't hand them out together). Tracked as a v0.25 follow-up.
- **Richer claim context** — Claim stores only `id + agent + ttl + note`. Structured fields ("which FRs I am working on") are a v0.25 item.

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
  - crates/auth/src/**
  - crates/api/src/auth/
domain: backend   # frontend | backend | api | infra | docs | testing | general
---
```

- **Without `affected_files`:** the dispatcher applies the R-2 safety bias and sends the artifact to the serial queue (treat as shared ground).
- **With `affected_files`:** the dispatcher computes Jaccard overlap against other candidates and decides if they are parallel-compatible.
- **With `domain`:** when the orchestrator passes `agent_skills`, the artifact routes to the matching agent. `domain` is validated against ASCII `[a-z0-9_-]` — non-ASCII (Cyrillic, RTL, ZWJ) is **rejected** (security CWE-176).

**Fallback for legacy artifacts** (no FM key): the dispatcher reads the `## Affected Files` markdown section in the body — backward-compatible.

## Identity stamping (Inc 2)

**Automatic** on every MCP write tool, provided the client passed `clientInfo` during `initialize`:

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

Exceeding the cap → error at the MCP boundary with a clear message.

## Further reading

- **PRD-057** (`.forgeplan/prds/PRD-057-*.md`) — full FR/AC/NFR + Growth Vision roadmap
- **EVID-077** (`.forgeplan/evidence/EVID-077-*.md`) — what was tested and how (R_eff=1.00, CL3)
- **CHANGELOG.md** `[0.24.0]` — full release notes
- [AGENT-HOOKS.md](AGENT-HOOKS.md) — how Forgeplan hooks (forge-safety, pre-commit-fmt) behave in a multi-agent setup
- [UNIFIED-WORKFLOW.md](../methodology/UNIFIED-WORKFLOW.md) — how Forgeplan × Orchestra × Hindsight fit together
