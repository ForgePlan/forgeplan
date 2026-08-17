# ForgePlan Project Map — MASTER SPEC (handoff brief)

> **Как пользоваться этим файлом.** Это самодостаточное задание. Скопируй его в репозиторий
> `forgeplan-web` (например `docs/PROJECT-MAP-SPEC.md`) и в `marketplace`, открой там Claude Code и
> выполняй по фазам из раздела **§12 MVP**. Агенту НЕ нужен исходный чат — здесь есть всё: видение,
> схема данных, шаблоны, агент, план рендерера на реальных файлах, фазы, уже принятые решения.
> Технические детали (схема, пути, типы) — на английском, чтобы исполняющий агент читал их однозначно.

---

## 1. Что это / Vision

Reusable system: drop it into ANY project that has `forgeplan` + skills. A marketplace **agent
(`cartographer`)** scans the project, classifies its type, extracts a **zone / layer / node / edge**
graph, picks a **pre-designed grid composition**, and writes **one layered JSON** at
`.forgeplan/map/map.json`. **`forgeplan-web`** reads it and renders an interactive, zoned
"understanding map" — easy to study "what is in this system and where". On re-run the agent appends
nodes; the map grows **deterministically** (content-hash IDs, nodes carry no x/y) and animates
organically.

**The non-negotiable bet (do not cut, even in the thinnest slice):**
1. **Layered JSON** that is a **strict superset of forgeplan-web's `{edges}`** model.
2. **Content-hash node IDs** (stable across runs).
3. **Nodes carry NO x/y** — geometry is the output of a pure layout function in the web app.

Everything else is negotiable / phaseable.

---

## 2. The three repos & ownership

| Repo | Owns | Notes |
|---|---|---|
| `github.com/ForgePlan/marketplace` (`~/Work/ForgePlanMarketplace/forgeplan-marketplace`) | the **agent + the contract** (schema, compositions, skills) | new plugin `plugins/forgeplan-map-pack/` |
| `github.com/ForgePlan/forgeplan-web` (`~/Work/ForgePlanWeb`) | the **renderer** (8th graph view) | SvelteKit + Svelte 5 runes, Feature-Sliced Design |
| `github.com/ForgePlan/forgeplan` (core/CLI) | **nothing mandatory** | optional thin `forgeplan map build/confirm` shelling the agent |
| spike `dev/forge-understand/spike/index.html` | **reference ground-truth** | layout(), tokens, curve(), minimap, run.mjs proven |

Emitted file `<target>/.forgeplan/map/map.json` is **gitignored like `lance/`** — derived, re-emittable.

---

## 3. forgeplan-web grounding — EXTEND, do not reinvent (verified)

`forgeplan-web` is already a mature graph frontend. It renders the forgeplan artifact graph with
**7 views** in `template/src/widgets/dependency-graph/ui/`: `ForceView`, `RadialView`, `TreeView`,
`SunburstView`, `MatrixView`, `LanesView`, `SankeyView` — plus layout libs (`tree-layout.ts`,
`sunburst-layout.ts`, `sankey-layout.ts`, `force-cluster-repel.ts`, `cluster.svelte.ts`), a view
picker, a11y/error boundaries, dense-graph resilience (100 artifacts), and a **Mosaic** multi-graph
dashboard (`widgets/mosaic/`, split-tree, drag, persist; RFC-015).

**Verified facts (load-bearing — confirmed against the real code):**
- Data model is minimal: `entities/graph/model/types.ts` → `GraphResponse { edges: GraphEdge[] }`,
  `GraphEdge { from, to, relation }` (exactly 3 fields), fetched from `/api/graph` via a poller.
  **Nodes are implicit; there is NO zone/layer/grid concept.**
- In `HomePage.svelte`: `liveNodes` comes from `listPoller`, `liveEdges` from `graphPoller` —
  **DIFFERENT sources.** The 7 views consume `ArtifactSummary {id,kind,status,title}`, NOT a generic
  node. → **A map node is NOT a drop-in for the 7 views.**
- In `DependencyGraph.svelte` **line 155** the `{:else}` falls through to `LanesView`. Selecting an
  unregistered view would silently render Lanes (PROB-035/039 silent-failure class).
- The view picker validates against a **`GRAPH_VIEW_IDS` Set** (≈ line 43); `paneView` persistence
  rejects unknown ids. So a new view must be registered in **THREE** places: the `GraphView` union
  (lines 34-41), the `GRAPH_VIEWS` array (line 19), and the `GRAPH_VIEW_IDS` Set (line 43).
- The 7 views use **d3-zoom transform on SVG**; the spike uses **manual scroll/scale on a `<div>`**.
  These differ → the interaction layer is a **rewrite onto d3-zoom**, not free reuse.

→ Our system adds a **new 8th "map" view** that reads `/api/map` **exclusively** and owns its own
`MapNode` type. The compatibility guarantee with the 7 views is **edges-only** (and edges ARE a
byte-exact superset of `GraphEdge`).

---

## 4. The layered JSON schema — `forgeplan.map/v1`

Validated by `plugins/forgeplan-map-pack/schemas/map.schema.json` **and** the TS `MapResponse`.
**LOAD-BEARING INVARIANT: nodes carry NO x/y.** Geometry = pure-fn output in the web app.

