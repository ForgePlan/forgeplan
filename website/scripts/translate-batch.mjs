#!/usr/bin/env node
// Batch translate all EN docs to RU using Gemini API.
// Part of PRD-047 — FR-006.
//
// Usage:
//   GEMINI_API_KEY=... node scripts/translate-batch.mjs [--category cli|mcp|guides|methodology|getting-started|marketplace|reference|all] [--dry-run] [--concurrency 3]

import { readFileSync, writeFileSync, readdirSync, statSync, existsSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve, relative, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const EN_ROOT = resolve(__dirname, '../src/content/docs/docs');
const RU_ROOT = resolve(__dirname, '../src/content/docs/ru/docs');
const GLOSSARY_PATH = resolve(__dirname, '../src/i18n/glossary-ru.yaml');

const args = process.argv.slice(2);
const dryRun = args.includes('--dry-run');
const forceAll = args.includes('--force');
const catIdx = args.indexOf('--category');
const category = catIdx >= 0 ? args[catIdx + 1] : 'all';
const concIdx = args.indexOf('--concurrency');
const concurrency = concIdx >= 0 ? parseInt(args[concIdx + 1], 10) : 3;

function loadGlossary() {
  const raw = readFileSync(GLOSSARY_PATH, 'utf8');
  const entries = [];
  for (const line of raw.split('\n')) {
    if (!line.trim() || line.startsWith('#')) continue;
    const m = line.match(/^([^:]+):\s*(.+?)(\s*#.*)?$/);
    if (m) entries.push({ en: m[1].trim(), ru: m[2].trim() });
  }
  return entries;
}

function glossaryPrompt(entries) {
  const lines = entries
    .filter(e => !e.ru.includes('[keep'))
    .map(e => `- "${e.en}" → "${e.ru}"`)
    .join('\n');
  const keeps = entries
    .filter(e => e.ru.includes('[keep') || e.en === e.ru)
    .map(e => `"${e.en}"`)
    .join(', ');
  return `## Glossary\n${lines}\n\n## Keep in English\n${keeps}`;
}

function walk(dir, acc = []) {
  if (!existsSync(dir)) return acc;
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    const st = statSync(full);
    if (st.isDirectory()) walk(full, acc);
    else if (name.endsWith('.md') || name.endsWith('.mdx')) acc.push(full);
  }
  return acc;
}

async function translateOne(content, glossaryBlock) {
  const apiKey = process.env.GEMINI_API_KEY;
  if (!apiKey) throw new Error('GEMINI_API_KEY not set');

  const model = process.env.GEMINI_MODEL || 'gemini-2.0-flash';
  const url = `https://generativelanguage.googleapis.com/v1beta/models/${model}:generateContent?key=${apiKey}`;

  const systemInstruction = `You are a professional technical translator (English → Russian) for developer documentation about Forgeplan.

RULES:
1. Translate prose to natural, fluent Russian
2. PRESERVE as-is: YAML frontmatter, code blocks, inline code, URLs, file paths, CLI commands, markdown structure, mermaid, admonitions
3. Use glossary for consistency
4. Output ONLY translated markdown — no commentary

${glossaryBlock}`;

  const resp = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      system_instruction: { parts: [{ text: systemInstruction }] },
      contents: [{ parts: [{ text: `Translate to Russian:\n\n${content}` }] }],
      generationConfig: { maxOutputTokens: 65536, temperature: 0.3 },
    }),
  });

  if (!resp.ok) {
    const err = await resp.text();
    throw new Error(`API ${resp.status}: ${err}`);
  }

  const data = await resp.json();
  const text = data.candidates?.[0]?.content?.parts?.[0]?.text;
  if (!text) throw new Error('Empty Gemini response');
  return text;
}

// Run N translations concurrently
async function pool(tasks, concurrency, fn) {
  const results = [];
  let idx = 0;
  async function worker() {
    while (idx < tasks.length) {
      const i = idx++;
      results[i] = await fn(tasks[i], i);
    }
  }
  await Promise.all(Array.from({ length: Math.min(concurrency, tasks.length) }, worker));
  return results;
}

async function main() {
  const glossary = loadGlossary();
  const glossaryBlock = glossaryPrompt(glossary);

  let enFiles;
  if (category === 'all') {
    enFiles = walk(EN_ROOT);
  } else {
    const catDir = join(EN_ROOT, category);
    if (!existsSync(catDir)) {
      // Maybe it's a top-level file like changelog.md
      const topFile = join(EN_ROOT, category + '.md');
      if (existsSync(topFile)) enFiles = [topFile];
      else { console.error(`Not found: ${catDir}`); process.exit(1); }
    } else {
      enFiles = walk(catDir);
    }
  }

  const toTranslate = [];
  for (const enFile of enFiles) {
    const rel = relative(EN_ROOT, enFile);
    const ruFile = join(RU_ROOT, rel);
    if (!forceAll && existsSync(ruFile)) {
      const enMtime = statSync(enFile).mtimeMs;
      const ruMtime = statSync(ruFile).mtimeMs;
      if (ruMtime >= enMtime) continue;
    }
    toTranslate.push({ en: enFile, ru: ruFile, rel });
  }

  console.log(`[batch] category=${category} | EN files=${enFiles.length} | to translate=${toTranslate.length} | concurrency=${concurrency}`);

  if (dryRun) {
    for (const f of toTranslate.slice(0, 30)) console.log(`  ${f.rel}`);
    if (toTranslate.length > 30) console.log(`  ... and ${toTranslate.length - 30} more`);
    console.log(`\n[batch] estimated Gemini calls: ${toTranslate.length}`);
    return;
  }

  if (!process.env.GEMINI_API_KEY) {
    console.error('[batch] GEMINI_API_KEY not set');
    process.exit(1);
  }

  let done = 0, errors = 0;

  await pool(toTranslate, concurrency, async (f, i) => {
    try {
      const content = readFileSync(f.en, 'utf8');
      const translated = await translateOne(content, glossaryBlock);
      mkdirSync(dirname(f.ru), { recursive: true });
      writeFileSync(f.ru, translated);
      done++;
      process.stdout.write(`\r[batch] ${done}/${toTranslate.length} done (${errors} errors)`);
    } catch (err) {
      errors++;
      console.error(`\n[batch] ERROR ${f.rel}: ${err.message}`);
    }
  });

  console.log(`\n[batch] complete: ${done} translated, ${errors} errors, ${enFiles.length - toTranslate.length} skipped`);
}

main().catch(err => {
  console.error(`[batch] FATAL: ${err.message}`);
  process.exit(1);
});
