#!/usr/bin/env node
// Crawl dist/ and report broken internal links.
// Part of PRD-046 — C2 dead links check.

import { readFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve, relative } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const DIST = resolve(__dirname, '../dist');

function walk(dir, acc = []) {
  for (const name of readdirSync(dir)) {
    const full = resolve(dir, name);
    const st = statSync(full);
    if (st.isDirectory()) walk(full, acc);
    else if (name.endsWith('.html')) acc.push(full);
  }
  return acc;
}

function extractLinks(html) {
  const hrefs = [];
  const re = /href\s*=\s*"([^"#?]+)(?:[#?][^"]*)?"/g;
  let m;
  while ((m = re.exec(html)) !== null) {
    const h = m[1];
    if (!h) continue;
    if (h.startsWith('http://') || h.startsWith('https://')) continue;
    if (h.startsWith('mailto:')) continue;
    if (h.startsWith('data:')) continue;
    if (h.startsWith('//')) continue;
    hrefs.push(h);
  }
  return hrefs;
}

function resolveTarget(fromFile, href) {
  let base;
  if (href.startsWith('/')) {
    base = resolve(DIST, '.' + href);
  } else {
    base = resolve(dirname(fromFile), href);
  }
  const candidates = [
    base,
    base + '.html',
    resolve(base, 'index.html'),
  ];
  for (const c of candidates) {
    if (existsSync(c)) return c;
  }
  return null;
}

function main() {
  const files = walk(DIST);
  console.log(`[links] scanning ${files.length} HTML files`);

  let totalLinks = 0;
  let broken = [];
  for (const f of files) {
    const html = readFileSync(f, 'utf8');
    const links = extractLinks(html);
    totalLinks += links.length;
    for (const link of links) {
      if (!link || link === '/' || link.startsWith('pagefind/')) continue;
      const target = resolveTarget(f, link);
      if (!target) {
        broken.push({ from: relative(DIST, f), href: link });
      }
    }
  }

  console.log(`[links] checked ${totalLinks} internal links across ${files.length} pages`);
  if (broken.length === 0) {
    console.log(`[links] OK no broken internal links`);
    process.exit(0);
  }
  const seen = new Set();
  const uniq = [];
  for (const b of broken) {
    const key = b.href;
    if (seen.has(key)) continue;
    seen.add(key);
    uniq.push(b);
  }
  console.log(`[links] FAIL ${broken.length} broken links (${uniq.length} unique targets):`);
  for (const b of uniq.slice(0, 50)) {
    console.log(`  ${b.href}  (from ${b.from})`);
  }
  if (uniq.length > 50) console.log(`  ... and ${uniq.length - 50} more`);
  process.exit(1);
}

main();
