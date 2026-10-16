# ForgePlan Design System

Version 1.1 · 14 August 2026  
Labels: **fact** — present in production code; **proposal** — a completed gap; **deprecated** — scheduled for migration.

## 0. Decisions and inconsistencies

1. **The canonical accent is `#FF6B35` (`--forge-ember`).** It is defined in `global.css`, the documentation theme and Tailwind tokens. Its contrast on `#0D0D0D` is `6.85:1`; the blog’s `#FF5A1F` reaches `6.23:1`. `--accent: #FF5A1F` is deprecated.
2. Neither orange works as normal text on the light background: `2.59:1` and `2.85:1`. **Proposal:** use `--forge-ember-text: #C94400` for light-theme text and links (`4.60:1` on `#F5F5F0`). Keep base ember for fills, borders, large graphics and focus rings.
3. **The blog diverges from the contract:** separate `#050505/#FFFFFF` backgrounds, a separate neutral scale, `6px` cards/code blocks and extra accent colours. Treat this as migration debt, not as a second design system.
4. **Radii are normalized.** `0` by default; `2px` for compact labels and micro-controls; `3px` only for inline-code backgrounds; `50%` for avatars and status dots. Remove `4px` and `6px` from shared components.
5. `--header-h` is a state contract: `88px` expanded and `36px` compact.

### Accent migration

1. Add `--forge-ember`, `--forge-ember-text` and `--forge-ember-soft` to the shared token layer.
2. Replace `var(--accent)` by role: text/link → `var(--forge-ember-text)`; fill/border/indicator → `var(--forge-ember)`.
3. Replace `#ff5a1f` literals and fallbacks in `blog-theme.css`.
4. Remove `--accent` after one release cycle. A temporary `--accent: var(--forge-ember)` alias is acceptable.
5. Gate with `rg -n '#ff5a1f|--accent\\b' website/src`.

## 1. Foundations

### 1.1 Palette

| Token | Dark | Light | Purpose | Status |
|---|---:|---:|---|---|
| `--forge-bg` | `#0D0D0D` | `#F5F5F0` | page background | fact |
| `--forge-fg` | `#E8E8E8` | `#1A1A1A` | primary text | fact |
| `--forge-surface` | `#161616` | `#FFFFFF` | raised surface | fact |
| `--forge-line` | `#3A3A3A` | `#D4D4D0` | border/divider | fact |
| `--forge-dim` | `#949494` | `#6B6B6B` | secondary text | fact |
| `--forge-ember` | `#FF6B35` | `#FF6B35` | accent; not body text in light | fact |
| `--forge-ember-text` | `#FF6B35` | `#C94400` | accessible accent text | proposal |
| `--forge-green` | `#28C840` | `#28C840` | source success hue | fact |
| `--forge-error` | `#EF4444` | `#EF4444` | source error hue | fact |
| `--forge-warning` | `#F59E0B` | `#F59E0B` | warning | proposal |
| `--forge-info` | `#60A5FA` | `#60A5FA` | information | proposal |

Semantic colours are not secondary brand accents. They appear only in status context and always have a word or glyph.

### 1.2 Contrast

WCAG thresholds: normal text `≥4.5:1`, large text `≥3:1`; control boundaries and states `≥3:1` against adjacent colours.

| Foreground | Dark bg | Dark surface | Light bg | Light surface | Result |
|---|---:|---:|---:|---:|---|
| theme primary text | 15.86 | 14.77 | 15.91 | 17.40 | AA/AAA |
| theme `dim` | 6.41 | 5.97 | 4.87 | 5.33 | AA |
| theme `line` | 1.71 | 1.59 | 1.36 | 1.49 | divider only; insufficient alone for controls |
| `ember #FF6B35` | 6.85 | 6.38 | 2.59 | 2.84 | text in dark only |
| legacy `#FF5A1F` | 6.23 | 5.80 | 2.85 | 3.12 | still fails light; deprecated |
| `green #28C840` | 8.72 | 8.12 | 2.04 | 2.23 | light text needs `#166534` |
| `error #EF4444` | 5.16 | 4.81 | 3.44 | 3.76 | light text needs `#991B1B` |
| `warning #F59E0B` | 9.05 | 8.43 | 1.96 | 2.15 | light text needs `#92400E` |

