# Image prompts batch — серия «Decision Cycle» · посты 2–8

Бренд-обёртка из `ForgePlanMarketing/STYLE-GUIDE.md` §3.1. Brand orange — `#FF5A1F` (sparingly), dark bg `#050505` + dot grid 10% opacity / 24px, sans Inter, mono JetBrains Mono.

Для каждого поста: 2 ассета (cover 16:9 1600×900 + inline image 1280×720). Опционально 3-й (готовый screenshot из существующего guide).

После генерации ассеты кладём в `website/public/blog-covers/<slug>.webp` и обновляем frontmatter post-а: `cover: '/blog-covers/<slug>.webp'`.

---

## Пост 2 · «Перед тем как чинить — выпишите три версии» (ADI)

**Slug**: `before-fixing-write-three-versions` (примерный, согласовать)
**Guide reference**: `/guides/decision-cycle`
**Концепция**: следователь не фиксируется на первой версии. ADI = три гипотезы → проверяемые следствия → проверка.

### Cover (Шаблон B — concept illustration)

```
Style: dark mode editorial illustration, clean engineering aesthetic.
Background: near-black #050505 with subtle dot-grid pattern (white dots,
10% opacity, 24px spacing).

Subject: A horizontal split landscape composition. Left third — one
narrow straight path leading off-frame into darkness, with a single
clock icon above it labeled "30 min · random fix" in JetBrains Mono
gray. Right two-thirds — a trail-fork branching into three paths from
a central waypoint marker, each ending in a small diagnostic icon
(magnifier, gauge, ruler). Above the fork, a clock label "5 min ·
diagnosis" — but in brand orange #FF5A1F. The fork waypoint marker
is the only other orange element.

No people. No vehicles. Just paths, icons, and labels. Style:
flat 1px line drawing, like a topographic schematic rendered in
editorial monochrome with a single accent color.

Aspect ratio: 16:9 (1600×900 px).

Strict no-nos: no neon glow, no cyberpunk, no 3D render, no glitter,
no faces, no robot hands, no AI hands.
```

### Inline (Шаблон A — terminal mockup)

```
[brand wrapper as above]

Subject: A realistic macOS-style terminal window with traffic-light
buttons in the top-left. Inside the terminal, three hypothesis blocks
stacked vertically. Header line in white:

    $ forgeplan reason PRD-018

Then three indented hypothesis blocks (mono font):

    1. Index can't keep up with 50k entries
       deduction → EXPLAIN ANALYZE shows seq scan on notes table
       check → ❌ refuted (index hit ratio 99.2%)

    2. Ranking eats the budget
       deduction → top-K rerank includes 2000 candidates
       check → ✅ supported (rerank takes 84% of latency)

    3. Window of candidates too large
       deduction → fanout > 500 on 50k entries
       check → ⚠ partially (fanout=380, contributes)

The word "supported" is in green #22c55e, "refuted" in red #ef4444,
"partially" in orange #FF5A1F. Identifiers (PRD-018) in orange too.

Background: subtle dot grid visible behind terminal. Aspect 16:9.

No watermarks. Photorealistic terminal, no shadows on text.
```

---

## Пост 3 · «Среднее обманывает — как измерять доверие к решению» (R_eff)

**Slug**: `averages-lie-trust-calculus`
**Guide reference**: `/guides/trust-calculus` (3D scene) + `/guides/spec-cycle`
**Концепция**: WLNK > average. Whitepaper F7G8R2 vs slack-замер F4G8R7 — среднее одинаковое, но weakest-link разный.

### Cover (Шаблон D — split-screen / comparison card)

```
[brand wrapper]

Subject: Two evidence cards side-by-side on dark #050505 background,
separated by a thin vertical line. Each card frame is 400×500 px,
1px gray border, JetBrains Mono labels in 11px uppercase tracking 0.18em.

Left card title (orange eyebrow): "VENDOR WHITEPAPER"
Three horizontal bars stacked, each labeled F / G / R:
  F: ████████████░░░  7
  G: █████████████░░  8
  R: ██░░░░░░░░░░░░░  2
Below bars: "AVG = 5.7" in gray. Then red line below: "WLNK = 2 →
weakest link: source independence."

Right card title (orange eyebrow): "SLACK BENCHMARK"
Same three bars:
  F: ████░░░░░░░░░░░  4
  G: █████████████░░  8
  R: ████████████░░░  7
Below: "AVG = 6.3" gray. Then green line: "WLNK = 4 → still risky,
no proper rigour, but actionable."

Bottom of frame, in 11px mono uppercase orange tracking 0.18em:
"AVERAGES DON'T DIFFERENTIATE · MINIMUMS DO"

Style: editorial poster, flat 1px line drawing, no 3D. Aspect 16:9.

Strict: no glow, no cyberpunk, no faces, no AI clichés. Bars filled
with solid color (no gradients), brand orange #FF5A1F used only on
eyebrows and the bottom line text.
```

