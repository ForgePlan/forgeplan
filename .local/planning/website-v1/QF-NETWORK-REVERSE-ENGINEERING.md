# QF Network — Full Reverse Engineering

> Deep technical analysis of https://qfnetwork.xyz/ extracted via Playwright DevTools.
> Source code, styles, SVG, animations, JS logic — everything needed to reproduce the aesthetic.

---

## 1. Tech Stack

| Component | Technology | Details |
|-----------|-----------|---------|
| Framework | **SvelteKit** | SSR + hydration, `data-sveltekit-preload-data="hover"` |
| CSS | **Tailwind CSS** | Utility-first + CSS Custom Properties |
| Animations | **GSAP 3.13.0** | Bundled in `D9L8phF1.js` (~96KB) |
| Plugins | **GSAP MorphSVG** (paid) | SVG path morphing between states |
| Plugins | **GSAP ScrollTrigger** | Scroll-driven animations with pin |
| Build | **Vite** | Chunk splitting, modulepreload |
| Visual | **SVG only** | No Three.js/WebGL/Canvas |
| Theme | **mode-watcher** | Dark/light with localStorage |
| Font | **Fractul** | Custom, 18 weights (50-900 + italic) |

### CSS Files:
```
0.DSoWlivq.css          — Main styles
ThemeToggle.zHxZYy-o.css — Theme toggle component
FreedomEngineering.Bb0M6iKh.css — Footer section
2.DMUsteX5.css           — Page-specific styles
```

### JS Chunks:
```
D9L8phF1.js — GSAP core + ScrollTrigger + MorphSVG
0.Dl7zVbVC.js — Root layout
Bk_PuRoi.js, Bi9Esu1T.js, etc. — Svelte components
```

---

## 2. CSS Design System

### 2.1 Custom Properties (CSS Variables)

```css
:root {
  --primary-dark: #161616;
  --primary-light: #dadada;
}

/* Dark mode (default) */
.dark {
  --line-color: var(--primary-light);      /* #dadada — text, strokes */
  --border-color: var(--primary-dark);     /* #161616 — borders, bg */
  --theme-color: var(--primary-dark);      /* #161616 — backgrounds */
  --background-color: var(--primary-dark); /* #161616 */
  --scrollbar-bg-color: black;
}

/* Light mode */
:root {
  --line-color: var(--primary-dark);       /* #161616 */
  --border-color: var(--primary-light);    /* #dadada */
  --theme-color: var(--primary-light);     /* #dadada */
  --background-color: var(--primary-light);/* #dadada */
  --scrollbar-bg-color: white;
}
```

**Принцип**: всего 2 цвета + автоинверсия. Один accent `#0051FF` для ссылок. Никаких градиентов.

### 2.2 Tailwind Custom Breakpoints

```css
xs:  400px   /* custom */
s:   550px   /* custom */
sm:  640px   /* Tailwind default */
md:  768px
lg:  1024px
xl:  1280px
2xl: 1536px
```

### 2.3 Border Utility

Ключевой паттерн — `border-border` class:
```css
.border-border {
  border-color: var(--line-color);
}
```
Используется повсюду — header, sections, grid cells. Создаёт "blueprint" aesthetic.

### 2.4 Typography

```css
body {
  font-family: Fractul;
  font-weight: 300; /* Light */
}

h1, h2, h3, h4, h5, h6 {
  font-family: Fractul;
  font-weight: 400; /* Regular */
}

p { line-height: 1.27; }
strong { font-weight: 700; }
```

Hero heading: `font-size: 65px` (JS-calculated), `font-weight: 400`, `text-transform: capitalize`.

### 2.5 Hover Effects (extracted CSS rules)

```css
/* Wipe effect on nav links */
a::before {
  content: var(--tw-content);
  position: absolute;
  inset: 0;
  transform: translateX(-100%);
  background: var(--line-color);
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}
a:hover::before {
  transform: translateX(0);
}

/* CTA button bounce */
.group:hover .group-hover\:translate-x-\[100\%\] {
  transform: translateX(100%);
}
.group:hover .group-hover\:-translate-x-1\/2 {
  transform: translateX(-50%);
}

/* Scale on hover */
.hover\:scale-\[1\.1\]:hover {
  transform: scale(1.1);
}

/* Duration */
.duration-300 { transition-duration: 0.3s; }
.duration-\[400ms\] { transition-duration: 0.4s; }
```

---

## 3. Page Structure (7 sections)

### 3.1 Scroll Architecture

```
body, html → overflow: hidden (нативный скролл отключён)

.content-wrapper {
  position: fixed;
  top: 0; left: 0;
  overflow: auto;
  width: 100vw;
  height: 100vh;
}

.body-content {
  pointer-events: none; /* на контейнере */
  > * { pointer-events: auto; } /* на дочерних */
}
```

Custom momentum scroll через `requestAnimationFrame`:
- Wheel: `friction = 0.85`, `minVelocity = 0.1`
- Touch: 5-sample velocity averaging, 30% overscroll resistance
- Boundary clamping at top/bottom

### 3.2 Header

