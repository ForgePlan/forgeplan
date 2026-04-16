#!/usr/bin/env node
// Generate Starlight MCP reference pages by parsing #[tool(...)] attributes
// in crates/forgeplan-mcp/src/server.rs.
// Part of PRD-046 — closes MCP coverage gap from 1/45 to 45/45.

import { readFileSync, writeFileSync, mkdirSync, existsSync, readdirSync, unlinkSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../..');
const SERVER_RS = resolve(REPO_ROOT, 'crates/forgeplan-mcp/src/server.rs');
const OUT_DIR = resolve(__dirname, '../src/content/docs/docs/mcp');

function escapeYaml(s) {
  return s.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
}

function parseServer() {
  const src = readFileSync(SERVER_RS, 'utf8');
  const lines = src.split('\n');
  const tools = [];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (!line.trim().startsWith('#[tool(')) continue;

    // Collect the full attribute (may span multiple lines until matching ')]')
    let attr = line.trim().slice('#[tool('.length);
    let end = i;
    while (!attr.includes(')]')) {
      end += 1;
      if (end >= lines.length) break;
      attr += ' ' + lines[end].trim();
    }
    attr = attr.replace(/\)\]\s*$/, '').trim();

    // Extract description = "..." (may contain escaped quotes)
    let description = '';
    const dm = attr.match(/description\s*=\s*"((?:[^"\\]|\\.)*)"/);
    if (dm) description = dm[1].replace(/\\"/g, '"').replace(/\\n/g, ' ');

    // Skip attribute lines and find the async fn declaration
    let fnLine = -1;
    for (let j = end + 1; j < Math.min(end + 15, lines.length); j++) {
      const ll = lines[j];
      if (ll.includes('async fn ')) { fnLine = j; break; }
      if (ll.includes('pub async fn ')) { fnLine = j; break; }
    }
    if (fnLine < 0) continue;

    const nameMatch = lines[fnLine].match(/async fn ([a-z_][a-z0-9_]*)/);
    if (!nameMatch) continue;
    const name = nameMatch[1];

    // Collect signature until opening brace or Result<...>
    let sig = lines[fnLine];
    let sj = fnLine;
    while (!sig.includes('Result<') && sj < Math.min(fnLine + 40, lines.length)) {
      sj += 1;
      sig += ' ' + lines[sj];
    }

    // Extract parameters between first `(` and matching `)` before `->`
    const parenStart = sig.indexOf('(');
    let depth = 0;
    let parenEnd = -1;
    for (let k = parenStart; k < sig.length; k++) {
      if (sig[k] === '(') depth++;
      else if (sig[k] === ')') {
        depth--;
        if (depth === 0) { parenEnd = k; break; }
      }
    }
    const paramsRaw = parenEnd > 0 ? sig.slice(parenStart + 1, parenEnd) : '';

    // Split parameters on top-level commas
    const params = [];
    {
      let d = 0;
      let buf = '';
      for (const ch of paramsRaw) {
        if (ch === '<' || ch === '(' || ch === '[' || ch === '{') d++;
        else if (ch === '>' || ch === ')' || ch === ']' || ch === '}') d--;
        if (ch === ',' && d === 0) {
          if (buf.trim()) params.push(buf.trim());
          buf = '';
        } else {
          buf += ch;
        }
      }
      if (buf.trim()) params.push(buf.trim());
    }

    // Skip &self / &mut self
    const userParams = params
      .filter(p => !/^&(mut\s+)?self$/.test(p.trim()))
      .map(p => {
        // Common shape: "Parameters(params): Parameters<ToolXxxParams>"
        // Or: "name: Type"
        const m = p.match(/([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(.+)$/);
        if (!m) return { name: p.trim(), type: 'unknown' };
        return { name: m[1], type: m[2].trim() };
      });

    tools.push({ name, description, params: userParams });
  }
  return tools;
}

function categoryFor(name) {
  if (/^forgeplan_(init|migrate|import|export|scan|reindex)/.test(name)) return 'Workspace & Data';
  if (/^forgeplan_(new|capture|generate)/.test(name)) return 'Creating Artifacts';
  if (/^forgeplan_(list|get|search|graph|tree|order|blocked|journal|session|progress|log)/.test(name)) return 'Reading Artifacts';
  if (/^forgeplan_(update|delete|link|tag|untag|unlink)/.test(name)) return 'Editing Artifacts';
  if (/^forgeplan_(validate|score|fgr|review|estimate|calibrate|decay|stale|drift|coverage|guard)/.test(name)) return 'Quality & Validation';
  if (/^forgeplan_(activate|supersede|deprecate|renew|reopen)/.test(name)) return 'Lifecycle';
  if (/^forgeplan_(reason|decompose|route|context)/.test(name)) return 'Reasoning & AI';
  if (/^forgeplan_(health|status|gaps|blindspots)/.test(name)) return 'Dashboards';
  if (/^forgeplan_fpf_/.test(name)) return 'FPF Knowledge Base';
  if (/^forgeplan_discover/.test(name)) return 'Brownfield Discovery';
  return 'Other';
}

function exampleFor(tool) {
  const examples = {
    forgeplan_init: `{ "force": false, "scan": true }`,
    forgeplan_new: `{ "kind": "prd", "title": "Authentication system" }`,
    forgeplan_list: `{ "kind": "prd", "status": "active" }`,
    forgeplan_get: `{ "id": "PRD-001" }`,
    forgeplan_validate: `{ "id": "PRD-001" }`,
    forgeplan_score: `{ "id": "PRD-001" }`,
    forgeplan_link: `{ "source": "EVID-001", "target": "PRD-001", "relation": "informs" }`,
    forgeplan_search: `{ "query": "authentication flow", "limit": 5 }`,
    forgeplan_route: `{ "description": "add rate limiting to API" }`,
    forgeplan_review: `{ "id": "PRD-001" }`,
    forgeplan_activate: `{ "id": "PRD-001" }`,
    forgeplan_reason: `{ "id": "PRD-001", "fpf": true }`,
    forgeplan_decompose: `{ "id": "PRD-001" }`,
    forgeplan_generate: `{ "kind": "prd", "description": "OAuth2 login flow" }`,
    forgeplan_health: `{}`,
    forgeplan_status: `{}`,
    forgeplan_journal: `{ "limit": 10 }`,
    forgeplan_blindspots: `{}`,
    forgeplan_graph: `{}`,
    forgeplan_blocked: `{}`,
    forgeplan_order: `{}`,
    forgeplan_fpf_search: `{ "query": "trust calculus" }`,
    forgeplan_fpf_section: `{ "id": "B.3" }`,
    forgeplan_fpf_list: `{}`,
    forgeplan_discover_start: `{ "scope": "." }`,
  };
  return examples[tool.name] || `{}`;
}

function pageFor(tool) {
  const title = tool.name;
  const desc = tool.description || title;
  let body = `---\ntitle: ${title}\ndescription: "${escapeYaml(desc)}"\n---\n\n`;
  body += `${desc}\n\n`;
  body += `**Category**: ${categoryFor(tool.name)}\n\n`;

  body += `## Input parameters\n\n`;
  if (!tool.params.length) {
    body += `_No input parameters. Call this tool with an empty object \`{}\`._\n\n`;
  } else {
    body += `| Name | Rust type | Description |\n|---|---|---|\n`;
    for (const p of tool.params) {
      body += `| \`${p.name}\` | \`${p.type.replace(/\|/g, '\\|')}\` | see source |\n`;
    }
    body += `\n`;
    body += `_Parameter structs are defined in \`crates/forgeplan-mcp/src/types.rs\`. Refer to that file for exact field names and optionality._\n\n`;
  }

  body += `## Example invocation\n\n\`\`\`json\n${exampleFor(tool)}\n\`\`\`\n\n`;
  body += `## Usage from an AI agent\n\n`;
  body += `This tool is exposed via MCP (stdio transport). Start the server with \`forgeplan serve\` and configure your AI client (Claude Code, Cursor, etc.) to connect to it.\n\n`;
  body += `\`\`\`bash\nforgeplan serve\n\`\`\`\n\n`;
  body += `## See also\n\n`;
  body += `- [MCP overview](/docs/mcp/)\n`;
  body += `- [\`forgeplan serve\`](/docs/cli/serve/)\n`;
  body += `- [Methodology guide](/docs/methodology/overview/)\n`;
  return body;
}

function writeOverview(tools) {
  const groups = {};
  for (const t of tools) {
    const g = categoryFor(t.name);
    (groups[g] ||= []).push(t);
  }
  const order = [
    'Workspace & Data',
    'Creating Artifacts',
    'Reading Artifacts',
    'Editing Artifacts',
    'Quality & Validation',
    'Lifecycle',
    'Reasoning & AI',
    'Dashboards',
    'FPF Knowledge Base',
    'Brownfield Discovery',
    'Other',
  ];
  let body = `---\ntitle: MCP Tools\ndescription: "Reference for all ${tools.length} Model Context Protocol tools exposed by \`forgeplan serve\`."\n---\n\n`;
  body += `Forgeplan ships with **${tools.length} MCP tools** that an AI agent can call over the Model Context Protocol (stdio transport).\n\n`;
  body += `Start the MCP server:\n\n\`\`\`bash\nforgeplan serve\n\`\`\`\n\nConfigure your agent (Claude Code, Cursor, etc.) to connect, then invoke any of the tools listed below.\n\n`;
  for (const g of order) {
    const items = groups[g];
    if (!items || !items.length) continue;
    body += `### ${g}\n\n`;
    body += `| Tool | Description |\n|---|---|\n`;
    items.sort((a, b) => a.name.localeCompare(b.name));
    for (const t of items) {
      body += `| [\`${t.name}\`](/docs/mcp/${t.name}/) | ${t.description.replace(/\|/g, '\\|')} |\n`;
    }
    body += `\n`;
  }
  writeFileSync(resolve(OUT_DIR, 'index.md'), body);
}

function main() {
  mkdirSync(OUT_DIR, { recursive: true });
  for (const f of readdirSync(OUT_DIR)) {
    if (f.endsWith('.md')) unlinkSync(resolve(OUT_DIR, f));
  }

  const tools = parseServer();
  console.log(`[mcp-gen] found ${tools.length} tools`);

  for (const t of tools) {
    writeFileSync(resolve(OUT_DIR, `${t.name}.md`), pageFor(t));
  }
  writeOverview(tools);
  console.log(`[mcp-gen] wrote ${readdirSync(OUT_DIR).filter(f => f.endsWith('.md')).length} files to ${OUT_DIR}`);
}

main();
