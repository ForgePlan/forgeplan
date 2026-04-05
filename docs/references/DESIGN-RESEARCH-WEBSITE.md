# Design Research: Forgeplan Website & TUI Visual Language

> Deep research document for the official Forgeplan website and TUI (ratatui) design system.
> Based on analysis of qfnetwork.xyz source code + 15 reference sites.

---

## 1. Executive Summary

**Цель**: создать визуальный язык Forgeplan, применимый в двух контекстах:
1. **Web** — официальный лендинг (static site, SSG)
2. **TUI** — desktop CLI интерфейс (ratatui, терминал)

**Эстетика**: чёрно-белое, геометрические линии, полигональность, минимализм, метафоры forge (ковка) и plan (чертёж/граф).

**Ключевой вывод из исследования**: лучшие devtool сайты 2025 года используют centered layout, monochrome base + 1 акцент, geometric SVG (не 3D), и "no salesy BS" тон.

---

## 2. Анализ qfnetwork.xyz (Source Code Deep Dive)

### 2.1 Технологический стек

| Компонент | Технология | Детали |
|-----------|-----------|--------|
| Framework | **SvelteKit** | SSR + hydration, `data-sveltekit-preload-data="hover"` |
| CSS | **Tailwind CSS** | Utility-first + CSS Custom Properties |
| Animations | **GSAP 3.13.0** | ScrollTrigger + MorphSVG Plugin (платный!) |
| Build | **Vite** | Chunk splitting, modulepreload |
| Visual | **SVG only** | Нет Three.js/WebGL/Canvas — всё через SVG paths |
| Theme | **mode-watcher** | Dark/light с localStorage persistence |
| Typography | **Fractul** | Кастомный шрифт, 18 начертаний (50–900 + italic) |

### 2.2 Цветовая палитра (точные значения из кода)

```
ОСНОВНЫЕ:
  --primary-dark:   #161616    (почти-чёрный)
  --primary-light:  #DADADA    (светло-серый)

LIGHT MODE:
  --line-color:       #161616
  --border-color:     #DADADA
  --background-color: #DADADA

DARK MODE:
  --line-color:       #DADADA
  --border-color:     #161616
  --background-color: #161616

АКЦЕНТ:
  #0051FF              (ярко-синий, ссылки/CTA)

УТИЛИТАРНЫЕ:
  #CACACA              (чуть темнее light, bg-элементы)
  rgba(0,0,0,0)        (скрытые SVG)
```

**Принцип**: строго двухцветная система с автоинверсией через CSS variables. Один акцентный цвет. Никаких градиентов.

### 2.3 Типографика

- **Шрифт**: Fractul (кастомный geometric sans-serif)
- **Body**: weight 300 (Light), line-height 1.27
- **Заголовки**: weight 400 (Regular), line-height 1.0–1.2
- **Bold**: weight 700
- **Hero**: динамический размер через JS + ResizeObserver (80–120px+ на desktop)
- **Responsive scale**: text-sm → text-lg → text-xl → text-2xl (Tailwind)

### 2.4 Layout система

```
Breakpoints (Tailwind + кастомные):
  xs:  400px    (кастомный)
  s:   550px    (кастомный)
  sm:  640px    (Tailwind)
  md:  768px
  lg:  1024px
  xl:  1280px
  2xl: 1536px

Header grid:
  mobile:  grid-cols-[76px_minmax(0,auto)_65px]
  desktop: grid-cols-[90px_1fr_auto_90px]

Hero:
  mobile:  flex column
  desktop: grid-cols-5 (col-span-3 / col-span-2)

Секции:
  min-height: max(calc(100vh - var(--header-height)), 500px)
```

**Ключевой паттерн**: visible border grid — все бордеры видны (`border: 1px solid var(--line-color)`), создавая "чертёжную" / "blueprint" эстетику.

### 2.5 Инвентарь анимаций

#### A. SVG Morphing (GSAP MorphSVG) — ГЛАВНЫЙ ЭФФЕКТ

Геометрические SVG фигуры плавно трансформируются при скролле:

| Секция | Что морфится | Количество path |
|--------|-------------|-----------------|
| Hero | 14+ эллипсов разной эксцентричности | ~14 paths |
| Digital Freedom | Фигуры + смена цвета stroke/fill | ~8 paths |
| Built | cubicInside, tower, keyhole, gearHands, speedometr | ~12 paths |
| First SDK | Переход между "зонами" большого SVG | ~6 paths |
| Vibe Coding | 5 "кадров" последовательно | ~5 paths |

