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
├── design/                ← design assets (forgeplan-design-system/ — canonical DS; Pencil .pen files later)
├── scripts/               ← build + release + helper scripts
├── Formula/               ← Homebrew formula
└── .local/                ← gitignored — local notes, research, sessions
```

## Design system (single source of truth)

**`design/forgeplan-design-system/`** — the canonical, portable ForgePlan design system package. **Consult it whenever the task involves UI, styling, branding, colors, typography, components, web pages, slides, diagrams, README visuals, or any user-facing surface** — before writing any CSS/HTML/component code.

Package contents (self-contained, no build step, no external deps):

| File | Purpose |
|---|---|
| `DESIGN-SYSTEM.ru.md` / `.en.md` | Canonical documentation: palette, contrast table, typography, spacing, components, accessibility, print, per-surface rules |
| `tokens.css` | Drop-in CSS custom properties for dark + light themes (`data-theme` attribute) |
| `tokens.graph.css` | Optional module for graph/map/canvas surfaces (relation edges, canvas strokes, composed-map zones, dot-grid) — load after `tokens.css` |
| `tokens.json` | Machine-readable token contract with fact/proposal status per token |
| `components.html` | Self-contained bilingual component reference with theme switcher — open in browser |
| `cheatsheet.ru.html` / `.en.html` | A4 landscape quick references with print styles |

Key invariants (details in the docs above):

- **One accent:** `--forge-ember #FF6B35`. Legacy `--accent #FF5A1F` is deprecated — never use in new code.
- **Light-theme text accent:** `--forge-ember-text #C94400` (orange fails WCAG as text on light bg; ember stays for fills/borders/focus).
- **Radius 0 by default** (2px chips, 3px inline code, 50% avatars/dots only). No shadows — flatness via `1px solid var(--forge-line)`.
- **Fonts:** Space Grotesk (UI/text) + Geist Mono (code, IDs, R_eff numbers, metadata).
- **Layout:** `max-width 1280px`, `32px` gutter, sticky elements use `top: var(--header-h)` (88px full / 36px compact).
- **Reuse rule:** to apply the system elsewhere, copy the whole `design/forgeplan-design-system/` directory and link `tokens.css`; do not re-derive values from `website/` source (the blog theme diverges — it is documented tech debt).

`design/` is the home for **all** design assets:

- `forgeplan-design-system/` — the canonical package above, plus `brand-assets/` (logo SVGs, favicons, icons)
- `forgeplan-site.pen` — Pencil source file (site mockups + the design system as atomic-design components: Atoms → Molecules → Organisms → Layouts)
- `visual-guides/` — approved raster methodology guides (design-system foundation, quick start, 9 thematic 16:9 sheets)
- `command-map/` — command map by scenario (portrait + 16:9)
- `source-materials/` — source briefs for the guide set
- `MANIFEST.md` — exact file inventory
- `snapshots/<ts>/` — CANVAS pipeline DS snapshot exports (when the pipeline runs)

The design system follows **atomic design**: tokens → atoms → molecules → organisms → templates/layouts. Higher layers are composed from lower-layer components (refs, not copies).

## Language

- **Documentation & commit bodies:** Russian preferred (matches project conventions)
- **Code identifiers & commit descriptions:** English
- **Communication with the user:** Russian

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
- [`design/forgeplan-design-system/README.md`](design/forgeplan-design-system/README.md) — canonical design system package (tokens, components, cheatsheets)
