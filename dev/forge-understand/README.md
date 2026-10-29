# forge-understand (MVP)

An experimental June–July 2026 loop for a proposed Forgeplan "project understanding
view": **sample a repository → synthesise an interactive HTML map**. It combines
forgeplan artifacts, freeform documentation and code, using the visual style of
[`effective-html`](../effective-html). It is not a shipped `forgeplan map`
command, an exhaustive repository audit or proof of accuracy for every project.

> **Key correction this MVP encodes:** `effective-html` does **not** scan your
> project. It is only the *renderer* (the "how the HTML looks" half). The
> scanning + understanding is done by Claude Code's agent loop. This tool
> packages **both halves** into one runnable loop.
>
> The JSON renderer spike, the proposed map pack and ForgeFarm are distinct
> experiments/designs. `MASTER-SPEC.md` records a target architecture, not installed
> CLI/MCP/web functionality. Static maps retain historical counts and explanations;
> they do not query the current workspace or certify current runtime behavior.

## The loop

```
 run.mjs
   │  builds a prompt = scan-instructions + forge-diagram skill + style reference
   ▼
 claude -p  (headless, read-only: Read/Glob/Grep, --add-dir <repo>)
   │  ├─ scans .forgeplan/**           (PRD/RFC/ADR/Epic/Spec + typed links)
   │  ├─ scans README / docs/** / *.md (freeform docs, loose ADRs)
   │  └─ scans code structure          (packages, entry points, modules)
   │  synthesises zones · nodes · typed edges · toggleable flows
   ▼
 self-contained interactive HTML  →  out/understanding-<ts>.html  →  browser
```

## Run it

```bash
cd dev/forge-understand

# see exactly what will run, call nothing:
node run.mjs --dry-run

# real run against the Forgeplan repo (default), opens the result:
node run.mjs

# against any other repo:
node run.mjs --repo /path/to/some/project

# force a model / custom output / don't auto-open:
node run.mjs --model opus --out ./out/map.html --no-open
```

Requirements: an authenticated, compatible `claude` CLI on PATH and Node 18+.

## What's deliberately minimal

- **No backend, no web view yet** — this is the loop, headless. The web view
  (a route in `ForgePlanWeb`, SvelteKit) wraps this with a button + progress and
  embeds the resulting HTML.
- **Single-pass scan** — one agent, not a multi-agent sweep. Good enough to feel
  the output quality. Phase 2 splits the scan into parallel domain agents
  (forgeplan-graph / freeform-docs / code-structure).
- **Renderer is inlined, not registered** — `run.mjs` reads `SKILL.md` and feeds
  it into the prompt, so the loop works without installing a skill anywhere. The
  same `SKILL.md` is already shaped to drop into the Forgeplan marketplace as a
  real plugin skill later.
- **Reads the repo, writes one file** — `--allowedTools Read Glob Grep Write`,
  `--permission-mode acceptEdits`. The agent scans read-only and uses `Write`
  to save the HTML to the path `run.mjs` dictates, which then reads it back.
  ⚠️ This is **not a hard sandbox** — the agent *has* the `Write` tool. For an
  untrusted repo, run against a throwaway checkout, or scope writes with a
  settings allowlist (`Write(<outPath>)`). (Phase 2 will lock this down.)

## Known finding from the first real run

Capturing a ~40KB HTML over **stdout** (`-p --output-format text`) **truncates
mid-stream** at the token limit — the first run came back as prose ("output got
cut, so I wrote the file instead") and the agent saved the map itself. Fix
applied: the agent now `Write`s the HTML to a path we control and `run.mjs` reads
it back, instead of capturing stdout. A single-pass scan of a large repo also
took **~18 minutes** — motivating the Phase-2 parallel multi-agent scan.

## How it maps to the target system

| Target layer | Here (MVP) | Later |
|---|---|---|
| Web view | — | `ForgePlanWeb` route `/understand` + progress + embed |
| Orchestrator | `run.mjs` (shells `claude -p`) | Claude Agent SDK, multi-agent, cache |
| Scan | one read-only pass | parallel agents per domain |
| Renderer | `forge-diagram` SKILL.md (inlined) | same skill, shipped via marketplace |
| Output | `out/*.html`, opened locally | embedded inline in the web view |

## Attribution / license

The bundled style reference (`skills/forge-diagram/references/architecture-example.html`)
comes from `effective-html` by plannotator (MIT). Its nested `html-effectiveness`
corpus by Thariq Shihipar carries Apache-2.0 in that corpus's own `LICENSE`;
the wrapper's MIT license does not relabel the nested material. Preserve the
applicable upstream notices when reusing either source.
