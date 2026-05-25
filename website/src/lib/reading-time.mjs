import { toString } from 'mdast-util-to-string';

export function remarkReadingTime() {
  return (tree, file) => {
    const text = toString(tree);
    const wordCount = text.trim().split(/\s+/).filter(Boolean).length;
    const minutes = Math.max(1, Math.ceil(wordCount / 200));
    file.data.astro ??= {};
    file.data.astro.frontmatter ??= {};
    // Only auto-set if author didn't provide manual readingTime
    if (file.data.astro.frontmatter.readingTime === undefined) {
      file.data.astro.frontmatter.readingTime = minutes;
    }
  };
}
