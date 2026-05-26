# Handoff: следующая сессия после PR #327

> Версия: 2026-05-24
> Контекст: marathon-сессия, PR #327 merged in `dev`
> Кому: следующему Claude / dev, который сядет за этот worktree

---

## TL;DR

**Что произошло:**
PR #327 (`feat/blog-foundation` → `dev`) merged commit `2524f91d` 2026-05-24 07:14 UTC. 23 атомарных коммита, +16 471 / −136 LOC. CI: all-green после двух fix-итераций.

**Где мы сейчас:**
- Сайт `forgeplan.dev` имеет рабочий `/blog` с **10 evergreen лонгридов** (8 EN + 5 RU).
- `/docs/*` использует **наш unified Header** с Pagefind search и shrink-on-scroll.
- `/guides` каталог содержит **12 интерактивных уроков** (+FPF, +First Artifact добавлены).
- Tagline переписан без «Universal Rust» — теперь stack-agnostic.

**Что НЕ сделано** (backlog в этой же session):
1. Decision-cycle series Posts 2-8 (только outlines в `ForgePlanMarketing/posts/blog-series-decision-cycle.md`, готовых текстов нет).
2. Cover-картинки для blog-постов — prompts готовы в `website/docs/blog-image-prompts/`, генерация async.
3. 7 audit findings deferred (см. § «Audit backlog» ниже).

---

## Что лежит и где (карта репо)

### Worktree
```
/Users/explosovebit/Work/forgeplan-blog-foundation/    ← главный worktree
└── website/                                            ← Astro 6 + Starlight 0.38
    ├── src/
    │   ├── components/
    │   │   ├── Header.astro                ← landing/blog/guides
    │   │   ├── DocsHeader.astro            ← /docs slot via Starlight override
    │   │   ├── StarlightHeaderWrapper.astro
    │   │   ├── InstallMenu.astro           ← popup с cargo/brew/curl/AI-skill commands
    │   │   ├── SiteJsonLd.astro            ← JSON-LD structured data
    │   │   └── blog/
    │   │       ├── TOC.astro               ← !!! на самом деле полный sticky aside (Meta+TOC+Share+Author)
    │   │       ├── RelatedPosts.astro      ← series→topic→recent ranking
    │   │       ├── BlogSidebar.astro       ← на /blog index (About/Latest/Topics/Series/Archive)
    │   │       ├── SeriesBanner.astro      ← на /blog index
    │   │       ├── SeriesNav.astro         ← prev/next в посте если series
    │   │       ├── SeoMeta.astro
    │   │       ├── Hero.astro
    │   │       ├── TopicChip.astro
    │   │       ├── TopicFilter.astro
    │   │       └── BlogFooter.astro
    │   ├── content/
    │   │   ├── blog/
    │   │   │   ├── en/                     ← 8 постов сейчас
    │   │   │   └── ru/                     ← 5 постов сейчас
    │   │   └── docs/                       ← Starlight docs collection
    │   ├── content.config.ts               ← Astro Content Collection schema
    │   ├── data/guides.ts                  ← каталог /guides (12 entries)
    │   ├── layouts/
    │   │   ├── BlogPost.astro
    │   │   └── Landing.astro
    │   ├── pages/
    │   │   ├── blog/[...slug].astro + index + rss + series/[name]
    │   │   ├── ru/blog/...                 ← mirror
    │   │   ├── guides/[slug] + index
    │   │   └── ru/guides/...
    │   ├── styles/
    │   │   ├── global.css                  ← landing/blog/guides scope
    │   │   ├── forge-theme.css             ← /docs scope (Starlight customCss)
    │   │   └── blog-theme.css              ← blog-specific styles
    │   └── lib/
    │       ├── embed-html.ts               ← /guides HTML embed scoping
    │       └── reading-time.mjs            ← remark plugin
    ├── public/
    │   ├── guides-raw/                     ← 12 .html lessons (iframed/embedded)
    │   ├── blog-covers/                    ← !!! ПУСТО — covers ещё не сгенерированы
    │   └── llms.txt + robots.txt
    └── docs/blog-image-prompts/            ← READY-TO-PASTE prompts для GPT-Image/DALL-E
        ├── decision-graveyard.md           ← Post 1 detailed (cover + inline)
        └── series-decision-cycle-batch.md  ← Posts 2-8, 14 prompts
```

