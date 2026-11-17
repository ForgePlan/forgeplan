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
- **The exit code you print through a pipe is the wrong one.** `cmd | tail` and
  `echo $?` reports `tail`. `${PIPESTATUS[0]}` is a bashism and expands to the
  empty string in this repo's zsh, so the guard prints `exit=` and reads as
  fine. Redirect to a file and measure without a pipe:
  `cargo clippy ... > /tmp/x.log 2>&1; echo "exit=$?"`. Same family as the
  disk-full trap above — the number is real, it is just measuring something
  else.

## Findings have a home, and it is not the chat

A finding that lives only in a reply is gone at the end of the session. This has
already cost real work here: a measurement across 465 artifacts that cancelled a
planned fix lived nowhere but a chat message until someone thought to file it,
and a handoff document once pointed at a scratch directory that did not survive
the session it described.

**File it the moment you find it, before finishing the thought that produced
it.** Three kinds of finding, three homes:

| Finding | Home | Why there |
|---|---|---|
| A rule that changes how an agent works | **this file** | Read every session. Costs context every session, so it earns its place only by changing behaviour. |
| Work someone should do | **GitHub issue**, open | Assignable, closable, outside the context budget. |
| A measured dead end — checked, decided not to do | **GitHub issue, closed on arrival**, label `measured-not-planned` | Findable by `gh issue list --state closed --label measured-not-planned`. Costs nothing to keep. |

The third row is the one people skip, and it is the one that pays. A measurement
that **cancels** work is worth as much as one that starts it, and it evaporates
faster — nobody files "we checked and it does not matter". Then the next person
smells the same thing and re-derives it. #476 is the worked example: two
plausible defects, both measured, both left alone, numbers on the record.

Rules for filing:

- **Numbers, not adjectives.** "Zero verdicts change across 465 artifacts"
  survives a year. "Seems fine" does not.
- **Every finding carries a trigger to revisit, phrased as a condition.**
  `"later"` is not a trigger. `"if stripping comments would change the verdict
  on one or more artifacts"` is. Same rule ADRs already follow: name what must
  become true for someone to pick this up.
- **Say what was rejected and why.** A finding that records only the conclusion
  invites the next person to relitigate the option you already killed.
- **Do not file the same thing twice.** `gh issue list --search` before
  creating; extend the existing issue if one fits.

Revisit is a step, not a habit: **before opening a `release/v*` PR**, list the
`measured-not-planned` issues and re-check their triggers against the current
corpus. A trigger that has become true reopens the issue; one that has not gets
left alone without discussion.

## GitHub mechanics that silently do nothing

- **`Closes #N` in a PR body does not close anything when merging into `dev`.**
  GitHub honours closing keywords only on merges into the *default* branch, and
  ours is `main`. Feature branches merge into `dev`, so the keyword is inert and
  the issue stays open with no error anywhere. Close the issue by hand at
  dev-merge, naming the merge commit — leaving it open until the release means
  the board says "nobody did this" about finished work.
- **An empty `gh` result is not proof of absence.** `Post
  "https://api.github.com/graphql": EOF` prints nothing to stdout and can exit
  through a pipe as success. Verify a listing you are about to act on by
  querying the object directly, not by trusting that zero rows means zero
  things.

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
├── docs/                  ← contributor documentation — NOT the website (see below)
│   ├── README.md          ← documentation index
│   ├── methodology/       ← how to use Forgeplan
│   ├── operations/        ← agent hooks, enforcement, repo protection
│   └── schemas/           ← formal artifact schemas
├── templates/             ← markdown templates for each artifact kind
├── website/               ← forgeplan.dev — landing + docs + blog + guides (one Astro app)
├── marketplace/           ← plugin marketplace (plugins + skills)
├── scripts/               ← build + release + helper scripts
├── Formula/               ← Homebrew formula
└── .local/                ← gitignored — local notes, research, sessions
```

## The site: four surfaces in one Astro app

`website/` is **one** Astro 6 app at `https://forgeplan.dev` — no separate repo,
no subdomain, no second deploy. Four surfaces share it, and each one has a
different content source. Confusing them is the most common mistake here:
"update the docs" has four different answers depending on which one is meant.

