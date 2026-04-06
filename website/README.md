# Forgeplan Website

Official landing page + documentation portal for Forgeplan.

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

## Related Artifacts

- **PRD-024**: Official Forgeplan Website and Documentation Portal
- **Design Research**: `docs/references/DESIGN-RESEARCH-WEBSITE.md`
- **QF Reverse Engineering**: `docs/references/QF-NETWORK-REVERSE-ENGINEERING.md`
- **Design Concept**: `docs/references/WEBSITE-DESIGN-CONCEPT.md`
- **Pencil Mockups**: `/pencil-new.pen` (v3 screens at x:4000)
