---
name: forge-diagram
description: Build a single-screen, interactive HTML/SVG "understanding map" of a whole repository for fast recall. Weave together forgeplan artifacts (PRD/RFC/ADR/Epic/Spec + typed links), freeform docs (README, docs/**, loose ADRs), and code structure into one self-contained dark-mode HTML file with clickable nodes and toggleable flow paths. Use when someone wants to "click and understand" how a finished system works — not for design-time diagramming.
disable-model-invocation: true
---

# forge-diagram — repository understanding map

You are producing **one self-contained HTML file** whose only job is to make a
whole project *click fast* for someone returning to it later. This is a **recall
tool for a finished system**, not a design canvas. Light on prose, heavy on a
single high-quality interactive SVG stage.

## What to weave together

The map must reflect the **whole repository**, from three sources at once:

1. **Forgeplan artifacts** — read `.forgeplan/**/*.md` (PRD / RFC / ADR / Epic /
   Spec / Problem). These already carry **typed links** (`refs`, `informs`,
   `based_on`, parent/child). Treat them as authoritative nodes and edges.
2. **Freeform docs** — `README*`, `CLAUDE.md`, `docs/**/*.md`, design notes,
   and any ADRs that live *outside* `.forgeplan`. These fill in intent and
   context the artifacts don't capture.
3. **Code structure** — top-level packages/crates, entry points, the main
   modules and how data/control flows between them.

Reconcile the three: an artifact named in `.forgeplan` should line up with the
code module it describes and the doc that explains it. Where they disagree, the
**code is the source of truth** for what exists; the docs explain *why*.

## Layout

- **Full-screen SVG stage** — one `<svg viewBox=...>` that fills the viewport.
  Not prose-heavy. The diagram is the deliverable.
- **Zones** — group nodes into labelled regions by subsystem / Epic / layer
  (e.g. `CLI`, `Core`, `MCP`, `Storage`, `Web`). Give each zone a title + one
  sub-line of context.
- **Nodes** — components, services, or artifacts. Rounded rects, a title and a
  small sub-label (tech / kind / status). Keep them readable, not crowded.
- **Edges** — typed connections (data flow, control flow, "informs", "depends
  on"). Label the important ones (protocol, relation, payload).
- **Legend** — a small key, placed *outside* every zone boundary.

## Interactivity (this is what makes it worth opening)

- **Flow chips** — a row of toggle buttons (`Everything`, plus 3–6 named flows
  like `Create`, `Live edit`, `Read path`, `Login`). Clicking one highlights and
  animates just that path through the stage; the rest dims. Default `Everything`.
- **Clickable nodes** — clicking a node emphasises it and its direct edges.
- **Animated paths** — marching-ants dashes on the active flow's edges.
- **Dark mode** — hand-rolled CSS variables on `:root` / `html.dark`, a small
  theme-toggle button, `localStorage` persistence, and an apply-before-paint
  script in `<head>` (default to `prefers-color-scheme`). **Never hard-code hex
  inside the SVG** — style it through CSS classes that reference the variables,
  so the diagram follows the theme.

## Style reference

Match the look, density, and interaction model of
`references/architecture-example.html` — a finished example done well:
full-screen SVG stage, zone groups, flow chips that light up and animate request
paths, clean typography, theme toggle. Study it before you draw. Iterate on the
diagram more than anything else.

Suggested palette (map artifact kinds to accent vars, don't hard-code):
`PRD` cyan · `RFC` emerald · `ADR` violet · `Epic` amber · `Spec` rose ·
`code/module` slate. Keep one source of truth per data type visually distinct.

## Language

Write **all human-readable text** — header, zone titles and sub-lines, node
labels, flow-chip names, card copy, tooltips — in the **same language as the
project's primary documentation**, or in the language the caller specifies. Keep
code identifiers and established technical tokens verbatim and untranslated:
crate/package names (`forgeplan-core`), `R_eff`, `ADR-003`, `MCP`, `BGE-M3`,
`fpl`, file paths, relation names (`informs`, `based_on`). Translate the prose
around them, not the tokens themselves.

## Layout discipline (this is the #1 failure mode)

Crowding, overlapping arrows, and text colliding with boxes are what make a map
useless. Enforce all of these:

- **Generous canvas, not a cramped one.** Size the `viewBox` to fit the content
  with wide margins. Whitespace is free; overlap is fatal. When in doubt, make
  the canvas bigger and spread things out.
- **Columnar / layered placement.** Put zones in columns (or rows) at fixed
  x/y positions and align nodes to a consistent grid. Don't free-float boxes.
- **Minimum gaps:** ≥ 40px vertically and ≥ 60px horizontally between any two
  node boxes. Two boxes must never touch or overlap.
- **No overlapping text.** Every label sits fully inside its box or in clearly
  empty space. Sub-label on its own line under the title. If a label is too long,
  shorten or wrap it — never let it run over another element.
- **Clean arrows:**
  - Draw all edges **first** (right after the background grid). Then, under each
    node, draw an **opaque** background rect, then the styled semi-transparent
    node on top — so edges never bleed through or show *behind* a box.
  - Route edges **around** boxes, never straight through them. Prefer orthogonal
    or gently curved paths. Spread edge entry/exit points so many lines don't
    collapse into one illegible bundle. Keep arrowheads clear of borders.
- **Legend and captions outside** every zone boundary, below the lowest element,
  with the viewBox height extended to fit them.
- **Clarity over completeness.** If the graph is dense, show the important
  nodes/edges and summarise the rest. A readable map of 20 things beats an
  unreadable map of 50.

## Output contract

- A single self-contained `.html` document, starting with `<!DOCTYPE html>`.
- Inline SVG, embedded CSS, no external assets except (optionally) one web font.
- No markdown, no commentary around it — the file *is* the answer.

---

_Style derived from the `html-effectiveness` corpus by Thariq Shihipar (MIT) and
the `effective-html` architecture example by plannotator (MIT). This skill adapts
that visual language to whole-repository understanding maps for Forgeplan._