```jsonc
// .forgeplan/map/map.json
{
  "schema": "forgeplan.map/v1",

  // L0 identity + cache key
  "meta": { "map_id":"uuid", "status":"proposed",          // proposed | confirmed
            "project_type":"rust-cli-mcp", "composition_id":"rust-cli-mcp",
            "source_fingerprint":"sha1:...",                // unchanged → no-op refresh
            "version":3, "agent_run":"run-7" },             // seeds force sub-layout

  // L1 grid POLICY (knows nothing of zones/nodes). MVP: cols=1 stack-ttb only.
  "canvas": { "grid":{"cols":2,"rows":4}, "col_weights":[1,1],   // weights = PHASE 2
              "gap":{"x":88,"y":70}, "margin":40,
              "cell":{"card_w":190,"card_h":60,"card_gap":36,
                      "zpad":{"top":50,"side":24,"bottom":24}} },

  // L2 composition = Open/Closed seam. New project type = new block; nodes untouched.
  "composition": { "template":"rust-cli-mcp", "arrangement":"stack-ttb",
                   "entry_zone":"z.surfaces",
                   "placements":[ {"zone":"z.surfaces","cell":{"row":0,"col":0}},
                                  {"zone":"z.decisions","cell":{"row":3,"col":0,"col_span":2}} ],
                   "zone_connectors":[ {"from":"z.surfaces","to":"z.write","label":"commands"} ] },

  // L3 zones — identity + look + per-zone layout. accent = TOKEN name, never hex.
  "zones":[ { "id":"z.write", "label":"Core", "sub":"the single write path",
              "kind":"core", "accent":"emerald", "altitude":"container",   // C4
              "treatment":"neutral-dashed", "rule_edge":"off", "layout_rule":"grid",  // §16: neutral, NO rainbow/left-rule
              "cols":2,                          // PINNED, NOT ceil(n/3) — see §10 H1
              "layers":["l.write.out"] } ],

  // L4 layers — OPTIONAL drill-down band (omit for flat maps). PHASE 2.
  "layers":[ {"id":"l.write.out","zone":"z.write","label":"egress","order":1} ],

  // L5 nodes — NO x/y. ComposedMap OWNS this type (NOT shared with the 7 views).
  "nodes":[ { "id":"n_projection",                  // sha1("gate:"+path)[:12] — content-hash
              "label":"projection — write gate", "kind":"gate",
              "zone":"z.write", "layer":"l.write.out", "meta":"core · ADR-003",
              "status":"active", "r_eff":0.8, "artifact_id":"ADR-003",
              "provenance":{"source":"code","ref":".../projection/mod.rs","confidence":0.95},
              "found_at":"2026-06-22T10:00:00Z",    // append sort key (stability)
              "is_new":false },                      // true on append → animate-in
            { "id":"mn_core", "label":"Core", "kind":"mega", "zone":"z.write",   // MEGA-NODE: aggregates a cluster
              "is_mega":true, "children":["n_projection","n_routing"], "collapsed":true } ],
            // mega-nodes power C4 rollup (L0 Context shows mega-nodes → click expands to the sub-graph).
            // Guardian checks every child ∈ nodes and no nesting cycles.

  // L6 edges — STRICT SUPERSET of GraphEdge {from,to,relation} (the 3 fields verified).
  // Drop the extra keys → exactly today's GraphResponse.
  "edges":[ { "from":"evidence", "to":"ADR-003", "relation":"supports",   // ∈ 11 VALID_RELATIONS
              "namespace":"typed-link", "trust":"high" },                  // additive
            { "from":"projection", "to":"lancedb", "relation":"syncs",
              "namespace":"code-dep", "trust":"medium",
              "verified_by":"grep:use forgeplan_core@.../server.rs" } ],

  "flows":[ {"id":"f.create","name":"Create artifact","node_ids":["n_..."]} ],
  "increments":[ {"version":3,"added_node_ids":["n_..."],"stale_node_ids":[]} ]  // PHASE 2
}
```

**SOLID mapping:** L1 grid = policy (no zone knowledge). L2 composition = Open/Closed seam (new
project type = new data block, nodes untouched). L3 zones = the abstraction nodes depend on
(Dependency-Inversion: nodes name a `zone`, never coordinates). L5 nodes = single source of node
identity, owned by the map view (Interface-Segregation: the 7 views never see it). L6 edges = Liskov
at the document level (a `forgeplan.map` edge with its extra keys dropped IS a `GraphEdge`).

**Backward-compat default (lives ONLY in `entities/map/`, never touches `entities/graph/`):** an edge
without `namespace` ⇒ `typed-link` if `relation ∈ VALID_RELATIONS` (`informs, based_on, supersedes,
contradicts, refines, supports, demonstrates, covers, triangulates, references, belongs_to`), else
`code-dep`.

---

## 5. Data flow

```
TARGET PROJECT (any repo) ─ manifest · src tree · .forgeplan/ · git log
        ▼
AGENT «cartographer»  (marketplace: forgeplan-map-pack)  EMITTER profile
   SCAN ─► TYPE ─► EXTRACT ─► SELECT ─► EMIT
   glob   score   zones/      pick      assemble + validate(3 guards) +
   grep   formula layers/     template  content-hash IDs → status:proposed
                  nodes/edges  place
        │  Write (ONLY .forgeplan/map/map.json)
        ▼
.forgeplan/map/map.json  ← LAYERED JSON (the contract; gitignored, derived)
        │  GET /api/map (readFile)
        ▼
forgeplan-web (SvelteKit · FSD · Svelte 5)
   entities/map/api/store.ts  mapPoller = createPoller<MapResponse>('/api/map')
        │  $derived.by(() => computeComposedLayout(map))   ← PURE fn (port of spike layout())
        ▼  nodes get x/y HERE
   widgets/composed-map/  ComposedMap = 8th view (inserted BEFORE line 155 LanesView fallthrough)
        ▼
   INTERACTIVE ZONED MAP — drift · click · flows
        └── re-run: cartographer --refresh → re-emit whole file
            (content-hash IDs identical where unchanged) → poll → FLIP redraw (Phase 2)
```

---

## 6. Composition template system

A composition = `(canvas + zones[] + arrangement + zone_hints)` as **DATA, not code** → adding a
project type = dropping a YAML (the Open/Closed payoff). Lives at
`plugins/forgeplan-map-pack/compositions/*.yaml`; forgeplan-web embeds the **same set as a typed TS
const** (mirroring how `GRAPH_VIEWS` is a const — no runtime fetch).

**MVP ships 3 templates** (full library ≈16; each new one must be PULLED by a real repo, not pushed):
- `rust-cli-mcp` (`stack-ttb`) — **dogfood + CI fixture**: emitting on ForgePlan reproduces the spike
  grid. Detected by `.forgeplan/` + `crates/` + `rmcp` (conf 1.0).
- `web-fullstack` / `sveltekit-fsd` (`stack-ttb`) — second dogfood. Detected by `entities/` +
  `widgets/` + `pages/` dirs.
- `generic` (weighted-grid fallback) — `score<0.40` → one zone per top-level dir, cap 8. The floor
  that **always renders something**.

**Selection = pure fn `signals → (template, confidence)`, no LLM:**
```
score = Σ strong·0.40 + Σ weak·0.15 − Σ negative·0.50   (clamp 0..1)
 ≥0.70 & gap≥0.20 → SINGLE high-conf
 ≥0.70 & gap<0.20 → BLEND (host + grafted zones)         ← PHASE 2
 [0.40,0.70)      → SINGLE low-conf → status NEEDS_CONFIRM
 <0.40            → generic fallback
ALWAYS: .forgeplan/ present → append z.decisions zone to whatever won.
```
Conditional zones (e.g. `z.external`) drop when empty; neighbour `col_span` auto-grows. Every node
has a `default: z.core` home so nothing is unplaced.