### Marketing repo (source of content drafts)
```
/Users/explosovebit/Work/ForgePlanMarketing/
├── STYLE-GUIDE.md                          ← voice rules, anti-anglicisms (читай §1.2 перед написанием поста)
├── CONTENT-CALENDAR-V2.md                  ← планы публикаций нед 13-20
├── posts/
│   ├── blog-series-decision-cycle.md       ← outlines 8 постов серии (Posts 2-8 ещё не написаны)
│   ├── longform/INDEX.md                   ← карта всех longform drafts
│   └── longform/habr-blog1-decision-graveyard.md  ← Post 1 готов (уже импортирован)
```

### Auto-memory (cross-session context)
```
/Users/explosovebit/.claude/projects/-Users-explosovebit-Work-ForgePlan/memory/
├── MEMORY.md                               ← index, auto-loaded каждый turn
├── project_pr327_blog_foundation_merged.md ← summary этой работы
├── feedback_iframe_vs_direct_embed.md
├── feedback_match_widths_footer.md
├── feedback_modern_dev_blog_pattern.md
├── reference_eli_rum_authorship.md         ← author = Eli Rum, NEVER «Mike Kubal»
├── reference_seo_geo_2026.md
├── architecture_css_scope_strip_root.md
├── architecture_sticky_header_h_var.md
└── ... ещё ~30 файлов
```

---

## Что в /blog сейчас (13 постов)

### EN (8)
| Slug | Topic | Kind |
|---|---|---|
| `welcome` | methodology | explainer |
| `decision-graveyard` | methodology | explainer |
| `git-for-decisions` | methodology | explainer |
| `markdown-source-of-truth` | methodology | explainer |
| `r-eff-weakest-link` | r-eff | deep-dive |
| `forgeplan-as-agent-harness` | methodology | case-study |
| `harness-engineering-for-ai-agents` | mcp | case-study |
| `forgeplan-rust-lancedb` | methodology | deep-dive |

### RU (5)
| Slug | Topic | Kind |
|---|---|---|
| `welcome` | methodology | explainer |
| `decision-graveyard` | methodology | explainer |
| `gde-zhivut-resheniya` | methodology | explainer |
| `obvyazka-dlya-ai-agentov` | mcp | case-study |
| `reshenia-ne-prompty` | methodology | deep-dive |

**Series**: только `decision-cycle` (1 пост, нужно ещё 7).

---

## Открытый backlog (priority order)

### 🔴 P0 — content gap
1. **Decision-cycle Posts 2-8** (RU тексты). Outlines в `ForgePlanMarketing/posts/blog-series-decision-cycle.md` §«Пост 2» … §«Пост 8». Каждый ~1500-2500 слов, PM-tone, минимум англицизмов. Применять STYLE-GUIDE §1.2 чёрный список замен. Slugs (предлагаемые):
   - Post 2 — `ru/before-fixing-write-three-versions`
   - Post 3 — `ru/averages-lie-trust-calculus`
   - Post 4 — `ru/decision-record-with-expiry`
   - Post 5 — `ru/architect-doesnt-start-with-blueprints`
   - Post 6 — `ru/spec-vs-prd-tasting-test`
   - Post 7 — `ru/forgeplan-pos-terminal`
   - Post 8 — `ru/full-cycle-on-one-feature`

   Каждый — отдельный MDX с frontmatter:
   ```yaml
   ---
   title: "<...>"
   description: "<150-250 chars summary>"
   slug: <slug>
   lang: ru
   publishedAt: 2026-08-01      # или твоя дата
   kind: explainer | deep-dive | case-study
   topic: methodology | r-eff | adi | fpf | mcp
   draft: false
   series: decision-cycle
   seriesOrder: <2..8>
   seriesDescription: "Цикл одного решения · 8 постов о дисциплине принятия архитектурных решений"
   translations:
     en: <slug-en>   # когда EN перевод будет
   ---
   ```