**Реализация**:
```javascript
gsap.timeline({
  scrollTrigger: {
    trigger: element,
    start: `top-=${headerHeight} top`,
    end: `top+=${Math.max(window.innerHeight, 600)}px top`,
    scrub: true,
    pin: true,
    anticipatePin: 1,
    pinType: "fixed"
  }
})
```

#### B. Scroll-Triggered Pin Sections

Каждая крупная секция "приклеивается" к viewport, и в пределах скролл-дистанции проигрывается GSAP timeline с `scrub: true`.

#### C. CSS Hover эффекты

| Эффект | Easing | Длительность |
|--------|--------|-------------|
| CTA bounce | `cubic-bezier(0.68, -0.55, 0.265, 1.55)` | 400ms |
| Wipe (::before) | `translate-x-full → translate-x-0` | CSS transition |
| Scale | `hover:scale-[1.1]` | стандартный |

#### D. Rings Animation

Концентрические кольца/линии, центрированные через `absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2`. Используются в hero и SDK секциях.

#### E. Chevron Arrows (бесконечная лента)

18 параллельных `<` форм, горизонтальная лента с `timeline({repeat: -1})`.

#### F. Custom Scroll Hijack

- Body/HTML `overflow: hidden`
- `.content-wrapper` — фиксированный контейнер с `overflow: auto`
- Кастомный momentum через `requestAnimationFrame` + velocity tracking

### 2.6 Performance подход

- `overscroll-behavior: none` — нет bounce/pull-to-refresh
- `will-change: scroll-position` на scroll container
- `isolation: isolate` — новый stacking context
- `font-display: swap` — текст сразу, шрифт подгружается
- Модульные JS chunks через Vite
- `-webkit-overflow-scrolling: touch` на iOS

---

## 3. Reference Sites (15 аналогов)

### Tier 1 — Прямые конкуренты (CLI/dev tools)

| # | Сайт | Стек | Ключевая фишка | Релевантность |
|---|------|------|----------------|---------------|
| 1 | **charm.sh** | Go CLI | Минимализм, TUI-библиотеки (bubbletea/lipgloss), игривый тон | **Высшая** — CLI-first, аналогичный продукт |
| 2 | **astral.sh** | Rust CLI (ruff, uv) | Фокус на скорость, бенчмарки как визуал | **Высшая** — Rust CLI, та же аудитория |
| 3 | **zed.dev** | Rust editor | Monospace эстетика, dark-on-dark | **Высокая** — Rust, performance-first |
| 4 | **warp.dev** | Terminal | Dark gradient + glassmorphism, screenshot-driven | **Высокая** — terminal UX |
| 5 | **tauri.app** | Rust framework | Docs-driven, clean, code snippets на лендинге | **Высокая** — наша технология |
| 6 | **nushell.sh** | Rust shell | Terminal-демо, structured data | **Средняя** — подход к structured data |

### Tier 2 — Эстетические референсы

| # | Сайт | Ключевая фишка | Что взять |
|---|------|----------------|-----------|
| 7 | **linear.app** | Эталон dark SaaS, "precision engineering" | Gradient glow, модульная сетка |
| 8 | **vercel.com** | Ч/б + призма/геометрия, SVG-линии | Геометрическая абстракция как hero |
| 9 | **Cruip Gray** | Шаблон ч/б лендинга для tech/SaaS | Готовый паттерн hero+features+pricing |
| 10 | **bloomberg.com** | Brutalist grid, утилитарная эстетика | Grid-first без декора = "кузница" |

### Tier 3 — Инструменты и ресурсы

| # | Ресурс | Для чего |
|---|--------|----------|
| 11 | **Evil Martians LaunchKit** | Научно обоснованный devtool landing паттерн (исследование 100+ сайтов) |
| 12 | **fffuel.co/llline** | SVG генератор линейных паттернов |
| 13 | **SuperformulaSVG** | Полигональные паттерны из мат. формул |
| 14 | **Glyphic AI** | Deep black + светящиеся геометрические линии |
| 15 | **Lapa Ninja (Black)** | 1750+ примеров dark лендингов |

### Паттерны из исследования Evil Martians (100+ devtool сайтов)

