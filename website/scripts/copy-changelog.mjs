#!/usr/bin/env node
// Copy root CHANGELOG.md into src/content/docs/changelog.md with Starlight frontmatter.
// Part of PRD-046 — FR-003.

import { existsSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, relative, resolve, sep } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../..');
const SRC = resolve(REPO_ROOT, 'CHANGELOG.md');
const DEST = resolve(__dirname, '../src/content/docs/docs/changelog.md');

// Canonical repository source prefix. `main` is the repo default branch and the only
// branch reference this site already publishes (see the canonical-source line below and
// docs/guides/lifecycle-v2.md, which point at repo-tracked .forgeplan artifacts the same way).
const REPO_BLOB_BASE = 'https://github.com/ForgePlan/forgeplan/blob/main';

const raw = readFileSync(SRC, 'utf8');

// Strip the leading "# Changelog" H1 since Starlight adds its own title from frontmatter.
let stripped = raw.replace(/^# Changelog\s*\n+/, '');

// Strip relative links to .forgeplan/* artifacts - they don't resolve on the website.
// Keep the artifact ID as plain text. Pattern: [TEXT](.forgeplan/path/...)
stripped = stripped.replace(/\[([^\]]+)\]\(\.forgeplan\/[^)]+\)/g, '$1');

// Contributor docs (docs/operations/*, etc.) are repository sources with no website page, so a
// root-relative link copied here would resolve against /docs/changelog/ and 404. Point those links
// at the canonical repository source instead. Only relative targets that exist as a file
// inside this checkout are rewritten: a target that does not exist is left as-is so it stays
// visible as a broken link rather than being hidden behind an external URL.
// Anchor (#frag) and query (?q=) suffixes are carried over untouched, and the link text is kept.
const unresolvedRelative = [];
stripped = stripped.replace(
  /\[([^\]]+)\]\(([^)\s]+)\)/g,
  (match, label, target) => {
    // Leave absolute URLs, protocol-relative URLs, in-page anchors and site-root paths alone.
    if (/^[a-z][a-z0-9+.-]*:/i.test(target) || target.startsWith('/') || target.startsWith('#')) {
      return match;
    }

    const suffixAt = target.search(/[#?]/);
    const path = suffixAt === -1 ? target : target.slice(0, suffixAt);
    const suffix = suffixAt === -1 ? '' : target.slice(suffixAt);

    const absolute = resolve(REPO_ROOT, path);
    const repoRelative = relative(REPO_ROOT, absolute);
    const insideRepo = repoRelative !== '' && !repoRelative.startsWith('..') && !repoRelative.startsWith(sep);
    const isFile = insideRepo && existsSync(absolute) && statSync(absolute).isFile();

    if (!isFile) {
      unresolvedRelative.push(path);
      return match;
    }

    return `[${label}](${REPO_BLOB_BASE}/${repoRelative.split(sep).join('/')}${suffix})`;
  },
);

if (unresolvedRelative.length > 0) {
  console.warn(
    `[changelog] unresolved relative link(s), left as-is: ${[...new Set(unresolvedRelative)].join(', ')}`,
  );
}

// Normalise em-dashes to short hyphens on the published page (AI-tell avoidance).
// The root CHANGELOG.md keeps its original punctuation; only the website copy is rewritten.
stripped = stripped.replace(/—/g, '-');

const body = `---
title: Changelog
description: "Forgeplan release notes recorded from v0.17.0 onward, plus unreleased changes."
---

This page includes the changes recorded in the repository from v0.17.0 onward, plus unreleased work. It is not a complete history of every public version. The format loosely follows
[Keep a Changelog](https://keepachangelog.com/). Semver is \`MAJOR.MINOR.PATCH\`
with pre-1.0 minor bumps for breaking changes.

The canonical source is [\`CHANGELOG.md\`](https://github.com/ForgePlan/forgeplan/blob/main/CHANGELOG.md)
in the repository. This page is generated from it at build time via \`scripts/copy-changelog.mjs\`.

---

${stripped}`;

writeFileSync(DEST, body);
console.log(`[changelog] wrote ${body.length} UTF-16 code units to ${DEST}`);
