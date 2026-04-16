#!/usr/bin/env node
// Generate Starlight CLI reference pages from `forgeplan help <cmd>` output.
// Part of PRD-046 — closes CLI coverage gap from 1/58 to 58/58.
//
// Usage: node scripts/generate-cli-docs.mjs [--binary ../target/release/forgeplan]

import { execFileSync } from 'node:child_process';
import { writeFileSync, mkdirSync, existsSync, readdirSync, unlinkSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../..');
const OUT_DIR = resolve(__dirname, '../src/content/docs/docs/cli');

const args = process.argv.slice(2);
const binaryArgIdx = args.indexOf('--binary');
const BINARY = binaryArgIdx >= 0
  ? args[binaryArgIdx + 1]
  : resolve(REPO_ROOT, 'target/release/forgeplan');

function run(cmdArgs) {
  return execFileSync(BINARY, cmdArgs, { cwd: REPO_ROOT, encoding: 'utf8' });
}

function parseTopLevel(output) {
  const lines = output.split('\n');
  const commands = [];
  let inCommands = false;
  for (const line of lines) {
    if (line.trimStart().startsWith('Commands:')) {
      inCommands = true;
      continue;
    }
    if (!inCommands) continue;
    if (line.trimStart().startsWith('Options:')) break;
    const m = line.match(/^\s{2}([a-z][a-z0-9-]*)\s{2,}(.*)$/);
    if (m) {
      commands.push({ name: m[1], short: m[2].trim() });
    }
  }
  return commands.filter(c => c.name !== 'help');
}

function parseHelp(output) {
  const lines = output.split('\n');
  let description = '';
  let usage = '';
  const argsBlock = [];
  const optsBlock = [];
  const subBlock = [];

  let i = 0;
  // First non-empty line = description
  while (i < lines.length && !lines[i].trim()) i++;
  if (i < lines.length && !lines[i].startsWith('Usage:')) {
    description = lines[i].trim();
    i++;
  }

  for (; i < lines.length; i++) {
    const line = lines[i];
    if (line.startsWith('Usage:')) {
      usage = line.replace(/^Usage:\s*/, '').trim();
      continue;
    }
    if (line.trim() === 'Arguments:') {
      i++;
      while (i < lines.length && lines[i].startsWith('  ')) {
        argsBlock.push(lines[i]);
        i++;
      }
      i--;
      continue;
    }
    if (line.trim() === 'Options:') {
      i++;
      while (i < lines.length && lines[i].startsWith('  ')) {
        optsBlock.push(lines[i]);
        i++;
      }
      i--;
      continue;
    }
    if (line.trim() === 'Commands:') {
      i++;
      while (i < lines.length && lines[i].startsWith('  ')) {
        subBlock.push(lines[i]);
        i++;
      }
      i--;
      continue;
    }
  }
  return { description, usage, argsBlock, optsBlock, subBlock };
}

function fmtBlock(block) {
  if (!block.length) return '';
  return '```text\n' + block.join('\n') + '\n```\n';
}

function exampleFor(cmd) {
  const examples = {
    init: '```bash\nforgeplan init -y\n```',
    new: '```bash\nforgeplan new prd "Authentication System"\nforgeplan new problem "Slow search on 10k+ artifacts"\n```',
    list: '```bash\nforgeplan list\nforgeplan list --kind prd --status active\n```',
    status: '```bash\nforgeplan status\n```',
    validate: '```bash\nforgeplan validate PRD-001\nforgeplan validate PRD-001 --ci  # exit 1 on MUST errors\n```',
    score: '```bash\nforgeplan score PRD-001\n```',
    link: '```bash\nforgeplan link EVID-001 PRD-001 --relation informs\n```',
    search: '```bash\nforgeplan search "authentication flow"\nforgeplan search "auth" --kind prd --limit 5\n```',
    route: '```bash\nforgeplan route "add rate limiting to API"\n```',
    review: '```bash\nforgeplan review PRD-001\n```',
    activate: '```bash\nforgeplan activate PRD-001\n```',
    supersede: '```bash\nforgeplan supersede ADR-003 --by ADR-005\n```',
    deprecate: '```bash\nforgeplan deprecate PRD-001 --reason "obsoleted by PRD-015"\n```',
    renew: '```bash\nforgeplan renew ADR-001 --reason "still valid after architecture review" --until 2026-10-01\n```',
    reopen: '```bash\nforgeplan reopen PRD-010 --reason "re-evaluating approach"\n```',
    reason: '```bash\nforgeplan reason PRD-001\nforgeplan reason PRD-001 --fpf  # include FPF KB context\n```',
    generate: '```bash\nforgeplan generate prd "OAuth2 login flow"\n```',
    decompose: '```bash\nforgeplan decompose PRD-001\n```',
    health: '```bash\nforgeplan health\nforgeplan health --ci  # exit 1 on orphans/blindspots\n```',
    reindex: '```bash\nforgeplan reindex\n```',
    'scan-import': '```bash\nforgeplan scan-import\n```',
    serve: '```bash\nforgeplan serve  # starts MCP server on stdio\n```',
    discover: '```bash\nforgeplan discover\n```',
    tag: '```bash\nforgeplan tag PRD-001 security auth\n```',
    untag: '```bash\nforgeplan untag PRD-001 legacy\n```',
    get: '```bash\nforgeplan get PRD-001\n```',
    update: '```bash\nforgeplan update PRD-001 --status active\n```',
    delete: '```bash\nforgeplan delete NOTE-042\n```',
    graph: '```bash\nforgeplan graph > deps.mmd\n```',
    blocked: '```bash\nforgeplan blocked\n```',
    blindspots: '```bash\nforgeplan blindspots\n```',
    order: '```bash\nforgeplan order\n```',
    tree: '```bash\nforgeplan tree\n```',
    journal: '```bash\nforgeplan journal\n```',
    gaps: '```bash\nforgeplan gaps\n```',
    stale: '```bash\nforgeplan stale\n```',
    decay: '```bash\nforgeplan decay\n```',
    export: '```bash\nforgeplan export --output backup.json\n```',
    import: '```bash\nforgeplan import backup.json\n```',
    scan: '```bash\nforgeplan scan\n```',
    coverage: '```bash\nforgeplan coverage\n```',
    drift: '```bash\nforgeplan drift\n```',
    fgr: '```bash\nforgeplan fgr PRD-001\n```',
    progress: '```bash\nforgeplan progress\n```',
    session: '```bash\nforgeplan session\n```',
    estimate: '```bash\nforgeplan estimate PRD-001\n```',
    'calibrate-estimate': '```bash\nforgeplan calibrate-estimate\n```',
    calibrate: '```bash\nforgeplan calibrate PRD-001\n```',
    promote: '```bash\nforgeplan promote mem-xxx --kind prd\n```',
    context: '```bash\nforgeplan context PRD-001\n```',
    capture: '```bash\nforgeplan capture --to note\n```',
    embed: '```bash\nforgeplan embed\n```',
    log: '```bash\nforgeplan log\n```',
    remember: '```bash\nforgeplan remember "always pull dev before creating feature branch" --kind procedure\n```',
    recall: '```bash\nforgeplan recall "branch workflow"\n```',
    watch: '```bash\nforgeplan watch\n```',
    'git-sync': '```bash\nforgeplan git-sync\n```',
    migrate: '```bash\nforgeplan migrate\n```',
    fpf: '```bash\nforgeplan fpf search "trust calculus"\nforgeplan fpf section B.3\n```',
    unlink: '```bash\nforgeplan unlink PRD-001 PROB-035\n```',
    'setup-skill': '```bash\nforgeplan setup-skill\n```',
  };
  return examples[cmd] || '```bash\nforgeplan ' + cmd + '\n```';
}

function escapeYaml(s) {
  return s.replace(/"/g, '\\"');
}

function pageFor(cmd, help) {
  const { description, usage, argsBlock, optsBlock, subBlock } = parseHelp(help);
  const title = `forgeplan ${cmd}`;
  const desc = description || cmd;
  const example = exampleFor(cmd);

  let body = `---\ntitle: ${title}\ndescription: "${escapeYaml(desc)}"\n---\n\n`;
  body += `${desc}\n\n`;
  body += `## Usage\n\n\`\`\`text\n${usage || 'forgeplan ' + cmd}\n\`\`\`\n\n`;
  if (argsBlock.length) {
    body += `## Arguments\n\n${fmtBlock(argsBlock)}\n`;
  }
  if (optsBlock.length) {
    body += `## Options\n\n${fmtBlock(optsBlock)}\n`;
  }
  if (subBlock.length) {
    body += `## Subcommands\n\n${fmtBlock(subBlock)}\n`;
  }
  body += `## Example\n\n${example}\n\n`;
  body += `## See also\n\n`;
  body += `- [CLI overview](/docs/cli/)\n`;
  body += `- [Methodology guide](/docs/methodology/overview/)\n`;
  body += `- [\`forgeplan health\`](/docs/cli/health/) — session start check\n`;
  return body;
}

function pageForSubcommand(parent, subName, subDesc) {
  const title = `forgeplan ${parent} ${subName}`;
  let body = `---\ntitle: ${title}\ndescription: "${escapeYaml(subDesc)}"\n---\n\n`;
  body += `${subDesc}\n\n`;
  body += `## Usage\n\n\`\`\`text\nforgeplan ${parent} ${subName} [OPTIONS]\n\`\`\`\n\n`;
  body += `## Example\n\n\`\`\`bash\nforgeplan ${parent} ${subName}\n\`\`\`\n\n`;
  body += `## See also\n\n`;
  body += `- [\`forgeplan ${parent}\`](/docs/cli/${parent}/)\n`;
  body += `- [CLI overview](/docs/cli/)\n`;
  return body;
}

function writeOverview(commands) {
  let body = `---\ntitle: CLI Reference\ndescription: "Complete reference for all ${commands.length} Forgeplan CLI commands."\n---\n\n`;
  body += `Forgeplan ships with **${commands.length} top-level commands** covering the full Shape→Validate→ADI→Code→Evidence→Activate lifecycle.\n\n`;
  body += `All commands are listed below grouped by purpose. Click any command for full usage, arguments and examples.\n\n`;
  const groups = {
    'Workspace & setup': ['init', 'setup-skill', 'migrate', 'import', 'export'],
    'Creating artifacts': ['new', 'generate', 'capture', 'promote'],
    'Reading artifacts': ['list', 'get', 'tree', 'search', 'recall', 'log', 'journal', 'session', 'progress', 'graph', 'order'],
    'Editing artifacts': ['update', 'delete', 'tag', 'untag', 'link', 'unlink'],
    'Quality & validation': ['validate', 'score', 'fgr', 'review', 'estimate', 'calibrate', 'calibrate-estimate', 'decay', 'stale'],
    'Lifecycle transitions': ['activate', 'supersede', 'deprecate', 'renew', 'reopen'],
    'Reasoning & AI': ['reason', 'decompose', 'context', 'route'],
    'Dashboards & health': ['health', 'status', 'gaps', 'blocked', 'blindspots', 'drift', 'coverage'],
    'Indexing & sync': ['scan', 'scan-import', 'reindex', 'embed', 'watch', 'git-sync'],
    'Memory': ['remember', 'discover'],
    'FPF knowledge base': ['fpf'],
    'MCP server': ['serve'],
  };
  for (const [group, names] of Object.entries(groups)) {
    const rows = names
      .map(n => commands.find(c => c.name === n))
      .filter(Boolean);
    if (!rows.length) continue;
    body += `### ${group}\n\n`;
    body += `| Command | Description |\n|---|---|\n`;
    for (const c of rows) {
      body += `| [\`forgeplan ${c.name}\`](/docs/cli/${c.name}/) | ${c.short} |\n`;
    }
    body += `\n`;
  }
  const seen = new Set(Object.values(groups).flat());
  const others = commands.filter(c => !seen.has(c.name));
  if (others.length) {
    body += `### Other\n\n| Command | Description |\n|---|---|\n`;
    for (const c of others) {
      body += `| [\`forgeplan ${c.name}\`](/docs/cli/${c.name}/) | ${c.short} |\n`;
    }
    body += `\n`;
  }
  writeFileSync(resolve(OUT_DIR, 'index.md'), body);
}

function main() {
  if (!existsSync(BINARY)) {
    // Fallback to cargo run if release binary missing
    console.error(`[cli-gen] release binary not found at ${BINARY}; trying cargo run`);
    process.env.FORGEPLAN_VIA_CARGO = '1';
  }

  // Ensure out dir exists; clean previous generated files (keep health.md if present).
  mkdirSync(OUT_DIR, { recursive: true });
  for (const f of readdirSync(OUT_DIR)) {
    if (f.endsWith('.md') && f !== 'health.md' && f !== 'index.md') {
      unlinkSync(resolve(OUT_DIR, f));
    }
  }

  const topHelp = process.env.FORGEPLAN_VIA_CARGO
    ? execFileSync('cargo', ['run', '--quiet', '--bin', 'forgeplan', '--', '--help'], { cwd: REPO_ROOT, encoding: 'utf8' })
    : run(['--help']);
  const commands = parseTopLevel(topHelp);
  console.log(`[cli-gen] found ${commands.length} commands`);

  for (const cmd of commands) {
    let help;
    try {
      help = process.env.FORGEPLAN_VIA_CARGO
        ? execFileSync('cargo', ['run', '--quiet', '--bin', 'forgeplan', '--', 'help', cmd.name], { cwd: REPO_ROOT, encoding: 'utf8' })
        : run(['help', cmd.name]);
    } catch (err) {
      console.error(`[cli-gen] ${cmd.name}: help failed: ${err.message}`);
      continue;
    }
    const page = pageFor(cmd.name, help);
    writeFileSync(resolve(OUT_DIR, `${cmd.name}.md`), page);

    // Nested subcommands (fpf, help etc.)
    const parsed = parseHelp(help);
    if (parsed.subBlock.length) {
      for (const sub of parsed.subBlock) {
        const m = sub.match(/^\s{2}([a-z][a-z0-9-]*)\s{2,}(.*)$/);
        if (!m) continue;
        const subName = m[1];
        if (subName === 'help') continue;
        const subDesc = m[2].trim();
        const page2 = pageForSubcommand(cmd.name, subName, subDesc);
        writeFileSync(resolve(OUT_DIR, `${cmd.name}-${subName}.md`), page2);
      }
    }
  }

  writeOverview(commands);
  console.log(`[cli-gen] wrote ${readdirSync(OUT_DIR).filter(f => f.endsWith('.md')).length} files to ${OUT_DIR}`);
}

main();