2. **EN-переводы** decision-cycle Post 2-8. После RU. Адаптация per brief (например для Post 1 заменили Stripe/Paddle → Stripe/Lemon Squeezy).

### 🟡 P1 — image generation
3. **Cover images** — prompts готовы в `website/docs/blog-image-prompts/`. Сгенерировать в GPT-Image / DALL-E 3, положить как `.webp` в `public/blog-covers/<slug>.webp`, добавить в frontmatter:
   ```yaml
   cover: '/blog-covers/<slug>.webp'
   ```
   Schema enforces `regex(/^\/[\w\-/.]+\.(png|jpg|jpeg|webp|svg)$/)` — только local paths.

### 🟢 P2 — audit followups (`fix/blog-foundation-polish` branch)
4. **H2 duplicate ID rename**: `#theme-toggle` → `#theme-toggle-docs` в `DocsHeader.astro` (footgun prevention).
5. **H3 perf — memoize getCollection**: текущий `/blog` index делает 5+ вызовов `getCollection('blog')`. Cache в `src/lib/blog.ts`.
6. **M3 Starlight API contract**: добавить `// STARLIGHT-API: depends on internal selector site-search > button[data-open-modal]` комментарий в `DocsHeader.astro` + grep-check в CI.
7. **M4 .nav-cell duplication**: `.nav-cell` и `.docs-nav-cell` живут параллельно из-за Tailwind preflight constraint. Не критично, но дрейфуют независимо.
8. **M5 rename**: `TOC.astro` → `BlogAside.astro` (имя не отражает содержимое — там Meta+TOC+Share+Author).
9. **F-1 CODEOWNERS**: pin `public/guides-raw/*` к security-reviewer (HTML executes with first-party origin).
10. **F-4 fail-loud**: `Astro.site ?? 'https://forgeplan.dev'` fallback в 4 местах. Лучше throw at build if missing.

### 🟡 P1 — operational
11. **Dependabot alerts** (2 на default branch — 1 moderate, 1 low). Проверить перед next release per CLAUDE.md RED LINE #10. Команда:
    ```bash
    gh api repos/ForgePlan/forgeplan/dependabot/alerts --jq '.[] | {severity: .security_advisory.severity, package: .security_advisory.vulnerabilities[0].package.name, summary: .security_advisory.summary}'
    ```

---

## Конвенции которые нужно знать

### Author = Eli Rum (only)
- `https://elirum.me` — author page
- НИКОГДА не писать «Mike Kubal», «Forgeplan Author», «Anonymous», placeholder
- Codified в `AGENTS.md`, mirrored в auto-memory `reference_eli_rum_authorship.md`

### Header pair (две параллели — синхронизировать)
- `Header.astro` — для landing/blog/guides (внутри `#site-header` fixed)
- `DocsHeader.astro` — для /docs (внутри Starlight slot, БЕЗ outer `<header>` тега)
- Любая правка nav order / GitHub icon / Install popup / lang toggle — в **обоих**

### CSS scope rule (KRITICHESKI)
- `global.css` — landing/blog/guides
- `forge-theme.css` — /docs (loaded as Starlight customCss)
- **Нельзя импортировать `global.css` на /docs** — Tailwind preflight `* { padding: 0 }` коллапсит Starlight grid. Любые tokens нужные на /docs — mirror в `forge-theme.css`.