**Honest caveat:** ForgePlan itself is a hybrid (Rust-CLI-MCP **with** a SvelteKit sibling) → it trips
the `[0.40,0.70)` band on the flagship demo target. MVP acceptance ("reproduces spike grid on
ForgePlan") therefore relies on a **hand-tuned `rust-cli-mcp` template**, not on the selector. Blend
is the real fix (Phase 2). The `generic` floor guarantees correctness regardless.

---

## 7. Agent pipeline (marketplace)

New plugin `plugins/forgeplan-map-pack/` mirroring the verified `forgeplan-brownfield-pack` layout:
`.claude-plugin/plugin.json` + `agents/` + `skills/` + `compositions/` + `schemas/` + `playbooks/` +
`mappings/`.

**ORCHESTRATED (BMAD/SPARC/smith-style) — see the pack `README.md`.** A `map-orchestrator` directs
role-agents each in their OWN context (scanners → typer → selector → `zone-extractor` →
`edge-verifier` → `map-emitter`) with a **`map-guardian`** validating (schema + 3 invariants +
zone/node/mega-node/edge integrity) before the file flips `proposed → confirmed`. After validation the
web just renders — clean, because everything is validated. `cartographer` below is the umbrella name;
its stages are the separate agents above. All keep the EMITTER profile.

**Agent `cartographer` — EMITTER profile** (inverse of brownfield's reader profile):
- **Allowed:** `Read, Glob, Grep, Write` + read-only MCP (`forgeplan_graph/list/get`).
- **Denied:** `Edit` + ALL graph mutators (`forgeplan_new/update/link/activate/delete`).
- **Write target:** EXACTLY one file — `.forgeplan/map/map.json`.
- → This makes **RED-LINE #11 structurally impossible to violate** — the agent literally cannot call
  the mutators that desync LanceDB / markdown. The map is a **derived read-only view** → no ADR-003
  violation.

**Skills — 3 in MVP** (inline `project-typer` + `composition-selector` as ~40-line scoring fns; the
real boundaries are scan vs grep-gating vs assembly):
- `zone-extractor` — maps dirs/modules/artifact-kinds → zones via the chosen composition's
  `zone_hints`; content-hash IDs `sha1(kind+":"+path_or_slug)[:12]` (path/slug-based, NEVER name).
- `edge-verifier` — splits edges into 2 namespaces: `typed-link` from `forgeplan_graph` (high trust,
  `relation ∈ VALID_RELATIONS`); `code-dep` requires a Grep pass recording `verified_by`; unverified
  code-dep is **DROPPED**, not emitted as noise.
- `map-emitter` — assembles JSON, runs **3 invariant guards** (cell-overlap; every edge endpoint ∈
  nodes; every `node.zone ∈ zones`), emits `status:proposed`, atomic tmp-rename write.

**Determinism spine (the core bet):** content-hash IDs + `source_fingerprint` cache (sha1 of sorted
mtimes; unchanged → no-op) + append-only (vanished node → `status:stale`, never deleted). MVP
**re-emits the whole file each run** — because IDs are content-hashed and layout is pure, a re-emit IS
identical where nothing changed → determinism free; defer the fly-in (`map-differ`) to Phase 2.

**Human gate:** always emit `status:proposed` + sentinel `<<NEEDS_CONFIRM: N zones, M nodes, K edges
(J grep-verified)>>`. forgeplan-web renders proposed maps with an "unverified" ribbon; a human flips
to `confirmed` (web PATCH `/api/map` or `forgeplan map confirm`). The gate **informs, never blocks**.

**Headless invocation** (proven by spike `run.mjs`):
`claude -p <prompt> --add-dir <repo> --allowedTools Read Glob Grep Write`.

---

## 8. Renderer design (forgeplan-web) — the 8th view

Does NOT replace any of the 7 verified views.

- **Node type:** `ComposedMap` OWNS its `MapNode` and reads `/api/map` exclusively — **never shares
  nodes** with the 7 views (they use `ArtifactSummary` from a different poller). No adapter. The
  compatibility guarantee is **edges-only** (byte-exact `GraphEdge`).
- **Registration (3 places):** insert `{:else if view === 'map'}` **BEFORE line 155** in
  `DependencyGraph.svelte` (else Map silently renders Lanes); add to the `GraphView` union (34-41),
  the `GRAPH_VIEWS` array (19), and the `GRAPH_VIEW_IDS` Set (43).
- **Interaction layer:** rewrite onto **d3-zoom** (the spike's scroll/scale-the-SVG differs) — real
  work, budgeted, not free reuse.

**FILES CHANGED (additive, non-breaking):**
- `entities/graph/model/types.ts` (+`MapResponse` subtypes)
- `shared/config/ui-prefs.ts` (union + array + Set, ~3 lines + icon)
- `widgets/dependency-graph/ui/DependencyGraph.svelte` (`{:else if}` before 155)
- `pages/home/ui/HomePage.svelte` (start `mapPoller`, `$derived` layout)
- `app/styles/app.css` (`--zone-accent-*` / `--zslab-*` tokens, ported verbatim from spike
  `:root`/`html.dark`)

**FILES NEW:**
- `entities/map/api/store.ts` → `mapPoller = createPoller<MapResponse>('/api/map')`
- `routes/api/map/+server.ts` → `readFile`, 404 → `{}`, no new deps
- `widgets/composed-map/model/layout.ts` → `computeComposedLayout` **PURE fn** (direct port of spike
  `layout()` lines 332-348; runs in `$derived.by`; unit-tested)
- `widgets/composed-map/model/zone-layout.ts` → per-`layout_rule` strategies, each reusing a lib:
  `grid`=spike · `lanes`=LanesView · `radial`=`cluster.svelte.ts` · `dag`=`tree-layout.ts` ·
  `force`=`force-cluster-repel.ts` (seeded+bounded) · `graph`=spike `DEC_POS`
- `widgets/composed-map/ui/{ComposedMap,ZoneSlab,NodeCard,EdgeLayer,FlowChips,ZoneConnector,ComposedPanel,EntryAnchor}.svelte`
  (reuse `Minimap.svelte` unchanged; d3-zoom rewritten)

---

## 9. Organic re-layout

**Determinism is the foundation; FLIP animation is the organic layer on top (Phase 2).**

- **Stability:** nodes placed by `(zone, layer, found_at→id)`. Appended nodes sort to the end of
  their zone; the zone grows downward with **PINNED `cols`** (added to L3 schema). The macro-grid
  recomputes column widths/row heights, but zones that didn't grow keep their (x,y); only the grown
  zone and zones below it in the same column shift. **The other column is fully stable.**
  - *Why pinned cols (H1 fix):* the spike's `zonePlacement()` used `cols=ceil(n/3)`, which reshuffles
    a zone's grid every time node count crosses a multiple of 3. Pinning `cols` per zone makes
    append-stability hold by construction.
- **Animation (Svelte 5):** keyed `{#each layoutNodes as n (n.id)}` + `animate:flip` (transforms
  between two deterministic layouts, zero measurement). `is_new` nodes → `in:fly={{y:-24}}` +
  `in:scale`. Zone `<rect>` w/h → CSS transition. ALL gated by `prefersReducedMotion()` → 0ms snap.
- **Streaming:** `mapPoller` (8s) compares `meta.version`; growth → mark new ids from
  `increments[-1].added_node_ids`, recompute (pure, instant) → one `$derived` cycle → all animations
  same frame. Pan/zoom preserved (no auto-fit); Minimap via `onViewState`.
- **Accepted failure:** zone reclassification (a node moved to a different zone — a *semantic* change)
  is NOT bridged by FLIP → instant `in:fly` into the new zone. Intentional: a semantic change should
  be re-read, not smoothed over.

---

## 10. Key tradeoffs (decided)

- **Edge superset is real & free; node superset is NOT** → ComposedMap owns `MapNode`, reads
  `/api/map` only. Compatibility is edges-only, and that's enough.
- **Explicit nodes duplicate edge endpoints** → a validator asserts every edge endpoint ∈ nodes (same
  class as `adr_003_invariant.rs`). Upside: edgeless components become visible (the point) — renderer
  must handle a node with no edges.
- **Determinism > organic motion** → content-hash IDs + pure layout + pinned cols are the bet;
  animation is Phase-2 polish. The differ is cut from MVP (re-emit is already deterministic).
- **Token-only colors** (`accent` = token name, never hex) → dual-theme correctness > expressivity;
  matches forgeplan-web's hard `var(--…)` rule.
- **Classification misfires on hybrid repos are the NORM** → generic floor saves correctness; demo
  target needs a hand-tuned template; blend is Phase 2. Honest, not hidden.
- **`layout_rule:graph`/force inside a grid cell** = the one non-deterministic island → seed force per
  `agent_run`, bound to the zone box, cap nodes; fall back to spike `DEC_POS` if ugly.
- **d3-zoom interaction layer is a REWRITE**, not free reuse — named & budgeted.

---

## 11. Decisions already made (forks resolved; ★ = chosen)

1. **Node-type sharing** → ★ ComposedMap owns `MapNode`, reads `/api/map` only (no adapter).
2. **Incremental append in MVP** → ★ Re-emit whole file; defer differ + animation to Phase 2.
3. **Composition library size at launch** → ★ 3 (`rust-cli-mcp`, `web-fullstack`, `generic`).
4. **Virtual grid in MVP** → ★ Fixed single-column `stack-ttb` (configurable weighted grid = Phase 3).
5. **Brownfield coupling** → ★ Native scan; brownfield discover as optional Phase-2 fast-path.
6. **Skill granularity** → ★ 3 skills (`zone-extractor`, `edge-verifier`, `map-emitter`); inline the
   2 scorers.

---

## 12. MVP slice (~5–7 days, ONE vertical: agent → JSON → web renders a zoned map)

**Day 1-2 — Contract + render path, NO agent:**
1. `map.schema.json` + TS `MapResponse` (thin: drop `layers`, `col_weights`, `responsive`,
   `increments`; `provenance`→`{source,ref}`; PIN `zone.cols`).
2. `computeComposedLayout()` pure fn, unit-tested (port spike lines 332-348; **fixed stack-ttb
   single-column** — no virtual grid).
3. `ComposedMap.svelte` + `ZoneSlab` + `NodeCard` + `EdgeLayer` (static, no animation).
4. Register `"map"` in the `GraphView` union + `GRAPH_VIEWS` + `GRAPH_VIEW_IDS` Set;
   `{:else if view==='map'}` **BEFORE line 155**; `/api/map` readFile route; port spike tokens.
5. **HAND-WRITE** `map.json` for ForgePlan (spike IR re-keyed) → validates the ENTIRE render path
   with ZERO agent. ◄ **proof-of-render checkpoint.**

**Day 3-5 — Agent emits the same shape:**
6. `forgeplan-map-pack` skeleton + `plugin.json` + `cartographer` EMITTER brief + 3 skills
   (`zone-extractor`, `edge-verifier`, `map-emitter`); inline typer/selector; native scan; emits
   `status:proposed`.
7. The 3 invariant guards (cell-overlap; edge-endpoint ∈ nodes; node.zone ∈ zones).

**ACCEPTANCE:**
- `cartographer` on **ForgePlan** reproduces the hand-written spike grid.
- on **ForgePlanWeb** → a sane `web-fullstack` map.
- on a **no-manifest dir** → a non-empty `generic` map.
- **re-emitting after adding a node keeps existing node positions identical** (content-hash + pure fn;
  no animation needed to prove it).

**NON-NEGOTIABLE even in the thinnest slice:** layered JSON as superset of `{edges}`; content-hash
IDs; nodes carry no x/y. **Cut anything else first.**

---

## 13. Phased plan beyond MVP

- **Phase 2 (organic + O/C payoff):** `map-differ` incremental append + `animate:flip` + `in:fly/scale`
  + fingerprint cache + DriftBadge; extract `project-typer` + `composition-selector` as real skills;
  `layers` drill-down; remaining ≈13 compositions (each pulled by a real repo); blend mode for hybrid
  repos; grep-gated code-dep edges for JS/TS/Python; d3-zoom interaction rewrite; `forgeplan map
  confirm` CLI; brownfield discover fast-path via `discover-to-map.yaml`.
- **Phase 3 (scale + multi):** monorepo mosaic split-tree recursion (reuse RFC-015); microservices
  generative grid `cols=⌈√N⌉`; map as a pane in the mosaic dashboard; composition-override UI; flow
  editor; the configurable virtual grid (`col_weights`/`row_weights`/responsive) — earns its keep
  only here.

---

## 14. Reference — what the spike already proves

`dev/forge-understand/spike/index.html` (served via `python3 -m http.server`) is the visual + logic
ground-truth:
- pure `layout()` (lines 332-348) → ports ~verbatim into `computeComposedLayout`.
- zone-slab CSS tokens (`:root` lines ~22-40) → the token ground-truth for `app.css`.
- `curve()` (typed bezier edges), Minimap, flow-chips, drag-pan, ctrl/⌘-scroll zoom, light/dark
  (Anthropic cream/clay/olive), Decision-trail as a PRD-centred graph (`DEC_POS`).
- `run.mjs` → proves the EMITTER headless invocation (`claude -p … --allowedTools Read Glob Grep Write`).

Variety reference for future compositions: `dev/effective-html/skills/html-diagram/references/
html-effectiveness/` (20 distinct artifact "views").

---

## 15. UX, interaction & content requirements (decided with the user on the spike)

First-class requirements for the `composed-map` view (proven in the spike), NOT optional polish:

**Navigation:**
- **Drag-to-pan** the canvas (grab/grabbing cursor); a click that didn't move must still select
  (suppress the click after a drag > ~3px).
- **Zoom via scroll**: `Ctrl/⌘ + wheel` zooms at the cursor; plain wheel/trackpad **pans**. (NOT
  click-to-zoom-into-a-zone — the user explicitly rejected that.)
- **Minimap** bottom-left (reuse `Minimap.svelte`), viewport rect synced to pan/zoom, click-to-jump.
- **Esc / click on empty canvas** → reset (clear selection, zoom→1, scroll home).
- Smooth panning (drag follows the cursor 1:1).

**Click-to-detail (right panel `ComposedPanel.svelte`):**
- **Click a zone** (empty area or its title) → panel shows the zone label + sub, a full **description**
  (what this layer is, what's inside, how it interacts with the other layers), and a **"What's
  inside" list** of its nodes. Selected zone → clay border highlight.
- **Click a node** → panel shows label + meta + a full **description**, and an **auto-derived
  "Connections" list** (`→ target (relation)` / `← source (relation)`) computed from `edges` —
  never hand-written, always accurate.

**Flows:** flow chips toggle one end-to-end path (`flows[]`): dim everything, light the path's
nodes+edges, animate the lit edges, show the step caption.

**Drift:** a "what changed since last run / since main" badge marking changed nodes. The recurring
reason to reopen the map.

**Content / language rule (USER REQUIREMENT):**
- **Card + zone labels: ENGLISH**, verbatim like the code/source (`projection — write gate`,
  `Surfaces`, `R_eff`, `ADR-003`, crate names).
- **Right-panel descriptions: RUSSIAN**, neutral/accessible tone, **minimum anglicisms** (plain
  Russian; keep only essential technical tokens).
- This is a property of the emitted JSON: `label`/`meta` are EN; the node/zone **description** fields
  are RU. The emitter writes both.

**Dropped (do NOT build):** the **lens / heatmap overlay** (R_eff / freshness / blindspots tinting) —
the user found it uninformative. Drop `LensSelect`.

## 16. Zone visual — the FINAL decision (restraint)

The user's explicit final call: **zones must be NEUTRAL and calm — NOT colorful.**
- Zone background: a **subtle neutral fill** (`var(--zone)`) + a **dash-dot neutral border**
  (`var(--zone-line)`); serif title in `var(--ink)`; mono sub in `var(--muted)`; selected → clay
  border.
- **DO NOT** tint each zone with its accent color (the rejected `--zslab-<accent>` "rainbow") and
  **DO NOT** draw a per-zone left/top accent rule bar. Both were explicitly rejected
  ("разноцветное говно с бордюром слева").
- Color is **sparing**: kind-colored borders ONLY on Decision-trail artifact cards (PRD cyan / RFC
  emerald / ADR violet / Epic amber / Spec rose / Problem orange / Note slate / Evidence olive) + two
  semantic specials (`gate`=clay projection, `truth`=olive markdown). Everything else = neutral
  `var(--line)` border.
- → In the schema `zone.accent` exists but the renderer uses it **only** for a faint hover/selected
  hint, NEVER a full fill or rule bar. Default `treatment` = `neutral-dashed` (NOT `slab`);
  `rule_edge` off by default.
- Palette = Anthropic cream/clay/olive (spike `:root`/`html.dark`), token-only (never hex), light+dark.

## 17. Onboarding LAYOUT + conversational agent (the user's "chat") — Phase 2/3

A **separate onboarding LAYOUT** in forgeplan-web (NOT mixed with the standard artifact views) — its
job is to walk a newcomer through the project via the map:
- Renders the composed-map and **onboards in a navigation + animation mode**: a guided tour that moves
  the camera zone-by-zone, reveals the reading spine, and narrates "what is this system, where does X
  live, how does the Create flow work" — grounded in the map + the forgeplan artifacts.
- A **chat panel** (`widgets/map-chat/`) where an agent answers questions and, on demand ("research
  this deeper" / click a zone/node), **dispatches a scoped deeper scan** that **appends to `map.json`**
  → the canvas re-renders organically.
- **Drill into detail → the standard layer:** when the user wants raw artifacts, they jump to the
  existing 7 graph views / artifact pages. The onboarding layout is the high-altitude guide; the
  standard views are the detail.
- **Data prep by a local headless agent:** onboarding data is produced by a local headless agent
  (Claude Code, `claude -p` in the same directory) — i.e. the `forgeplan-map-pack` orchestrated
  process (see the marketplace README). The layout consumes the validated `map.json`.
- Architecture: the chat agent uses the same **EMITTER contract** (append to `map.json` only, never
  mutate forgeplan artifacts). Backed by `claude -p` / Agent SDK / a deep-agent runtime (e.g. Mastra).
  **Phase 2/3** — after the static map + emitter are solid. This is the user's headline "next big thing".

## 18. Arrangement for comprehension ("what's in the system & where")

The arrangement is the comprehension layer, not just aesthetics. Principles the templates encode:
- A clear **entry anchor** (`entry_zone`, top-left, `EntryAnchor.svelte`) — where the eye starts.
- A **dominant reading spine** matching the project's main flow (ForgePlan: Surfaces → Core →
  Storage).
- **Related zones adjacent** (keep `zone_connectors` short); **optional/peripheral zones at the edge**
  (`External` last); `z.decisions` as a wide band.
- Zone→zone relationships shown via labelled **`zone_connectors`**, not node spaghetti, at the
  overview altitude.
- Arrangement strategies (one per `arrangement` value): `stack-ttb` (layered top→bottom),
  `pipeline-lr` (left→right by lifecycle), `hub` (core-center + satellites), `lanes` (bands),
  `z-reading` (entry top-left + spine). **MVP = `stack-ttb`**; others are Phase-2 compositions.

## 19. Flexible composition grid engine (JSON-driven, CSS-grid-like, idempotent)

The whole layout is **one pure function** `computeComposedLayout(canvas, composition, zones, nodes)
→ positions`. It borrows the **mental model of CSS Grid + flex-wrap** but computes x/y itself
(no DOM reflow) → fully **idempotent** (same JSON → byte-identical positions). **Build order is
grid-first:** the agent gives layers/zones/nodes/edges; the engine first lays the GRID + COMPOSITION,
then places nodes into the prepared zones, then routes edges relative to those positions.

Three nested grid levels:
1. **Macro grid (`canvas`):** `grid {cols,rows}` + `col_weights`/`row_weights` (fractions, like
   `grid-template-columns: 1fr 2fr`) + `gap` + `margin`. Zones placed into cells via
   `composition.placements[].cell {row,col,col_span,row_span}` (like `grid-area`). Track size =
   max of the zones in that track (auto-sizing).
2. **Zone sub-grid:** inside a zone, nodes flow into `zone.cols` (pinned) columns, **wrapping** into
   more rows as needed (like `flex-wrap` / `grid-auto-flow`); the zone grows in height to fit.
3. **Capacity + overflow (the "fixed count → spill to next zone" rule):** a zone may declare
   `capacity` (max nodes). `overflow` strategy per zone:
   - `grow` (default) — no cap; wrap into more rows; the zone grows.
   - `spill` — cap at `capacity`; extra nodes flow into a **continuation zone** (the next zone in
     reading order) — like CSS regions/columns overflow.
   - `collapse` — extra nodes fold into a **mega-node "+N more"** that expands on click.

**Idempotency:** pure fn + content-hash IDs + stable sort `(zone, layer, found_at→id)` + no
measurement → appending a node is a minimal delta; everything else keeps its position (the
determinism bet, §1).

**Why not real CSS Grid in the DOM:** we compute positions ourselves so we control determinism, edge
routing, zoom, FLIP animation, and SVG export. We take CSS Grid's *model*, not its runtime. Lives in
`widgets/composed-map/model/layout.ts`, runs in `$derived.by`, unit-tested. MVP ships the
single-column `stack-ttb` path of this same engine; weighted multi-column tracks + `spill` are Phase
2/3 — but the engine's SHAPE is designed for them from day one (the canvas/composition schema already
carries `col_weights`, `cell.col_span`, `capacity`, `overflow`).

## 20. Schema validation tooling (Figma/Pencil-style)

The contract `forgeplan.map/v1` is validated at **THREE call sites, ONE schema** — so a malformed map
can never reach the canvas (Figma/Pencil reject a bad file; so do we):
1. **Emitter-side (agent):** `map-emitter` validates before writing; **`map-guardian`** re-validates
   as the gate (`proposed → confirmed`). See §7.
2. **CLI / script:** `forgeplan map validate <map.json>` (or a node `validate.mjs`) — a linter that
   prints structured errors with JSON paths (`zones[3].cols missing` · `edge e-x endpoint 'foo' ∉
   nodes` · `zone z.a cell overlaps z.b` · `node n_y zone 'z.z' ∉ zones` · `mega-node mn_c nesting
   cycle` · `zone z.w capacity<overflow:spill but no continuation zone`).
3. **Web-side (runtime):** `entities/map/lib/validate.ts` — forgeplan-web validates `map.json` on
   load; invalid → show the errors / refuse to render (never render garbage).

**Implementation:** a single **JSON Schema** `plugins/forgeplan-map-pack/schemas/map.schema.json`
(ajv) shared by all three, PLUS **semantic lint rules** beyond JSON-Schema (the 3 invariants +
mega-node cycles + grid-fit + capacity/overflow consistency + trust-namespace rules) as a small
shared rule list returning `{path, message, severity}`. The guardian and the web read the same rule
output. This IS the "validation scripts like Figma/Pencil": the schema is the contract, the validator
is the linter, run everywhere.

## 21. Composition archetype catalog — seeds for the template library (from `html-effectiveness`)

Studied the 20 reference layouts in `dev/effective-html/skills/html-diagram/references/
html-effectiveness/`. Behind the varied "document" purposes there are **6 reusable layout
primitives**. Our composition templates are **combinations of these placed on the virtual grid**;
their CSS column patterns become the grid engine's (§19) **track presets**.

| Archetype | Track preset (from the examples) | Role in our system | Ref examples |
|---|---|---|---|
| **A. Canvas + side-rail** | `1fr 280px` / `300px 1fr` | the macro FRAME: composed-map + `ComposedPanel` + minimap (≈universal) | 04,07,08,13,14,15,17,19,20 |
| **B. Card-grid / matrix** | `repeat(N,1fr)` · `repeat(auto-fill,minmax(96px,1fr))` | zone sub-grid (nodes) + macro grid of zones | 01,05,06,09,11,16,18,10 |
| **C. Graph / flow canvas** | inline `<svg>` + paths | the core map; `layout_rule:graph`/`dag`; flowchart pipelines | 02,04,09,11,13,15,16 |
| **D. Stacked sections** | `<section>×N`, single column | `arrangement:stack-ttb`; status/report compositions | 03,05,11,12,16,17 |
| **E. Lanes / swimlanes** | `repeat(4,1fr)` board | `arrangement:lanes`; Decision-trail kind-lanes; triage | 18,03 |
| **F. Timeline / gutter-list** | `36px 36px 1fr 96px` · `48px 18px 1fr` | a zone with a leading marker gutter; changelog / drift list | 03,12,09 |

Implications:
- The **grid engine (§19)** needs exactly these track shapes: `repeat(N,fr)`, `auto-fill minmax`,
  `1fr + fixed-px rail`, `gutter + content`. Ship them as named `track` presets in `canvas`/`zone`.
- A **composition template (§6)** is then a small recipe: macro frame (usually A) + zone placements +
  per-zone primitive (`layout_rule` ∈ {grid=B, graph/dag=C, stack=D, lanes=E, gutter=F}).
- Example mappings: `rust-cli-mcp` = A-frame + D-stacked zones, zones use B-grid, Decision-trail uses
  C-graph. A "report-style" project = D + B. A "triage/issue" project = E. The detail rail (A) and
  the minimap are constant across all.
- These 20 are the **inspiration corpus** for the Phase-2 composition library — each new composition
  is still PULLED by a real repo (§6), but its layout vocabulary comes from this archetype set.
- **Two composition FAMILIES** (the corpus is byte-identical across the `html-diagram` and `html-plan`
  skills; only the SKILL intent differs): the **map/architecture** family (graph canvas **C** + zones
  + edges — our primary `composed-map`, the `html-diagram` mode) and the **plan/report** family
  (archetypes **D** stacked-sections / **F** timeline / **B** cards, NO graph — pragmatic, the
  `html-plan` mode). A forgeplan project's plan-shaped content (implementation plans, RFC phases,
  roadmaps, status) can render as a **plan composition**; the map family is the default. The
  onboarding (§17) may use a plan composition for a "what to do next" / status panel beside the map.

## 22. Canonical "architecture" composition (from `architecture-example.html`)

The gold-standard reference (`dev/effective-html/skills/html-diagram/references/
architecture-example.html`) — our spike is modelled on it; it confirms the direction and pins these
concrete reusables:

**Composition = `pipeline-lr`** (the default for code/service architectures): zones as columns
left→right by data flow (`Clients → App → Durable/Compute → Backends`). Archetype **C (graph canvas)**.
Zones may be **freeform** (explicit x/y/w/h, can nest/overlap) — so the engine must allow a zone to be
placed by explicit coords OR by grid cell (`composition.placements` supports both; curated-grid is the
cleaner default, freeform is the escape hatch for hand-tuned maps).

**Node-kind visual vocabulary (formalize as the `kind → treatment` table; token-only):**
| `kind` | treatment | meaning |
|---|---|---|
| `gate` | `--clay-soft` fill + `--clay` stroke + a clay kind-label | single authz / the one write gate |
| `store` | `--surface2` fill | source-of-truth / store |
| `truth` | `--olive-soft` + `--olive` stroke (the example's `do`) | the load-bearing special node |
| `ext` | dashed stroke (`6 3`) | external / optional |
| *(default)* | `--surface` + `--line` | ordinary component |
Extra accents available for more kinds: `--gold #C9A45C`, `--blue #5B7E96` (+ dark variants).

**Flow schema (extend our `flows[]`):** `{ id, name, node_ids[], edge_ids[], steps[] }` — clicking a
flow chip dims the rest, lights the path's nodes+edges (clay + marching-ants `7 5` @0.9s), and shows a
**flowcap** card with NUMBERED steps (rich prose, `<code>` tokens). The emitter writes `steps` (RU
per §15) and `edge_ids`.

**Render mode — fit OR scroll:** small maps that fit one screen use **fit-to-screen** (a fixed
`viewBox` + `preserveAspectRatio="xMidYMid meet"`, no scroll — calmer, more "poster"); large maps use
the spike's scroll + ctrl/⌘-zoom + minimap. The renderer picks: fit if the computed layout ≤ viewport,
else scroll. Both reuse the same `computeComposedLayout`.

**Detail surface:** the example uses **floating cards** (detail bottom-right, flowcap bottom-left) over
the stage; our spike uses a fixed right panel. Both are valid — the onboarding layout (§17) may prefer
floating cards (less chrome, more map); keep it a render option.

**Edges:** the example hand-routes bezier paths (cleaner than auto) — confirms the §10 tradeoff: MVP
uses `curve()`, a gutter-router is deferred; for a hand-tuned "hero" map an author may override edge
paths via an optional `edges[].path` field (Phase 2).

## 23. Orchestrated process + onboarding — FINAL design (workflow `wurhtcszc`)

Supersedes the §7/§17 sketches.

> **BUILD DECISION (user): build the FULL target from the start — «делаем сразу хорошо».** Build the
> whole agent roster (orchestrator + `code/forgeplan/docs-scanner` + `zone-extractor` +
> `edge-verifier` + `map-emitter` + deterministic `map-guardian.mjs` + advisory LLM-guardian) +
> onboarding layout + the append loop — NOT the thin 2-3-agent MVP. The "MVP/thin" framing below was
> the critics' de-risking; we keep it ONLY as (a) the BUILD ORDER (render-proof → process →
> onboarding → chat → append loop) and (b) **5 non-negotiable safety controls** (these are
> correctness, not scope-cuts): 1) hand-written `map.json` render-proof BEFORE the agent; 2) the 3
> EMITTER controls (denylist + `map-emitter-gate.sh` write-path hook + guardian single-write); 3)
> per-scanner SEPARATE scratch files merged by the orchestrator — never a shared `map.json` write
> (PROB-060 race); 4) append = a LOCAL `forgeplan map serve` daemon, not a web route (the web server
> cannot spawn `claude` — verified `READ_ONLY_SUBCOMMANDS`); 5) the guardian gate is the deterministic
> script, LLM-guardian advisory on top. Everything else: build it full.

