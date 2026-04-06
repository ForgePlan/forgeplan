# Forgeplan Website — Design Concept v3

> Final design concept based on QF Network reverse engineering + creative adaptation.
> Pencil mockups: `/pencil-new.pen` (v3 screens, x:4000)

## Visual Philosophy

**Forgeplan = forge for thoughts.** Raw idea → structured artifact → proven decision.

NOT a copy of QF Network. Own visual metaphors:
- **Crystallization** (hero): chaos lines → concentric hexagons = idea → structure
- **Scoring Rings** (trust): octagonal concentric rings = R_eff confidence levels
- **DAG Graph** (graph): real artifact dependency graph with dagre layout
- **Spark Particles** (transitions): forge sparks on pipeline step transitions
- **Depth Layers** (routing): geological cross-section Tactical → Critical

## Design System

### Colors
```
--forge-bg:      #0D0D0D    (deep black)
--forge-fg:      #E8E8E8    (light gray)
--forge-surface: #161616    (card bg)
--forge-line:    #3A3A3A    (borders, grid)
--forge-dim:     #646464    (secondary text)
--forge-ember:   #FF6B35    (accent — ember/forge glow)
--forge-green:   #28C840    (success/evidence)
```

### Typography
```
Headings: Space Grotesk, weight 400, 56-72px (scale = weight, not boldness)
Body:     Space Grotesk, weight 300, 14-15px, line-height 1.6
Mono:     Geist Mono, weight 400-500, 11-13px
Labels:   Geist Mono, weight 500, 10-11px, letter-spacing 3px
```

### Layout Patterns
- Header: grid cells with borders (QF-style)
- Sections: split layout grid-cols (text left, visual right or vice versa)
- Dotted grid background: dashed 1px lines, 30px spacing, opacity 0.1-0.2
- Visible borders everywhere: 1px solid #3A3A3A
- Vertical side labels: rotated 90°, Geist Mono 10px

### Components
- Node (graph): 140×48, Geist Mono 11px, border color = type color
- Card (artifact): border + padding 20×24, title + description
- Terminal: #161616 bg, traffic light dots, Geist Mono 12px
- CTA: text only, "Get started →", no button background
- Theme toggle: 60px circle, half-filled

## Sections (6 screens)

1. **Hero** — Crystallization hexagons + "From Raw Idea To Proven Decision >>>"
2. **Trust** — R_eff octagonal scoring rings + "Trust Is Measured Not Assumed"
3. **Pipeline** — Depth routing (4 levels) + ADI reasoning (3 phases)
4. **Artifacts** — Interactive: 10-type grid (right) + preview with example (left)
5. **Graph** — Real DAG with 12 nodes, edges, legend
6. **Install+Footer** — "Start Where Ideas Are Forged" + cargo/brew/curl + nav grid

## Tech Stack

- **Astro** (SSG) — static HTML, partial hydration
- **Starlight** — docs portal (sidebar, Pagefind search)
- **Tailwind CSS** — utility-first, forge design tokens
- **dagre** — graph layout calculation (Sugiyama algorithm)
- **GSAP** (community) — scroll animations, crystallization morph
- **Space Grotesk** + **Geist Mono** — Google Fonts

## Graph Implementation

Library: `@dagrejs/dagre` (~30KB) for coordinate calculation
Render: Custom SVG in forge palette
Edge routing: Sugiyama algorithm (no edge crossings, proper spacing)
Animation: GSAP ScrollTrigger — edges draw on scroll

## Animation Plan

1. Hero crystallization: chaos lines morph → hexagons (anime.js/GSAP)
2. Section pin + scroll: GSAP ScrollTrigger (community, free)
3. Graph edges: SVG stroke-dasharray animation on scroll
4. Scoring rings: fill animation from center outward
5. Hover wipe: CSS ::before translateX(-100% → 0)
6. CLI typing: vanilla JS typewriter effect

---

*Created: 2026-04-05*
*Pencil file: /pencil-new.pen*
*PRD: PRD-024*