`#1A1A1A` on ember is `6.14:1`; white on ember is `2.84:1`. Primary buttons therefore use dark labels. `line` is valid for structure, but hover/control boundaries must strengthen to `dim` or ember.

### 1.3 Type

- `Space Grotesk`: headings, UI and prose.
- `Geist Mono`: commands, code, identifiers, reliability values and metadata.
- Body: `16px/1.55`; docs max `72ch`; blog `18px/1.7`, max `68ch`.
- Heading weight `500–600`; avoid decorative extra-bold display type.

| Token | px | Status | Use |
|---|---:|---|---|
| `xs` | 12 | proposal | metadata |
| `sm` | 14 | fact | compact UI |
| `base` | 16 | fact | body |
| `lg` | 18 | fact | lead/blog |
| `xl` | 20 | proposal | H4/card title |
| `2xl` | 24 | fact | H3 |
| `3xl` | 32 | fact | H2 |
| `4xl` | 40 | fact | document H1 |
| `5xl` | 48 | proposal | page H1 |
| `6xl` | 64 | proposal | landing hero only |

### 1.4 Rhythm and layout

**Proposal:** `0, 4, 8, 12, 16, 20, 24, 32, 40, 48, 64, 80, 96px`. Every value sits on a 4px base; major composition steps use 8px multiples.

- `4–8`: compact label interiors.
- `12–16`: controls and dense rows.
- `20–24`: cards and component sections.
- `32`: fixed desktop gutter; `1280 − 2×32 = 1216px` usable width.
- `48–64`: documentation sections.
- `80–96`: landing sections.
- Below `720px`, the gutter may become `20px` (**proposal**) while all surfaces keep a shared edge.

### 1.5 Borders, radii and height

Use `1px solid var(--forge-line)` for elevation, never a soft shadow. Avoid doubled nested borders. Zero-radius geometry is the baseline.

| Radius | Allowed use |
|---:|---|
| `0` | buttons, fields, cards, tables, blocks, dialogs |
| `2px` | compact label inside a dense row |
| `3px` | inline-code background |
| `50%` | avatar, status dot, spinner |

## 2. Components

### 2.1 Buttons

Every target is at least `44×44px`. `sm` keeps a 44px target while reducing type and horizontal padding; `md` is 44px; `lg` is 48px.

| Variant | Purpose | Rest | Hover/pressed |
|---|---|---|---|
| primary | the one main action | ember + `#1A1A1A` | brightness +8% / −8%, 1px shift |
| secondary | routine action | surface + line | strengthen border to `dim` |
| ghost | low-priority toolbar action | no border | reveal `line` |
| danger | destructive action | transparent + error border/text | subtle error background |

States: `focus-visible` uses a double `2px ember + 2px background` ring; disabled uses opacity `0.48` and no hover; loading keeps its label, adds a spinner, sets `aria-busy="true"` and blocks repeated activation. Icon-only controls require a familiar symbol and an `aria-label`.

### 2.2 Fields

Minimum 44px high, `10×12` padding, mono for technical input. Hover strengthens to `dim`; focus uses ember. Error requires message text, `aria-invalid` and a `!` glyph. Placeholder never replaces label. Distinguish disabled from read-only: disabled is faded; read-only remains legible and carries `READ ONLY`.

### 2.3 Cards

A card groups one entity. Surface + 1px line, 20/24px padding, radius 0. Do not turn every section into a card. An interactive card gets a border hover and one visible text action; its full hit area must have an accessible name.

### 2.4 Tables

| Density | Cell V×H | Type | Use |
|---|---:|---:|---|
| regular | 12×16 | 14 | docs, short comparisons |
| dense | 8×12 | 14 | artifact lists |
| ultra-dense | 4×8 | 12 mono | cheat sheets, logs |

Headers are sticky and opaque. Zebra uses at most a 3% foreground tint and only in wide tables. Row hover cannot be the only selection cue. Numbers align right in mono; identifiers align left in mono. Narrow screens scroll horizontally with a visible boundary; never shrink below 12px. Sorting uses label + arrow + `aria-sort`. Printed rows do not split.

### 2.5 Code

