// Build-time utility for embedding teaching-asset HTML into Astro pages.
// Extracts <body> innerHTML + <style> + <script> from a self-contained HTML
// document and scopes the CSS under a wrapper class so it does not bleed
// onto the surrounding site chrome.

import fs from 'node:fs';

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

export interface EmbedResult {
  bodyHtml: string;
  scopedCss: string;
  externalScripts: string[];
  inlineScripts: string[];
  title: string;
}

export function extractEmbed(filepath: string): EmbedResult {
  const html = fs.readFileSync(filepath, 'utf-8');

  const titleMatch = html.match(/<title[^>]*>([\s\S]*?)<\/title>/i);
  const title = titleMatch ? titleMatch[1].trim() : '';

  const bodyMatch = html.match(/<body[^>]*>([\s\S]*?)<\/body>/i);
  let bodyHtml = bodyMatch ? bodyMatch[1] : '';
  // Strip lesson's own footer block ('trust-calculus · explainer · v1 → forgeplan · forgeplan.dev')
  // so site BlogFooter is the only footer on the page.
  bodyHtml = bodyHtml.replace(/<div\s+class=["']footer["'][^>]*>[\s\S]*?<\/div>/gi, '');

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
