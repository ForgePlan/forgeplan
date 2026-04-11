#!/usr/bin/env node
// Translate a single .md docs page from EN to RU using Gemini API.
// Preserves: frontmatter YAML, code blocks, CLI commands, technical terms per glossary.
// Part of PRD-047 — FR-006.
//
// Usage:
//   GEMINI_API_KEY=... node scripts/translate-page.mjs <en-file> <ru-file> [--dry-run]

import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const GLOSSARY_PATH = resolve(__dirname, '../src/i18n/glossary-ru.yaml');

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
  return `## Glossary (mandatory translations)\n${lines}\n\n## Terms to keep in English\n${keeps}`;
}

async function translateWithGemini(content, glossary) {
  const apiKey = process.env.GEMINI_API_KEY;
  if (!apiKey) throw new Error('GEMINI_API_KEY not set');

  const model = process.env.GEMINI_MODEL || 'gemini-2.0-flash';
  const url = `https://generativelanguage.googleapis.com/v1beta/models/${model}:generateContent?key=${apiKey}`;

  const systemInstruction = `You are a professional technical translator (English → Russian) for developer documentation about Forgeplan — a Rust CLI tool for managing projects through structured artifacts.

RULES:
1. Translate all prose text to natural, fluent Russian. Use professional but accessible tone.
2. PRESERVE EXACTLY as-is (do NOT translate):
   - YAML frontmatter between --- markers (keep title/description values in English)
   - Code blocks (everything between \`\`\` markers)
   - Inline code (\`like this\`)
   - URLs and file paths
   - CLI command names (forgeplan, cargo, npm, git)
   - Markdown structure (headings ##, tables |, lists -, links [], admonitions :::)
   - Mermaid diagram syntax
3. Use the glossary below for consistent terminology — these are mandatory translations.
4. Keep the same heading hierarchy (##, ###, ####).
5. Translate link text but preserve link URLs unchanged.
6. Do NOT add any commentary, notes, or wrapping — output ONLY the translated markdown.
7. Do NOT add machine-translation disclaimers or badges.

${glossary}`;

  const resp = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      system_instruction: { parts: [{ text: systemInstruction }] },
      contents: [{ parts: [{ text: `Translate this documentation page to Russian:\n\n${content}` }] }],
      generationConfig: {
        maxOutputTokens: 65536,
        temperature: 0.3,
      },
    }),
  });

  if (!resp.ok) {
    const err = await resp.text();
    throw new Error(`Gemini API ${resp.status}: ${err}`);
  }

  const data = await resp.json();
  const text = data.candidates?.[0]?.content?.parts?.[0]?.text;
  if (!text) throw new Error('Empty response from Gemini');
  return text;
}

async function main() {
  const args = process.argv.slice(2);
  const dryRun = args.includes('--dry-run');
  const files = args.filter(a => !a.startsWith('--'));

  if (files.length < 2) {
    console.error('Usage: GEMINI_API_KEY=... translate-page.mjs <en-file> <ru-file> [--dry-run]');
    process.exit(1);
  }

  const [enPath, ruPath] = files;
  if (!existsSync(enPath)) {
    console.error(`EN file not found: ${enPath}`);
    process.exit(1);
  }

  const content = readFileSync(enPath, 'utf8');
  const glossary = loadGlossary();
  const glossaryBlock = glossaryPrompt(glossary);

  console.log(`[translate] ${enPath} → ${ruPath} (${content.split('\n').length} lines)`);

  if (dryRun) {
    console.log(`[translate] dry-run — would translate ${content.length} chars`);
    console.log(`[translate] glossary: ${glossary.length} entries`);
    return;
  }

  const translated = await translateWithGemini(content, glossaryBlock);
  mkdirSync(dirname(ruPath), { recursive: true });
  writeFileSync(ruPath, translated);
  console.log(`[translate] wrote ${translated.split('\n').length} lines to ${ruPath}`);
}

main().catch(err => {
  console.error(`[translate] ERROR: ${err.message}`);
  process.exit(1);
});