### Inline (Шаблон G — готовый ассет из guide)

```
Use existing screenshot: /public/guides-raw/trust-calculus.html
3D-scene (capture the F/G/R cube with 7 hypothesis points). Frame
in clean white border 4px, add caption underneath in 11px mono
gray: "Crank it with your mouse → /guides/trust-calculus"

No additional editing needed beyond crop + caption. Aspect 16:9
(letterbox 3D-scene to 1280×720).
```

---

## Пост 4 · «Мотивировочная часть для архитектурного решения» (DDR)

**Slug**: `decision-record-with-expiry`
**Guide reference**: `/guides/decision-cycle` (DDR-секция)
**Концепция**: 6 секций DDR. Самая важная — «условия пересмотра».

### Cover (Шаблон C — карточка артефакта)

```
[brand wrapper]

Subject: A stylized markdown decision file card centered on dark
background. The card is 720×640 px with a thin 1px gray border, dark
inner background #0b0b0b.

Card header (mono uppercase, 12px tracking 0.15em):
    DDR-042 · ACCEPTED · DECISION RECORD

Card title (sans 24px, white): "JWT vs sessions"

Six section headers in JetBrains Mono 13px white, each with a small
checkmark in green #22c55e and one line of preview text in gray:

    1. ✓ Context
       Friday before demo, no SSO, ...
    2. ✓ Alternatives considered
       JWT-rotation · Redis sessions · external OAuth
    3. ✓ Evidence
       latency benchmark · vendor docs · own bench
    4. ✓ Decision
       JWT with refresh-token rotation, TTL 7d
    5. ✓ Rejected alternatives
       Redis sessions: only 1 DevOps, can't run cluster
    6. ⚡ When to reopen        ← THIS LINE IS HIGHLIGHTED
       Revisit if JWT verify > 5ms or major library version

Section 6 is wrapped in an orange #FF5A1F frame, 2px stroke, with
a small "⚡" lightning icon. The frame has a soft glow (orange shadow
8px blur) to draw the eye.

Above the card, mono eyebrow in orange tracking 0.18em:
"THE SECTION TEAMS SKIP THE MOST"

Aspect 16:9. Editorial composition, no 3D, no shadows except the
intentional one on section 6 frame.

Strict: no neon, no cyberpunk, no AI illustrations.
```

### Inline (Шаблон C — горизонтальный таймлайн)

```
[brand wrapper]

Subject: A horizontal timeline diagram with 5 boxes connected by
thin lines + arrows. Each box is 200×120 px with 1px border.

Box 1: "accepted"        gray border, gray text, label below
                          "2024-Q4" in mono gray
Box 2: "active"          ORANGE border, orange text
                          "2025-Q1 → ..."
Box 3: "stale"           yellow border, yellow text
                          "2025-Q3 · valid_until expired"
Box 4 (top branch): "renew → still active"   green border
Box 4 (bottom branch): "supersede → new DDR-073"  pink border

Arrows from Box 3 fork into Box 4 top and bottom.

Above the timeline, mono eyebrow tracking 0.18em orange:
"DECISIONS HAVE EXPIRY DATES"

Aspect 16:9. Flat 1px lines, no 3D, no shadows.
```

---

## Пост 5 · «Архитектор не начинает с чертежей · BMAD»

**Slug**: `architect-doesnt-start-with-blueprints`
**Guide reference**: `/guides/bmad-cycle`
**Концепция**: 4 фазы — Brief → Mission → Architecture → Detailed plan. Аналогия архитектора дома.

### Cover (Шаблон C — 4 horizontal cards)

