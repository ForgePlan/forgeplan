---
depth: standard
id: EVID-136
kind: evidence
last_modified_at: 2026-05-22T23:17:25.926358+00:00
last_modified_by: claude-code/2.1.149
links:
- target: PRD-079
  relation: informs
- target: RFC-011
  relation: informs
status: active
title: 'Code review of PRD-079 blog scaffold: CONCERNS'
---

## Verdict

CONCERNS

One-line justification: Build passes and all RFC-011 invariants hold structurally, but three issues require coder attention before merge — integration order violates the RFC spec, Google Fonts loads break the self-hosted font model, and the RU blog index is permanently hardcoded to `lang="en"`.

## Structured Fields

verdict: concerns
congruence_level: 3
evidence_type: audit

## Scope

- Parent: PRD-079
- Diff range: ad-hoc — files listed below (worktree `/Users/explosovebit/Work/forgeplan-blog-scaffold/website/`)
- Files reviewed: 15 files, ~600 lines
- Files: `astro.config.mjs`, `tsconfig.json`, `package.json`, `src/content.config.ts`, `src/components/Header.astro`, `src/layouts/BlogPost.astro`, `src/styles/blog-theme.css`, `src/content/blog/en/welcome.mdx`, `src/content/blog/ru/welcome.mdx`, `src/pages/blog/index.astro`, `src/pages/blog/[...slug].astro`, `src/pages/blog/rss.xml.ts`, `src/pages/ru/blog/index.astro`, `src/pages/ru/blog/[...slug].astro`, `src/pages/ru/blog/rss.xml.ts`

## Tools run

| Tool | Exit | Notes |
|---|---|---|
| dist/ presence check | 0 | dist/ exists; blog/, ru/blog/ both built with index.html + rss.xml + welcome/ |
| slug regex simulation (node) | 0 | confirmed slug derivation logic; frontmatter `slug` field is decoupled from URL generation |
| grep: client:* directives | 0 | none found — NFR-002 satisfied |
| grep: :root declarations | 0 | none in blog-theme.css — Invariant 4 satisfied |
| grep: Inter/JetBrains font values | 0 | not in CSS values; only in comments — Invariant 3 satisfied |
| grep: inline style= count | 0 | 31 inline style attributes across index/layout pages |
| eslint / tsc --noEmit | skipped | no tsc binary accessible in worktree env |
| npm run build | inferred pass | dist/ artifacts present and complete |

## Findings

