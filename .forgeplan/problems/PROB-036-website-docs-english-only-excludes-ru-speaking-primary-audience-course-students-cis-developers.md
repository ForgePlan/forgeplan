---
depth: tactical
id: PROB-036
kind: problem
status: deprecated
title: Website docs English-only — excludes RU-speaking primary audience (course students, CIS developers)
---

# PROB-036: Website docs English-only

## Signal

Forgeplan website (147 pages, PRD-046) fully English. Primary audience — RU-speaking course students and CIS developers — must read technical methodology docs in a second language. CLAUDE.md itself is Russian, CLI output is English, methodology guide is Russian in repo but English on website. Disconnect between repo language and public docs language.

Course material (`forgeplan-course-brief.md`) targets Russian-speaking students who will need docs reference during lab exercises.

## Constraints

- Starlight framework must support the i18n approach natively (no custom SSR)
- Existing EN URLs (`/docs/cli/init/`) must NOT break (SEO, backlinks, bookmarks)
- Translation must be reproducible on each release (not manual-only)
- Code blocks, CLI commands, frontmatter YAML must remain English in RU translation
- Build time must stay under 15s for ~300 pages (2× current 147)

## Optimization Targets

- RU docs coverage: 100% of pages available in Russian
- Translation quality for P0 pages (getting-started, methodology): human-reviewed
- Drift detection: stale RU translations flagged within 1 commit of EN change

## Observation Indicators (Anti-Goodhart)

- Page count alone (having 147 RU stubs with machine gibberish = worse than 0)
- Translation speed (rushing = quality loss)
- Build time regression (doubling pages could push past budget)

## Acceptance Criteria

- [ ] Starlight i18n configured: `defaultLocale: 'en'`, RU locale at `/ru/docs/...`
- [ ] EN URLs unchanged (no `/en/` prefix for default locale)
- [ ] Language switcher visible in Starlight sidebar
- [ ] 147+ RU pages in `src/content/docs/ru/docs/`
- [ ] Glossary file with 30+ canonicalized terms
- [ ] Machine-translated pages flagged with disclaimer
- [ ] P0 pages (getting-started/*, methodology/*) manually reviewed
- [ ] Landing page strings extracted + translated
- [ ] `npm run build` passes with ~294 pages, <15s
- [ ] Pagefind indexes both EN and RU
- [ ] Drift checker script exists and reports outdated RU pages

## Blast Radius

- **High**: `website/` — all content restructured (docs/ → en/docs/ + ru/docs/)
- **Medium**: SEO — hreflang tags, sitemap per locale, canonical URLs
- **Low**: Rust crate code — no changes

## Reversibility

**Medium** — folder restructure is a `git mv` that can be reverted, but Starlight i18n config affects URL routing. Reverting after deploy would break RU bookmarks.

---

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-046 | based_on (EN docs completed) |
| PRD-047 | informs (solution) |
| PRD-024 | based_on (original website) |