### Blog content schema (`src/content.config.ts`)
- `kind`: `explainer | case-study | teaching | release-notes | deep-dive`
- `topic`: `r-eff | adi | fpf | mcp | methodology | release`
- `cover`: regex-constrained local path
- `translations`: `{ en: slug, ru: slug }` для cross-link
- `series` + `seriesOrder` + `seriesDescription` для серий
- generateId использует file path (НЕ `slug` поле) — потому что слаги дублируются между EN и RU

### Blog post sidebar (TOC.astro)
- Aside рендерится всегда (даже без h2/h3 headings)
- 4 секции: About this post → TOC → Share → Author
- TOC секция показывается only when `tocHeadings.length > 0`
- Sticky position calculated from `--header-h` CSS var (DocsHeader публикует это var)

### RelatedPosts ranking
1. Same series + `seriesOrder ASC` (если оба score=3)
2. Same topic
3. Recent (publishedAt DESC)
4. Renders 3 cards if ≥1 candidate

### Header nav order (final)
```
F●RGEPLAN  Docs · Blog · Guides · CLI Ref · MCP Tools · RU/EN · Install▾ · [GitHub icon] · [theme toggle]
```
Mobile drawer — тот же порядок vertically.

### Install popup (`InstallMenu.astro`)
- 4 commands: cargo / brew / shell curl / AI skill
- Click outside / Escape close
- Copy buttons с `✓` feedback (1.2s, no layout jump — fixed 24×24 box)
- Linked from `/#install` for "All install options" footer

### Grid templates (must stay in sync)
- Full: `.header-full .header-nav { grid-template-columns: repeat(4, 110px) 140px 60px 105px 48px }` (5 text · RU · Install · GitHub)
- Compact: `.header-compact .header-nav { grid-template-columns: repeat(4, 74px) 94px 44px 76px 36px }`

### MCP tool count drift
- Health Gate CI check rejects mentions of outdated «63 MCP tools» — current = 73
- При написании поста с упоминанием числа: ЛИБО актуализировать LIBO добавить комментарий «(2026-05 snapshot)»

---

## Команды quick-start для новой сессии

### Чтобы продолжить работу в worktree
```bash
cd /Users/explosovebit/Work/forgeplan-blog-foundation
git status                                    # должна быть чистая на feat/blog-foundation
git log --oneline -5                          # увидишь 23 коммитов до merge

# Запустить dev server
cd website && npm run dev > /tmp/astro-dev.log 2>&1 &
# Ready: http://localhost:4321/

# Build verify
npm run build                                 # ожидаем: 387+ pages, 0 errors
```

### Чтобы создать новый пост (Decision-cycle Post 2)
```bash
# 1. Прочитать outline:
cat /Users/explosovebit/Work/ForgePlanMarketing/posts/blog-series-decision-cycle.md | sed -n '/## Пост 2/,/## Пост 3/p'

# 2. Прочитать STYLE-GUIDE:
cat /Users/explosovebit/Work/ForgePlanMarketing/STYLE-GUIDE.md | sed -n '/## Часть 1/,/^## Часть 2/p'

# 3. Создать файл:
$EDITOR /Users/explosovebit/Work/forgeplan-blog-foundation/website/src/content/blog/ru/before-fixing-write-three-versions.mdx

# 4. Verify:
cd /Users/explosovebit/Work/forgeplan-blog-foundation/website && npm run build
```

### Чтобы создать follow-up branch для audit fixes
```bash
cd /Users/explosovebit/Work/forgeplan-blog-foundation
git fetch origin
git checkout -b fix/blog-foundation-polish origin/dev
# работаем по списку P2 backlog выше
```

### Чтобы сгенерировать cover image
```bash
# 1. Скопируй EN prompt из:
cat /Users/explosovebit/Work/forgeplan-blog-foundation/website/docs/blog-image-prompts/series-decision-cycle-batch.md
# (или decision-graveyard.md для Post 1)

# 2. Вставь в GPT-Image (ChatGPT Plus/Pro) или Midjourney v6
# 3. Скачай результат, конвертируй в WebP, положи:
cp ~/Downloads/cover.png /Users/explosovebit/Work/forgeplan-blog-foundation/website/public/blog-covers/<slug>.webp

# 4. Добавь в frontmatter поста:
#    cover: '/blog-covers/<slug>.webp'
```

