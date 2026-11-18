import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { createInterface } from 'node:readline';
import { parseArgs } from 'node:util';
import { fileURLToPath } from 'node:url';

export const START = '<!-- forgeplan-runtime:start -->';
export const END = '<!-- forgeplan-runtime:end -->';
export const locales = ['en', 'ru'];

export function options() {
  const { values } = parseArgs({ options: {
    binary: { type: 'string' },
    check: { type: 'boolean', default: false },
    'out-dir': { type: 'string' },
  } });
  if (!values.binary) throw new Error('Pass --binary <built forgeplan>. These scripts never run Cargo.');
  const binary = resolve(values.binary);
  if (!existsSync(binary)) throw new Error(`Binary does not exist: ${binary}`);
  return {
    binary,
    check: values.check,
    outDir: values['out-dir'] ? resolve(values['out-dir'])
      : fileURLToPath(new URL('../src/content/docs/', import.meta.url)),
    version: execFileSync(binary, ['--version'], { encoding: 'utf8', timeout: 30000 }).trim(),
  };
}

export function localePaths(outDir, locale, section) {
  return {
    dir: join(outDir, locale === 'en' ? 'docs' : 'ru/docs', section),
    url: locale === 'en' ? '/docs' : '/ru/docs',
  };
}

export function tableText(value) {
  return String(value ?? '').replaceAll('&', '&amp;').replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;').replaceAll('|', '\\|').replace(/\r?\n/g, '<br>');
}

export function managed(content) {
  return `${START}\n${content.trimEnd()}\n${END}`;
}

// Only this marked block is owned on an existing detail page. Examples and
// explanations outside it survive regeneration byte-for-byte. Overviews are
// wholly generated; removed interface pages are reported, never silently deleted.
export function updatePage(path, block, newPage, check) {
  const current = existsSync(path) ? readFileSync(path, 'utf8') : null;
  let expected;
  if (current === null) expected = newPage(block);
  else {
    const start = current.indexOf(START);
    const end = current.indexOf(END);
    if ((start < 0) !== (end < 0) || (start >= 0 && (end < start
      || current.indexOf(START, start + START.length) >= 0
      || current.indexOf(END, end + END.length) >= 0))) {
      throw new Error(`Malformed or repeated generated block: ${path}`);
    }
    expected = start < 0
      ? current + (current.endsWith('\n\n') ? '' : current.endsWith('\n') ? '\n' : '\n\n') + block + '\n'
      : current.slice(0, start) + block + current.slice(end + END.length);
  }
  return updateWholePage(path, expected, check);
}

export function updateWholePage(path, expected, check) {
  const current = existsSync(path) ? readFileSync(path, 'utf8') : null;
  if (current === expected) return false;
  if (check) console.error(`[reference] missing or stale: ${path}`);
  else {
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, expected);
  }
  return true;
}

// Discover metadata in an empty temporary directory, with a private HOME and
// no inherited workspace binding. No tool is called and no real index is opened.
export async function toolCatalog(binary, profile) {
  const root = realpathSync(mkdtempSync(join(tmpdir(), 'forgeplan-reference-')));
  // The server discovers workspaces upwards; a private HOME alone does not
  // isolate TMPDIR inside an existing project. Reject before starting MCP.
  for (let ancestor = root; ; ancestor = dirname(ancestor)) {
    if (existsSync(join(ancestor, '.forgeplan'))) {
      rmSync(root, { recursive: true, force: true });
      throw new Error(`Refusing MCP discovery under workspace ${ancestor}; set TMPDIR outside any workspace.`);
    }
    if (dirname(ancestor) === ancestor) break;
  }
  const home = join(root, 'home');
  mkdirSync(home);
  // The CLI loads ancestor .env files; an existing empty value blocks reload
  // while the workspace resolver ignores it in favor of the sandbox default.
  const env = { ...process.env, HOME: home, NO_COLOR: '1', FORGEPLAN_WORKSPACE: '' };
  const child = spawn(binary, ['serve', '--profile', profile, '--workspace', root], {
    cwd: root, env, stdio: ['pipe', 'pipe', 'pipe'], timeout: 45000, killSignal: 'SIGKILL',
  });
  const pending = new Map();
  let sequence = 0;
  let stderr = '';
  let closed = false;
  let failure;
  const fail = error => {
    failure = error;
    for (const item of pending.values()) item.reject(error);
    pending.clear();
  };
  const exit = new Promise(resolveExit => child.once('close', (code, signal) => {
    closed = true;
    if (pending.size) fail(new Error(`MCP ${profile} closed before replying (${code ?? signal}): ${stderr}`));
    resolveExit({ code, signal });
  }));
  child.on('error', fail);
  child.stdin.on('error', fail);
  child.stderr.setEncoding('utf8');
  child.stderr.on('data', chunk => { stderr += chunk; });
  const lines = createInterface({ input: child.stdout });
  lines.on('line', line => {
    try {
      const message = JSON.parse(line);
      const item = pending.get(message.id);
      if (!item) return;
      pending.delete(message.id);
      if (message.error) item.reject(new Error(JSON.stringify(message.error)));
      else item.resolve(message.result);
    } catch (error) { fail(error); child.kill('SIGKILL'); }
  });
  const request = (method, params) => new Promise((resolveRequest, reject) => {
    if (failure) { reject(failure); return; }
    const id = ++sequence;
    pending.set(id, { resolve: resolveRequest, reject });
    child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n');
  });
  try {
    await request('initialize', {
      protocolVersion: '2024-11-05', capabilities: {},
      clientInfo: { name: 'forgeplan-reference-generator', version: '1' },
    });
    child.stdin.write(JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n');
    const tools = [];
    let cursor;
    do {
      const page = await request('tools/list', cursor ? { cursor } : {});
      if (!Array.isArray(page?.tools)) throw new Error(`MCP ${profile}: no tools/list payload`);
      tools.push(...page.tools);
      cursor = page.nextCursor;
    } while (cursor);
    if (!tools.length) throw new Error(`MCP ${profile}: empty tool catalog; discovery did not happen`);
    const names = new Set();
    for (const tool of tools) {
      if (!/^[a-z][a-z0-9_]*$/.test(tool.name) || names.has(tool.name) || !tool.inputSchema) {
        throw new Error(`Invalid or duplicate tool in ${profile}: ${tool.name}`);
      }
      names.add(tool.name);
    }
    child.stdin.end();
    const result = await exit;
    if (result.code !== 0) throw new Error(`MCP ${profile} exit ${result.code ?? result.signal}: ${stderr}`);
    return tools;
  } finally {
    child.stdin.end();
    if (!closed) child.kill('SIGKILL');
    await exit;
    lines.close();
    rmSync(root, { recursive: true, force: true });
  }
}
