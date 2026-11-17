#!/usr/bin/env node
// Refresh captured CLI help blocks; maintain authored prose in the Markdown pages.
// Usage: node scripts/generate-cli-docs.mjs --binary /path/to/forgeplan [--sync-ru] [--report /path/to/report.json]

import { execFileSync } from 'node:child_process';
import { writeFileSync, readFileSync, mkdirSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, relative, resolve } from 'node:path';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const OUT_DIR = resolve(REPO_ROOT, 'website/src/content/docs/docs/cli');
const RU_DIR = resolve(REPO_ROOT, 'website/src/content/docs/ru/docs/cli');
const REVIEWED_VERSION = 'forgeplan 0.38.0';

function parseArgs(argv) {
  const options = { binary: resolve(REPO_ROOT, 'target/release/forgeplan'), syncRu: false, report: null };
  const seen = new Set();
  for (let index = 0; index < argv.length; index++) {
    const flag = argv[index];
    if (!['--binary', '--sync-ru', '--report'].includes(flag)) {
      throw new Error(`Unknown argument: ${flag}`);
    }
    if (seen.has(flag)) throw new Error(`Duplicate argument: ${flag}`);
    seen.add(flag);
    if (flag === '--sync-ru') {
      options.syncRu = true;
      continue;
    }
    const value = argv[++index];
    if (!value || value.startsWith('-')) throw new Error(`${flag} requires a path`);
    options[flag === '--binary' ? 'binary' : 'report'] = value;
  }
  return options;
}

// Reject malformed invocations before file reads or binary execution.
const options = parseArgs(process.argv.slice(2));

function run(command) {
  return execFileSync(options.binary, command, { cwd: REPO_ROOT, encoding: 'utf8' });
}

function parseTopLevel(output) {
  const commands = [];
  let inCommands = false;
  for (const line of output.split('\n')) {
    if (line.trimStart().startsWith('Commands:')) {
      inCommands = true;
      continue;
    }
    if (!inCommands) continue;
    if (line.trimStart().startsWith('Options:')) break;
    const match = line.match(/^\s{2}([a-z][a-z0-9-]*)\s{2,}(.*)$/);
    if (match && match[1] !== 'help') commands.push(match[1]);
  }
  return commands;
}

function parseHelp(output) {
  const sections = {};
  let section = 'description';
  for (const line of output.trimEnd().split('\n')) {
    const heading = line.match(/^(Usage|Arguments|Options|Commands):\s*(.*)$/);
    if (heading) {
      section = heading[1];
      sections[section] = heading[2] ? [heading[2]] : [];
    } else {
      (sections[section] ||= []).push(line);
    }
  }
  const block = name => (sections[name] || []).join('\n').trimEnd().split('\n').filter((line, index, lines) => line || (index > 0 && index < lines.length - 1));
  return {
    description: (sections.description || []).join('\n').trim(),
    usage: (sections.Usage || []).join('\n').trim(),
    argsBlock: block('Arguments'),
    optsBlock: block('Options'),
    subBlock: block('Commands'),
  };
}

function helpSections(help, ru) {
  const parsed = parseHelp(help);
  return [
    [ru ? 'Использование' : 'Usage', parsed.usage, ['Usage', 'Использование']],
    [ru ? 'Аргументы' : 'Arguments', parsed.argsBlock.join('\n'), ['Arguments', 'Аргументы']],
    [ru ? 'Опции' : 'Options', parsed.optsBlock.join('\n'), ['Options', 'Опции', 'Параметры']],
    [ru ? 'Подкоманды' : 'Subcommands', parsed.subBlock.join('\n'), ['Commands', 'Subcommands', 'Команды', 'Подкоманды']],
  ];
}