---

## RED LINES (нельзя нарушать)

Из главного CLAUDE.md:
1. **DO NOT push to `feat/blog-foundation` после merge** — squash loses late commits. Создавать новые branches от `dev`.
2. **DO NOT git push без user approval после ревью** (RED LINE #2).
3. **DO NOT commit directly to `dev` или `main`** — всегда feature branch → PR → merge.
4. **DO NOT редактировать `.forgeplan/{prds,adrs,specs,rfcs,evidence,notes}/*.md` напрямую** через `Edit`/`Write`/`sed`. Только через `forgeplan` CLI или MCP tools.
5. **Authorship**: Eli Rum + `https://elirum.me`. Никаких placeholders.

---

## Decision log (почему так)

- **Sticky aside в blog**: Smashing-style (TOC + meta + share + author) выбрано из FPF brainstorm. Альтернативы (Substack/Vercel-style full-width without sidebar) отвергнуты — user explicitly попросил sidebar.
- **GitHub icon last**: внешние/утилитарные ссылки (RU · Install · GitHub) сгруппированы справа. Внутренние ссылки (Docs · Blog · Guides · CLI · MCP) — слева.
- **Install как popup, не якорь**: на /blog/* и /docs/* якорь `#install` не существует. Popup даёт consistent UX на всех routes.
- **Tagline без «Universal Rust»**: PM/head-of-eng filter-out при первом сканировании. FPF brainstorm дал 2 финалиста: A = «Engineering methodology for decisions that last» (meta+JSON-LD), B = «Shape decisions. Score evidence. Ship with confidence.» (visible footer).
- **`cover` schema regex**: F-3 security finding — защищает от `javascript:` / `data:` / `//evil.tld` schemes если в будущем будем принимать posts от внешних авторов.
- **transition-property explicit (no `all`)**: full↔compact toggle создавал «fat double border flicker» когда оба border-bottom анимировались. Решение: исключить border-* из transition.
- **InstallMenu copy button — fixed 24×24**: предыдущая версия меняла intrinsic width при свопе SVG ↔ ✓. Сейчас absolute positioning поверх invisible svg.

---

## Где найти контекст если что-то непонятно

| Вопрос | Куда смотреть |
|---|---|
| Что было решено и почему | `git log --oneline origin/dev` (23 коммита от `b962812..0d40517`) |
| Voice / tone правила | `/Users/explosovebit/Work/ForgePlanMarketing/STYLE-GUIDE.md` §1 |
| Расписание публикаций | `/Users/explosovebit/Work/ForgePlanMarketing/CONTENT-CALENDAR-V2.md` |
| Outline для непоказанных постов серии | `/Users/explosovebit/Work/ForgePlanMarketing/posts/blog-series-decision-cycle.md` |
| Audit findings (verbatim) | `git log --grep "audit quick wins" --grep "drift" --all` или этот файл § «Audit backlog» |
| Какие image prompts | `website/docs/blog-image-prompts/` |
| Кто merge'нул PR | `gh pr view 327 --repo ForgePlan/forgeplan --json mergedBy,mergedAt` |

---

## Что НЕ ломать

- Existing 10 blog posts — body verbatim из marketing drafts (не переписывать без причины)
- Header pair sync — Header.astro + DocsHeader.astro changes должны быть симметричны
- `global.css` ↔ `forge-theme.css` mirror — skip-link и forge tokens в обоих
- Schema constraints в `content.config.ts` — особенно `generateId` (9 call-sites зависят от format `{lang}/{slug}`)

---

**Конец handoff. Удачи.**
