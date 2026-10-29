#!/usr/bin/env node
// Capture nested command paths discovered through --help; preserve prose and examples outside owned blocks.
import { execFileSync } from 'node:child_process';
import { readdirSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { options, locales, localePaths, managed, tableText, updatePage, updateWholePage } from './reference-runtime.mjs';

const config = options();
const run = args => execFileSync(config.binary, args, {
  encoding: 'utf8', timeout: 30000, maxBuffer: 8 * 1024 * 1024,
  env: { ...process.env, NO_COLOR: '1' },
});
function children(help) {
  const section = help.match(/^Commands:\n([\s\S]*?)(?=^Options:|^Arguments:|$(?![\s\S]))/m)?.[1] ?? '';
  return [...section.matchAll(/^  ([a-z][a-z0-9-]+)\s{2,}(.*)$/gm)]
    .filter(match => match[1] !== 'help').map(match => ({ name: match[1], short: match[2].trim() }));
}
const slugFor = path => path.length === 1 && path[0] === 'index' ? 'index-command' : path.join('-');
const queue = [{ path: [], help: run(['--help']) }];
const commands = [];
const slugs = new Set();
// Fetch the entire interface before touching files. A failed nested help is fatal.
while (queue.length) {
  const current = queue.shift();
  for (const child of children(current.help)) {
    const path = [...current.path, child.name];
    const help = run([...path, '--help']);
    const slug = slugFor(path);
    if (slugs.has(slug)) throw new Error(`CLI page slug collision: ${slug}`);
    slugs.add(slug);
    const item = { path, slug, short: child.short, help };
    commands.push(item);
    queue.push(item);
  }
}
if (!commands.length) throw new Error('Empty CLI inventory; help discovery did not happen.');
let changed = 0;
let obsolete = 0;
for (const locale of locales) {
  const { dir, url } = localePaths(config.outDir, locale, 'cli');
  const ru = locale === 'ru';
  for (const command of commands) {
    const title = `forgeplan ${command.path.join(' ')}`;
    const nested = commands.filter(item => item.path.length === command.path.length + 1
      && item.path.slice(0, -1).join(' ') === command.path.join(' '));
    const related = nested.length ? '\n' + nested.map(item =>
      `- [\`forgeplan ${item.path.join(' ')}\`](${url}/cli/${item.slug}/)`).join('\n') + '\n' : '';
    const block = managed(`## ${ru ? 'Снимок CLI help' : 'Captured CLI help'}\n\n`
      + `${ru ? 'Источник' : 'Source'}: \`${config.version}\`, \`${title} --help\`.\n\n`
      + '```text\n' + command.help.trimEnd() + '\n```\n' + related);
    changed += updatePage(join(dir, command.slug + '.md'), block, generated =>
      `---\ntitle: ${JSON.stringify(title)}\ndescription: ${JSON.stringify(command.short || title)}\n---\n\n`
      + `## ${ru ? 'Использование' : 'Usage'}\n\n`
      + (ru ? 'Точная сигнатура и параметры выбранной сборки приведены ниже. Описания сохранены дословно из --help и не являются независимой проверкой поведения или поддержки всех раскладок workspace.\n\n'
        : 'The exact signature and options of the selected build appear below. Descriptions are preserved literally from --help, not independently verified behavior or support for every workspace layout.\n\n')
      + generated + '\n\n'
      + `## ${ru ? 'Примеры' : 'Examples'}\n\n`
      + `- [${ru ? 'Воспроизводимый сценарий Constellation' : 'Reproducible Constellation walkthrough'}](${url}/guides/constellation/)\n`
      + `- [${ru ? 'Быстрый старт с артефактами' : 'Artifact quick start'}](${url}/getting-started/quick-start/)\n\n`
      + `## ${ru ? 'См. также' : 'See also'}\n\n`
      + `- [${ru ? 'Справочник CLI' : 'CLI reference'}](${url}/cli/)\n- [${ru ? 'Справочник Constellation' : 'Constellation reference'}](${url}/reference/constellation/)\n`, config.check) ? 1 : 0;
  }
  const top = commands.filter(command => command.path.length === 1);
  const overview = `---\ntitle: ${JSON.stringify(ru ? 'Справочник CLI' : 'CLI Reference')}\ndescription: ${JSON.stringify(ru ? 'Команды и вложенные команды выбранной сборки Forgeplan.' : 'Commands and nested commands in the selected Forgeplan build.')}\n---\n\n`
    + `${ru ? 'Снимок интерфейса' : 'Interface snapshot'}: \`${config.version}\`. **${top.length} ${ru ? 'корневых команд' : 'top-level commands'}; ${commands.length} ${ru ? 'путей команд всего' : 'command paths in total'}**.\n\n`
    + (ru ? 'Это инвентарь CLI help, не заявление о полной runtime-приёмке. Ограничения новой раскладки описаны в [Constellation reference]'
      : 'This is an inventory of CLI help, not a full runtime acceptance claim. New-layout limitations are documented in the [Constellation reference]')
    + `(${url}/reference/constellation/).\n\n`
    + [top, commands.filter(command => command.path.length > 1)].map((items, index) =>
      `## ${index === 0 ? (ru ? 'Корневые команды' : 'Top-level commands') : (ru ? 'Вложенные команды' : 'Nested commands')}\n\n`
      + `| ${ru ? 'Команда' : 'Command'} | ${ru ? 'Назначение' : 'Purpose'} |\n|---|---|\n`
      + items.map(command => `| [\`forgeplan ${command.path.join(' ')}\`](${url}/cli/${command.slug}/) | ${tableText(command.short)} |`).join('\n') + '\n').join('\n');
  changed += updateWholePage(join(dir, 'index.md'), overview, config.check) ? 1 : 0;
  if (existsSync(dir)) for (const file of readdirSync(dir)) {
    if (file.endsWith('.md') && file !== 'index.md' && !slugs.has(file.slice(0, -3))) {
      console.error(`[cli-gen] page without a registered command (review manually): ${join(dir, file)}`);
      obsolete++;
    }
  }
}
console.log(`[cli-gen] ${commands.length} command paths, 2 locales; ${changed} ${config.check ? 'missing/stale' : 'updated'} pages; ${obsolete} unmatched pages`);
if ((config.check && changed) || obsolete) process.exitCode = 1;
