# Forgeplan Website

Source for the forgeplan.dev landing page, documentation, blog and interactive guides. These are four sections of one Astro application, not separate deployments.

## Stack

- **Astro 6** — SSG, partial hydration (React islands)
- **Starlight** — docs portal (sidebar, Pagefind search)
- **React 19** — interactive components (Hero animation)
- **GSAP** — ScrollTrigger for Hero pin + crystallization animation
- **Tailwind CSS 4** — utility-first styling with forge design tokens

## Development

```bash
cd website
npm install
npm run dev      # http://localhost:4321
npm run build    # static output in dist/
```

## Content sources

| Section | Routes | Source |
|---|---|---|
| Landing | `/` | `src/pages/index.astro`, Astro components and React islands |
| Documentation | `/docs/*`, `/ru/docs/*` | `src/content/docs/docs/`, `src/content/docs/ru/docs/` |
| Blog | `/blog/*`, `/ru/blog/*` | `src/content/blog/en/`, `src/content/blog/ru/` |
| Guides | `/guides/*`, `/ru/guides/*` | `public/guides-raw/*.html`, catalog `src/data/guides.ts` |

The landing has no Russian counterpart. Blog and documentation have separate EN/RU sources. Guide catalog titles and descriptions are bilingual, but both guide routes currently inline the same Russian lesson body. English lesson translations are tracked in [#506](https://github.com/ForgePlan/forgeplan/issues/506).

The repository-root `docs/` directory is contributor documentation; it is not published by this app. `extractEmbed` in `src/lib/embed-html.ts` inlines guide bodies, scopes their CSS and wraps scripts; guides are not iframes.

Restart the dev server after changing `src/data/guides.ts`: its static routes are read at startup. Changes to existing HTML, CSS and MDX content use HMR.

## Architecture

```
src/
├── components/
│   ├── Header.astro              — Grid header, shrinks on scroll (full→compact)
│   ├── Hero.astro                — Thin wrapper, loads HeroSection island
│   ├── HeroSection.tsx           — StickySection + CrystallizationAnimation + bottom text
│   ├── CrystallizationAnimation.tsx — Physics simulation + scroll crystallization (rAF loop)
│   ├── StickySection.tsx         — GSAP ScrollTrigger pin wrapper (ONLY for Hero)
│   ├── TrustSection.tsx          — R_eff scoring rings (CSS sticky, scroll-driven)
│   ├── PipelineSection.tsx       — Depth routing + ADI (CSS sticky, scroll-driven)
│   └── Install.astro            — Install cards + footer grid (static)
├── content/docs/                — Starlight documentation (markdown)
├── layouts/Landing.astro        — HTML shell, fonts, meta
├── pages/index.astro            — Landing page assembly
├── styles/
│   ├── global.css               — Tailwind @theme tokens + header styles + dot-grid
│   └── forge-theme.css          — Starlight theme override (ember accent, sharp corners)
├── tokens.ts                    — Shared design tokens for JS (COLORS, geometry utils)
└── assets/                      — Logo SVGs, images
```

## Pin Strategy (critical knowledge)

**ONE GSAP ScrollTrigger pin per page.** Multiple GSAP pins from separate React islands conflict because GSAP requires strict top-to-bottom registration order, but React islands hydrate in arbitrary order.

| Section | Pin Method | Why |
|---------|-----------|-----|
| Hero | **GSAP ScrollTrigger** `pin:true` | Complex rAF animation needs GSAP scrub |
| Trust | **CSS `position: sticky`** | Simple scroll-reveal, no GSAP conflict |
| Pipeline | **CSS `position: sticky`** | Same — CSS sticky is independent |
| Install | None (static) | No animation needed |

### CSS Sticky pattern:
```html
<section style="height: 200vh">           <!-- tall = scroll room -->
  <div class="sticky top-0 h-screen">    <!-- content sticks -->
    <!-- animated content, progress from scroll position -->
  </div>
</section>
```

Progress calculated from section scroll position:
```typescript
const scrolled = -rect.top;
const scrollRange = sectionHeight - viewportHeight;
const progress = clamp(scrolled / scrollRange, 0, 1);
```

## Design System

### Colors (tokens.ts + global.css)
```
--forge-bg:      #0D0D0D     (deep black)
--forge-fg:      #E8E8E8     (light gray)
--forge-surface: #161616     (card backgrounds)
--forge-line:    #3A3A3A     (borders, grid)
--forge-dim:     #949494     (secondary text, WCAG AA 7.2:1)
--forge-ember:   #FF6B35     (accent — forge glow)
```

### Fonts
- **Space Grotesk** — headings + body (weight 300-700)
- **Geist Mono** — code, labels, monospace elements

### Key Patterns
- Dot grid background: `radial-gradient` with forge-dim dots, 30px spacing
- Visible borders: `1px solid forge-line` everywhere (blueprint aesthetic)
- Header states: full (72px floating) → compact (36px solid bar)

## Animation: Crystallization

Hero animation sequence (GSAP ScrollTrigger progress 0→1):

```
0%:       Chaos — lines fly, bounce off walls/each other, dots travel along lines
0-35%:    Lines converge toward hex edge positions
35-65%:   Hexagons fly in from 4x scale (biggest first, ember last)
65-80%:   Outer hexagons fade, only ember hex + dot + iso cube remain
80-100%:  Ember hex + dot shift left, "Forge your plan" typewriter
```

### Physics Engine
- 16 lines (10 artifact + 6 ambient), each with position, velocity, angle
- Segment-to-segment collision detection (not center-to-center)
- Wall bounce by endpoints, torque on collision
- MAX_SPEED: 0.35 px/frame, MAX_ANGULAR: 0.002 rad/frame
- Collision epsilon: 0.5 (prevents NaN from near-zero distances)

## Known Issues / Gotchas

### Astro scoped CSS breaks parent→child selectors
`.header-full .header-logo` gets `:where(.astro-xxx)` scope. **Fix**: put parent→child selectors in `global.css`.

### prefers-reduced-motion
Currently removed. If re-adding: show static initial state (chaos), NOT final scene. Test on device with Reduce Motion enabled.

### Multiple GSAP pins = conflict
Never create multiple `ScrollTrigger.create({pin:true})` from separate React islands. Use CSS sticky for additional pinned sections.

### Pencil mockups: rotation vs position
`snapshot_layout` shows rotated bounding box. Center = `(x + width/2, y + height/2)` in unrotated coords. Don't fix positions by snapshot data.

## Updating the documentation reference

Use one explicit release as the source for CLI help and MCP schemas. Check the binary before generating pages:

```bash
# Run from website/.
forgeplan --version
node scripts/generate-cli-docs.mjs --binary /path/to/forgeplan
node scripts/generate-mcp-docs.mjs --schema-file /path/to/tools-list.json
MARKETPLACE_DIR=/path/to/pinned-marketplace node scripts/generate-marketplace-docs.mjs
```

`tools-list.json` must contain both a nonempty `source.version` and the `tools` array returned by MCP `tools/list`, including each tool's `inputSchema`: `{"source":{"version":"0.38.0"},"tools":[...]}`. Capture the version and schemas from the same release whose CLI help is being documented; a raw JSON-RPC response needs this capture envelope before generation. A server from another worktree, or an aggregate list of all tools installed in an AI client, is not the release's API.

The CLI generator currently accepts release 0.38.0 only. Review the curated command contracts before changing its target version. It refreshes captured Usage, Arguments, Options and Commands blocks, and adds missing catalog links; descriptions, examples, return values and runtime cautions remain authored in Markdown. `--sync-ru` refreshes those same technical blocks in existing RU pages without translating or rewriting their prose, metadata, examples or index.

The MCP generator requires a SemVer capture version and rejects omitted existing tools by default. Use `--prune` only after confirming that a complete capture deliberately removes those tools; it deletes the omitted EN pages, not an incomplete capture's mistakes.

The MCP generator owns the input-parameter and example-invocation sections. It preserves curated usage, return-value and refusal guidance; review those sections against the implementation when a contract changes. Do not fix a generated parameter table by hand.

The marketplace is a separate repository. Pin its public revision before generating; a local checkout with unpublished edits is not evidence that a feature ships. Its plugin catalog, versions and entry points can differ from the plugins installed on a developer's machine.

Generators produce EN reference pages by default. Update and review the corresponding RU translations in the same change; the CLI generator's optional `--sync-ru` handles captured help blocks only. Keep identifiers, flags, JSON field names, required fields and documented limitations consistent. Keep commands, skills, agents and runtime-specific hooks distinct.

`npm run build` generates the EN changelog page from the repository-root `CHANGELOG.md`; do not edit that copy or create a second source of release notes. Sidebar groups are configured in `astro.config.mjs`; CLI, MCP, marketplace, guides and reference entries are discovered from their directories, while getting-started and methodology entries are listed explicitly.

After the content and translations are ready:

```bash
npm run build
node scripts/check-dead-links.mjs
```

A successful build does not prove that command examples work or pages remain readable. Exercise changed examples in a disposable workspace, then open the affected EN/RU pages and navigation in a browser. Do not run examples that invoke an external LLM, download model weights or change a real project's artifacts merely to check documentation.

## Related Artifacts

- **PRD-024**: Official Forgeplan Website and Documentation Portal
- **Design Research**: `docs/references/DESIGN-RESEARCH-WEBSITE.md`
- **QF Reverse Engineering**: `docs/references/QF-NETWORK-REVERSE-ENGINEERING.md`
- **Design Concept**: `docs/references/WEBSITE-DESIGN-CONCEPT.md`
- **Pencil Mockups**: `/pencil-new.pen` (v3 screens at x:4000)
