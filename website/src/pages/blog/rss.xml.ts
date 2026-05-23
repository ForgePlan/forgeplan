import rss from '@astrojs/rss';
import { getCollection } from 'astro:content';
import type { APIContext } from 'astro';

export async function GET(context: APIContext) {
  const posts = await getCollection('blog', ({ data }) => data.lang === 'en' && !data.draft);
  return rss({
    title: 'Forgeplan Blog',
    description: 'Long-form essays on engineering decisions — how to structure them, score evidence, and stop forgetting why they were made.',
    site: context.site ?? 'https://forgeplan.dev',
    items: posts
      .sort((a, b) => b.data.publishedAt.valueOf() - a.data.publishedAt.valueOf())
      .map(post => ({
        title: post.data.title,
        description: post.data.description,
        pubDate: post.data.publishedAt,
        link: `/blog/${post.id.replace(/^en\//, '').replace(/\.(md|mdx)$/, '')}`,
      })),
  });
}
