---
depth: standard
id: PRD-054
kind: prd
status: draft
title: Activity log — append-only JSONL log of MCP tool calls with query tool
---

# PRD-054: Activity log — append-only JSONL log of MCP tool calls with query tool

## Executive Summary

### Vision

Every MCP tool invocation is recorded in an append-only JSONL log under `.forgeplan/logs/tools-YYYY-MM-DD.jsonl`. An agent or human operator can later query "what did the agent do in the last hour/day/session?" via `forgeplan_activity`. This closes the visibility gap that made it impossible to reconstruct agent behaviour after a session ended.

### Problem

When Claude Code (or any MCP client) calls forgeplan tools — 45 of them, dozens of times per session — there is no persistent record of those calls anywhere on disk. Diagnostic logs from the rmcp server are emitted to stderr, which Claude Code aggregates in its UI but discards when the session ends or the MCP subprocess restarts. The `forgeplan_journal` tool shows chronologically-ordered decision artifacts (ADR, Note, Problem, Solution) but not the thousands of read-and-write tool calls that produced them. There is no way to answer the question "what did the agent do in the last hour?" after the session window closes.

Concretely, if the agent: creates a PRD that turns out wrong; activates an artifact prematurely before evidence lands; links evidence to the wrong target; spends several dollars of LLM tokens on a failed `forgeplan_reason` loop; or silently swallows an error because of a transient LanceDB issue — there is no forensic trail. Operators cannot diagnose why their workspace ended up in its current state, and agents resuming a session tomorrow have no memory of yesterday's attempts beyond whatever survived in committed artifacts.

**Impact**:
- **Operator**: cannot diagnose "why did my workspace end up in this state?" after the fact.
- **Agent**: cannot self-correct — if it re-enters workspace tomorrow, no memory of yesterday's attempts.
- **Security**: no audit trail for compliance scenarios (who read which artifact, who deleted what, when).
- **Cost**: no way to attribute LLM-token spend to specific tool-call sequences.

### Target Users

| Persona | Описание | Ключевая боль |
|---------|----------|---------------|
| Solo developer | Uses forgeplan via Claude Code for personal projects | "What did I do yesterday? Why is this PRD half-filled?" |
| AI agent | Claude Code / Cursor / Windsurf calling MCP tools autonomously | Cannot continue interrupted work without context |
| Reviewer / compliance | Audits agent behaviour in regulated contexts | No way to prove what agent did without reproducible log |

### Differentiators

- **Append-only by design** — no log tampering possible without filesystem access.
- **JSONL for grep/jq friendliness** — no binary log format, no DB query language to learn.
- **Zero config** — file rotation by date, no setup.
- **Agent-queryable** — `forgeplan_activity` returns structured JSON suitable for LLM context.

---

## Success Criteria

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | Every MCP tool call is logged | Coverage of logged calls | 0 of 45 tools | 45 of 45 tools | v0.21.0 release | Integration test replays 10 calls, asserts 10 log lines |
| SC-2 | Log write adds negligible latency | P95 overhead per call | unknown | < 2 ms per call | v0.21.0 release | Benchmark: 1000 no-op tool calls, measure overhead vs no-log build |
| SC-3 | Agent can query "last N minutes" | Query response under real load | no tool | < 100 ms for 10000-entry log | v0.21.0 release | `forgeplan_activity --since 1h` on synthetic 10k log |
| SC-4 | Log rotation prevents single file > 100 MB | File size bound | unbounded | rotated daily, one file per day | v0.21.0 release | 30-day synthetic load, check files/sizes |
| SC-5 | No PII or secrets in log by default | Content sanitization | unknown | zero secrets in log body | v0.21.0 release | Inject API-key-looking string into title, assert log does not contain it |

---

## Product Scope

### MVP (In-Scope)

- **File layout**: `.forgeplan/logs/tools-YYYY-MM-DD.jsonl` — one file per UTC day, append-only. No log directory → auto-create on first write.
- **Log entry schema** (per line, one JSON object):
  - `ts`: ISO-8601 UTC timestamp with millisecond precision
  - `tool`: MCP tool name (e.g. `"forgeplan_score"`)
  - `args_hash`: SHA-256 of canonical JSON-serialized args (12-char hex prefix) — lets operators correlate repeats without storing args content
  - `duration_ms`: integer wall-clock duration
  - `status`: `"ok"` | `"tool_err"` | `"rpc_err"`
  - `workspace`: absolute path to `.forgeplan/` (helps multi-workspace operators)
  - `client_info`: optional `{name, version}` from MCP `initialize` — identifies which agent called
- **Rotation**: file is opened in append mode, named by UTC date. New day → new file automatically.
- **Privacy**: args CONTENT is never logged — only `args_hash`. Prevents secret leakage (API keys in task descriptions, PII in titles, etc.). A separate opt-in flag `log_args: true` in `.forgeplan/config.yaml` can enable full args logging for debugging.
- **Two new MCP tools**:
  - `forgeplan_activity` — query log with filters (`--since duration`, `--tool name`, `--status ok|err`, `--limit N`)
  - `forgeplan_activity_stats` — aggregates per tool (call count, error rate, p50/p95 duration)
