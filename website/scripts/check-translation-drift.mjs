#!/usr/bin/env node
// Check which EN docs pages have been modified since their RU translation.
// Reports stale RU translations that need re-translating.
// Part of PRD-047 — FR-009.
//
// Usage: node scripts/check-translation-drift.mjs [--verbose]

import { readdirSync, statSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve, relative, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const EN_ROOT = resolve(__dirname, '../src/content/docs/docs');
const RU_ROOT = resolve(__dirname, '../src/content/docs/ru/docs');
const verbose = process.argv.includes('--verbose');

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

function main() {
  const enFiles = walk(EN_ROOT);
  let upToDate = 0, stale = 0, missing = 0;
  const staleList = [];
  const missingList = [];

  for (const enFile of enFiles) {
    const rel = relative(EN_ROOT, enFile);
    const ruFile = join(RU_ROOT, rel);

    if (!existsSync(ruFile)) {
      missing++;
      missingList.push(rel);
      continue;
    }

    const enMtime = statSync(enFile).mtimeMs;
    const ruMtime = statSync(ruFile).mtimeMs;

    if (enMtime > ruMtime) {
      stale++;
      const daysStale = Math.round((enMtime - ruMtime) / (1000 * 60 * 60 * 24));
      staleList.push({ rel, daysStale });
    } else {
      upToDate++;
    }
  }

  console.log(`[drift] EN files: ${enFiles.length} | RU up-to-date: ${upToDate} | stale: ${stale} | missing: ${missing}`);

  if (staleList.length > 0) {
    console.log(`\nStale translations (EN modified after RU):`);
    for (const s of staleList) {
      console.log(`  ${s.rel} (${s.daysStale}d behind)`);
    }
  }

  if (missing > 0 && verbose) {
    console.log(`\nMissing RU translations:`);
    for (const m of missingList.slice(0, 20)) {
      console.log(`  ${m}`);
    }
    if (missingList.length > 20) console.log(`  ... and ${missingList.length - 20} more`);
  }

  if (stale > 0 || missing > 0) {
    console.log(`\nFix: GEMINI_API_KEY=... node scripts/translate-batch.mjs`);
    process.exit(1);
  }

  console.log(`\n[drift] All translations up-to-date.`);
}

main();
