#!/usr/bin/env node
// Generate Starlight marketplace pages from the ForgePlan marketplace repo.
//
// Source of truth: <MARKETPLACE_DIR>/.claude-plugin/marketplace.json (+ per-plugin
// commands/agents/skills directories). Default dir is the sibling checkout; override
// with MARKETPLACE_DIR.
//
// Usage: node scripts/generate-marketplace-docs.mjs
//
// Idempotent: a second run with no source change rewrites identical bytes.

import { readFileSync, writeFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = resolve(__dirname, '../src/content/docs/docs/marketplace');
const OVERVIEW = resolve(OUT_DIR, 'overview.md');
const MARKETPLACE_DIR =
  process.env.MARKETPLACE_DIR || '/Users/explosovebit/Work/ForgePlanMarketplace/forgeplan-marketplace';

const MARKER_START = '<!-- plugins:start -->';
const MARKER_END = '<!-- plugins:end -->';
const PROFILES_START = '<!-- profiles:start -->';
const PROFILES_END = '<!-- profiles:end -->';

function fail(msg) {
  console.error(`[mkt-gen] ${msg}`);
  process.exit(1);
}

function unquote(s) {
  const t = s.trim();
  if ((t.startsWith('"') && t.endsWith('"')) || (t.startsWith("'") && t.endsWith("'"))) {
    return t.slice(1, -1).replace(/\\"/g, '"').replace(/\\'/g, "'");
  }
  return t;
}

// Minimal frontmatter reader: enough for `name` and a scalar/block `description`.
function parseFrontmatter(text) {
  const m = text.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  if (!m) return {};
  const lines = m[1].split('\n');
  const out = {};
  for (let i = 0; i < lines.length; i++) {
    const nameMatch = lines[i].match(/^name:\s*(.*)$/);
    if (nameMatch) {
      out.name = unquote(nameMatch[1]);
      continue;
    }
    const descMatch = lines[i].match(/^description:\s*(.*)$/);
    if (!descMatch) continue;
    const rest = descMatch[1].trim();
    if (rest === '|' || rest === '>' || rest === '|-' || rest === '>-') {
      const block = [];
      for (let j = i + 1; j < lines.length; j++) {
        const line = lines[j];
        if (line.trim() === '') {
          block.push('');
        } else if (/^\s+\S/.test(line)) {
          block.push(line.trim());
        } else {
          break;
        }
      }
      out.description = block.join(' ').replace(/\s+/g, ' ').trim();
    } else {
      out.description = unquote(rest);
    }
  }
  return out;
}

function readDescription(file) {
  if (!existsSync(file)) return '';
  return parseFrontmatter(readFileSync(file, 'utf8')).description || '';
}

// Agent descriptions carry `EN:` / `RU:` halves; the EN one is what the site needs.
function englishOnly(desc) {
  if (!desc) return '';
  const enIdx = desc.indexOf('EN:');
  if (enIdx === -1) return desc;
  const after = desc.slice(enIdx + 3);
  const ruIdx = after.indexOf('RU:');
  return (ruIdx === -1 ? after : after.slice(0, ruIdx)).trim();
}

function firstSentence(s) {
  const t = s.replace(/\s+/g, ' ').trim();
  if (!t) return '';
  const m = t.match(/^(.+?[.!?])(\s|$)/);
  return (m ? m[1] : t).trim();
}

// Markdown table cells cannot hold pipes or newlines.
function cell(s) {
  return s.replace(/\|/g, '\\|').replace(/\s+/g, ' ').trim();
}

function listDirFiles(dir, ext = '.md') {
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((n) => n.endsWith(ext) && statSync(join(dir, n)).isFile())
    .map((n) => n.slice(0, -ext.length))
    .sort();
}

function listSkillDirs(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((n) => statSync(join(dir, n)).isDirectory() && existsSync(join(dir, n, 'SKILL.md')))
    .sort();
}

function normEntries(arr) {
  if (!Array.isArray(arr)) return [];
  return arr
    .map((e) => (typeof e === 'string' ? { name: e } : e))
    .filter((e) => e && e.name);
}

function renderRequires(req) {
  if (!req || Object.keys(req).length === 0) return 'Nothing beyond Claude Code';
  const structured = ['cli', 'mcp', 'plugins'].some((k) => Array.isArray(req[k]));
  if (structured) {
    const parts = [];
    for (const [label, key] of [['CLI', 'cli'], ['MCP', 'mcp'], ['Plugins', 'plugins']]) {
      const items = normEntries(req[key]).map((e) => {
        let s = e.name + (e.version ? ` ${e.version}` : '');
        if (e.optional) s += ' (optional)';
        return s;
      });
      if (items.length) parts.push(`${label}: ${items.join(', ')}`);
    }
    return parts.length ? parts.join('; ') : 'Nothing beyond Claude Code';
  }
  const parts = Object.entries(req).map(([k, v]) =>
    typeof v === 'string' ? v : `${k}: ${JSON.stringify(v)}`
  );
  return parts.join(' ') || 'Nothing beyond Claude Code';
}

// The published docs are swept free of em dashes - they read as an AI tell, and
// the site-wide sweep removed them. Inside a fenced block they are verbatim tool
// output and stay exactly as the binary printed them, so only prose is touched.
function stripEmDashes(text) {
  let fence = null;
  return text
    .split('\n')
    .map((line) => {
      const s = line.trimStart();
      if (fence === null && (s.startsWith('```') || s.startsWith('~~~'))) {
        fence = s.slice(0, 3);
        return line;
      }
      if (fence !== null && s.startsWith(fence)) {
        fence = null;
        return line;
      }
      if (fence !== null) return line;
      return line.replace(/[—–]/g, '-');
    })
    .join('\n');
}

function yamlEscape(s) {
  return s.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
}

function pageFor(plugin, root) {
  const fullDesc = (plugin.description || '').replace(/\s+/g, ' ').trim();
  const shortDesc = firstSentence(fullDesc);
  const pdir = join(root, plugin.source || `./plugins/${plugin.name}`);

  let body = `---\ntitle: ${plugin.name}\ndescription: "${yamlEscape(shortDesc)}"\n---\n\n`;
  body += `${fullDesc}\n\n`;
  body += `**Cost**: ${plugin.cost || 'n/a'} | **Stability**: ${plugin.stability || 'n/a'} | **Version**: ${plugin.version || 'n/a'} | **Category**: ${plugin.category || 'n/a'}\n\n`;
  body += `**Targets**: ${(plugin.targets || []).join(', ') || 'n/a'}\n\n`;
  body += `**Requires**: ${renderRequires(plugin.requires)}\n\n`;
  if (plugin.deprecated) {
    body += `**Superseded by**: ${plugin.supersededBy || 'n/a'}\n\n`;
  }

  const commands = listDirFiles(join(pdir, 'commands'));
  if (commands.length) {
    body += `## Commands\n\n| Command | What it does |\n|---|---|\n`;
    for (const c of commands) {
      const desc = firstSentence(readDescription(join(pdir, 'commands', `${c}.md`)));
      body += `| \`/${c}\` | ${cell(desc)} |\n`;
    }
    body += `\n`;
  }

  const agents = listDirFiles(join(pdir, 'agents'));
  if (agents.length) {
    body += `## Agents\n\n| Agent | What it does |\n|---|---|\n`;
    for (const a of agents) {
      const desc = firstSentence(englishOnly(readDescription(join(pdir, 'agents', `${a}.md`))));
      body += `| \`${a}\` | ${cell(desc)} |\n`;
    }
    body += `\n`;
  }

  const skills = listSkillDirs(join(pdir, 'skills'));
  if (skills.length) {
    body += `## Skills\n\n`;
    for (const s of skills) {
      const desc = firstSentence(readDescription(join(pdir, 'skills', s, 'SKILL.md')));
      body += `- \`${s}\` - ${cell(desc)}\n`;
    }
    body += `\n`;
  }

  body += `## Installation\n\n\`\`\`bash\n/plugin marketplace add ForgePlan/marketplace\n/plugin install ${plugin.name}@ForgePlan-marketplace\n\`\`\`\n\n`;
  body += `## See also\n\n`;
  body += `- [Marketplace overview](/docs/marketplace/overview/)\n`;
  body += `- [Commands Reference](/docs/marketplace/commands/)\n`;
  return body;
}

function pluginsTable(plugins) {
  let out = `${MARKER_START}\n`;
  out += `| Plugin | Purpose | Cost | Stability | Page |\n|---|---|---|---|---|\n`;
  for (const p of [...plugins].sort((a, b) => a.name.localeCompare(b.name))) {
    const purpose = cell(firstSentence((p.description || '').replace(/^\[DEPRECATED[^\]]*\]\s*/, '')));
    out += `| **${p.name}** | ${purpose} | ${p.cost || 'n/a'} | ${p.stability || 'n/a'} | [Details](/docs/marketplace/${p.name}/) |\n`;
  }
  out += `${MARKER_END}`;
  return out;
}

