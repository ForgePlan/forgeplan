import { defineCollection, z } from 'astro:content';
import { docsLoader, i18nLoader } from '@astrojs/starlight/loaders';
import { docsSchema, i18nSchema } from '@astrojs/starlight/schema';
import { glob } from 'astro/loaders';

const blog = defineCollection({
	loader: glob({
		pattern: '**/*.{md,mdx}',
		base: './src/content/blog',
		// generateId override is REQUIRED to prevent slug-collision when EN and RU posts
		// share the same `slug` field in frontmatter (e.g. both have slug: "welcome").
		//
		// Default Astro behavior: when frontmatter contains `slug`, it becomes the entry ID
		// — causing en/welcome.mdx and ru/welcome.mdx to collide at id="welcome".
		//
		// Fix: use file path as ID. Six call-sites depend on the format `{lang}/{slug}`:
		//   - src/pages/blog/index.astro                   (regex .replace(/^en\//, ''))
		//   - src/pages/blog/[...slug].astro               (regex .replace(/^en\//, ''))
		//   - src/pages/blog/rss.xml.ts                    (regex .replace(/^en\//, ''))
		//   - src/pages/ru/blog/index.astro                (regex .replace(/^ru\//, ''))
		//   - src/pages/ru/blog/[...slug].astro            (regex .replace(/^ru\//, ''))
		//   - src/pages/ru/blog/rss.xml.ts                 (regex .replace(/^ru\//, ''))
		//   - src/components/blog/SeriesNav.astro          (NEW PR-2A — uses stripLang)
		//   - src/pages/blog/series/[name].astro           (NEW PR-2A — uses stripLang)
		//   - src/pages/ru/blog/series/[name].astro        (NEW PR-2A — uses stripLang)
		// If this format changes (e.g. to support categories like en/methodology/post),
		// ALL nine call-sites must be updated in lockstep.
		generateId: ({ entry }) => entry.replace(/\.(md|mdx)$/, ''),
	}),
	schema: z.object({
		title: z.string(),
		description: z.string(),
		slug: z.string(),
		lang: z.enum(['en', 'ru']),
		publishedAt: z.coerce.date(),
		updatedAt: z.coerce.date().optional(),
		kind: z.enum(['explainer', 'case-study', 'teaching', 'release-notes', 'deep-dive']),
		topic: z.enum(['r-eff', 'adi', 'fpf', 'mcp', 'methodology', 'release']),
		artifacts: z.array(z.string()).optional(),
		// Cover image must be a local path under /public — prevents `javascript:`,
		// `data:`, or `//evil.tld/track` schemes from sneaking into `<img src>` or
		// `og:image`. If we ever ingest externally-submitted posts, this guard
		// becomes load-bearing.
		cover: z.string().regex(/^\/[\w\-/.]+\.(png|jpg|jpeg|webp|svg)$/, {
			message: 'cover must be a local /path/to/file.{png|jpg|jpeg|webp|svg}',
		}).optional(),
		draft: z.boolean().default(false),
		readingTime: z.number().optional(),
		translations: z.record(z.string(), z.string()).optional(),

		// PR-2A (PRD-080): series support for narrative arcs spanning multiple posts.
		// All optional — backwards-compatible with PR-1 frontmatter.
		series: z.string().optional(),
		seriesOrder: z.number().int().positive().optional(),
		seriesDescription: z.string().optional(),
	}),
});

export const collections = {
	docs: defineCollection({ loader: docsLoader(), schema: docsSchema() }),
	i18n: defineCollection({ loader: i18nLoader(), schema: i18nSchema() }),
	blog,
};