- **CLI parity**: `forgeplan activity` subcommand mirrors MCP tool.
- **Retention**: daily files kept forever by default (no auto-delete). Operator manually cleans old files.

### Out of Scope

- Undo mechanism (soft-delete, restore, undo_last) — will be **separate PRD-055** (Deep depth, 1 week).
- Centralized log shipping (Syslog, Datadog, CloudWatch) — out of scope for v1.
- Binary log format or log compression.
- Full-text indexed search of args content.
- Real-time streaming subscription (SSE, WebSocket).
- GDPR right-to-erasure — logs are append-only by design; operator responsibility if needed.

### Growth Vision

- Integration with `forgeplan_recent` / `forgeplan_undo_last` in PRD-055.
- Opt-in structured args logging for specific trusted tools.
- Aggregated dashboards (weekly report, cost attribution).
- Export to OpenTelemetry / SIEM systems.

---

## User Journeys

### Journey 1: Solo developer reconstructs yesterday's session

**Цель пользователя**: "Я вчера вечером что-то творил с forgeplan, сегодня workspace в странном состоянии. Что случилось?"

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | Open Claude Code, ask "what did you do yesterday in forgeplan?" | Agent calls `forgeplan_activity --since 24h` | Agent uses new tool |
| 2 | Agent parses response | Sees 47 calls: 12 new, 8 update, 5 activate, 22 read-only | Structured JSON |
| 3 | Agent reports summary | "Yesterday you created PRD-050..053, activated all four, then deprecated PRD-051. Here's the timeline." | Human-readable |
| 4 | User asks "why did I deprecate PRD-051?" | Agent calls `forgeplan_get PRD-051` + checks `forgeplan_journal` | No extra tool |

**Результат**: User reconstructs context in one tool call. Before: impossible.

### Journey 2: Agent attributes LLM-token spend

**Цель пользователя**: "Агент сжёг $3 на LLM за час. Куда?"

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan_activity_stats --since 1h --filter llm-only` | Per-tool stats | |
| 2 | System returns | `{forgeplan_reason: 8 calls, avg 4200ms, errors 2; forgeplan_decompose: 3 calls, avg 6800ms, errors 0}` | |
| 3 | User sees `forgeplan_reason` spent most time, 2 failed | Can dig: `forgeplan_activity --tool forgeplan_reason --status tool_err --since 1h` | |
| 4 | Identifies runaway loop | Can raise `rate_limit.reason_per_hour` in config | Uses PRD-062 rate-limit |

**Результат**: Cost attribution from log data, no external observability needed.

### Journey 3: Compliance audit

**Цель пользователя**: "Покажи все destructive operations (delete/supersede/deprecate) за неделю."

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan_activity --since 7d --tool forgeplan_delete,forgeplan_supersede,forgeplan_deprecate` | Filtered list | Multi-tool filter |
| 2 | Export to JSONL stream | Each line is audit-ready | grep/jq friendly |
| 3 | Review for anomalies | Any unexpected IDs? Unexpected agent client? | `client_info.name` field |

**Результат**: Audit trail without separate compliance tooling.

---

## Functional Requirements

- [ ] FR-001: MCP server can append one JSONL entry per tool invocation to the current day's log file (Journey 1/2/3, Must)
- [ ] FR-002: Agent can query the log via `forgeplan_activity` with at minimum a time-window filter (Journey 1, Must)
- [ ] FR-003: Agent can aggregate statistics via `forgeplan_activity_stats` grouped by tool name (Journey 2, Must)
- [ ] FR-004: Operator can prevent sensitive args content from being written to the log by default (All, Must)
- [ ] FR-005: Agent can filter activity log by tool name, status, and time window (Journey 3, Must)
- [ ] FR-006: System can append a log entry without exceeding 2 ms p95 overhead per call (SC-2, Must)
- [ ] FR-007: CLI user can invoke `forgeplan activity --since 1h` outside of MCP context for scripting (Should)
- [ ] FR-008: System can recover gracefully from a corrupted or truncated log line (Should)
- [ ] FR-009: Agent can receive `_next_action` hint from `forgeplan_activity` pointing at the next likely inspection step (Journey 2, Could)

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Performance | Log append latency | < 2 ms p95 | Under 100 tool calls/sec sustained | Benchmark `cargo bench --bench activity_log_write` |
| NFR-002 | Durability | Log writes survive crash | No entry lost if process killed after successful `fsync` | SIGKILL immediately after tool handler returns Ok | Chaos test: kill -9 mid-run, replay, assert log matches pre-kill state |
| NFR-003 | Correctness | Log is strictly append-only | Zero in-place edits, truncations, or reorderings | Across 10000 synthetic writes | Read-back check: bytes offset of line N stable across calls |
| NFR-004 | Security | No secret strings in log body by default | Regex scan for API-key-shaped tokens returns 0 matches | Default config, full smoke replay | Test: inject "sk-…64 chars…" into task description, grep log — 0 hits |
| NFR-005 | Portability | Works identically on macOS, Linux, Windows | Same log file format, same path resolution | Cross-platform CI | GitHub Actions matrix macos/ubuntu/windows passes integration test |
| NFR-006 | Scalability | Query 10000-entry log within 100 ms | `forgeplan_activity --since 24h` on 10k synthetic log | Steady-state | Benchmark with synthetic generator |

