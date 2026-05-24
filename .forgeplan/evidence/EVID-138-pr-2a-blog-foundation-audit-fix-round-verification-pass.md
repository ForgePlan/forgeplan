---
depth: standard
id: EVID-138
kind: evidence
last_modified_at: 2026-05-23T00:38:44.078116+00:00
last_modified_by: claude-code/2.1.149
links:
- target: PRD-080
  relation: informs
- target: RFC-012
  relation: informs
status: active
title: 'PR-2A blog foundation: audit + fix-round verification PASS'
---

# EVID-138: PR-2A blog foundation — audit + fix-round verification

## Verdict

PASS

One-line: All 12 FRs implemented; adversarial 2-reviewer audit found 1 CRITICAL + 4 HIGH + 5 MEDIUM/LOW; CRITICAL + 4 HIGH + 4 MEDIUM/LOW closed by fix-coder в one round; build green 13.34s (vs baseline 15.38s — NEGATIVE degradation); 7/7 RFC-012 invariants intact.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: audit

## Scope

- Parent PRD: PRD-080 (Blog navigation + visual + SEO foundation)
- Parent RFC: RFC-012 (Schema v2 + TOC + hero + SEO meta + Callout)
- Worktree: `/Users/explosovebit/Work/forgeplan-blog-foundation/website/`
- Branch: `feat/blog-foundation` (off `9d426e0` PR-1 merge)
- Reviewers: agents-core:code-reviewer + agents-pro:architect-reviewer (parallel)
- Fix-coder: agents-core:coder (single round)
- Coordinator: forge-cycle/v2+pr-2a-orchestrator

## Audit findings — closure matrix

| # | Finding | Sev | Reviewer | Status | Fix |
|---|---------|----:|---|---|---|
| C-1 | JSON-LD XSS via unescaped `</script>` in title/description | CRITICAL | code-reviewer | FIXED | `.replace(/</g,'<')...` before set:html в SeoMeta.astro |
| H-1 | hreflang self-link missing (Google duplicate-content penalty) | HIGH | both reviewers | FIXED | emitEn/emitRu fallback to canonical когда lang matches; verified в dist/blog/welcome/index.html содержит `hreflang="en"` self-link |
| H-2 | og:image SVG rejected by Twitter/X/LinkedIn | HIGH | code-reviewer | FIXED | sharp SVG→PNG conversion, 27 KB, dist references `/og-default.png` |
| H-3 | `.blog-hero-topic-*` CSS rules missing — dead contract | HIGH | architect-reviewer | FIXED | 12 CSS rules added (6 topics × stripe + eyebrow) referencing `--t-<topic>` |
| H-4 | RU posts render English nav (SeriesNav/AuthorBlock/TOC labels) | HIGH | architect-reviewer | FIXED | Localised maps + lang prop wiring; verified `dist/ru/blog/welcome/index.html` содержит «Автор» |
| M-1 | Callout inline `style={...}` couples logic+style | MEDIUM | code-reviewer | FIXED | 4 CSS rules `.blog-callout-<type> .blog-callout-icon` |
| M-2 | TOC mobile = display:none, RFC promised `<details>` fallback | MEDIUM | architect | FIXED | Two-sibling pattern (aside desktop + details mobile), responsive switching |
| M-3 | generateId comment stale (6 callsites listed; 9 actual после PR-2A) | MEDIUM | architect | FIXED | Comment updated to enumerate 9 call-sites |
| L-1 | AuthorBlock `<div>` → semantic `<address>` | LOW | code-reviewer | FIXED | (bundled with H-4 fix) |
| L-2 | JSON-LD missing `publisher` field (SEO best-practice) | LOW | code-reviewer | DEFERRED | PR-2B (cosmetic) |
| L-3 | SeriesNav idx===-1 silent failure surface | LOW | architect | DEFERRED | PR-2B (observability) |
| L-4 | Series landing 0-pages silent (no warn log) | LOW | code-reviewer | DEFERRED | PR-2B (observability) |
| M-4 | CSS rollback path для shared blog-theme.css | MEDIUM | architect | ACKNOWLEDGED | Section markers `/* === PR-2A: visual primitives === */` + `/* === PR-2A: discovery === */` дают rollback boundaries; partial rollback документировано в RFC-012 |

**Net**: 1 CRITICAL fixed, 4/4 HIGH fixed, 3/5 MEDIUM/LOW fixed, 3 LOW deferred to PR-2B с явным owner. **0 actively-open CRITICAL or HIGH**.

## RFC-012 invariant compliance — post-fix verification