| Surface | Routes | Source of truth | Rendered by |
|---|---|---|---|
| **Landing** | `/` only — **EN, no RU twin** | `src/pages/index.astro` → `Hero.astro` (wraps the `HeroSection.tsx` island) + React islands (`TrustSection`, `PipelineSection`, `ArtifactsSection`, `GraphSection`, `AISection`) + `Install.astro` | Astro + Tailwind 4 + GSAP ScrollTrigger |
| **Docs** | `/docs/*`, `/ru/docs/*` | `website/src/content/docs/docs/**` (225) + `ru/docs/**` (224) | Starlight — sidebar is hand-written in `astro.config.mjs` |
| **Blog** | `/blog`, `/blog/<slug>`, `/blog/series/<name>`, `/blog/rss.xml` + `/ru/blog/*` | `website/src/content/blog/{en,ru}/*.mdx` (22 + 22) | Astro content collection `blog` |
| **Guides** | `/guides`, `/guides/<slug>` + `/ru/guides/*` | `website/public/guides-raw/*.html` (21 files = 20 lessons + the standalone `index.html`) + catalog `src/data/guides.ts` (20 entries) | inlined at build via `src/lib/embed-html.ts` |

**There is no RU landing page.** `src/pages/ru/` holds `blog/`, `docs/`,
`guides/` and nothing else; `public/_redirects` sends `/ru` and `/ru/` to `/`
with a 302. Three surfaces are bilingual, the landing is not.

The docs EN/RU gap of exactly one is `changelog.md` — generated EN-only by the
`prebuild` step, never translated. Any other gap is real drift.

Counts drift; re-measure before quoting them:

```bash
cd website && echo "blog $(ls src/content/blog/en | wc -l)/$(ls src/content/blog/ru | wc -l)" \
  "docs $(find src/content/docs/docs -type f | wc -l)/$(find src/content/docs/ru/docs -type f | wc -l)" \
  "guides $(ls public/guides-raw | wc -l)/$(grep -c "^    slug: '" src/data/guides.ts)"
```

**Repo-root `docs/` is not the site.** It holds 178 contributor markdown files
(198 files in total — methodology, operations, schemas) that never reach the
browser. The site's docs live in `website/src/content/docs/`. Two different
things, two different audiences.

### Invariants that break silently

- **Integration order in `astro.config.mjs` is load-bearing**: `sitemap()` →
  `starlight()` → `mdx()` → `react()`. Starlight bundles `astro-expressive-code`,
  which throws a hard error if `mdx()` is registered first. The generic Astro
  docs say the opposite; the ECE constraint wins.
- **Blog entry IDs are `{lang}/{slug}`**, forced by the `generateId` override in
  `src/content.config.ts`, so EN and RU posts sharing a `slug` field don't
  collide. Nine call-sites consume that format — the config comment lists all of
  them. Change the format and every one must move in the same commit.
- **Blog frontmatter fails the build, not the page.** `title`, `description`,
  `slug`, `lang`, `publishedAt`, `kind`, `topic` are required; `cover` must
  match `/^\/[\w\-/.]+\.(png|jpg|jpeg|webp|svg)$/` — a local path only, so
  `javascript:`, `data:` and `//evil.tld` never reach `<img src>` or `og:image`.
- **Editing `src/data/guides.ts` requires a dev-server restart.** `getStaticPaths`
  reads it once at startup and HMR does not reload it — new slugs 404 in the
  browser while `npm run build` passes. HTML/CSS/MDX changes do hot-reload.
- **Guides are inlined, never iframed** (the owner rejected iframes twice).
  `extractEmbed` pulls body + styles + scripts out of the standalone HTML,
  scopes CSS under `.guide-embedded`, wraps inline scripts in an IIFE, and
  strips the lesson's own footer so `BlogFooter` is the only one on the page.
- **Three stylesheets, three scopes.** `global.css` (Tailwind tokens, used by
  `Header.astro`), `blog-theme.css` (shared by blog *and* guides),
  `forge-theme.css` (Starlight override). `blog-theme.css` carries `!important`
  rules that beat lesson inline CSS inside `.guide-embedded` — read
  `website/CLAUDE.md` before touching guide layout.
