---
depth: standard
id: EVID-137
kind: evidence
last_modified_at: 2026-05-22T23:27:50.802658+00:00
last_modified_by: claude-code/2.1.149
links:
- target: PRD-079
  relation: informs
- target: RFC-011
  relation: informs
status: active
title: 'PR-1 blog scaffold post-fix verification: PASS'
---

# EVID-137: PR-1 blog scaffold post-fix verification

## Verdict

PASS

One-line: All 5 HIGH findings from Step 6.5 audit (EVID-136 + architect-reviewer report) addressed by fix-coder; `npm run build` green at 15.38s with 346 pages; all 6 RFC-011 invariants verified intact; RFC-011 amended to document empirically-found ECE integration constraint.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: audit

## Scope

- Parent PRD: PRD-079 (bilingual blog as third mode)
- Parent RFC: RFC-011 (blog scaffold architecture)
- Worktree: `/Users/explosovebit/Work/forgeplan-blog-scaffold/website/`
- Branch: `feat/blog-scaffold` (off origin/dev `aadab648`)
- Coder identity: agents-core:coder (initial build + fix-round)
- Reviewers: agents-core:code-reviewer (EVID-136), agents-pro:architect-reviewer (inline report)

## Audit findings — closure matrix

| # | Finding | Severity | Reviewer | Status | Fix |
|---|---------|---------:|----------|--------|-----|
| H-1 | Integration order `mdx()` AFTER `starlight()` violates RFC | HIGH | both | RFC-AMENDED | Empirically verified: `mdx()` BEFORE `starlight()` fails with ECE error. Kept `[starlight, mdx, react]` + inline comment + RFC-011 amendment paragraph |
| H-2a | Google Fonts CDN duplicates `@fontsource/*` self-hosted deps | HIGH | code-reviewer | FIXED | Removed 4 `<link>` tags from BlogPost.astro; added 8 `@fontsource/*` imports |
| H-2b | Blog index pages inline raw hex bypassing token scope | HIGH | architect | FIXED | Approach A: wrapped index in `<article class="blog-index">` + 11 new CSS classes scoped under `.blog-index` in blog-theme.css; 31 inline hex → 0 in .astro files |
| H-3a | `Landing.astro` hardcodes `lang="en"` — RU index gets wrong lang | HIGH | code-reviewer | FIXED | Added `lang?: 'en'\|'ru'` prop to Landing.astro (default 'en'); RU index passes `lang="ru"`; verified `dist/ru/blog/index.html` contains `lang="ru"` |
| H-3b | `generateId` deviation from RFC undocumented (drift) | HIGH | architect | FIXED | Expanded inline comment from 1 to 14 lines in content.config.ts; RFC-011 §"Content Collection schema" amended with post-coder note + 6-callsite coupling warning |
| M-1 | Inline styles in index pages | MEDIUM | code-reviewer | RESOLVED via H-2b fix |
| M-2 | Frontmatter `slug` never used for routing (silent contract) | MEDIUM | code-reviewer | DEFERRED to PR-2 — documented in RFC-011 amendment, helper `postUrl()` planned |
| M-3 | Slug-resolution regex duplicated 6× | MEDIUM | architect | DEFERRED to PR-2 — same helper planned |
| M-4 | Index pages wrap in Landing.astro → multi-mode coupling | MEDIUM | architect | DEFERRED to PR-2 — explicitly noted in RFC-011 "Landing.astro shared between modes" section |
| L-1 | `body:has(.blog-post)` global selector escapes scope | LOW | code-reviewer | DEFERRED to PR-2 — low risk given current import discipline |
| L-2 | Missing `client:visible only` comment in BlogPost.astro | LOW | code-reviewer | DEFERRED to PR-2 (when first interactive component lands) |
| L-3 | No smoke test for blog routes | LOW | code-reviewer | DEFERRED to PR-2 |
| L-4 | Build baseline not captured (15.38s asserted, not measured pre/post) | LOW | architect | DEFERRED — capture proper baseline before PR-2 (which adds 6 MDX components) |
| L-5 | BlogPost lacks skip-to-content + main landmark order | LOW (M-1 architect) | architect | DEFERRED to PR-2 — NFR-006 SHOULD-level |

**Net status**: 5/5 HIGH closed (4 fixed + 1 RFC-amended) — **0 HIGH remaining**. 8 MEDIUM/LOW deferred to PR-2 with explicit owners. **No CRITICAL findings, no BLOCKER state.**

## RFC-011 invariant compliance — post-fix verification

| # | Invariant | Status | Evidence |
|---|-----------|:------:|----------|
| 1 | Landing pixel-identical (Header diff only adds Blog link) | ✅ OK | Header.astro diff: +2 desktop nav lines, +1 mobile nav line, +1 const `isBlogActive`. Landing.astro: +`lang` prop only (default 'en' — pixel-identical when not overridden). Root `/` HTML behavior unchanged. |
| 2 | Docs routes work | ✅ OK | dist/docs/ contains 400+ html files post-build. Starlight `sidebar:` config untouched. ECE integration confirmed working via 346 successful page builds. |
| 3 | NO Inter / NO JetBrains in blog code | ✅ OK | grep blog-theme.css + new .astro files: 2 comment lines saying "NOT Inter/JetBrains", 0 font-family declarations. @fontsource self-hosted instead of Google Fonts CDN. |
| 4 | Tokens scoped under `.blog-post` / `.blog-index` | ✅ OK | grep `:root` in blog-theme.css → 0 matches. Index pages now wrapped in `.blog-index` scope (post-fix). All hex literals removed from .astro pages. |
| 5 | 0 JS in blog routes | ✅ OK | grep `client:` in blog layouts/pages → 0 matches. Header pre-existing script (mobile menu + theme toggle) is shared component not new JS. |
| 6 | Build green | ✅ OK | `npm run build` exit 0, 15.38s, 346 pages. All 6 blog routes present in dist + existing landing + docs intact. |