---

## Acceptance Criteria

### AC-1: Happy path — tool call is logged

```gherkin
Given a fresh workspace with no existing log file
When the agent calls forgeplan_health via MCP
Then .forgeplan/logs/tools-YYYY-MM-DD.jsonl exists
And the file contains exactly one JSONL line
And the line parses to a valid JSON object with required fields: ts, tool, args_hash, duration_ms, status, workspace
And the `tool` field equals "forgeplan_health"
And the `status` field equals "ok"
```

### AC-2: Args content is not logged by default

```gherkin
Given a workspace with default config (no log_args flag)
When the agent calls forgeplan_new kind=prd title="Secret key is sk-proj-ABC123..."
Then the log line for that call exists
And the line does NOT contain the substring "sk-proj-ABC123"
And the line's args_hash field is a 12-char hex prefix
And the line's tool field is "forgeplan_new"
```

### AC-3: Query by time window returns correct entries

```gherkin
Given a log containing 50 entries spanning 3 days
When the agent calls forgeplan_activity --since 24h
Then the response contains only entries from the last 24 hours
And entries are sorted by ts ascending
And total count is reported correctly
And a _next_action hint is present
```

### AC-4: Query by tool name filter

```gherkin
Given a log containing 20 calls of various tools
When the agent calls forgeplan_activity --tool forgeplan_score
Then the response contains only entries with tool == "forgeplan_score"
And entries from other tools are absent
```

### AC-5: Rotation across UTC day boundary

```gherkin
Given a workspace where today's log file has 10 entries
When UTC midnight passes and the agent makes a new tool call
Then a new file tools-YYYY-MM-DD.jsonl is created for the new day
And the new file contains the new entry
And yesterday's file is untouched
```

### AC-6: Corrupted line does not break query

```gherkin
Given a log file where line 5 is truncated (missing closing })
When the agent calls forgeplan_activity --since 24h
Then lines 1..4 and 6..end are returned
And line 5 is reported in a "warnings" field with reason "parse error"
And the tool does not panic or return RPC_ERROR
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| `tokio::fs` async file I/O | Runtime | Ready | stdlib |
| `serde_json` for JSONL serialization | Runtime | Ready | workspace dep |
| `chrono` for UTC date bucketing | Runtime | Ready | workspace dep |
| No changes to existing 45 tool handlers | Internal | Will be wrapped at dispatch layer | — |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | fsync on every call causes perf regression | Medium | Medium | Measure NFR-001; if > 2 ms, batch writes with 100 ms window and fsync on flush | Core |
| R-2 | Log file grows unbounded on long-lived workspaces | High | Medium | Daily rotation is MVP; compression / TTL-based cleanup deferred to PRD-066 | Core |
| R-3 | Disk full → log write fails → tool fails silently | Medium | High | Log-write failure logs via tracing but does NOT fail the tool call (activity log is an observer, not a gate) | Core |
| R-4 | Concurrent appends from 2 MCP processes interleave badly on some filesystems | Low | Medium | Use O_APPEND semantics (atomic appends on POSIX); add cross-platform integration test | Core |
| R-5 | Args hash collisions hide distinct calls | Very Low | Low | 12-char hex of SHA-256 = 48 bits; collision probability ~2^-24 at 10k calls | — |

---

## Timeline

| Milestone | Target Date | Description |
|-----------|-------------|-------------|
| PRD Approved | 2026-04-18 | This doc validated |
| RFC Approved | 2026-04-19 | Architecture decided (append-only file, dispatch wrapper) |
| MVP | 2026-04-20 | FR-001..006 shipped on dev |
| v0.21.0 Release | 2026-04-22 | Tagged, binaries built, brew updated |

---

## Stakeholders

| Role | Name | Sign-off |
|------|------|----------|
| Product Owner | user (project owner) | [ ] |
| Engineering Lead | gogocat | [ ] |
| Design | n/a | [x] |
| QA | n/a (integrated with Rust test suite) | [x] |

---

## Affected Files

- crates/forgeplan-core/src/activity/ (new module)
- crates/forgeplan-core/src/config/ (add `log_args` flag)
- crates/forgeplan-mcp/src/server.rs (wrap tool dispatch)
- crates/forgeplan-cli/src/commands/activity.rs (new command)
- crates/forgeplan-mcp/tests/activity_log.rs (new integration test)

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| PRD-055 | Sibling — undo mechanism builds on activity log | Draft (next) |
| PRD-062 | Sibling — LLM rate limiting uses activity_stats | Draft (later) |
| PROB-039 | Inspired by — post-v0.19.0 need for better diagnostics | Closed |

---

> **Next step**: После approve → создать RFC (архитектура: single writer task, tokio channel, dispatch wrapper).