- **Two font stacks, deliberately.** Landing/blog/docs use Space Grotesk +
  Geist Mono (`@fontsource`, self-hosted). `public/guides-raw/*.html` still use
  Inter + JetBrains Mono. Don't unify without asking.
- **i18n is mirror routes, no middleware.** `/docs/...` = EN, `/ru/docs/...` =
  RU; same for blog, guides. Bare `/docs` and `/ru/docs` 302 to the installation
  page; `public/_redirects` sends `/ru` → `/`.

### Generated docs — never hand-edit

- `npm run build` runs a `prebuild` step: `scripts/copy-changelog.mjs` copies
  the root `CHANGELOG.md` into `src/content/docs/docs/changelog.md`. Edit the
  root file, not the copy.
- `scripts/generate-cli-docs.mjs` refreshes captured help blocks in `docs/cli/*.md`
  against a real binary (`--binary <path>`). Markdown owns curated descriptions,
  examples and runtime cautions. Optional `--sync-ru` refreshes technical blocks
  in existing RU pages only; it does not translate or rewrite their prose.
- `scripts/generate-mcp-docs.mjs` writes `docs/mcp/*.md` from a real, versioned
  MCP `tools/list` capture (`--schema-file <path>`), not Rust function wrappers.
  Input must contain `source.version` and a nonempty `tools` array with
  `inputSchema`. Generate against the same released binary as the CLI reference.
- `scripts/generate-marketplace-docs.mjs` writes `docs/marketplace/*.md` from
  the **separate marketplace repo** — `<MARKETPLACE_DIR>/.claude-plugin/
  marketplace.json` plus per-plugin files. `MARKETPLACE_DIR` is required;
  use an explicitly pinned public source, never an unrelated dirty local draft.
- All three emit **EN by default**; CLI `--sync-ru` is technical-block-only.
  Synchronize and review RU prose in the same change.
  Optional translation scripts are `scripts/translate-batch.mjs` /
  `translate-page.mjs` (Gemini, term glossary `src/i18n/glossary-ru.yaml`).
- CLI and MCP generators preserve curated prose outside technical sections.
  Fix generated help/parameter/invocation blocks in the generator, then regenerate;
  edit authored workflow examples, usage and limitations in Markdown and review
  them against the implementation.
- Health checks: `check-dead-links.mjs` (needs a built `dist/`),
  `check-translation-drift.mjs` (stale RU), `check-content-completeness.mjs`.
- Only `copy-changelog` is wired into npm scripts (as `prebuild`). The rest are
  run by hand — nothing regenerates the CLI/MCP/marketplace reference
  automatically when the surface changes.

### Build verify

`npm run build` in `website/` must close green before commit — baseline
**544 pages / ~15 s** (measured 2026-09-17: `build` 11.6 s + Pagefind index
2.4 s). Two warnings are expected and not regressions:

- `[vite] Some chunks are larger than 500 kB after minification` — the GSAP +
  React island bundle.
- `Entry docs → 404 was not found.` while generating `/404.html`.

Anything beyond those two is new. `build` PASS does **not** validate layout:
complex layout changes get checked in a browser (see the Playwright recipe in
`website/CLAUDE.md`).

### Design artifacts behind these surfaces

`PRD-024` (site + docs portal), `PRD-046` (docs catch-up + Cloudflare Pages),
`PRD-047` (RU locale), `PRD-079` (bilingual blog), `PRD-080` (blog nav / visual
/ SEO), `RFC-011` (blog scaffold), `RFC-012` (blog foundation).

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

Five places in `website/src/` carry the name today — all five move together:

- Blog footer author line (`components/blog/BlogFooter.astro`)
- Blog sidebar About card (`components/blog/BlogSidebar.astro`)
- Blog aside byline in the TOC (`components/blog/TOC.astro`)
- JSON-LD `author.name` + `Person.url` + OpenGraph `article:author`
  (`components/blog/SeoMeta.astro`)
- Schema.org `Organization.founder` (`components/SiteJsonLd.astro`)

Guide pages (`src/pages/guides/[slug].astro`) emit `LearningResource` JSON-LD
with **no author field at all** — an open gap, not a place to keep in sync.

It also applies to:

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