1. **Centered layout с max-width** — работает лучше всего
2. **Hero = bold headline + geometric visual** — ни один devtool не использует stock фото
3. **Monochrome base + 1 акцентный цвет**
4. **CLI demo прямо на лендинге** (asciinema, скриншот, или inline code)
5. **"No salesy BS, clever and simple wins"**
6. **Performance metrics как визуальный элемент** (бенчмарки, цифры)

---

## 4. Forgeplan Design System: Предложение

### 4.1 Цветовая палитра

```
FORGE PALETTE (inspired by qfnetwork.xyz + наковальня):

Primary:
  --forge-dark:    #0D0D0D    (глубокий чёрный — раскалённый металл в тени)
  --forge-light:   #E8E8E8    (светлый серый — холодная сталь)
  --forge-mid:     #2A2A2A    (тёмно-серый — граффит/уголь)

Accent (выбрать один):
  Option A: --forge-ember:  #FF6B35    (тлеющий уголь / forge orange)
  Option B: --forge-steel:  #4A9EFF    (холодная сталь / blueprint blue)
  Option C: --forge-spark:  #FFD700    (искра / gold spark)

Lines:
  --forge-line:    #3A3A3A    (линии grid / borders в dark mode)
  --forge-line-lt: #C0C0C0    (линии в light mode)

DARK MODE (default):
  background:  --forge-dark
  text:        --forge-light
  lines:       --forge-line
  accent:      --forge-ember (или steel)

LIGHT MODE:
  background:  --forge-light
  text:        --forge-dark
  lines:       --forge-line-lt
  accent:      --forge-ember (или steel)
```

### 4.2 Типографика

```
PRIMARY FONT:
  Space Grotesk (Google Fonts, free)
  — geometric sans-serif, похож на Fractul
  — weights: 300, 400, 500, 600, 700

MONOSPACE (для CLI демо, код):
  JetBrains Mono или Berkeley Mono
  — ligatures для кода
  — weights: 400, 700

HIERARCHY:
  Hero:        Space Grotesk 700, ~80-120px (fluid)
  H2:          Space Grotesk 600, 36-48px
  H3:          Space Grotesk 500, 24-32px
  Body:        Space Grotesk 300, 16-18px, line-height 1.5
  Code:        JetBrains Mono 400, 14-16px
  Caption:     Space Grotesk 300, 12-14px
```

### 4.3 Layout

```
GRID:
  max-width: 1200px (centered)
  columns: 12-column (desktop), 4-column (mobile)
  gutter: 24px
  visible borders: 1px solid var(--forge-line)  ← "blueprint" aesthetic

SECTIONS:
  min-height: 100vh (pinned scroll sections)
  padding: 80px vertical, 24-48px horizontal

HEADER:
  fixed, border-bottom
  logo | nav | CTA
  height: 64px
```

### 4.4 Визуальный язык

#### Метафоры Forge + Plan:

| Концепт | Визуальная метафора | SVG реализация |
|---------|-------------------|----------------|
| **Forge** (ковка) | Наковальня, молот, искры | SVG path morphing: наковальня → артефакт |
| **Plan** (план) | Чертёж, blueprint, grid | Visible borders, dashed guide lines |
| **Artifact** | Геометрическая фигура | Polygon / hexagon с border |
| **Lifecycle** | Трансформация формы | SVG morph: draft(circle) → active(hexagon) → done(star) |
| **Graph** | Dependency DAG | SVG lines + nodes, animated connections |
| **Scoring** (R_eff) | Gauge / progress arc | SVG arc с fill по score |
| **Evidence** | Весы / checkmark | SVG scale с balance indicator |

#### Hero концепция:

```
┌─────────────────────────────────────────────────┐
│                                                   │
│         F O R G E P L A N                        │
│                                                   │
│    From idea to implementation                   │
│    through structured artifacts                   │
│                                                   │
│         [SVG: наковальня morphs                  │
│          в граф зависимостей                     │
│          при скролле]                            │
│                                                   │
│    $ forgeplan health ▊                           │
│                                                   │
│         [Install]    [Docs]                       │
│                                                   │
└─────────────────────────────────────────────────┘
```

### 4.5 Анимации (адаптация qfnetwork.xyz)

