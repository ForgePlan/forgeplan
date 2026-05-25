# Standalone blog posts — image brief batch

Готовые промпты для 11 постов, у которых сейчас нет обложки. Все — standalone (вне серии Decision-cycle).

После генерации:
1. Сохрани файл как `<filename-указанный-в-блоке>.png` в `website/public/blog-covers/`.
2. Скажи мне «положил X» — я конвертирую в webp и подключу cover в frontmatter.

---

## Brand wrapper (добавь к каждому prompt'у)

```
Style: dark mode UI screenshot, clean engineering aesthetic.
Background: near-black #050505 with subtle dot-grid pattern (white dots, 10% opacity, 24px spacing).
Brand accent: #ff5a1f (warm orange) — used sparingly for highlighted elements only.
Text on image: white #f5f5f5 (primary), gray #a3a3a3 (secondary).
Typography: Inter or Geist sans-serif for labels, JetBrains Mono for code and identifiers.
Mood: precise, focused, technical. No glitter, no neon glow, no AI-stock cliches, no robot hands, no glowing brains.
Aspect ratio: 16:9 (1600×900).
```

---

## EN posts (7)

### 1. `welcome.png` — EN welcome post

> Welcome to the Forgeplan blog. This blog will document decisions, releases, and methodology insights from Forgeplan development.

```
[brand wrapper]

Subject: Minimalist editorial cover for a blog manifesto. Centered: large stylized text "F●RGEPLAN" with the orange dot accent on the bullet. Below in smaller mono uppercase: "ENGINEERING DECISIONS · MADE TO LAST". A horizontal thin orange line under the title. Bottom-right corner: tiny mono "vol.01 · welcome".

Background: black with dot grid. No people, no icons, no illustrations — typography-only cover.
```

---

### 2. `forgeplan-as-agent-harness.png`

> ForgePlan: an agent harness disguised as a decision framework. Five harness subsystems, PROB-034, three roadmap gaps.

```
[brand wrapper]

Subject: Concentric diagram. Center: small white-outlined circle labeled "MODEL". Around it, five outer arcs labeled in 11px uppercase mono: "RULES", "TOOLS", "ENV", "STATE", "FEEDBACK". The FEEDBACK arc is highlighted in orange #ff5a1f — the others are thin white outlines.

Below the diagram, a single line of mono text: "harness = everything around the weights".

Editorial line-art, NOT 3D. Black background with dot grid.
```

---

### 3. `forgeplan-rust-lancedb.png`

> Forgeplan: agent harness for AI coding agents (Rust + LanceDB). Local-first CLI, markdown source of truth, weakest-link trust scoring.

```
[brand wrapper]

Subject: Split-screen terminal-style layout. Left side: a stylized file tree showing `.forgeplan/` directory with subfolders `prds/`, `adrs/`, `evidence/`, each containing one or two .md files. Right side: a code snippet in JetBrains Mono showing a Rust function signature `pub fn sync_file_to_store(path: &Path) -> Result<Artifact>`.

Top eyebrow in mono uppercase orange: "RUST · LOCAL-FIRST · MCP".

Both sides on dark background with dot grid.
```

---

### 4. `git-for-decisions.png`

> "Git for decisions" — six months of dogfooding. Treats engineering decisions like first-class artifacts.

```
[brand wrapper]

Subject: Stylized git-log style timeline. Vertical thin line with 5-6 commit-style nodes. Each node has a short uppercase mono label: "ADR-001 · sse-streaming", "ADR-005 · jwt-rotation", "ADR-012 · lancedb-storage", "EVID-094 · benchmark", etc. One node in the middle is highlighted with orange dot — labeled "← active".

Right side: handwritten-style annotation arrow pointing to the active node: "decisions, version-controlled".

Black background with dot grid. No glowing effects.
```

---

### 5. `harness-engineering-for-ai-agents.png`

> Harness engineering for AI coding agents. Why your prompt isn't the problem.

```
[brand wrapper]

Subject: Two side-by-side cards on dark background, separated by thin vertical line.

Left card: header "PROMPT" in uppercase mono. Below it a wireframe of a chat bubble with placeholder text lines. Faded, gray, low contrast.

Right card: header "HARNESS" in uppercase mono ORANGE #ff5a1f. Below it five small rectangles in a pentagon layout labeled "RULES / TOOLS / ENV / STATE / FEEDBACK". Sharp, white-outlined.

Bottom mono text: "the failure is rarely in the prompt".

Editorial style. Black background with dot grid.
```

---

### 6. `markdown-source-of-truth.png`

> Markdown as source of truth, LanceDB as derived index. 32 violations, 4 audit rounds, compile-time enforcement.