| # | Invariant | Status | Evidence |
|---|-----------|:---:|---|
| 1 | welcome.mdx (PR-1) still functional | ✅ | dist/blog/welcome + dist/ru/blog/welcome built; CSS hero variant active (no cover field); no SeriesNav (no series field); description+meta present |
| 2 | /docs/* + /ru/docs/* resolve 200 | ✅ | dist/docs/* contains 400+ html files; Starlight sidebar/search/mermaid intact; integration order [starlight, mdx, react] preserved |
| 3 | Font stack: Space Grotesk + Geist Mono ONLY | ✅ | grep blog-theme.css + new components: 0 Inter/JetBrains references; @fontsource/* self-hosted |
| 4 | Tokens scoped to `.blog-post, .blog-index` | ✅ | grep `:root` в blog-theme.css → 0 matches; 6 topic colors под `.blog-post, .blog-index { --t-*: ... }` |
| 5 | 0 client:* directives | ✅ | grep `client:` across 7 new components + 2 modified layouts → 0 matches |
| 6 | Build degradation ≤ 15% | ✅ | Final build 13.34s vs PR-1 baseline 15.38s = **−13% (negative degradation, faster)** |
| 7 | RFC-011 6 invariants from PR-1 still hold | ✅ | Landing.astro modified only +1 optional `lang` prop (default 'en' = no behavior change для root /); Header.astro untouched; existing routes preserved |

**7/7 invariants OK post-fix.**

## Files in scope

20 files modified/created:

```
NEW components (7):
  src/components/blog/{TOC,Hero,Callout,SeriesNav,TopicChip,AuthorBlock,SeoMeta}.astro

NEW infra (3):
  src/pages/blog/series/[name].astro
  src/pages/ru/blog/series/[name].astro
  src/lib/reading-time.mjs

NEW assets (2):
  public/og-default.svg (source)
  public/og-default.png (27 KB, sharp-converted)

MODIFIED (8):
  src/content.config.ts            (Lead Phase A: +3 schema fields + comment update)
  src/layouts/BlogPost.astro       (Worker 1 + Lead: hero/TOC/SeriesNav/SeoMeta/AuthorBlock integration)
  src/styles/blog-theme.css        (Both workers + fix-round: visual primitives + discovery + hero-topic rules)
  src/pages/blog/index.astro       (Worker 2: chips + thumbnails)
  src/pages/ru/blog/index.astro    (Worker 2: same)
  src/pages/blog/[...slug].astro   (Lead: pass new props)
  src/pages/ru/blog/[...slug].astro (Lead: pass new props)
  astro.config.mjs                 (Worker 2: remark plugin)
  package.json + lock              (+ mdast-util-to-string@4.0.0)
```

## dist verification (post-fix-round)

```
dist/blog/index.html                    ✓ (topic-chip-methodology present, thumbnail gradient)
dist/blog/welcome/index.html            ✓ (hero CSS variant, JSON-LD, hreflang="en" self + "ru" alt, AuthorBlock)
dist/blog/rss.xml                       ✓ (PR-1 RSS preserved)
dist/ru/blog/index.html                 ✓ (topic chips + thumbnails)
dist/ru/blog/welcome/index.html         ✓ («Автор» RU label, hreflang="ru" self + "en" alt)
dist/ru/blog/rss.xml                    ✓ (PR-1 RSS preserved)
dist/index.html                         ✓ (Landing untouched, lang="en" default)
dist/docs/**                            ✓ (400+ pages, Starlight integration)
dist/sitemap-0.xml                      ✓ (blog routes included, hreflang annotations)
public/og-default.png                   ✓ (27 KB, 1200×630, brand-coherent)
```

## Tools run

| Tool | Exit | Notes |
|------|------|-------|
| npm install (clean) | 0 | Baseline restore |
| npm install mdast-util-to-string | 0 | New dep for reading-time |
| sharp SVG→PNG CJS | 0 | og-default.svg @density=200 → og-default.png 1200×630 |
| npm run build (post-Phase A schema) | 0 | 14.28s |
| npm run build (post-worker consolidation) | 0 | 13.00s |
| npm run build (post-fix-round) | 0 | 13.34s |
| grep `:root` blog-theme.css | 0 matches | Invariant 4 |
| grep `client:` new files | 0 matches | Invariant 5 |
| grep `Inter\|JetBrains` blog code | 0 matches | Invariant 3 |
| dist `topic-chip-methodology` | 1+ match | FR-008 verify |
| dist `hreflang="en"` on EN welcome | 1+ match | FIX-2 verify (self-link present) |
| dist `Mike Kubal` / `Автор` | matches both lang | FR-010 + RU localisation |

## Verdict rationale

`supports` — 12/12 FRs реализованы; 1 CRITICAL + 4/4 HIGH closed; 3/5 MEDIUM/LOW closed, 3 LOW deferred с явным owner; 7/7 invariants intact; build negative degradation. Нет active CRITICAL/HIGH блокеров.

`congruence_level: 3` — same context: audit + fix-round + verification all на one branch state; reviewers и fix-coder работали на одном snapshot; диff verifiable.

`evidence_type: audit` — adversarial review + fix verification — не measurement, не test_result.

## Recommendation

PROCEED to activate PRD-080 + RFC-012. Commit в 1-2 коммита на `feat/blog-foundation`. STOP перед push per RED LINE #2 — user-approval required.

После approve push → `gh pr create --base dev` → merge → запуск PR-2B cycle (Cycles tetralogy launch).

## Deferred items (PR-2B scope)

- JSON-LD publisher field (cosmetic SEO completeness)
- SeriesNav idx===-1 build-time warn (observability for series authoring errors)
- Series landing 0-pages warn log (observability)

Все 3 — LOW severity, не блокеры. Documented в RFC-012 backlog при PR-2B kickoff.

## Cross-references

- `Refs: PRD-080, RFC-012, PRD-079 (parent), RFC-011 (parent), feat/blog-foundation worktree, EVID-136 (PR-1 audit), EVID-137 (PR-1 post-fix)`
- Reviewer identities: claude-code/sonnet-4-6/code-reviewer-task-pr-2a, claude-code/opus-4.7/architect-reviewer-task-pr-2a
- Fix-coder: agents-core:coder (one round, 9 fixes)
- FPF Evaluate output (Trust Calculus 22 candidates → 12 MUST PR-2A) — basis for FR selection