| Эффект QF | Адаптация для Forgeplan | Техника |
|-----------|------------------------|---------|
| SVG morph эллипсов | Morph: наковальня → DAG → артефакт | GSAP MorphSVG или anime.js |
| Scroll pin sections | Pin: Hero → Features → CLI Demo → Install | GSAP ScrollTrigger |
| Chevron лента | Pipeline: Shape → Validate → Code → Evidence → Activate | CSS infinite scroll |
| Rings animation | Концентрические кольца = scoring rings (R_eff) | SVG + CSS animation |
| Border grid | Blueprint grid = visible layout borders | CSS border |
| Hover wipe | CTA hover с forge-ember wipe | CSS ::before |
| Dark-first | `#0D0D0D` loading, dark mode default | CSS variables + mode-watcher |

### 4.6 Секции лендинга (структура)

```
1. HERO
   ├── Bold headline: "Forgeplan"
   ├── Subtitle: "From idea to implementation"
   ├── SVG animation: forge → plan morph
   └── CTA: [Install] [Documentation]

2. WHAT IS IT (scroll-pinned)
   ├── "Universal platform for structured project artifacts"
   ├── 3 columns: Shape → Build → Verify
   └── SVG: lifecycle animation

3. CLI DEMO (scroll-pinned)
   ├── Terminal window с real commands
   ├── forgeplan health → forgeplan route → forgeplan new prd
   └── Animated typing effect

4. FEATURES (grid cards)
   ├── 10 Artifact Types (icon grid)
   ├── Quality Scoring (R_eff gauge)
   ├── Semantic Search (search demo)
   ├── Smart Routing (depth levels)
   ├── Evidence Tracking (timeline)
   └── FPF Reasoning (ADI cycle)

5. WORKFLOW PIPELINE (horizontal scroll)
   ├── Shape → Validate → ADI → Code → Evidence → Activate
   └── Chevron/arrow animation (как QF)

6. NUMBERS
   ├── 33 CLI commands
   ├── 28 MCP tools
   ├── 225+ tests
   └── 10 artifact types

7. INSTALL
   ├── cargo install forgeplan
   ├── brew install forgeplan
   ├── curl -fsSL install.sh | sh
   └── One-liner copy button

8. FOOTER
   ├── GitHub | Docs | Changelog
   └── "Built with Rust. Forged with purpose."
```

---

## 5. TUI Design System (ratatui)

### 5.1 Перенос визуального языка в терминал

| Web элемент | TUI аналог (ratatui) | Unicode |
|-------------|---------------------|---------|
| Visible borders | `Block::bordered()` | `─│┌┐└┘├┤┬┴┼` |
| Grid layout | `Layout::horizontal/vertical` | — |
| SVG lines | Braille drawing | `⠁⠂⠄⡀⠈⠐⠠⢀` |
| Progress arc | `Gauge` widget | `█░▓▒` |
| Color accent | `Style::fg(Color::Rgb(255,107,53))` | — |
| Blueprint grid | Double borders | `═║╔╗╚╝╠╣╦╩╬` |
| Hover | Focus highlight | inverse colors |
| Scroll animation | Smooth scroll / tick-based render | — |

### 5.2 TUI Color Palette

```rust
// Forge TUI palette (for ratatui)
const FORGE_DARK: Color    = Color::Rgb(13, 13, 13);     // #0D0D0D
const FORGE_LIGHT: Color   = Color::Rgb(232, 232, 232);  // #E8E8E8
const FORGE_MID: Color     = Color::Rgb(42, 42, 42);     // #2A2A2A
const FORGE_EMBER: Color   = Color::Rgb(255, 107, 53);   // #FF6B35
const FORGE_LINE: Color    = Color::Rgb(58, 58, 58);     // #3A3A3A
const FORGE_DIM: Color     = Color::Rgb(100, 100, 100);  // #646464

// Semantic colors
const STATUS_ACTIVE: Color  = Color::Rgb(76, 175, 80);   // green
const STATUS_DRAFT: Color   = Color::Rgb(158, 158, 158); // gray
const STATUS_STALE: Color   = Color::Rgb(255, 193, 7);   // amber
const SCORE_HIGH: Color     = Color::Rgb(76, 175, 80);   // green (R_eff > 0.7)
const SCORE_MED: Color      = Color::Rgb(255, 193, 7);   // amber (0.3-0.7)
const SCORE_LOW: Color      = Color::Rgb(244, 67, 54);   // red (< 0.3)
```

### 5.3 TUI Layout Patterns