### Process (marketplace `forgeplan-map-pack`) — THIN MVP, not 8 agents
Flow: `precondition(.forgeplan/ exists) → SCAN → type(inline) → select(inline) → EXTRACT → VERIFY →
EMIT → VALIDATE`. **Each LLM stage is a SEPARATE Task dispatch** = fresh isolated context (BMAD
generator≠verifier). Orchestrator carries only scratch-file paths + content-hashes, never a worker
transcript. Scratch in `.forgeplan/map/.work/` (gitignored); single output `.forgeplan/map/map.json`.

**DECISION (user — overrides the thin-MVP cut): build the FULL 8-agent orchestrated process from the
start.** The 8 roles, each in its own isolated Task context:
`map-orchestrator` (conductor — dispatches stages, enforces gates G1–G4, writes NOTHING) · 3 parallel
scanners `code-scanner` / `forgeplan-scanner` / `docs-scanner` · `zone-extractor` (THE HEART:
dirs/kinds → zones/layers/nodes/mega-nodes; IDs `sha1(kind+':'+path_or_slug)[:12]`; PINNED `cols`;
>8 nodes → collapsed mega-node) · `edge-verifier` (typed-link from `forgeplan_graph` vs grep-gated
code-dep, unverified DROPPED) · `map-emitter` (the SOLE writer of `map.json`; assembles, 3 guards,
atomic tmp-rename, `status:proposed` + `<<NEEDS_CONFIRM…>>`) · `map-guardian` (read-only: runs the
deterministic `map-guardian.mjs` + an advisory LLM CONCERNS review on top).

