# AGENTS.md

Instructions for AI coding agents (Claude Code, Aider, Cursor, Continue, etc.) working in this repository.

This file is the **entry point**. For full details, read the files it points to.

## Start here

1. **`CLAUDE.md`** — complete project instructions: methodology, git workflow, commit conventions, storage model, quality gates, and hard rules. **Read this first.**
2. **`docs/README.md`** — documentation index with cross-references to methodology, operations, schemas.
3. **`forgeplan health`** — run this in the terminal to see current project state (blind spots, orphans, stale artifacts).

## What this project is

**Forgeplan** — Rust-based methodology engine (CLI + MCP server + future Desktop app) for managing engineering artifacts (PRD, RFC, ADR, Epic, Spec, Evidence) with quality scoring, semantic search, and decision tracking.

- **Language:** Rust 1.75+ (crates workspace)
- **Storage:** Markdown files in `.forgeplan/` as source of truth (ADR-003), LanceDB as derived index
- **Distribution:** cargo-dist binaries, brew formula, install script
- **Website:** Astro + Starlight at `website/` (see `website/README.md`)

## Hard rules (non-negotiable)

1. **Follow the Forgeplan methodology itself** when making non-trivial changes:
   - `forgeplan route "task"` → determine depth (tactical / standard / deep / critical)
   - `forgeplan new <kind>` → create artifact for Standard+ depth
   - `forgeplan validate` → must PASS before coding
   - `forgeplan reason` → ADI reasoning (mandatory for Deep+)
   - Code → test each `pub fn` immediately
   - `forgeplan new evidence` + link + score + activate

2. **Never commit to `main` or `dev` directly.** Always feature branch → PR.

3. **Never delete `.forgeplan/` without `forgeplan export` first.**

4. **Never push `--force` to `main`.** The safety hook blocks this.

5. **`cargo fmt` + `cargo check` before every commit.** Git hooks enforce this.

6. **Write tests for every new `pub fn` immediately** — do not move to the next function without a test.

7. **Markdown files in `.forgeplan/` are the source of truth** (per ADR-003). The LanceDB index in `.forgeplan/lance/` is derived — rebuild via `forgeplan scan-import` if needed.

## Parallel agents and the build directory

`target/` in this repository runs 20-45 GB. The disk it lives on is routinely
under 20 GB free. Several agents compiling at once is the single most reliable
way to break a session here, and it has happened four times.

**The failure does not look like a failure.** When the disk fills, cargo dies
with `ld: write() failed, errno=28`, and the harness around it reports
`passed=0 failed=0` at exit 0 — or an empty summary. A check that never ran and
a check that passed are indistinguishable in every summary format we use. On
one occasion this produced 26 phantom clippy warnings that did not exist. Twice
the result was reported as green before anyone noticed.

Rules for anyone dispatching sub-agents in this repo:

- **Never let more than one agent compile at a time.** No parallel `cargo
  build` / `test` / `check` / `clippy`. Feature unification means agents with
  different flags will also thrash each other's artifacts even when the disk
  holds.
- **Prefer read-only review.** Reading source with Read/Grep answers most
  review questions. When execution is genuinely needed, hand agents the
  already-built `./target/debug/forgeplan` and let them work in `/tmp`
  workspaces via `forgeplan init -y`. Say so in the prompt — agents reach for
  `cargo test` by default.
- **Check `df -h .` before any full run**, and again in the same command that
  reports the result. Under ~10 GB free, clean first:
  `cargo clean -p forgeplan -p forgeplan-core -p forgeplan-mcp` recovers
  15-37 GB while keeping compiled dependencies. `find target -maxdepth 2 -type
  d -name incremental -exec rm -r {} +` is the cheap version — note that the
  safety hook blocks `rm -rf` but permits this form.
- **Make the harness say when a run did not happen.** Count the test binaries,
  not just the pass/fail line:

  ```bash
  n=$(grep -c '^test result:' "$log")
  if [ "$n" -eq 0 ]; then echo "RUN DID NOT HAPPEN"; df -h . | tail -1; fi
  ```

  A summary that cannot distinguish "zero failures" from "zero tests" is worse
  than no summary.

Two related traps, same family:

- **Never edit source while a background build runs.** The binary captures the
  file mid-edit, so the tree and the artifact disagree. It presents as a real
  defect visible only in E2E while unit tests pass — suspect the binary before
  the code.
- **Two agents in one worktree is a race.** Use `git worktree add` per agent
  for anything that writes. A concurrent commit from a second session has
  already cost a lost commit-message file and half an hour of misdiagnosis in
  this repo.

