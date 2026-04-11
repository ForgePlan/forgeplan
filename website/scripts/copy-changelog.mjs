#!/usr/bin/env node
// Copy root CHANGELOG.md into src/content/docs/changelog.md with Starlight frontmatter.
// Part of PRD-046 — FR-003.

import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../..');
const SRC = resolve(REPO_ROOT, 'CHANGELOG.md');
const DEST = resolve(__dirname, '../src/content/docs/docs/changelog.md');

const raw = readFileSync(SRC, 'utf8');

// Strip the leading "# Changelog" H1 since Starlight adds its own title from frontmatter.
const stripped = raw.replace(/^# Changelog\s*\n+/, '');

const body = `---
title: Changelog
description: "Forgeplan release notes — every public version with added features, fixes, and breaking changes."
---

All notable changes to Forgeplan are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/). Semver is \`MAJOR.MINOR.PATCH\`
with pre-1.0 minor bumps for breaking changes.

The canonical source is [\`CHANGELOG.md\`](https://github.com/ForgePlan/forgeplan/blob/main/CHANGELOG.md)
in the repository. This page is generated from it at build time via \`scripts/copy-changelog.mjs\`.

---

${stripped}`;

writeFileSync(DEST, body);
console.log(`[changelog] wrote ${body.length} bytes to ${DEST}`);
