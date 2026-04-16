#!/usr/bin/env node
// Verify every CLI and MCP reference page has minimum content:
// - at least 50 lines
// - has "## Usage" or "## Input parameters" section
// - has "## Examples" or "## Example" section
// - has "## See also" section
// Part of PRD-046 — C3 content completeness.

import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const DOCS = resolve(__dirname, '../src/content/docs/docs');

const CHECKS = [
  {
    dir: resolve(DOCS, 'cli'),
    minLines: 60,
    mustHave: ['## Usage', '## See also'],
    eitherOr: [['## Example', '## Examples']],
  },
  {
    dir: resolve(DOCS, 'mcp'),
    minLines: 50,
    mustHave: ['## Input parameters', '## See also'],
    eitherOr: [['## Example', '## Examples']],
  },
];

function check(dir, minLines, mustHave, eitherOr) {
  const files = readdirSync(dir).filter(f => f.endsWith('.md') && f !== 'index.md');
  const issues = [];
  for (const f of files) {
    const path = resolve(dir, f);
    const content = readFileSync(path, 'utf8');
    const lines = content.split('\n').length;
    if (lines < minLines) {
      issues.push({ file: f, issue: `only ${lines} lines (min ${minLines})` });
      continue;
    }
    for (const mh of mustHave) {
      if (!content.includes(mh)) {
        issues.push({ file: f, issue: `missing section: ${mh}` });
      }
    }
    for (const [a, b] of eitherOr) {
      if (!content.includes(a) && !content.includes(b)) {
        issues.push({ file: f, issue: `missing: ${a} or ${b}` });
      }
    }
  }
  return { total: files.length, issues };
}

function main() {
  let totalFiles = 0;
  let totalIssues = 0;
  for (const c of CHECKS) {
    const { total, issues } = check(c.dir, c.minLines, c.mustHave, c.eitherOr);
    totalFiles += total;
    totalIssues += issues.length;
    console.log(`[content] ${c.dir.split('/').slice(-2).join('/')}: ${total} files, ${issues.length} issues`);
    for (const i of issues.slice(0, 10)) {
      console.log(`  ✗ ${i.file}: ${i.issue}`);
    }
    if (issues.length > 10) console.log(`  ... and ${issues.length - 10} more`);
  }
  console.log(`[content] total ${totalFiles} files scanned, ${totalIssues} issues`);
  process.exit(totalIssues === 0 ? 0 : 1);
}

main();