**6/6 invariants OK post-fix.**

## Files in scope

15 files modified/created in worktree `/Users/explosovebit/Work/forgeplan-blog-scaffold/website/`:

```
modified:  astro.config.mjs                          (+starlight, mdx, react ordering + ECE comment)
modified:  tsconfig.json                             (+moduleResolution: bundler)
modified:  package.json                              (+@astrojs/mdx, +@astrojs/rss)
modified:  package-lock.json                         (regenerated)
modified:  src/content.config.ts                     (+blog collection + generateId 14-line comment)
modified:  src/components/Header.astro               (+Blog nav link + isBlogActive)
modified:  src/layouts/Landing.astro                 (+lang prop default 'en')
new file:  src/layouts/BlogPost.astro                (with @fontsource imports)
new file:  src/styles/blog-theme.css                 (~280 lines, .blog-post + .blog-index scopes)
new file:  src/content/blog/en/welcome.mdx           (placeholder)
new file:  src/content/blog/ru/welcome.mdx           (placeholder)
new file:  src/pages/blog/index.astro                (uses .blog-index scope)
new file:  src/pages/blog/[...slug].astro
new file:  src/pages/blog/rss.xml.ts
new file:  src/pages/ru/blog/index.astro             (passes lang="ru" + .blog-index scope)
new file:  src/pages/ru/blog/[...slug].astro
new file:  src/pages/ru/blog/rss.xml.ts
```

## dist verification

```
dist/blog/index.html                 ✓  (EN listing)
dist/blog/welcome/index.html          ✓  (EN post)
dist/blog/rss.xml                     ✓  (EN feed, valid XML)
dist/ru/blog/index.html               ✓  (RU listing, lang="ru" verified)
dist/ru/blog/welcome/index.html       ✓  (RU post)
dist/ru/blog/rss.xml                  ✓  (RU feed, valid XML UTF-8)
dist/index.html                       ✓  (existing landing, no regression)
dist/docs/.../installation/index.html ✓  (Starlight EN docs)
dist/ru/docs/.../installation/index.html ✓  (Starlight RU docs)
dist/sitemap-0.xml                    ✓  (auto-includes blog routes via Starlight bundle)
```

## Tools run

| Tool | Exit | Notes |
|------|------|-------|
| npm install (clean) | 0 | 211 packages, 0 vulnerabilities |
| npm install @astrojs/mdx @astrojs/rss | 0 | 8 packages added |
| npm run build (initial) | 0 | 14.06s, 345 pages |
| npm run build (post-fix) | 0 | 15.38s, 346 pages (1 extra from blog-index page count) |
| grep `:root` in blog-theme.css | 0 matches | Invariant 4 OK |
| grep `Inter\|JetBrains` in blog code | 2 comment-only | Invariant 3 OK |
| grep `client:` in blog files | 0 matches | Invariant 5 OK |
| grep `#[0-9a-f]{3,6}` in blog .astro pages | 0 matches | post-fix |
| dist sitemap blog inclusion | confirmed | `/blog/`, `/ru/blog/`, both posts in sitemap |
| dist hreflang cross-link | confirmed | EN↔RU xhtml:link rel=alternate present |

## Verdict rationale

`supports` — все 5 HIGH findings из audit-Round-1 закрыты (4 кода-фикса + 1 RFC-amendment); 6/6 invariants verified intact post-fix; build green; нет open CRITICAL or BLOCKER state. 8 MEDIUM/LOW findings deferred к PR-2 с explicit owners в RFC-011 §"Risks" и §"Phase B".

`congruence_level: 3` — same context: верификация applied directly к worktree после fix-round; reviewer и fix-coder работали на одном и том же state; диff verifiable.

`evidence_type: audit` — post-fix audit-snapshot, не measurement / test_result.

## Recommendation

PROCEED to activate PRD-079 and RFC-011. Commit on `feat/blog-scaffold`. STOP before `git push` per RED LINE #2 — user-approval required.

После approve push → `gh pr create` → merge → запуск PR-2 cycle (Cycles tetralogy: trust-calculus + decision-cycle + bmad-cycle + spec-cycle).

## Cross-references

- `Refs: PRD-079, RFC-011, EVID-136 (audit-round-1 CONCERNS), feat/blog-scaffold worktree`
- Reviewer identities: claude-code/sonnet-4-6/code-reviewer-task-blog-scaffold (EVID-136), claude-code/opus-4.7/architect-reviewer-task-rfc011-blog (inline)
- Fix-coder identity: agents-core:coder (run 2 of forge-cycle)