**MANDATORY mitigations for the 8-agent shape (the workflow's safety findings — non-negotiable, or
the parallel fan-out reintroduces the PROB-060 race that broke a prior run):**
- **Separate scratch file per scanner** (`.work/.scan.code.json` / `.scan.fpl.json` / `.scan.docs.json`),
  merged by the orchestrator — the 3 scanners NEVER write a shared file.
- **`map-emitter` is the ONLY writer of `map.json`** (single-writer; enforced by `map-emitter-gate.sh`).
- **Each LLM stage = a separate Task dispatch** (fresh context, generator≠verifier); the orchestrator
  carries only scratch-file paths + content-hashes between stages, never a worker transcript.
- Accepted tradeoff: more context spin-up + coordination per single-pass map, in exchange for full
  isolation, parallel scan, and the complete pipeline from day one.

**Gates G1–G4** (mechanical, fail-closed, orchestrator checks from scratch files; never silently pass —
PROB-035/039 class): scan→extract (facts parse, ≥1 module or generic floor) · extract→verify (every
node has 12-hex id + zone + provenance, no dup ids, cols pinned) · verify→emit (every code-dep has
`verified_by`, relations valid, endpoints exist) · emit→validate (file exists, schema-valid,
proposed, sentinel). On FAIL → loop to the named stage, max 3 rounds, then `<<NEED_USER_INPUT>>`.