```
┌─ FORGEPLAN ─────────────────────────────────────┐
│ Health │ Artifacts │ Graph │ Search │ Settings   │
├────────┬────────────────────────────────────────┤
│        │                                         │
│ ◆ PRD  │  PRD-018: OpenSpec DAG Integration      │
│ ◇ RFC  │  ══════════════════════════════         │
│ ◆ ADR  │                                         │
│ ◇ Note │  Status:  ● active                      │
│ ◆ Epic │  Score:   ████████░░ R_eff: 0.82        │
│        │  Depth:   Deep                          │
│ ─────  │  Evidence: 3 packs (2 support, 1 weak)  │
│ Score  │                                         │
│ ████░░ │  ## Problem                              │
│ 0.71   │  Need DAG for artifact dependencies...   │
│        │                                         │
├────────┴────────────────────────────────────────┤
│ > forgeplan health                         [cmd] │
└─────────────────────────────────────────────────┘
```

### 5.4 Reference: Claude Code CLI

Для TUI также изучить:
- **Claude Code CLI** — ink (React for terminals), TypeScript
- **charm.sh/bubbletea** — Go TUI framework, lipgloss для стилей
- **ratatui examples** — Rust TUI framework (наш выбор)
- **gitui** — Rust Git TUI, отличный reference для layout patterns
- **lazygit** — Go Git TUI, UX patterns

---

## 6. Tech Stack Recommendations (Website)

### Для Forgeplan лендинга:

| Компонент | QF использует | Рекомендация для Forgeplan | Почему |
|-----------|--------------|---------------------------|--------|
| Framework | SvelteKit | **Astro** (SSG) | Static site, zero JS по умолчанию, partial hydration |
| CSS | Tailwind | **Tailwind CSS** | Тот же подход, utility-first |
| Animations | GSAP + MorphSVG | **GSAP** (Community) + **anime.js** | MorphSVG платный, anime.js free alternative |
| SVG | Hand-crafted | **SVG + GSAP** | Тот же подход, без 3D |
| Deploy | ? | **GitHub Pages** или **Vercel** | Free, fast, git-integrated |
| Font | Fractul (custom) | **Space Grotesk** (Google Fonts) | Free geometric sans-serif |
| Icons | SVG inline | **Lucide** или custom SVG | Minimalist icon set |

### Альтернатива (если хотим Rust-native):

| Компонент | Технология | Плюсы |
|-----------|-----------|-------|
| SSG | **Zola** (Rust) | Rust ecosystem, fast, Tera templates |
| CSS | **Tailwind** (via CLI) | Не зависит от Node |
| JS | **Vanilla + GSAP** | Минимум зависимостей |

---

## 7. Next Steps

1. **[ ] Выбрать акцентный цвет**: ember (#FF6B35) vs steel (#4A9EFF) vs spark (#FFD700)
2. **[ ] Выбрать tech stack**: Astro vs Zola
3. **[ ] Создать SVG прототипы**: forge → plan morph concept
4. **[ ] Изучить Claude Code CLI**: исходники для TUI patterns
5. **[ ] Wireframe**: Figma/Pencil mockup лендинга
6. **[ ] TUI прототип**: ratatui demo с forge palette
7. **[ ] Домен**: forgeplan.dev? forgeplan.sh? forgeplan.rs?

---

## Appendix A: QF Network Animation Source (Key Code Patterns)

### SVG Morph Pattern (GSAP):
```javascript
// Scroll-driven SVG morphing (адаптировано из QF)
gsap.timeline({
  scrollTrigger: {
    trigger: "#hero-section",
    start: "top top",
    end: "+=100vh",
    scrub: true,
    pin: true,
  }
})
.to("#anvil-path", { morphSVG: "#dag-path", duration: 1 })
.to("#dag-path", { morphSVG: "#artifact-path", duration: 1 })
```

### CSS Variables Theme Switch:
```css
:root {
  --forge-bg: #0D0D0D;
  --forge-fg: #E8E8E8;
  --forge-line: #3A3A3A;
  --forge-accent: #FF6B35;
}

.light {
  --forge-bg: #E8E8E8;
  --forge-fg: #0D0D0D;
  --forge-line: #C0C0C0;
}
```

### Visible Border Grid:
```css
.section {
  border: 1px solid var(--forge-line);
  min-height: 100vh;
}

.grid-cell {
  border-right: 1px solid var(--forge-line);
  border-bottom: 1px solid var(--forge-line);
}
```

---

*Document generated: 2026-04-05*
*Sources: qfnetwork.xyz source analysis, Evil Martians devtool study, 15 reference sites*
