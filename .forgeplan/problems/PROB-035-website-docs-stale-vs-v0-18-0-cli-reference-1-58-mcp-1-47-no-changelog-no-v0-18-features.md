---
depth: tactical
id: PROB-035
kind: problem
status: deprecated
title: Website docs stale vs v0.18.0 — CLI reference 1/58 MCP 1/47 no CHANGELOG no v0.18 features
---

# PROB-035: Website docs stale vs v0.18.0

## Signal

Audit of `website/src/content/docs/` (2026-04-11) показал, что docs portal отстал от реального состояния Forgeplan v0.18.0:

- **CLI reference**: 1 страница (`cli/health.md`) из **58 команд** (покрытие ~1.7%).
- **MCP reference**: 1 страница (`mcp/artifact_create.md`) из **47 tools** (покрытие ~2.1%).
- **CHANGELOG**: отсутствует на сайте — v0.17.x и v0.18.0 features (BM25 production, Russian morphology, Snowball stemmer, tag canonicalization, health-debt cleanup, reindex-from-zero fix) нигде не описаны.
- **404.md** отсутствует — Starlight при билде выдаёт warning `Entry docs → 404 was not found`.
- **astro.config.mjs** не задаёт `site` → `@astrojs/sitemap` пропускает генерацию sitemap.xml.
- **Migration guide v0.17 → v0.18**: нет.
- **Stubs**: `reference/glossary.md` (38 строк, без примеров), `guides/ten-rules.md` (34), `marketplace/dev-toolkit.md` (39).

Последний релиз docs — PRD-024 v0.15.0 (2026-04-06). С тех пор прошло 3 релиза (0.16, 0.17, 0.18), всё добавлялось в код и CLI, но docs portal не обновлялся.

## Constraints

- Документация должна генерироваться из **source of truth** (CLI `--help` для команд, Rust source для MCP tools), иначе снова разойдётся на следующем релизе.
- Starlight sidebar уже настроен на `autogenerate: { directory: 'docs/cli' | 'docs/mcp' }` — генерируемые страницы должны иметь корректный frontmatter (`title`, `description`) чтобы попадать в sidebar.
- Build должен оставаться < 10s (сейчас 3.45s для 24 страниц; ожидается ~110 страниц → бюджет ~8s).
- Версия Astro 6, Starlight 0.38.2, React 19 — не повышать в рамках этого спринта.

## Optimization Targets (1-3 max)

1. **CLI + MCP reference coverage → 100%** — авто-генерация из кода.
2. **Feature parity v0.18.0** — CHANGELOG mdx + guide по BM25/Russian morphology/lifecycle v2.
3. **Deployability** — сайт на production URL (Cloudflare Pages) с working sitemap и 404.

## Observation Indicators (Anti-Goodhart)

- Количество страниц само по себе — плохая метрика. Avoid padding stub pages чтобы набить 100%.
- Avg строк на страницу — мониторим, но не гонимся за толщиной.
- Pagefind search recall@3 на запросы `init`, `validate`, `score`, `reason`, `serve` — должен возвращать релевантные страницы (качественный тест, не только количественный).

## Acceptance Criteria

- [ ] `cli/*.md` покрывает все 58 CLI команд (генератор из `forgeplan help <cmd>`).
- [ ] `mcp/*.md` покрывает все 47 MCP tools (генератор из `crates/forgeplan-mcp/src/tools/`).
- [ ] `changelog.mdx` импортирует и рендерит корневой `CHANGELOG.md` (v0.10 .. v0.18.0).
- [ ] Guide `guides/search-v2.md` описывает BM25 + Russian morphology + feature flag `semantic-search`.
- [ ] Guide `guides/lifecycle-v2.md` описывает stale/renew/reopen transitions с примерами.
- [ ] `src/content/docs/404.md` существует; при билде 0 warnings про 404.
- [ ] `astro.config.mjs` содержит `site: 'https://forgeplan.dev'`; sitemap.xml генерируется.
- [ ] `npm run build` → exit 0, 0 warnings, ≥ 100 страниц в `dist/`.
- [ ] Pagefind search index построен для всех новых страниц.
- [ ] Сайт задеплоен на Cloudflare Pages (beta subdomain минимум), Hero GSAP + mobile nav работают.
- [ ] PR merged в `dev` с evidence + R_eff > 0.

## Blast Radius

- **High**: `website/**` — затрагивается весь docs portal, landing остаётся без изменений.
- **Low**: код CLI/MCP — только read-only парсинг `--help` и source файлов.
- **Medium**: CI — если добавится workflow по docs-regen, затронет `.github/workflows/`.
- **External**: публичный сайт — после деплоя станет доступен пользователям; риск показать устаревшую информацию если catch-up неполный.

## Reversibility

**High** — все изменения в `website/` изолированы; откат = revert PR. Cloudflare Pages deploy откатывается в 1 клик на предыдущий билд. Никакой миграции данных, никакого breaking API.

---

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-024  | based_on (original website PRD, v0.15.0) |
| PRD-046  | informs (solution in progress) |


