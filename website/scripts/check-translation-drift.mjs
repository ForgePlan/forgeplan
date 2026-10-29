#!/usr/bin/env node
// EN/RU presence plus a filesystem-mtime heuristic, not semantic equivalence.
// The generated changelog is intentionally EN-only.
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
    if (rel === 'changelog.md') continue;
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

  console.log(`[drift] EN files (including changelog): ${enFiles.length} | RU mtime not older: ${upToDate} | mtime candidates: ${stale} | unexpected missing: ${missing}`);

  if (staleList.length > 0) {
    console.log(`\nReview candidates (EN mtime is newer; this does not prove semantic drift):`);
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
    console.log(`\nReview the listed pairs. Do not touch timestamps or batch-retranslate merely to make this heuristic green.`);
    process.exit(1);
  }

  console.log(`\n[drift] Expected RU files exist; no mtime candidates. Changelog is intentionally excluded. Semantic equivalence was not checked.`);
}

main();