- Inline: mono 13px, `ember-text`, 14% ember background, 3px radius.
- Command: a visually separate `$`; copy without `$`.
- Output: `dim`, no `$`; failures include `ERROR`/`×`, not colour alone.
- Block: surface, line, radius 0, 16/20px padding, `overflow-x:auto`, `white-space:pre`.
- Copy target ≥44px with a text label; switch to `Copied` for two seconds.
- Do not wrap commands by default; prose code may use `overflow-wrap:anywhere`.

### 2.6 Alerts

All alerts use a 3px left border, glyph and explicit title: `i INFO`, `✓ SUCCESS`, `! WARNING`, `× DANGER`. The body background stays neutral. Coloured text uses accessible `*-text` tokens. Alerts do not replace inline field errors. Dynamic results use `role=status`; critical failures use `role=alert`.

### 2.7 Artifact types

**Proposal:** one `[glyph] Label` badge, mono, neutral border:

`P PRD`, `R RFC`, `A ADR`, `S Spec`, `E Epic`, `V Evidence`, `! Problem`, `✓ Solution`, `N Note`, `↻ Refresh`.

Colour may be a secondary channel, but name and glyph remain. Ten saturated type colours would violate the single-accent rule.

### 2.8 Reliability 0.0–1.0

**Proposal:** combine value, named band and segmented bar:

- `0.00–0.24 LOW`
- `0.25–0.49 LIMITED`
- `0.50–0.74 MODERATE`
- `0.75–0.89 HIGH`
- `0.90–1.00 PROVEN`
- `— NOT RUN`: empty hatched bar, never `0.00`.

Use mono with two decimals. Never rely on a red-to-green ramp. `0.00` means a calculated zero; `—` means no calculation.

## 3. Composition

1. One `max-width:1280px; padding-inline:32px` container.
2. Header, sidebar, main content and footer share vertical edges.
3. Build density with tables, rules and type—not floating card clouds.
4. One primary per visible task. Everything else is secondary/ghost.
5. One hot spot per composition; ember does not colour every heading.
6. Sticky elements use `top:var(--header-h)`.
7. The header may animate `88→36px`; reduced motion switches state without intermediate frames.

## 4. Accessibility

- Apply contrast by role, not hex alone. Base status hues cannot be normal light-theme text.
- Never remove the global `:focus-visible` on hover. Recommended: `outline:2px solid ember; outline-offset:2px; box-shadow:0 0 0 4px bg`.
- Minimum target 44×44; the glyph inside may be smaller.
- Type/status/reliability uses word + glyph + optional colour.
- Tables need a caption or accessible name; sorting needs `aria-sort`.
- Loading announces text and `aria-busy`.
- `prefers-reduced-motion:reduce` reduces transitions/animations to `0.01ms`, removes smooth scroll and parallax.
- Skip link is the first focusable control.
- At 200% zoom, the page itself does not scroll horizontally; code/table regions may.

## 5. Print

```css
@media print {
  :root,[data-theme]{--forge-bg:#fff;--forge-fg:#111;--forge-surface:#fff;
    --forge-line:#777;--forge-dim:#444;--forge-ember-text:#8A2F0B}
  *{box-shadow:none!important;text-shadow:none!important}
  body{background:#fff!important;color:#111!important;font-size:10pt}
  nav,.no-print,button{display:none!important}
  a[href^="http"]::after{content:" (" attr(href) ")";font:8pt var(--forge-font-mono)}
  table{display:table;width:100%} thead{display:table-header-group}
  tr,pre,.card,.alert{break-inside:avoid}
  pre{white-space:pre-wrap;overflow-wrap:anywhere}
  @page{size:A4;margin:12mm}
}
```

Do not print the dot grid. Remove coloured fills but keep borders. Do not expose internal-anchor URLs. Repeat table headers on every page.

## 6. Surfaces

| Surface | Changes | Remains fixed |
|---|---|---|
| landing | 5xl/6xl, 80–96px sections, optional dot grid | palette, shared edge, one ember, sharp corners |
| docs | 16px body, 72ch, TOC, code-heavy | tokens, focus, tables, `header-h` |
| blog | 18/1.7, 68ch, longer rhythm | theme background, ember, type, radius 0 |
| cheat sheet | ultra-dense, 12px, A4 | distinguishability, borders, tokens |
| GitHub README | GitHub system fonts, limited CSS | terminology, order, no emoji-only meaning |
| slides | 32–64px, less detail, 16:9 | colour contract, mono IDs/commands, one accent |