```
[brand wrapper]

Subject: Four horizontal cards in a row, each 280×400 px, separated
by 24px gaps. Each card has increasing visual density (Phase 1 nearly
empty, Phase 4 fully detailed) to show progressive elaboration.

Card 1 (light blue border #60a5fa):
   Title: "B · Brief"
   Content: One short paragraph stub (3 lines of "lorem"-style placeholder)
   Footer mono: "expand the question space"

Card 2 (green border #22c55e):
   Title: "M · Mission"
   Content: Five small dotted bullets, three of them filled in,
            two left as open circles
   Footer: "decompose · edge cases"

Card 3 (orange border #FF5A1F, slightly THICKER 2px):
   Title: "A · Architecture"
   Content: A small tree diagram with one chosen branch highlighted
            in orange, two greyed-out alternatives
   Footer: "pick one · with tradeoffs"

Card 4 (white border #f5f5f5):
   Title: "D · Detailed plan"
   Content: A miniature PRD document mockup with 13 numbered sections
            visible (very small, like a thumbnail)
   Footer: "contract + implementation map"

Above the row, mono eyebrow tracking 0.18em orange:
"FOUR PHASES · ARCHITECT'S DISCIPLINE"

Aspect 16:9. Cards sit on dark #050505 with subtle dot grid.

Strict: no 3D, no glow, no isometric perspective, flat editorial.
```

### Inline (Шаблон C — PRD-074 mockup, выделены 3 раздела)

```
[brand wrapper]

Subject: A miniature PRD document card centered on dark background.
Card 480×600 px, thin gray border, dark inner.

Card header (mono 11px orange tracking 0.18em):
    PRD-074 · DRAFT · 13 SECTIONS

Card title (sans 18px white): "Tags for notes (MVP)"

13 section rows, each with index number + title:
    1. Problem statement
    2. Goals
    3. Non-Goals                          ← HIGHLIGHTED (orange frame)
    4. Functional Requirements
    5. Non-functional Requirements
    6. User stories
    7. UX flows
    8. Data model
    9. API contract
    10. Risks                             ← HIGHLIGHTED (orange frame)
    11. Open Questions                    ← HIGHLIGHTED (orange frame)
    12. Migration plan
    13. Success criteria

The three highlighted sections (3, 10, 11) have a thin 2px orange
#FF5A1F outline + tiny "⚠" warning icon next to them.

Above card, mono eyebrow:
"THE THREE SECTIONS TEAMS RUSH THROUGH"

Aspect 16:9. Editorial, flat, no 3D.
```

---

## Пост 6 · «Спецификация — это не PRD, это контрольный вкус»

**Slug**: `spec-vs-prd-tasting-test`
**Guide reference**: `/guides/spec-cycle`
**Концепция**: PRD = intent. Spec = verifiable behavior. PRD как рецепт, Spec как вкус блюда.

### Cover (Шаблон D — split-screen PRD vs Spec)

```
[brand wrapper]

Subject: Two cards side-by-side with vertical divider. Both 500×580 px.

Left card (gray border, gray internal):
   Mono eyebrow orange 11px: "PRD · INTENT"
   Title white sans: "Add tags to notes"
   Body lorem-style stub of about 4 lines, deliberately vague:
     "Users should be able to organize their notes."
     "Tags should help with discovery."
     "We'll support typical tagging operations."
     "The system should perform well."
   Bottom mono gray: "STATUS · GUIDANCE"

Vertical divider — thin orange line #FF5A1F, 1px wide.

Right card (orange border #FF5A1F 2px, dark internal):
   Mono eyebrow orange: "SPEC · BEHAVIOR"
   Title: "POST /notes/:id/tags"
   Body in JetBrains Mono small (11px):
     [request]
     { "tags": ["work", "urgent"] }   (1..10 strings, 1..32 chars each)

     [response · 200]
     { "id": 123, "tags": [...] }

     [edge cases]
     · empty array → 400
     · duplicate tag → idempotent
     · tag with >32 chars → 400 · error.code="TAG_TOO_LONG"

   Bottom mono orange: "STATUS · CONTRACT"

Above both cards, eyebrow orange tracking 0.18em:
"RECIPE VS TASTE TEST"

Aspect 16:9. Flat editorial, no 3D.

Strict: no glow except 2px orange border, no neon, no robots.
```

### Inline (Шаблон C — delta block ADDED/MODIFIED/REMOVED)