```html
<header class="fixed top-0 z-[10001] px-4 pt-4"
        style="background-color: var(--border-color); height: 106px; width: 1440px;">
  <div class="border-border grid h-full grid-cols-[90px_1fr_auto_90px] border">
    <!-- Cell 1: Logo (90px) -->
    <div class="border-border flex items-center justify-center border-r w-[90px]">
      <svg><!-- QF logo SVG --></svg>
    </div>
    <!-- Cell 2: Brand name -->
    <div class="flex items-center px-4">
      <svg><!-- QFNETWORK wordmark --></svg>
    </div>
    <!-- Cell 3: Nav grid -->
    <div class="flex">
      <!-- 2x3 grid of nav cells with borders -->
      <a class="... border-b ... overflow-hidden before:absolute before:inset-0 before:-translate-x-full ...">
        Testnet
      </a>
      <a>Github</a>
      <a>Manifesto</a>
      <a>Team</a>
      <a class="border-r">Spin Consensus Protocol</a>
      <a class="border-r">Litepaper</a>
    </div>
    <!-- Cell 4: Theme toggle circle (90px) -->
    <div class="flex items-center justify-center">
      <button class="rounded-full w-[88px] h-[88px]">
        <!-- Half-circle SVG for dark/light toggle -->
      </button>
    </div>
  </div>
</header>
```

**Key**: nav links have `border-b` (top row) and `border-r` creating a grid of cells. Each has `::before` wipe-on-hover.

Mobile: `grid-cols-[76px_minmax(0,auto)_65px]` — logo, space, hamburger.

### 3.3 Hero Section (#hero-section2)

```
Structure:
├── SVG rings animation (absolute center, 73 paths)
│   ├── 42 vertical lines (parallel, opacity 0.5)
│   ├── 16 ellipse/ring paths (cubic bezier curves)
│   └── center dot + connecting lines
├── Bottom split (grid-cols-2, border-t)
│   ├── Left: h1 "The Backbone Of >>> \n>>>Digital Freedom" (65px)
│   └── Right: description text + border-l
└── border-b (1px solid #dadada)
```

Min-height: `794px` (100vh - header). Display: `flex, flex-col`.

### 3.4 Section Layout Pattern

ALL sections follow one of these patterns:

**Pattern A: grid-cols-5 (text 2 + visual 3)**
```
digital-freedom-section:
  md:grid md:grid-cols-5
  ├── col-span-2: text (border-r)
  └── col-span-3: SVG visual
```

**Pattern B: grid-cols-10 (text 4 + visual 6)**
```
built-section:
  lg:grid lg:grid-cols-10
  ├── col-span-4: FAST OPEN BUILT TO SCALE + feature list
  └── col-span-6: isometric SVG illustration
```

**Pattern C: full-width**
```
freedom-engineering-section:
  flex flex-col
  ├── h1 "Start where freedom is engineered"
  └── footer grid (links + social)
```

### 3.5 All Sections

| # | ID | Layout | Content |
|---|----|--------|---------|
| 1 | `hero-section2` | flex-col + grid-cols-2 bottom | Rings SVG + heading |
| 2 | `digital-freedom-section` | grid-cols-5 | "Reclaiming Digital Freedom" + isometric chip SVG (197 paths) |
| 3 | `digital-life-section` | grid-cols-5 | Digital Life pillars |
| 4 | `built-section` | grid-cols-10 | "FAST OPEN BUILT TO SCALE" + features list |
| 5 | `first-sdk-section` | flex-col | "Native-First SDK" + platform cards |
| 6 | `vide-coding-section` | grid-cols-5 | "Vibe Coding" + scroll-driven animation |
| 7 | `freedom-engineering-section` | flex-col | Footer: tagline + nav grid |

---

## 4. SVG System

### 4.1 Rings Animation (Hero)

```
viewBox: "0 -1000 1400 2462"
class: "rings-animation absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2"
Total paths: 73
```

**Composition:**
- **42 vertical lines**: parallel, spaced ~37.5px apart, extending full height
  ```svg
  <path d="M 700.5,-1300 L 700.5,1762" stroke="var(--line-color)" stroke-width="1" opacity="0.5"/>
  <path d="M 737.5,-1300 L 737.5,1762" stroke="var(--line-color)" stroke-width="1" opacity="0.5"/>
  <!-- ...40 more, radiating outward from center -->
  ```
- **16 ellipse rings**: cubic bezier curves forming concentric ellipses
  ```svg
  <path d="M568.74 81.49 C640.89 81.49 699.49 148.35 699.49 231 699.49 313.63 640.89 380.5 568.74 380.5 ..."
        stroke="var(--line-color)" fill="none"/>
  ```
  Each ring has different eccentricity (from nearly circular to very elongated).
- **Center dot**: ellipse at center point
- **Connecting lines**: converging to center

**GSAP MorphSVG** morphs these paths at scroll — ellipses change eccentricity, creating "breathing" effect.

### 4.2 Chevron Arrows (#arrows-svg)

```
viewBox: "133.7142 24.5 474 144"
aspect-ratio: 474/110
18 parallel chevron paths
```