## Repository structure (quick map)

```
ForgePlan/
├── CLAUDE.md, AGENTS.md, README.md
├── crates/                ← Rust workspace (core + cli + mcp)
├── .forgeplan/            ← artifact workspace (markdown tracked, lance/cache/config local)
│   ├── adrs/, rfcs/, prds/, epics/, specs/
│   ├── evidence/, problems/, solutions/, notes/
│   ├── lance/             ← gitignored (derived)
│   └── config.yaml        ← gitignored (local)
├── docs/                  ← production documentation
│   ├── README.md          ← documentation index
│   ├── methodology/       ← how to use Forgeplan
│   ├── operations/        ← agent hooks, enforcement, repo protection
│   └── schemas/           ← formal artifact schemas
├── templates/             ← markdown templates for each artifact kind
├── website/               ← official website (Astro + Starlight)
├── marketplace/           ← plugin marketplace (plugins + skills)
├── scripts/               ← build + release + helper scripts
├── Formula/               ← Homebrew formula
└── .local/                ← gitignored — local notes, research, sessions
```

## Language

- **Documentation & commit bodies:** Russian preferred (matches project conventions)
- **Code identifiers & commit descriptions:** English
- **Communication with the user:** Russian

## How to explain things to the owner

Set by the owner on 2026-09-05, after comparing two answers about the same
decisions: one written in architecture-speak, one in plain language. The
plain one worked; the other one — «хрен поймёшь, о чём речь».

When presenting a decision, a trade-off, or a piece of analysis:

- **Plain, but not banal.** Simple words carrying real content — never
  simple words instead of content. The test: the reader should be able to
  retell the decision to someone else after one read.
- **Name the collision before the answer.** Most decisions exist because two
  things conflict. Show the conflict first («два действующих ADR говорили
  противоположное»), then the resolution. An answer without its tension
  reads as arbitrary.
- **One live example beats three definitions.** «Приходит задача — кто
  решает, что сначала запустится аналитик, потом кодер?» explains an
  orchestration boundary faster than any glossary.
- **Minimal anglicisms.** Code identifiers, artifact kinds, and command
  names stay as-is (`claim`, `EvidencePack`, `based_on`). Everything else
  gets a Russian phrasing: «замок на запись», not «лок»; «происхождение»,
  not «провенанс»; «поверхность команд» needs an explanation the first time
  it appears.
- **Consequences, not just verdicts.** «Deprecated» is a verdict; «хоронить,
  не лечить — всё, что она обещала, уже делают NOTE и Hindsight» is a
  decision someone can agree or argue with.
- **Metaphors must carry weight.** «Нотная тетрадь и приёмная комиссия» is
  good because both halves map to real subsystems. Decoration is worse than
  nothing.

## Authorship (single author)

Forgeplan is a single-author project. When generating ANY author-attributed content, use:

- **Name:** `Eli Rum`
- **URL:** `https://elirum.me`

This applies to:

- Blog post `AuthorBlock` (`website/src/components/blog/AuthorBlock.astro`)
- Blog footer author line (`website/src/components/blog/BlogFooter.astro`)
- JSON-LD `author.name` + `Person.url` (`website/src/components/blog/SeoMeta.astro`)
- Schema.org `Organization.founder` (`website/src/components/SiteJsonLd.astro`)
- OpenGraph `article:author` meta tags
- `package.json` `author` field (if updated)
- Any author byline in `docs/`, `.forgeplan/notes/`, blog `.mdx` frontmatter
- Conventional commit `Co-Authored-By` lines for solo work (do not add — Eli Rum is sole author)

**DO NOT use** placeholder names: `Mike Kubal`, `Forgeplan Author`, `Anonymous`, `Maintainer`, etc.

If a future contributor joins, this section will be updated to reflect multi-author conventions. Until then, treat the single-author invariant as load-bearing for credibility, SEO authority signal, and brand consistency.

## See also

- [`CLAUDE.md`](CLAUDE.md) — full project instructions (primary)
- [`docs/README.md`](docs/README.md) — documentation index
- [`docs/methodology/FORGEPLAN-GUIDE.md`](docs/methodology/FORGEPLAN-GUIDE.md) — full methodology reference
- [`docs/operations/AGENT-ENFORCEMENT.md`](docs/operations/AGENT-ENFORCEMENT.md) — agent rules and guardrails
- [`website/README.md`](website/README.md) — website architecture notes
