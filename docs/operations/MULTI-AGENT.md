[English](MULTI-AGENT.md) · [Русский](MULTI-AGENT.ru.md)

# Multi-Agent Workflow (v0.24.0+)

Since **v0.24.0** (PRD-057), Forgeplan provides advisory artifact dispatch and claims. The typical target was 2–5 agents; the current core bound is 1–64. The returned plan is not a file lock or a guarantee of conflict-free execution: inter-bucket file overlap is not checked.

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

These tools help coordinate the failure modes; they do not eliminate them automatically.

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
  "serial_queue": [{"id": "PRD-905", "reason": "no matching skill"}],
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

**Read-only:** does not mutate the workspace or take `workspace_lock`. This is not a 1 Hz performance/SLA guarantee; re-plan when needed rather than polling by default.

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

**Atomic visibility** uses tempfile + rename. This is not a power-loss durability guarantee. Claims are advisory markers, not access-control locks or an enforced one-claim-per-agent limit. Ordinary reads omit expired claims without deleting their files. Explicit claim-agent input is capped at 64 bytes.

### `forgeplan_release` — "done"

```jsonc
{
  "name": "forgeplan_release",
  "arguments": {
    "id": "PRD-901",
    "agent": "worker-1",  // optional in schema; defaults to MCP identity, which must match holder
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

Illustrative CLI coordination (replace sample IDs with actual IDs):

```text
1. orchestrator → forgeplan dispatch --agents 3 --epic EPIC-005
   Inspect buckets, serial_queue objects and file ownership before delegation.
2. worker-1 → forgeplan claim PRD-A --agent worker-1 --ttl-minutes 30
   worker-2 → forgeplan claim PRD-B --agent worker-2 --ttl-minutes 30
   worker-3 → forgeplan claim PRD-C --agent worker-3 --ttl-minutes 30
3. worker-1 renews before expiry with the same claim command and holder.
4. worker-1 finishes → forgeplan release PRD-A --agent worker-1
5. orchestrator → forgeplan dispatch --agents 3
```

### An agent crashed — orchestrator reaps the claim

```text
worker-2 crashed; a live claim remains on PRD-B.
orchestrator → forgeplan release PRD-B --force
orchestrator → forgeplan dispatch --agents 3
```

Alternatively wait until that claim's actual `expires_at`; expiry releases the
advisory reservation, not the crashed worker's processes or file writes.

## What v0.24.0 does NOT ship

Historical v0.24.0 inventory, not a statement that all items remain absent:

- **CLI parity** was absent in v0.24.0; the current selected executable includes `forgeplan dispatch/claim/release/claims`. CLI claim uses `--ttl-minutes`; its explicit agent validator rejects slash, unlike canonical MCP `name/version`.
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
- **With `affected_files`:** Jaccard checks apply within a bucket, not across agents. Glob-like strings do not prove that expanded file sets are disjoint.
- **With `domain`:** when the orchestrator passes `agent_skills`, the artifact routes to the matching agent. `domain` is validated against ASCII `[a-z0-9_-]` — non-ASCII (Cyrillic, RTL, ZWJ) is **rejected** (security CWE-176).

**Fallback for legacy artifacts** (no FM key): the dispatcher reads the `## Affected Files` markdown section in the body — backward-compatible.

## Identity stamping (Inc 2)

**Best-effort on wired new/update handlers**, if `initialize` supplied a valid `clientInfo`; stamping failure is logged rather than failing the artifact write. It is not implemented on every MCP write tool:

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