```
[brand wrapper]

Subject: Two stacked horizontal bars representing layers.

Top bar (taller, more prominent): label "MARKDOWN · SOURCE OF TRUTH" in white mono uppercase. Contains stylized file icons with names like "PRD-018.md", "ADR-012.md", "EVID-094.md". Orange dot in left corner.

Below it a thin downward arrow labeled "projection".

Bottom bar (shorter, more faded): label "LANCEDB · DERIVED INDEX" in gray mono uppercase. Contains abstract vector-grid pattern.

Top-right corner: small badge "32 violations → 0" with orange checkmark.

Black background with dot grid.
```

---

### 7. `r-eff-weakest-link.png`

> R_eff math: trust = weakest link, never the average.

```
[brand wrapper]

Subject: Vertical bar chart with five bars representing evidence scores. Heights from left to right: 7, 8, 2, 8, 9. The third bar (height 2) is highlighted in orange #ff5a1f, all others are white-outlined.

Above the chart: formula in JetBrains Mono — "R_eff = min(...)" with the min function emphasized.

Below the chart: two text labels side by side:
- "average: 6.8" — gray, struck through with horizontal line
- "weakest: 2.0" — white, with orange arrow pointing at it

Right side: small annotation "trust ≠ average".

Black background with dot grid. Editorial chart style, not 3D.
```

---

## RU posts (4)

### 8. `welcome-ru.png` — RU welcome post

> Добро пожаловать в блог Forgeplan. Разбираем решения, релизы и методологические находки.

```
[brand wrapper]

Subject: Тот же layout что и `welcome.png` (EN), но текст под главным логотипом на русском.

Centered: large stylized text "F●RGEPLAN" with orange dot accent on the bullet. Below in smaller mono uppercase Cyrillic: "РЕШЕНИЯ · КОТОРЫЕ ОСТАЮТСЯ ЖИТЬ". Horizontal thin orange line under the title. Bottom-right corner: tiny mono "том 01 · приветствие".

Black background with dot grid. Typography-only cover.
```

---

### 9. `gde-zhivut-resheniya.png`

> Где живут решения вашей команды, если их вообще не теряли. Шесть месяцев открытой разработки ForgePlan.

```
[brand wrapper]

Subject: Cemetery metaphor stylized as a grid of tombstone-shaped cards on dark background. 8-10 cards in a 4×2 or 5×2 grid. Each card has a faded label in uppercase mono cyrillic: "JWT?", "REDIS?", "STRIPE?", "POSTGRES?", "OAUTH?", etc. — each followed by a question mark and a small "?2024" date stamp.

ONE card in the grid stands out — solid orange #ff5a1f outline, label clearly readable: "ADR-012 · 2026-05 · АКТИВНО".

Top eyebrow in mono uppercase orange Cyrillic: "КЛАДБИЩЕ РЕШЕНИЙ".

Editorial flat illustration, NOT photorealistic. Black background with dot grid.
```

---

### 10. `obvyazka-dlya-ai-agentov.png`

> Обвязка для AI-агентов. Пять подсистем обвязки, ADR-003, три дыры в дорожной карте.

```
[brand wrapper]

Subject: Same pentagon harness diagram as `forgeplan-as-agent-harness.png` (EN), but labels in Russian.

Center: small white-outlined circle labeled "AI" in latin. Around it, five outer arcs labeled in 11px uppercase mono CYRILLIC: "ПРАВИЛА", "ИНСТРУМЕНТЫ", "СРЕДА", "СОСТОЯНИЕ", "ПРОВЕРКИ". The "ПРОВЕРКИ" arc is highlighted in orange #ff5a1f.

Below the diagram, a single line of mono Cyrillic text: "обвязка — это всё кроме весов модели".

Editorial line-art, NOT 3D. Black background with dot grid.
```

---

### 11. `reshenia-ne-prompty.png`

> Кладбище решений: почему мой AI каждую сессию начинает один и тот же спор заново.

```
[brand wrapper]

Subject: A loop diagram. Three stylized chat bubbles in a triangular layout, connected by curved arrows forming a closed loop. Each bubble contains a short cyrillic mono label:

- top bubble: "СЕССИЯ 1 · обсуждаем JWT vs sessions"
- bottom-right bubble: "СЕССИЯ 2 · обсуждаем JWT vs sessions"
- bottom-left bubble: "СЕССИЯ 3 · обсуждаем JWT vs sessions"

The loop is in faded gray. Outside the loop, a single solid box labeled in orange #ff5a1f Cyrillic mono: "ADR-005 · место для решения" — with an arrow pointing into the loop, breaking it.

Top eyebrow: "DEEP-DIVE · METHODOLOGY".

Editorial flat illustration. Black background with dot grid.
```

---

## After generation — what I do

Когда положишь файл `<filename>.png` в `public/blog-covers/`, скажи мне «положил X». Я выполню:

1. `cwebp -q 82 -m 6 <X>.png -o <X>.webp` (94-97% компрессия)
2. Добавлю поле во frontmatter поста:
   ```yaml
   cover: '/blog-covers/<X>.webp'
   ```
3. `npm run build` verify
4. Скажу тебе ссылку для проверки
