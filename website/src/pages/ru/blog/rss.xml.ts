import rss from '@astrojs/rss';
import { getCollection } from 'astro:content';
import type { APIContext } from 'astro';

export async function GET(context: APIContext) {
  const posts = await getCollection('blog', ({ data }) => data.lang === 'ru' && !data.draft);
  return rss({
    title: 'Forgeplan Блог',
    description: 'Развёрнутые заметки о R_eff, ADI, FPF, MCP и методологии Forgeplan.',
    site: context.site ?? 'https://forgeplan.dev',
    items: posts
      .sort((a, b) => b.data.publishedAt.valueOf() - a.data.publishedAt.valueOf())
      .map(post => ({
        title: post.data.title,
        description: post.data.description,
        pubDate: post.data.publishedAt,
        link: `/ru/blog/${post.id.replace(/^ru\//, '').replace(/\.(md|mdx)$/, '')}`,
      })),
  });
}
