// Build-time utility for embedding teaching-asset HTML into Astro pages.
// Extracts <body> innerHTML + <style> + <script> from a self-contained HTML
// document and scopes the CSS under a wrapper class so it does not bleed
// onto the surrounding site chrome.
//
// Lesson-to-lesson hrefs are rewritten here, at build time, because inlining a
// lesson changes its base URL: a link written for the standalone file
// (/guides-raw/<slug>.html) resolves somewhere else once its body is inlined
// into /guides/<slug>/ (or /ru/guides/<slug>/). The raw files stay standalone
// and untouched — the rewrite exists only in the inlined copy.

import fs from 'node:fs';
import { getGuide } from '../data/guides.ts';

const SCOPE_CLASS = '.guide-embedded';

interface StackEntry {
  kind: string;
  depth: number;
}

function scopeCss(css: string): string {
  const out: string[] = [];
  let i = 0;
  const n = css.length;
  let depth = 0;
  let buf = '';
  const stack: StackEntry[] = [];

  function flushSelectors(selectorText: string): { selector: string; drop: boolean } {
    const t = selectorText.trim();
    if (!t) return { selector: '', drop: false };
    if (t.startsWith('@')) {
      const kindMatch = t.match(/^@(\w[\w-]*)/);
      const kind = kindMatch ? kindMatch[1] : 'unknown';
      stack.push({ kind, depth: depth + 1 });
      return { selector: t, drop: false };
    }
    const top = stack[stack.length - 1];
    if (top && (top.kind === 'keyframes' || top.kind === 'font-face' || top.kind === 'page' || top.kind === 'counter-style' || top.kind === 'property')) {
      stack.push({ kind: 'inner-noprefix', depth: depth + 1 });
      return { selector: t, drop: false };
    }
    // Drop :root blocks entirely — site tokens come from our :root in global.css.
    // Lesson-specific values are remapped via .guide-embedded aliasing in blog-theme.css.
    if (t === ':root' || /^:root\s*,/.test(t) || /,\s*:root\s*$/.test(t) || t === 'html' || t === 'body' || /^(html|body)\s*,\s*(html|body)$/.test(t)) {
      stack.push({ kind: 'rule', depth: depth + 1, drop: true } as StackEntry);
      return { selector: t, drop: true };
    }
    stack.push({ kind: 'rule', depth: depth + 1 });
    const prefixed = t.split(',').map(s => {
      const sel = s.trim();
      if (!sel) return sel;
      if (sel === '*') return `${SCOPE_CLASS} *`;
      if (sel.startsWith(SCOPE_CLASS + ' ') || sel === SCOPE_CLASS) return sel;
      return `${SCOPE_CLASS} ${sel}`;
    }).filter(Boolean).join(', ');
    return { selector: prefixed, drop: false };
  }

  let dropDepth = -1;  // depth at which we're inside a dropped block; -1 = not dropping

  while (i < n) {
    const ch = css[i];

    if (ch === '/' && css[i + 1] === '*') {
      const end = css.indexOf('*/', i + 2);
      if (end === -1) { out.push(css.slice(i)); break; }
      out.push(css.slice(i, end + 2));
      i = end + 2;
      continue;
    }
    if (ch === '"' || ch === "'") {
      const quote = ch;
      let j = i + 1;
      while (j < n && css[j] !== quote) {
        if (css[j] === '\\') j++;
        j++;
      }
      buf += css.slice(i, j + 1);
      i = j + 1;
      continue;
    }

    if (ch === '{') {
      const result = flushSelectors(buf);
      buf = '';
      depth++;
      if (result.drop && dropDepth === -1) {
        dropDepth = depth;
      } else if (dropDepth === -1) {
        out.push(result.selector + ' {');
      }
      i++;
      continue;
    }
    if (ch === '}') {
      if (dropDepth === -1 && buf.trim()) out.push(buf.trim());
      buf = '';
      if (dropDepth === -1) out.push('}');
      while (stack.length > 0 && stack[stack.length - 1].depth > depth) stack.pop();
      if (dropDepth === depth) dropDepth = -1;
      depth--;
      i++;
      continue;
    }
    if (ch === ';' && depth > 0) {
      if (dropDepth === -1) out.push(buf.trim() + ';');
      buf = '';
      i++;
      continue;
    }

    buf += ch;
    i++;
  }
  if (buf.trim()) out.push(buf.trim());

  return out.join('\n');
}

export type GuideLocale = 'en' | 'ru';

// Canonical guide route prefixes, matching the links the catalog pages emit
// (/guides/<slug>, /ru/guides/<slug> — no trailing slash).
const GUIDE_ROUTE_PREFIX: Record<GuideLocale, string> = {
  en: '/guides/',
  ru: '/ru/guides/',
};

