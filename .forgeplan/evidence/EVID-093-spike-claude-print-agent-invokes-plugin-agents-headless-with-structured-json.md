---
depth: tactical
id: EVID-093
kind: evidence
links:
- target: ADR-011
  relation: informs
status: active
title: 'Spike: claude --print --agent invokes plugin agents headless with structured JSON'
---

# EVID-093: Spike — `claude --print --agent` validates Plugin/Agent dispatcher mechanism

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-04-30 |
| Valid Until | 2027-04-30 |
| Target | ADR-011 (Plugin/Agent dispatchers invoke claude --print directly) |

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement

## Measurement

Live invocation of `claude` CLI from this Forgeplan workspace (host: macOS 26.0,
`claude` from `~/.local/bin/claude`, plugin `c4-architecture` v1.0.0 installed
through `claude-code-workflows` marketplace). Five tests covering full Plugin
dispatcher contract.

### Test 1 — agent reachability

```
claude --print --agent c4-code "Print just the words: agent reached"
```

Result: `agent reached` returned. Exit 0.

### Test 2-3 — argument-parsing investigation (negative)

Prompt as positional argument AFTER `--allowedTools <tools...>` variadic flag:
process exits 1 with stderr `Input must be provided either through stdin or
as a prompt argument`. Cause: variadic `<tools...>` consumed positional prompt.

Mitigation documented in ADR-011: pass prompt via stdin pipe.

### Test 4 — structured invocation with stdin prompt

```
echo "List the .rs files in crates/forgeplan-core/src/scan/ ..." | \
  claude --print --agent c4-code \
    --output-format json \
    --max-budget-usd 0.50 \
    --allowedTools "Read" "Glob" "Grep"
```

Result:
- Exit 0
- Duration: 10s
- Cost: $0.34
- num_turns: 2
- `.result` text contained accurate file list (5 .rs files in scan/)
- JSON output had 17 fields including `total_cost_usd`, `duration_ms`,
  `is_error`, `permission_denials`, `session_id`

### Test 5 — produces_at flow with Write tool

```
echo "Analyze crates/forgeplan-core/src/scan/detect.rs ... Write to <path>" | \
  claude --print --agent c4-code \
    --output-format json \
    --max-budget-usd 0.50 \
    --allowedTools "Read" "Glob" "Grep" "Write" \
    --add-dir .local/spike-claude-print/output-files/
```

Result:
- File `detect-summary.md` created (855 bytes, qualitatively excellent —
  accurate analysis of detect.rs including PROB-047 mitigation note, REF-
  prefix logic, frontmatter precedence)
- Exit 1 BUT file was written before halt
- Cost: $0.52 (exceeded `--max-budget-usd 0.50` cap → halt)
- num_turns: 4
- `permission_denials`: empty array

Interpretation: budget cap is a **soft halt** that preserves partial output.
Dispatcher must:
1. Parse JSON output before deciding success/failure
2. Check `total_cost_usd >= max_budget_usd` separately from `is_error`
3. Verify `produces_at` file existence as primary success signal

## Result

**`claude --print` is sufficient as Plugin/Agent dispatcher invoker.** All
required capabilities — agent resolution by name, tool permissions, budget
enforcement, structured output, file writes via `--add-dir` — work end-to-end
on a fresh Forgeplan repo with installed plugins from real marketplace.

No need to:
- Wait for Anthropic to ship `claude-code-plugin` binary
- Bundle `anthropic-sdk-rust` direct API
- Write custom plugin manifest parser
- Maintain a separate `task-tool` shim

Existing Claude Code login session is reused automatically — no
`ANTHROPIC_API_KEY` required when user is already authenticated.

### Implications for Forgeplan dispatchers

Argv shape for `PluginDispatcher.dispatch()` after ADR-011 implementation:

```
claude
  --print
  --agent <step.delegate_to::Plugin::target>
  --output-format json
  --max-budget-usd <step.budget_usd | 1.00>
  --allowedTools <T1> <T2> ...      # variadic, separate Vec entries
  --add-dir <dirname(produces_at)>  # if produces_at present
```

stdin pipe carries the assembled prompt:
```
<step.input.task>

Write output to `<step.produces_at>` using the Write tool.
```

`AgentDispatcher` is symmetric — only difference is
`step.delegate_to::Agent::name` vs `Plugin::target` for the `--agent` value.

## Interpretation

The hypothesis (Plugin/Agent integration must wait for upstream binaries) was
**falsified** by direct empirical investigation. The actual primitive needed
already exists, is stable, and is universally available in Claude Code user
environment.

This closes the largest open item in PRD-072 / EPIC-007 backlog. Implementation
effort drops from estimated days (writing shim from scratch) to ~3-4 hours
(rewrite two dispatcher methods to invoke `claude` instead of phantom binaries).

## Congruence Level Justification

**CL3 (same-context measurement)**. Tests run on the actual target environment
(this Forgeplan workspace, real installed plugin from production marketplace,
real `claude` CLI). Outputs measured directly — exit codes, JSON structure,
costs, file writes. Not a simulation, not a model, not extrapolation. The
exact mechanism that future `PluginDispatcher::dispatch()` will execute was
exercised end-to-end with all required capabilities engaged simultaneously.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| ADR-011 | informs (this evidence supports the architectural decision) |
| PRD-072 | informs (closes Plugin/Agent dispatcher real-binary deferred item) |
| ADR-010 | informs (refined by ADR-011) |

## Source artefacts

- `.local/spike-claude-print/findings.md` — full investigation log
- `.local/spike-claude-print/output3.json` — Test 4 raw output
- `.local/spike-claude-print/test5.json` — Test 5 raw output (includes budget cap)
- `.local/spike-claude-print/output-files/detect-summary.md` — qualitative output sample
- `.local/spike-claude-print/stderr*.log`, `test5.log` — debug capture


