# Next Sprint — Website + Infrastructure

## P0 (must do first)

### 1. Merge PR #105 → dev
- Review, merge, tag
- Close PR #104 (superseded)

### 2. Mobile Navigation
- Burger button in header (lg:hidden)
- Off-canvas drawer with nav links
- aria-expanded, focus trap

### 3. Self-host Fonts
- Install @fontsource/space-grotesk + geist-mono
- Remove Google CDN `<link>` tags from Landing.astro
- Update global.css @font-face

## P1 (important)

### 4. Docs Content (33 CLI + 28 MCP pages)
- Auto-generate CLI reference from `forgeplan --help` output
- Auto-generate MCP reference from tool definitions
- Methodology pages: fill stubs with content from docs/guides/

### 5. Trust v2 Full-Width
- Implement concept from TRUST-SECTION-V2-CONCEPT.md
- Full-width rings, cards around perimeter
- Replace current split layout

### 6. CrystallizationAnimation Decomposition
- Extract: usePhysicsLoop hook
- Extract: buildSvgElements factory
- Extract: useAnimationStages hook
- Keep component as thin render shell

### 7. Install: Fix curl|sh
- Full URL: https://github.com/ForgePlan/forgeplan/releases/...
- Add sha256 verification step
- Demote curl|sh below cargo/brew

### 8. Pencil Sync
- Update /pencil-new.pen mockups to match current code
- 7 screens (was 6 in Pencil, now 7 in code)
- Design system tokens sync

## P2 (next sprint)

### 9. PRD-025: Nx Monorepo
- Install nx + @monodon/rust
- project.json for crates + website
- packages/tokens extraction
- CI: nx affected

### 10. Graph: dagre layout
- Replace hardcoded SVG coords with dagre calculation
- Real edge routing (no crossing)

### 11. Deploy
- GitHub Pages or Vercel
- CI: auto-deploy on merge to main
- Domain: forgeplan.dev / forgeplan.sh

### 12. Artifacts Interactive
- Click card → animated preview transition
- Show real artifact example from dogfood

---

*Created: 2026-04-05*
*Based on: session output, audit findings, backlog items*