// Replace only an immediately fenced help block, not prose, examples, metadata,
// or authored subcommand navigation. Missing help sections are added separately.
function refreshContracts(body, help, ru = false) {
  let additions = '';
  for (const [title, content, aliases] of helpSections(help, ru)) {
    const pattern = new RegExp(`(^## (?:${aliases.join('|')})\\n\\n)\`\`\`(?:text)?\\n[\\s\\S]*?\\n\`\`\`(?=\\n|$)`, 'm');
    if (pattern.test(body)) {
      body = body.replace(pattern, (_, heading) => `${heading}\`\`\`text\n${content}\n\`\`\``);
    } else if (content) {
      additions += `## ${title}\n\n\`\`\`text\n${content}\n\`\`\`\n\n`;
    }
  }
  if (additions) {
    const firstSection = body.search(/^## /m);
    body = firstSection < 0 ? `${body}\n${additions}` : body.slice(0, firstSection) + additions + body.slice(firstSection);
  }
  return body;
}

function pageFor(command, help) {
  const title = `forgeplan ${command.join(' ')}`;
  const synopsis = parseHelp(help).description;
  let body = `---\ntitle: ${title}\ndescription: ${JSON.stringify(`Captured CLI help for ${title}.`)}\n---\n\n`;
  body += `Captured from ${REVIEWED_VERSION} help. This is tool output, not behavioral verification.\n\n`;
  if (synopsis) body += `## Captured help synopsis\n\n\`\`\`text\n${synopsis}\n\`\`\`\n\n`;
  body = refreshContracts(body, help);
  body += '## See also\n\n- [CLI reference](/docs/cli/)\n';
  return body;
}

function catalogSummary(page, existing) {
  let description = existing?.match(/^description:[ \t]*(.+)$/m)?.[1].trim();
  if (description?.startsWith('"')) {
    try {
      description = JSON.parse(description);
    } catch {
      description = null;
    }
  } else if (description?.startsWith("'") && description.endsWith("'")) {
    description = description.slice(1, -1).replaceAll("''", "'");
  }
  if (!description || /^[>|]/.test(description)) {
    description = `Help synopsis: ${parseHelp(page.help).description.split('\n')[0] || page.command.join(' ')}`;
  }
  return description.replaceAll('|', '\\|').replaceAll('\n', ' ');
}

// Existing catalog rows and summaries are authored. Only missing command links
// are appended, using an existing page's description or a labelled help synopsis.
function refreshIndex(before, pages, documents) {
  const missing = pages.filter(page => !before?.includes(`](/docs/cli/${page.name}/)`));
  if (!missing.length) return before;
  const heading = before ? '## Additional commands' : '## Commands';
  const rows = missing.map(page => `| [\`forgeplan ${page.command.join(' ')}\`](/docs/cli/${page.name}/) | ${catalogSummary(page, documents.get(page.name))} |`).join('\n');
  const body = before ?? '---\ntitle: CLI Reference\ndescription: "Forgeplan CLI command reference."\n---\n';
  return `${body}${body.endsWith('\n') ? '\n' : '\n\n'}${heading}\n\n| Command | Description |\n|---|---|\n${rows}\n`;
}

function generationReport(outputs, pages, version) {
  return {
    binary: options.binary,
    published_version: version,
    commands_captured: pages.map(page => page.command.join(' ')),
    command_count: pages.length,
    files: outputs.map(({ path, before, body }) => ({
      path: relative(REPO_ROOT, path),
      status: before === body ? 'unchanged' : before === null ? 'created' : 'changed',
      reason: before === body ? 'Existing content retained.' : 'Refreshed captured help blocks or added missing catalog entries; no authored prose migration.',
    })),
    new_files: outputs.filter(output => output.before === null).map(output => relative(REPO_ROOT, output.path)),
    deleted_files: [],
    verification: 'Documentation generation only; not behavioral verification.',
  };
}

function main() {
  if (!existsSync(options.binary)) throw new Error(`Binary not found: ${options.binary}. Build or install it separately.`);
  const version = run(['--version']).trim();
  if (version !== REVIEWED_VERSION) {
    throw new Error(`Unsupported binary version: ${version}; review the reference before changing ${REVIEWED_VERSION}`);
  }
  const pages = [];
  for (const name of parseTopLevel(run(['--help']))) {
    const help = run(['help', name]);
    pages.push({ name, command: [name], help });
    for (const sub of parseTopLevel(help)) {
      pages.push({ name: `${name}-${sub}`, command: [name, sub], help: run(['help', name, sub]) });
    }
  }

  // Prepare every requested output before writing. RU synchronization refreshes
  // help blocks in existing translations only; it never translates prose/examples.
  const outputs = [];
  const documents = new Map();
  for (const page of pages) {
    const path = resolve(OUT_DIR, `${page.name}.md`);
    const before = existsSync(path) ? readFileSync(path, 'utf8') : null;
    documents.set(page.name, before);
    outputs.push({ path, before, body: before === null ? pageFor(page.command, page.help) : refreshContracts(before, page.help) });
    if (options.syncRu) {
      const path = resolve(RU_DIR, `${page.name}.md`);
      if (!existsSync(path)) throw new Error(`Missing translation: ${path}`);
      const before = readFileSync(path, 'utf8');
      outputs.push({ path, before, body: refreshContracts(before, page.help, true) });
    }
  }
  const indexPath = resolve(OUT_DIR, 'index.md');
  const indexBefore = existsSync(indexPath) ? readFileSync(indexPath, 'utf8') : null;
  outputs.push({ path: indexPath, before: indexBefore, body: refreshIndex(indexBefore, pages, documents) });
  if (options.syncRu) {
    const path = resolve(RU_DIR, 'index.md');
    if (!existsSync(path)) throw new Error(`Missing translation: ${path}`);
    const before = readFileSync(path, 'utf8');
    outputs.push({ path, before, body: before });
  }

  mkdirSync(OUT_DIR, { recursive: true });
  for (const { path, before, body } of outputs) {
    if (body !== before) writeFileSync(path, body);
  }
  if (options.report) {
    writeFileSync(options.report, `${JSON.stringify(generationReport(outputs, pages, version), null, 2)}\n`);
  }
  console.log(`[cli-gen] captured ${pages.length} command help pages; preserved authored content`);
}

main();