function profilesTable(profiles) {
  let out = `${PROFILES_START}\n`;
  out += `| Profile | Plugins | Count |\n|---|---|---|\n`;
  for (const [name, list] of Object.entries(profiles || {})) {
    out += `| **${name}** | ${list.join(', ')} | ${list.length} |\n`;
  }
  out += `${PROFILES_END}`;
  return out;
}

function replaceBlock(text, start, end, replacement, label) {
  const s = text.indexOf(start);
  const e = text.indexOf(end);
  if (s === -1 || e === -1 || e < s) fail(`${label} markers ${start} / ${end} not found in overview.md`);
  return text.slice(0, s) + replacement + text.slice(e + end.length);
}

function main() {
  if (!existsSync(MARKETPLACE_DIR)) {
    fail(
      `marketplace repo not found at ${MARKETPLACE_DIR}\n` +
        `       set MARKETPLACE_DIR=/path/to/forgeplan-marketplace or clone ` +
        `git@github.com:ForgePlan/marketplace.git`
    );
  }
  const claudeManifest = join(MARKETPLACE_DIR, '.claude-plugin/marketplace.json');
  const ompManifest = join(MARKETPLACE_DIR, '.omp-plugin/marketplace.json');
  const manifestPath = existsSync(claudeManifest) ? claudeManifest : ompManifest;
  if (!existsSync(manifestPath)) fail(`no marketplace.json under ${MARKETPLACE_DIR}`);

  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  const plugins = manifest.plugins || [];
  if (!plugins.length) fail(`manifest ${manifestPath} lists no plugins`);
  console.log(`[mkt-gen] source ${manifestPath} | ${plugins.length} plugins`);

  let wrote = 0;
  for (const plugin of plugins) {
    writeFileSync(join(OUT_DIR, `${plugin.name}.md`), stripEmDashes(pageFor(plugin, MARKETPLACE_DIR)));
    wrote++;
  }

  if (!existsSync(OVERVIEW)) fail(`overview page not found at ${OVERVIEW}`);
  let overview = readFileSync(OVERVIEW, 'utf8');
  overview = replaceBlock(overview, MARKER_START, MARKER_END, pluginsTable(plugins), 'plugins');
  if (overview.includes(PROFILES_START)) {
    overview = replaceBlock(overview, PROFILES_START, PROFILES_END, profilesTable(manifest.profiles), 'profiles');
  }
  writeFileSync(OVERVIEW, stripEmDashes(overview));

  console.log(`[mkt-gen] wrote ${wrote} plugin pages + overview table to ${OUT_DIR}`);
}

main();