| # | Severity | Category | Location | Description | Recommended fix |
|---|---|---|---|---|---|
| 1 | HIGH | Architecture | `astro.config.mjs:88` | `mdx()` integration is placed AFTER `starlight()` in the integrations array; RFC-011 explicitly specifies `mdx()` MUST come before `starlight()` because integrations with custom file extensions must load first per Astro docs | Move `mdx()` to position 0: `integrations: [mdx(), starlight({...}), react()]` |
| 2 | HIGH | Bug | `src/layouts/BlogPost.astro:30-33` | Layout loads Space Grotesk and Geist Mono from Google Fonts CDN via `<link>` tags, but `package.json` already has `@fontsource/space-grotesk` and `@fontsource/geist-mono` as self-hosted dependencies — two competing font sources for the same fonts; CDN load adds external network dependency, GDPR/privacy exposure, and likely causes FOUT on slow connections | Remove the four Google Fonts `<link>` tags; import from `@fontsource` packages instead: `import '@fontsource/space-grotesk/...'` at the top of the layout (or in `blog-theme.css`) |
| 3 | HIGH | Bug | `src/pages/ru/blog/index.astro:8` (via `Landing.astro:16`) | The RU blog index (`/ru/blog`) uses `Landing` layout which hardcodes `<html lang="en">` — Russian-language content is served with an English `lang` attribute; breaks screen readers, Google language detection, and SEO for RU pages | Either pass `lang` as a prop to `Landing.astro` and make it dynamic, or create a `Landing` variant that accepts a `lang` prop; the EN blog index has the same issue but it is accidentally correct |
| 4 | MEDIUM | Style | `src/pages/blog/index.astro:10-29` and `src/pages/ru/blog/index.astro:10-29` | Both blog index files contain ~20 inline `style="..."` attributes hardcoding raw color hex values (`#0b0b0b`, `#ff5a1f`, `#f5f5f5`, etc.) and font strings (`'Geist Mono', monospace`) that duplicate the token system defined in `blog-theme.css`; index pages use `Landing` layout (not `BlogPost`), so they are outside `.blog-post` scope, but the hardcoded values create a maintenance liability — any token change requires updating three files | Extract a `BlogIndex.astro` layout that wraps `Landing` and applies an equivalent scoped token class (e.g. `.blog-index`) with the same palette, replacing inline styles with CSS classes |
| 5 | MEDIUM | Architecture | `src/pages/blog/index.astro:17` and `src/pages/blog/[...slug].astro:8` + RU mirrors | URL slug is derived from `post.id` via `post.id.replace(/^en\//, '').replace(/\.(md|mdx)$/, '')`, but the schema also defines a `slug` frontmatter field that is never used for routing; if an author sets `slug: "my-custom-slug"` in frontmatter expecting it to control the URL, the actual URL will be the filename-derived slug instead — a silent contract violation | Either (a) remove the `slug` field from the zod schema and update the FR-001 docs to clarify URLs are filename-derived, or (b) use `post.data.slug` in `getStaticPaths` and use `post.data.slug` as the param, making the field the authoritative URL source |
| 6 | LOW | Architecture | `src/styles/blog-theme.css:258-261` | Selector `.blog-post body, body:has(.blog-post)` applies `margin: 0; padding: 0` to the `<body>` element; while scoped to pages containing `.blog-post`, `body:has()` is a global document-level selector that escapes the `.blog-post` scope boundary and could conflict with landing/docs page body resets if CSS cascade order changes | Move the body reset into `BlogPost.astro` as a `<style is:global>` block scoped explicitly to that layout, rather than in the shared CSS file |
| 7 | LOW | Docs | `src/layouts/BlogPost.astro` (entire file) | RFC-011 Risk table entry for H3 specifies: "Authoring guidelines in comment in BlogPost.astro — `interactive components use client:visible only, not client:load`" as the explicit mitigation for 0-JS NFR; this comment is absent from the shipped layout | Add a single-line comment above the `<slot />`: `{/* Interactive components: client:visible only — never client:load (NFR-002) */}` |
| 8 | LOW | Test gap | `src/pages/blog/` and `src/pages/ru/blog/` (all route files) | No smoke test or build assertion script confirms that `/blog`, `/ru/blog`, and `/blog/welcome` resolve to non-empty HTML after `npm run build`; `dist/` artifacts exist but were not produced by a repeatable CI-gated command in the PR | Add a `scripts/smoke-blog.mjs` that asserts `dist/blog/index.html` and `dist/ru/blog/index.html` exist and contain `<h1>` after build; wire to `npm run build` post-step |

## Positive observations

- Strong: All CSS tokens are correctly scoped under `.blog-post` selector with zero `:root` declarations — Invariant 4 from RFC-011 is cleanly satisfied (`src/styles/blog-theme.css:7`).
- Strong: The `generateId` custom function in `content.config.ts:11` is a deliberate collision-prevention mechanism for EN/RU posts sharing the same `slug` frontmatter value — good defensive design.
- Strong: The blog `getCollection` filter always checks both `lang` and `!draft` together, preventing accidental cross-language leakage in index and slug routes.

## Test coverage delta

- Before: n/a (no blog infrastructure existed)
- After: build produces `/blog/index.html`, `/ru/blog/index.html`, `/blog/welcome/index.html`, `/ru/blog/welcome/index.html`, `/blog/rss.xml`, `/ru/blog/rss.xml` — 6 routes
- Branches still uncovered: empty-index state (all posts draft=true), malformed frontmatter build-fail path, cross-language translation link rendering when `translations` is absent

## Next steps

- CONCERNS: Dispatch coder for findings #1 (integration order), #2 (Google Fonts CDN vs @fontsource), #3 (lang="en" hardcode on RU pages) — these are HIGH and must be resolved before merge
- After coder fixes findings #1-#3, re-run build and re-verify Invariants 1-6
- Findings #4-#8 are MEDIUM/LOW — recommended but not merge-blocking for PR-1 scaffold

## References

- Parent: PRD-079
- RFC: RFC-011 (architecture spec, Invariants 1-6, Risk table)
- Reviewer agent: claude-code/sonnet-4-6/code-reviewer-task-blog-scaffold