README cannot literally reproduce the theme. Use SVG/PNG from `favicon_pack`, Markdown tables, fenced code and short status labels. Do not rely on custom properties.

## 7. Do not

- fire gradients, glow, sparks, glass or soft shadows;
- default rounding or blog `6px` in new shared components;
- pure white as the light page background;
- `#FF5A1F` in new code;
- orange body text on the light theme;
- white text on ember buttons;
- colour without a word/glyph;
- ambiguous icon without a label;
- multiple primaries in one task;
- sticky `top:88px` instead of `var(--header-h)`;
- motion without `prefers-reduced-motion`;
- `line` as the only focus indicator.

## 8. Unified system: modules and consumers (v1.1)

The system is single across every ForgePlan surface and ships as modules:

| Module | File | Contents | Provenance |
|---|---|---|---|
| Base | `tokens.css` | palette, typography, rhythm, radii, layout | site audit 2026-07-25 |
| Neutral steps | `tokens.css` (v1.1 block) | `dim-2`, `dim-3`, `surface-2`, `scrim`, `overlay`, `on-ember` | ported from forgeplan-web, rebased on the canon |
| Graph/map | `tokens.graph.css` | relation edges, canvas strokes, nodes, composed-map zones, dot-grid | ported from forgeplan-web (production @forgeplan/web) |

Best-of merge rules:

1. **The canonical base always wins**: backgrounds `#0D0D0D/#F5F5F0`, ember `#FF6B35`, Space Grotesk + Geist Mono, radius 0, no shadows. forgeplan-web's shadows (`--shadow-card`) and fonts (Inter/JetBrains Mono) were **not** taken.
2. **The edge palette is not a second accent.** It is a data-viz channel for relation kinds (`informs`/`refines`/`contains`/`supersedes`), lives only inside graph scenes, and never appears in regular UI.
3. **The `orch` theme** (pure black + lavender) stays a forgeplan-web private theme, outside the canon.
4. Neutral steps were added because dense graph scenes need more than two levels (`fg`/`dim`); values are contrast-aligned to the canonical backgrounds.

Everything rejected is recorded visually: the `DS / X · Not taken` frame in `design/forgeplan-site.pen` holds a specimen of every not-taken decision with an explanation — the legacy accent, `#050505`, Inter/JetBrains Mono, shadows, 6px radii, the `orch` theme, white text on ember.

### Consumers

| Consumer | Status | Action |
|---|---|---|
| `website/` landing + docs | canonical | none |
| `website/` blog-theme | debt: `#FF5A1F`, `#050505`, 6px radii | migrate per §0 |
| `forgeplan-web` (`@forgeplan/web`) | debt: legacy accent, own fonts, shadows | mapping below |
| Pencil (`design/forgeplan-site.pen`) | canonical: `canon-*` variables, atomic layers | CANVAS source |

### forgeplan-web migration mapping

| Was (app.css) | Becomes |
|---|---|
| `--accent #ff5a1f` | `--forge-ember` (fill/border) / `--forge-ember-text` (text) |
| `--bg #050505`, `--bg-1..3` | `--forge-bg`, `--forge-surface`, `--forge-surface-2` |
| `--fg-1..4` | `--forge-fg`, `--forge-dim`, `--forge-dim-2`, `--forge-dim-3` |
| `--line-1..3` | `--forge-line` (+ `dim` for emphasis) |
| `--font-sans Inter`, `--font-mono JetBrains Mono` | `--forge-font-sans`, `--forge-font-mono` |
| `--shadow-*` | drop: flatness via 1px borders |
| `--on-accent` (light: `#fff`) | `--forge-on-ember #1A1A1A` — dark label always |
| `--edge-*`, `--canvas-*`, `--map-*`, dot-grid | `tokens.graph.css` (same roles, `--forge-` prefix) |
| `orch` theme | stays app-specific on top of the base |

## 9. Contract

The executable reference is `components.html`; CSS contract is `tokens.css`; machine-readable contract is `tokens.json`; A4 references are `cheatsheet.*.html`.

```html
<link rel="stylesheet" href="tokens.css">
<html data-theme="dark">
```

```css
.component {
  color: var(--forge-fg);
  background: var(--forge-surface);
  border: 1px solid var(--forge-line);
  border-radius: var(--forge-radius-0);
}
```
