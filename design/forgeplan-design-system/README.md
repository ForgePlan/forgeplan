# ForgePlan Design System package (v1.1 — unified)

The single design system for every ForgePlan surface (website, forgeplan-web, Pencil, guides, slides). v1.1 merged the best of forgeplan-web into the canon: stepped neutrals + the graph/map token module. See §8 of DESIGN-SYSTEM.*.md for merge rules and consumer migration mappings.

- `DESIGN-SYSTEM.ru.md` / `DESIGN-SYSTEM.en.md` — canonical documentation.
- `tokens.css` — drop-in CSS custom properties for both themes (base + v1.1 neutral steps).
- `tokens.graph.css` — optional module for graph/map/canvas surfaces (edges, canvas strokes, map zones, dot-grid). Load after tokens.css.
- `tokens.json` — machine-readable token contract with fact/proposal/ported status.
- `components.html` — self-contained bilingual component reference with theme switcher.
- `cheatsheet.ru.html` / `cheatsheet.en.html` — A4 landscape references with print styles.
- `brand-assets/` — logo SVGs, favicons, icons.

Open the HTML files directly in a browser. No build step or external dependency is required. The declared font stacks fall back to system fonts when Space Grotesk or Geist Mono are not installed.

Source audit: `website/src/styles/global.css`, `forge-theme.css`, `blog-theme.css`, header/components and the live site, inspected 25 July 2026.