// Href shapes in public/guides-raw/*.html that mean "another lesson of this
// catalog", captured for the path part only (query/fragment are split off
// before matching and re-appended after).
//
//   '../guides/<slug>'  written for the inlined route; from the standalone file
//                       base it resolves to /guides-raw/../guides/<slug>
//   'guides/<slug>'     same intent, relative form
//   '/guides/<slug>'    already a site route, but pinned to one locale
//   '/ru/guides/<slug>' ditto, pinned to RU
//
// A trailing slash is tolerated on any of them.
const GUIDE_ROUTE_HREF = /^\/?(?:\.\.\/)?(?:ru\/)?guides\/([a-z0-9][a-z0-9-]*)\/?$/;

// Sibling link between two standalone lesson files: '<slug>.html'. Valid while
// the lesson is served from /guides-raw/, dead once inlined — but not a dead
// link in the standalone copy, which is why it is rewritten only on the inlined
// one.
const RAW_LESSON_HREF = /^([a-z0-9][a-z0-9-]*)\.html$/;

// href attribute with either quote style. The leading \s keeps data-href and
// xlink:href out of scope.
const HREF_ATTRIBUTE = /(\shref\s*=\s*)(["'])([^"']*)\2/gi;

// src/data/guides.ts is the authority on what a lesson route is: a path whose
// last segment is not a catalog slug keeps its original base, so a stale link
// stays visible to check-dead-links instead of becoming a fabricated route.
function normalizeGuideHref(href: string, locale: GuideLocale): string {
  const cut = href.search(/[?#]/);
  const path = cut === -1 ? href : href.slice(0, cut);
  const tail = cut === -1 ? '' : href.slice(cut);

  const match = GUIDE_ROUTE_HREF.exec(path) ?? RAW_LESSON_HREF.exec(path);
  if (!match || !getGuide(match[1])) return href;

  return GUIDE_ROUTE_PREFIX[locale] + match[1] + tail;
}

export interface EmbedResult {
  bodyHtml: string;
  scopedCss: string;
  externalScripts: string[];
  inlineScripts: string[];
  title: string;
}

export interface EmbedOptions {
  /** Locale of the page the lesson is inlined into — picks the guide route prefix. */
  locale: GuideLocale;
}

export function extractEmbed(filepath: string, options: EmbedOptions): EmbedResult {
  const html = fs.readFileSync(filepath, 'utf-8');

  const titleMatch = html.match(/<title[^>]*>([\s\S]*?)<\/title>/i);
  const title = titleMatch ? titleMatch[1].trim() : '';

  const bodyMatch = html.match(/<body[^>]*>([\s\S]*?)<\/body>/i);
  let bodyHtml = bodyMatch ? bodyMatch[1] : '';
  // The page emits collected scripts separately. Leaving them in bodyHtml
  // executes them twice during HTML parsing (including chart constructors).
  bodyHtml = bodyHtml.replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, '');
  // Strip lesson's own footer block ('trust-calculus · explainer · v1 → forgeplan · forgeplan.dev')
  // so site BlogFooter is the only footer on the page.
  bodyHtml = bodyHtml.replace(/<div\s+class=["']footer["'][^>]*>[\s\S]*?<\/div>/gi, '');
  // Inlining moves the lesson's base URL from /guides-raw/<slug>.html to
  // /guides/<slug>/ (or /ru/guides/<slug>/), so links between lessons are
  // re-pointed at the catalog routes of the page locale.
  // Everything else — external URLs, fragments, non-lesson site paths — is left as authored.
  bodyHtml = bodyHtml.replace(HREF_ATTRIBUTE, (_match, prefix: string, quote: string, href: string) =>
    `${prefix}${quote}${normalizeGuideHref(href, options.locale)}${quote}`);

  const styleBlocks = Array.from(html.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/gi)).map(m => m[1]);
  const rawCss = styleBlocks.join('\n');
  const scopedCss = scopeCss(rawCss);

  const externalScripts = Array.from(html.matchAll(/<script\s+[^>]*\bsrc=["']([^"']+)["'][^>]*>\s*<\/script>/gi)).map(m => m[1]);

  // Wrap each inline script in IIFE so its variables (e.g. const evidence = ..., const artifacts = ...)
  // don't collide with other lessons' identifiers or with site scripts in the page.
  const inlineScripts = Array.from(html.matchAll(/<script(?![^>]*\bsrc=)[^>]*>([\s\S]*?)<\/script>/gi))
    .map(m => `(function(){\n${m[1]}\n})();`);

  return { bodyHtml, scopedCss, externalScripts, inlineScripts, title };
}