### Guardian = a DETERMINISTIC script (`scripts/map-guardian.mjs`), not an LLM
Mirrors `adr_003_invariant.rs`. **6 checks:** (1) JSON ∈ `schemas/map.schema.json`; (2) the 3
invariants recomputed independently (no cell overlap, every edge endpoint ∈ nodes, every node.zone ∈
zones); (3) mega-node integrity (children ∈ nodes, no DFS cycle); (4) typed-link relation ∈ 11
VALID_RELATIONS, every code-dep has non-empty `verified_by`; (5) **single-write**: `git status
--porcelain .forgeplan/` shows ONLY `map/map.json` dirty (catches a stray write the denylist can't
see); (6) **determinism**: re-derive a sample of node IDs from `(kind,path)`; if `source_fingerprint`
unchanged but IDs differ → BLOCKER. Plus 2 cross-source checks a self-check structurally can't do:
every typed-link edge actually exists in `.scan.json`/`forgeplan_graph`; re-grep each `verified_by`
pattern (drop if stale). `exit 0` = PASS → the script (and only it) flips `proposed → confirmed`. LLM
guardian = advisory CONCERNS-only layer in Phase 4. Honest scope: guardian guarantees STRUCTURAL
trust; the human confirm guarantees SEMANTIC correctness (a structurally-valid but mis-binned node is
the human's catch).

### EMITTER-safe needs THREE controls, not one (corrected)
The denylist alone is NOT structurally safe — it allows `Write`, which could target
`.forgeplan/prds/*.md` and desync LanceDB. "RED-LINE #11 impossible" is true only for mutator TOOLS.
The write-PATH surface is closed by: (1) the EMITTER **denylist** (Edit + all `forgeplan_*` mutators);
(2) a **PreToolUse `hooks/map-emitter-gate.sh`** (fail-closed like `bmad-gate.sh`: deny any Write to
`.forgeplan/` except exactly `map/map.json` + `.work/`; deny `map.json` write from any agent ≠
`map-emitter`); (3) the **guardian single-write check** (after-the-fact). Denylist + hook + check.

### Onboarding = a SEPARATE `/onboard` route reusing the 8th widget
Full-bleed calm chrome (logo, project name from `map.meta`, "Exit to standard view →"; NO
Filters/InsightsRail) wrapping the same `ComposedMap` widget that is ALSO the dashboard's 8th view
(one widget, two hosts). The 8th view is registered via the verified triple (union + `GRAPH_VIEWS` +
`GRAPH_VIEW_IDS` Set) and `{:else if view==='map'}` **before line 155**.
- **Tour engine** = a ~120-line **data-driven state machine** (NOT a framework) reading data ALREADY
  in the map (`composition.entry_zone`, `zones[]` reading order, `flows[]`, `zone_connectors[]`) and
  driving the existing camera (d3-zoom tween, easeCubicInOut ~600ms). Deterministic, no model call.
  States `IDLE→TOUR_ZONE_N→TOUR_PAUSED→BROWSE`. The tour is a **suggestion**: ANY canvas click →
  pause (user drives); Esc / "I got it" / last zone → browse; reduced-motion → 0ms snap, starts
  paused. (`flows[]` + `zone_connectors[]` are BOTH render data AND tour script — that dual use is
  why no separate tour framework is needed.)
- **Narration RU, SPECIFIC** ("the projection module enforces ADR-003 — every write passes through
  here"), written into `map.json` by the docs-scanner **from real docs** — never auto-generated from
  zone names; no source → the zone is shown WITHOUT narration (tour skips it), never faked.
- **First-impression <3s:** mega-nodes collapsed, ≤~20 visible nodes, zoom-to-fit the whole map.
- **Chat (`widgets/map-chat`)** answers ~80% MAP-GROUNDED, CLIENT-SIDE from the loaded `map.json`
  (zone/node RU descriptions + auto-derived Connections + flows) — instant, no model, zero boundary
  risk; can MOVE THE CAMERA ("покажи Create" → `tour.go(f.create)`). **Grounding rule enforced:** every
  answer cites a source inline (→ ADR-003, z.storage, `…/projection/mod.rs`); can't ground → "not
  enough info — run a deeper scan?" (never a confident hallucination). Deeper-scan (model) = explicit,
  gated, Phase 3/5.
- **Drill to the 7 views:** a node with `artifact_id` → `/?selected=<id>&view=force` (HomePage's
  existing mechanism); a pure code-dep node has no such affordance (nothing in the artifact graph).

### Headless bridge — CUT from MVP (verified impossible as a web route)
`forgeplan-web/shared/server/forgeplan.ts` refuses every subcommand outside `READ_ONLY_SUBCOMMANDS`
and only spawns the `forgeplan` binary — **a SvelteKit route CANNOT spawn `claude`.** The bridge is a
**LOCAL co-process the user starts** (`forgeplan map serve` / `onboard-bridge.mjs`) that watches
`.forgeplan/map/.jobs/`; the web route only does narrow file I/O (writes a UUID-validated
`.req.json`), the daemon runs the scoped `claude -p '/map-build --refresh --scope <zone>'` (EMITTER,
append-only, FIFO-serialized, localhost-bind, explicit-click-gated), guardian re-validates,
`mapPoller.refresh()` → `animate:flip`. **Phase 5.** MVP refresh = the user re-runs `/map-build`; the
8s poller sees `meta.version` bump; canvas re-renders. **Determinism, honest:** node identity +
geometry deterministic (no reshuffle/dupes/drift), node DISCOVERY is LLM-variable — append-stability
(the bet) survives; the discovery set does not, by nature (acceptable — discovery is additive +
validated + explicitly "research deeper").

### Phased order
P0 render (hand-written `map.json` + pure `computeComposedLayout` + static `ComposedMap`, no agent) →
P1 process (the **full 8-agent pipeline** + `map-guardian.mjs` + `map-emitter-gate.sh` + `map-build`
playbook/skill + 3 compositions, with the mandatory separate-scratch-file + single-writer mitigations
above) → P2 onboarding (`/onboard` + tour) → P3 chat (query-only, grounded) → P5 append loop + local
daemon. (The earlier "P4 fan-out" is folded into P1 by this decision.) **MVP acceptance:** `/map-build` on ForgePlan reproduces the
spike grid, guardian flips `confirmed`; `/onboard` walks Surfaces→Core→Storage→Decisions + the
`f.create` flow with RU narration, Esc skips, node click drills to `/?selected`; chat answers "где
живёт Create?" grounded + drives camera; **re-run after adding a node keeps positions byte-identical**
(determinism check #6 — proves the bet WITHOUT the append loop).

---

_Source: two design workflows this session (`wf_1f87869e-692`, `wf_ac5e9538-05e`) + the master
architecture workflow (`wf_87cfbaff-0e6`, 11 agents, critique-integrated, verified against real
forgeplan-web code). Spike + this spec are self-contained — an agent in forgeplan-web/marketplace
needs nothing else._