Each chevron:
```svg
<path d="M1 0.5 L97.5 97 L1 193.5" stroke="var(--line-color)" stroke-width="1" fill="none"/>
```
Spaced ~78.5px apart. Used in "Digital Freedom" section as decorative element with infinite horizontal scroll animation.

### 4.3 Isometric Illustrations

**Digital Freedom section**: 197-path isometric circuit board/chip
- viewBox: `0 0 927 810`
- Full-bleed, absolute positioned, centered
- All strokes use `var(--line-color)` — auto-inverts with theme

**Built section**: isometric server rack, tower, gears, speedometer
- Multiple SVG groups that morph between states via GSAP MorphSVG

### 4.4 QF Logo

5-path SVG (527x475 viewBox):
- Top row: horizontal bar with 2 circles
- Middle row: 3 circles
- Bottom row: horizontal bar with 2 circles
- All `fill="var(--line-color)"` — auto-inverts

---

## 5. Animation System

### 5.1 GSAP ScrollTrigger Pattern

Every major section uses this config:
```javascript
gsap.timeline({
  scrollTrigger: {
    trigger: element,
    start: `top-=${headerHeight} top`,
    end: `top+=${Math.max(window.innerHeight, 600)}px top`,
    scrub: true,          // Animation tied to scroll position
    pin: true,            // Section stays fixed during animation
    anticipatePin: 1,
    pinType: "fixed"
  }
})
```

### 5.2 SVG Morph Animations

Hero rings:
```javascript
// Ellipses morph to different eccentricities
.to(ellipsePath, { morphSVG: targetPath, duration: 1 })
// Parallel: dot positions animate
.to(dotElement, { cx: newX, cy: newY, r: newR })
```

Built section:
```javascript
// Complex shapes morph: cube → tower → keyhole → gears → speedometer
timeline
  .to('#cubicInside', { morphSVG: '#tower' })
  .to('#tower', { morphSVG: '#keyhole' })
  // etc.
```

### 5.3 Custom Scroll (Momentum)

```javascript
// Wheel scroll with momentum
bodyContent.addEventListener('wheel', function(e) {
  e.preventDefault();
  contentWrapper.scrollTop += e.deltaY;
}, { passive: false });

// Friction-based deceleration
function animate() {
  if (Math.abs(wheelVelocity) > 0.1) {
    contentWrapper.scrollTop += wheelVelocity;
    wheelVelocity *= 0.85;  // friction
    requestAnimationFrame(animate);
  }
}

// Touch: 5-sample velocity averaging
// Overscroll: 30% resistance at boundaries
// Boundary snap-back on touchend
```

### 5.4 Dynamic Font Sizing

Hero heading font-size calculated via JS:
```javascript
// ResizeObserver watches container width
// Calculates font-size to fill width
// Applied as inline style: style="font-size: 65px"
```

### 5.5 Hover Wipe Effect

Nav links have `::before` pseudo-element:
```css
/* Initial state */
a::before {
  content: "";
  position: absolute;
  inset: 0;
  background: var(--line-color);
  transform: translateX(-100%);
  transition: transform 0.3s;
}

/* Hover state */
a:hover::before {
  transform: translateX(0);
}
```
Creates left-to-right wipe fill on hover.

### 5.6 Theme Toggle

Circle button (88x88px) with half-black/half-white fill.
Uses `mode-watcher` library → toggles `.dark` class on `<html>`.
All colors auto-invert via CSS variables.

---

## 6. Adaptation Strategy for Forgeplan

### What to keep exactly:
1. **2-color system** with CSS variable auto-inversion
2. **Grid header** with bordered cells
3. **Split layouts** (grid-cols-5, grid-cols-10)
4. **Visible borders everywhere** (`border-border`)
5. **Hover wipe effect** on nav links
6. **Theme toggle circle**
7. **Vertical side labels** (rotated text)
8. **Chevron arrows** `>>>` as decorative/flow indicator

### What to adapt:
1. **Rings SVG** → Forge-themed: concentric hexagons (anvil shape) or DAG graph
2. **Isometric illustrations** → Artifact lifecycle, pipeline flow, scoring gauge
3. **Font** → Space Grotesk (free) instead of Fractul (custom)
4. **Accent color** → `#FF6B35` (forge ember) instead of `#0051FF`
5. **Heading content** → "The Forge For >>> Your Ideas" instead of blockchain
6. **Section content** → methodology/CLI/features instead of blockchain features

### What to drop:
1. Custom scroll hijack (not needed for static landing)
2. GSAP MorphSVG (paid) → use anime.js or CSS scroll-driven animations
3. Video loader
4. PWA manifest

### Implementation priority:
1. Static layout (Astro + Tailwind) — grid header, split sections, borders
2. SVG illustrations (custom forge-themed isometric)  
3. CSS hover effects (wipe, scale)
4. Scroll animations (Intersection Observer or GSAP community)
5. Theme toggle (dark/light)

---

*Extracted: 2026-04-05 via Playwright DevTools*
*Source: https://qfnetwork.xyz/*
