---
title: fpl-hsmem
description: "Project-scoped cross-session memory for Hindsight API >=0.10.2 and <0.11.0."
---

**Catalog description:** Project-scoped cross-session memory for Hindsight API >=0.10.2 and <0.11.0. Use when a repository explicitly opts in via .hindsight.json; hooks default off, enrichment is operator-gated, and BankGate is client safety, not server authorization.

**Cost**: light | **Stability**: stable | **Version**: 3.7.0 | **Category**: memory

**Targets**: claude-code

**Requires**: Hindsight API >=0.10.2 and <0.11.0 at the explicit url in the project's .hindsight.json. No URL is implied. The config must declare banks and defaultBank; selected banks must already exist on the server. Credentials are optional for unauthenticated local servers; when required, use a contained tokenFile or an explicitly consented tokenCommand.

**Packaged surface**: 0 command files, 1 agent files, 10 skills

## Agents

Claude Code scoped names are shown below. Its [agent loader](https://code.claude.com/docs/en/plugins/components#agents) scans `agents/` recursively, keeps subfolder segments and applies frontmatter `name` to the final segment. Other runtimes may use different names. Descriptions are packaged metadata except for curated corrections to documented runtime behavior; runtime boundaries, where provided, take precedence over optimistic metadata wording.

| Agent | Source path | What it does |
|---|---|---|
| `fpl-hsmem:memory-curator` | `agents/memory-curator.md` | Methodology: curation of a Hindsight memory bank through this relay - the correction cycle (find → retire with a reason → write the correction → rebuild what rested on it → verify the job finished) plus the read discipline that decides between searching, enumerating and synthesising. Scoped to MEMORY: it never writes source files and never touches forgeplan artifacts. The agent that knows how this memory relay is meant to be used. Answers "what do we know about X" with the right tool rather than the first one (recall ranks by meaning, list enumerates by structure, reflect writes a conclusion - they are not interchangeable). Corrects wrong memory without destroying the record: retire with a stated reason, write the correction in full, rebuild the derived beliefs, then check that the background job actually finished. Audits a bank's privacy posture before filling it further. Refuses, by construction, to delete documents, rewrite what the extractor keeps, or change the bank's persona - those are operator decisions with no undo. |

## Skills

Skills are listed separately from command files. In Claude Code, user-invocable skills can also be slash entry points; invocation flags below are taken from their frontmatter. Descriptions are packaged metadata with curated runtime corrections where needed; they are not independent guarantees of enforcement.

- `audit-bank` - Read-only audit of a bank in six calls, ending in a written verdict rather than a number. Answers the three questions a count cannot: is it safe to keep filling, is it actually working, and did anyone choose this bank on purpose. Posture audit - masking on or off, what a single document is worth if deleted, which jobs failed silently, whether the bank id was chosen or derived. Run on day one of a new bank and whenever the answer to "is this safe" is a shrug. Changes nothing. NOT a quick health check - that is /fpl-hsmem:status.
- `bootstrap` - One-time seeding of a bank for a project that has just been activated: set the bank's persona, ingest the formal artifacts that already exist, and create a small starter set of knowledge pages. Not the technical wiring - that is the setup CLI - but the layer that decides what this bank is for before it accumulates thousands of facts nobody chose. Seed a new bank - mission, existing PRDs/RFCs/ADRs, two or three starter pages. Use on a fresh project or when memory_status shows an empty bank in an active repo. Run /fpl-hsmem:audit-bank first if the bank is NOT empty: bootstrap assumes it is starting from nothing.
- `correct-memory` - The workflow that makes a bank correctable instead of cumulative: find the exact stored row, retire it with a stated reason, write the correction in full, rebuild the beliefs that rested on it, and verify the background job finished. Deleting a wrong fact is the easy answer and the wrong one - six months later nobody can tell why the mistake was ever believed. Fix a wrong fact without destroying the record. Use when memory returns something you know is stale or false. Has side effects, so it is invoked deliberately, never inferred. NOT for a badly SHAPED answer that is factually right - that is /fpl-hsmem:directives. **Invocation**: `disable-model-invocation: true` (explicit invocation only).
- `diagnose` - The full diagnostic when memory is not doing what it should: server reachable, bank resolved as intended, hooks actually firing, config not overridden somewhere you forgot, and jobs completing. Written to distinguish the failure modes that look identical from the outside - a dead server and an empty bank both return nothing. Six-step integration diagnostic with a remediation per finding. Use for "recall never fires", "retain seems broken", "I do not see memory happening", or before changing settings. NOT a content audit - whether the bank is worth having is /fpl-hsmem:audit-bank.
- `directives` - The standing rules a bank follows when it synthesises an answer - precedence between conflicting decisions, what an answer must always name, what to leave out. The distinction that makes this useful: a factually wrong answer is a memory problem, a consistently badly-shaped answer is a rules problem, and adding more facts never fixes the second. List, add and retire the rules that govern synthesis. Use when reflect keeps preferring a superseded decision, or omits the source, or answers in a shape you keep correcting by hand. NOT for wrong facts - that is /fpl-hsmem:correct-memory. NOT the bank's persona - that is one setting, memory_set_mission.
- `enrich` - Turns knowledge the team already has - design records (ADRs, RFCs, notes, problem and evidence records), decision ledgers, review threads, chat threads - into short, self-contained memory items (a decision and why, a rejected option and why, a lesson, a pitfall, a rule) that link back to the artifact. This skill writes local JSONL and produces a read-only dry-run plan; it never applies the plan or writes to a bank. Memory is not a document store: this skill distils, it never copies. Enrich a memory bank from existing sources. Use when asked to fill memory with past decisions, pull decisions out of design records, or load lessons from reviews and threads. NOT for fixing a wrong fact (/fpl-hsmem:correct-memory), seeding a brand-new bank's mission and pages (/fpl-hsmem:bootstrap), storing a whole document, or capturing raw session transcripts.
- `export-bank` - A readable markdown snapshot of a bank for backup, audit or handing context to a person. Says plainly what it is not: this is for humans, and moving memory between banks is a different mechanism entirely. Snapshot a bank to disk before something risky, or to share context without giving out server access. NOT the way to move memory between banks - that is the document-transfer path, documented in this skill. NOT disaster recovery either; for that, back up the database volume.
- `memory-setup` - Look at a project and propose the retrieval and memory layers it should have - what to install, what each layer answers, and what it will never answer. The reason this exists: an agent that can only grep is slow in a way nobody notices, and an agent with no memory re-learns the same lesson every session. Both are invisible from inside a single conversation. Inspect a repository and recommend a stack - search layers (LSP, indexed text, AST, semantic) and memory layers (a Hindsight bank, knowledge pages, formal decision records). Says what each is worth on THIS repo, in this order, and shows the commands. Never installs anything silently. NOT the wiring of an already-chosen bank - that is /fpl-hsmem:bootstrap.
- `mental-model` - Guided creation of a living knowledge page - a standing answer to a recurring question, rebuilt from memories after every consolidation. The whole value is in the source query: a narrow question produces a page worth reading for months, "everything about X" produces noise that nobody opens. Create, retune, rebuild or blank a knowledge page. Use when the same question keeps being asked and re-searched. Content starts empty and fills on the next consolidation - that is normal, not a failure. NOT a place to write facts by hand; pages are derived, and anything typed into one is overwritten on the next rebuild.
- `status` - A thirty-second read of one memory bank: which bank, who chose it, how much is in it, and whether anything is silently broken. The first thing to run when memory "feels off" - most such feelings turn out to be the session reading one bank while something else writes to another. Quick health + statistics for the active bank. Use for "is memory working", "what bank am I on", "how many memories". NOT an audit - it does not look at privacy posture or failed jobs; that is /fpl-hsmem:audit-bank.

## Hooks

Registered events in `hooks/hooks.json` (3): `SessionEnd`, `Stop`, `UserPromptSubmit`.

Registration does not guarantee activation in every client. Client support, setup and defaults-off settings still apply.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install fpl-hsmem@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