```
[brand wrapper]

Subject: A horizontal delta-document block, looking like a git diff
visualization. Three sections stacked vertically, each prefixed with
a coloured marker.

Section 1: green left bar #22c55e width 4px
   ADDED
   + POST /notes/:id/tags · accepts array of 1..10 strings
   + Idempotent semantics for duplicates

Section 2: amber left bar #FFA500 width 4px
   MODIFIED
   ~ GET /notes/:id (BEFORE)
   ~ GET /notes/:id (AFTER · now returns tags array in response)

Section 3: red left bar #ef4444 width 4px
   REMOVED
   - DELETE /notes/:id/tags/all (no use case found)

Each block in JetBrains Mono 12px. Lines indented 16px from the
coloured bar.

Above the block, eyebrow orange tracking 0.18em:
"DELTA · WHAT CHANGED"

Aspect 16:9. Dark #050505 background, subtle dot grid.
```

---

## Пост 7 · «POS-терминал для методологий · что такое Forgeplan»

**Slug**: `forgeplan-pos-terminal`
**Guide reference**: `/guides/forgeplan-cycle` + `/guides/depth-calibrator` + `/guides/dag-explorer`
**Концепция**: Forgeplan забирает механику FPF/BMAD/OpenSpec/QuintCode, оставляя человеку — содержание.

### Cover (Шаблон D — split-screen manager desk vs terminal)

```
[brand wrapper]

Subject: Photographic editorial split-screen. Two halves of one
horizontal frame.

Left half (Без Forgeplan):
   A manager's desk seen from slightly above. Cluttered: stack of
   loose paper, 3 sticky notes with crossed-out words, 2 open Notion
   tabs on a laptop screen (only edge of screen visible), a half-empty
   coffee mug, a notebook with hand-drawn boxes and arrows. The mood:
   "everything is somewhere but nothing is in order."
   Lighting: cool, dim — single overhead light, no warmth.

Right half (С Forgeplan):
   A single dark terminal window full-screen. The only thing visible:

       $ forgeplan validate PRD-018
       ✓ Format · BMAD compliance
       ✓ Coherence · OpenSpec graph
       ✓ Alternatives · FPF rigour (3+ found)
       PASS · ready to activate

   The word "PASS" is in green #22c55e. Identifier PRD-018 in
   orange #FF5A1F. Other lines in light gray on dark.

Thin vertical line in middle, 1px gray.

Aspect 16:9 (1600×900). Photorealistic left, terminal-mockup right.
Brand orange visible ONLY on right side (terminal identifier).

Strict: no 3D, no neon, no robots, no AI hands. Left side empty
of people. Right side clean modern macOS terminal aesthetic.
```

### Inline #1 (Шаблон C — 10 типов записей, 6 core)

```
[brand wrapper]

Subject: A horizontal funnel diagram showing 10 record types arrayed
in 2 rows. Top row has 6 "core" types, bottom row has 4 "situational".

Top row (each box 140×100 px, orange border 2px #FF5A1F):
   PRD · RFC · ADR · Spec · Epic · Evidence

Bottom row (each box 140×100 px, gray border 1px):
   Problem · Solution · Note · Refresh

Each box contains:
   Type name in JetBrains Mono 14px uppercase
   Below: tiny icon (mini PRD page, mini graph, etc.)
   Footer mono 9px: count in active project, e.g. "x47"

Above the funnel, eyebrow orange tracking 0.18em:
"TEN ARTIFACTS · SIX IN ACTIVE USE"

Aspect 16:9.
```

### Inline #2 (готовый ассет — 3D-граф)

```
Use existing screenshot from /public/guides-raw/dag-explorer.html
Capture the Plotly 3D scatter of Knowledge Vault MVP artifacts
(22 nodes, 30 edges). Frame with thin border + caption:

   "22 artifacts · 30 connections · /guides/dag-explorer"

Letterbox to 1280×720 if scene is wider.
```

---

## Пост 8 · «Полный цикл на одной фиче · capstone»

**Slug**: `full-cycle-on-one-feature`
**Guide reference**: `/guides/lifecycle-cycle` + капстон ссылки на все 11 предыдущих guides
**Концепция**: 7 шагов на одной реальной задаче (Vault streaming endpoint). От строки в Slack до активного решения.

### Cover (Шаблон C — 7-step horizontal progression)

