---
title: fpl-hsmem
description: "Long-term memory across sessions for Claude Code, wrapping a Hindsight server."
---

Long-term memory across sessions for Claude Code, wrapping a Hindsight server. Recalls relevant history before every prompt and captures the conversation after every response, so decisions stop being re-litigated each session; 27 MCP tools let an agent search memory, browse it by structured filter, correct a wrong fact without destroying the record, and see which background jobs failed. Use when a project runs across more than a few sessions, when memory returns something stale, or when you need to know what was tried and rejected - the code shows what won, never what lost. Ships 3 auto-hooks, 9 skills (status, bootstrap, mental-model, diagnose, export-bank, correct-memory, audit-bank, directives, memory-setup) and the memory-curator agent. v3.6.0: size-aware retrieval - RETRIEVAL-AND-MEMORY.md and /memory-setup now put a repository into a tracked-file bracket before proposing anything, and warn out loud when a recommendation leans on a borrowed timing. Measured at 2,154 tracked files the index LOST (36 ms unindexed vs 39 ms indexed, best of three), and the band above that is unmeasured here - so the brackets say where to look, never what to do. v3.4.0: RETRIEVAL-AND-MEMORY.md - which question goes to which layer, and the correction that recorded decisions (ADR/RFC/PRD) live in the repository and are asked BEFORE memory. v3.3.0: audit against the official docs - one version source read at handshake, plugin-scoped skill references, the canonical prompt-defense preamble, hooks in exec form. v3.2.0: MEMORY_WRITE_TOOLS as the single source a CI gate derives agent denylists from. v3.1.0: the server explains itself via MCP `instructions`; bilingual skill descriptions with model tiers. v3.0.0 (breaking): the relay stopped being write-only - memory became correctable; `retain_mission` removed as a tool argument; bank identity anchors on the project root. Full history in CHANGELOG.md.

**Cost**: light | **Stability**: stable | **Version**: 3.6.3 | **Category**: memory

**Targets**: claude-code

**Requires**: Hindsight server reachable at HINDSIGHT_URL (default https://hindsight.tools.orch.so or self-hosted via Docker). HINDSIGHT_API_KEY env var required. HINDSIGHT_BANK_ID env var optional (derived from cwd if unset).

## Agents

| Agent | What it does |
|---|---|
| `memory-curator` | The agent that knows how this memory relay is meant to be used. |

## Skills

- `audit-bank` - Read-only audit of a bank in six calls, ending in a written verdict rather than a number.
- `bootstrap` - One-time seeding of a bank for a project that has just been activated: set the bank's persona, ingest the formal artifacts that already exist, and create a small starter set of knowledge pages.
- `correct-memory` - The workflow that makes a bank correctable instead of cumulative: find the exact stored row, retire it with a stated reason, write the correction in full, rebuild the beliefs that rested on it, and verify the background job finished.
- `diagnose` - The full diagnostic when memory is not doing what it should: server reachable, bank resolved as intended, hooks actually firing, config not overridden somewhere you forgot, and jobs completing.
- `directives` - The standing rules a bank follows when it synthesises an answer - precedence between conflicting decisions, what an answer must always name, what to leave out.
- `export-bank` - A readable markdown snapshot of a bank for backup, audit or handing context to a person.
- `memory-setup` - Look at a project and propose the retrieval and memory layers it should have - what to install, what each layer answers, and what it will never answer.
- `mental-model` - Guided creation of a living knowledge page - a standing answer to a recurring question, rebuilt from memories after every consolidation.
- `status` - A thirty-second read of one memory bank: which bank, who chose it, how much is in it, and whether anything is silently broken.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install fpl-hsmem@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
