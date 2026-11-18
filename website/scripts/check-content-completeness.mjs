#!/usr/bin/env node
// Check registered CLI paths + both MCP profiles and owned runtime blocks in EN/RU.
// This is interface coverage/freshness, NOT a semantic review of handwritten prose.
// Usage: node scripts/check-content-completeness.mjs --binary <built forgeplan>
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

let failed = false;
for (const name of ['generate-cli-docs.mjs', 'generate-mcp-docs.mjs']) {
  const script = fileURLToPath(new URL(name, import.meta.url));
  const result = spawnSync(process.execPath, [script, ...process.argv.slice(2), '--check'], { stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.status !== 0) failed = true;
}
if (failed) {
  console.error('[content] FAIL: interface pages are missing/stale, discovery failed, or obsolete pages need review.');
  process.exitCode = 1;
} else {
  console.log('[content] PASS: CLI and both MCP profiles covered in EN/RU; generated blocks match the selected binary. Handwritten prose was not semantically reviewed by this check.');
}