```
[brand wrapper]

Subject: Seven horizontal cards arranged in a row, each 180×280 px,
progressing left to right, with thin connecting lines + arrows
between them. Each card has its own depth-tier color.

Card 1 (gray): "1 · route"        → step icon: 🎯
Card 2 (blue): "2 · shape"        → icon: 📐
Card 3 (blue): "3 · validate"     → icon: ✓
Card 4 (purple): "4 · reason"     → icon: 💭
Card 5 (purple): "5 · build"      → icon: ⚒
Card 6 (orange #FF5A1F): "6 · activate"  → icon: ⚡ · GREEN CHECK MARK
Card 7 (gray dashed border): "7 · 6mo later · renew/supersede"  → icon: ⏳

Below the row, in mono eyebrow tracking 0.18em orange:
"FROM SLACK MESSAGE TO ACTIVE DECISION · 18 MONTHS"

Above row, large mono in white: "7 STEPS"

Aspect 16:9. Flat, editorial, 1px lines and 2px borders, no 3D.
```

### Inline #1 (Шаблон A — финальный health terminal)

```
[brand wrapper]

Subject: A realistic macOS-style terminal showing forgeplan health
final state. Content (verbatim, mono):

    $ forgeplan health
    Verdict ✓ HEALTHY
    Active artifacts        12  (was: 8)
    Stale artifacts          0
    Orphan artifacts         0
    Coverage                 100%
    R_eff (lowest)           0.84 · DECIDED
    Next reopen window       2026-02-15 (ADR-018 valid_until)

The word "HEALTHY" in green #22c55e. "Next" line in orange.
Numbers right-aligned in mono.

Above terminal, mono eyebrow orange:
"AFTER ONE COMPLETE CYCLE"

Aspect 16:9. Clean modern macOS terminal aesthetic, dot grid
behind. Photorealistic terminal.
```

### Inline #2 (готовый ассет — dag-explorer subset)

```
Use existing 3D scene /public/guides-raw/dag-explorer.html, filter
to ~6 artifacts of THE specific feature being discussed
(PRD-018, RFC-001, ADR-001, SPEC-003, EVID-001, EVID-006).

Caption:
   "What one decision looks like as a graph · /guides/dag-explorer"

Letterbox 1280×720.
```

---

## Общие правила для всех 7 постов

### Anti-patterns (никогда не запрашиваем)
- No neon glow, no cyberpunk, no sci-fi lighting
- No AI-stock clichés (robot hands typing, brain-of-wires, glowing CPU)
- No 3D rendered UI (terminal screens are FLAT mockups)
- No faces / no people in frame (если человек нужен — силуэт со спины)
- No clocks at 10:10 (AI cliché)
- No RGB-keyboards
- No glittering text effects, no lens flares

### Brand палитра (точные hex)
- Bg dark: `#050505` (NOT `#000000`)
- Fg gray scale: `#a3a3a3` (secondary), `#f5f5f5` (primary)
- Brand orange: `#FF5A1F` (sparingly — 1-2 элемента, не везде)
- Green success: `#22c55e`
- Red error: `#ef4444`
- Amber warning: `#FFA500`
- Linear blue: `#60a5fa`

### Negative prompt (для Midjourney / Stable Diffusion если используются)
```
neon glow, cyberpunk, 3D render of UI, glowing brain, AI hands, robot,
glitter, lens flare, RGB keyboard, faces, smiling people, stock photo
woman, clock at 10:10, glowing CPU, motherboard, futuristic hologram,
fantasy elements
```

### Куда класть готовые ассеты
```
website/public/blog-covers/<slug>.webp           # cover
website/public/blog-covers/<slug>-inline.webp    # inline #1
website/public/blog-covers/<slug>-inline-2.webp  # optional inline #2
```

Frontmatter post-а:
```yaml
cover: '/blog-covers/<slug>.webp'
```

Inline ассеты в MDX:
```mdx
<img src="/blog-covers/<slug>-inline.webp"
     alt="<описание>" width="1280" height="720" />
```

### Workflow генерации (рекомендация)
1. Использовать **GPT-Image (gpt-image-1)** через ChatGPT Plus / Pro — best для текста на whiteboard, кода в terminal mockup, читаемых надписей.
2. Если GPT-Image не справляется с текстом — fallback на **Midjourney v6** + negative prompt из общих правил.
3. Альтернатива (если важна скорость) — собрать в **Figma/Pencil** по STYLE-GUIDE §4 шаблонам, экспортировать как PNG → конвертировать в WebP.
4. Для inline ассетов с 3D guides — просто capture screenshot из браузера (open guide → screenshot → crop → caption).

---

**Версия документа**: v1, 2026-05-23.
**Источник брифов**: `ForgePlanMarketing/posts/blog-series-decision-cycle.md` §Посты 2–8.
**Связанный prompt-документ**: `decision-graveyard.md` (Пост 1).
