#!/usr/bin/env node
// Use live tools/list JSON Schema for both profiles, never Rust-wrapper heuristics.
import { readdirSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { options, locales, localePaths, managed, tableText, toolCatalog, updatePage, updateWholePage } from './reference-runtime.mjs';

const config = options();
const profiles = [];
// Metadata discovery is isolated. No tool call, model download or real index access.
for (const name of ['full', 'constellation-read']) profiles.push({ name, tools: await toolCatalog(config.binary, name) });
const allNames = new Set();
for (const profile of profiles) for (const tool of profile.tools) {
  if (allNames.has(tool.name)) throw new Error(`Tool page collision between profiles: ${tool.name}`);
  allNames.add(tool.name);
}
function typeName(schema) {
  if (schema.$ref) return schema.$ref.split('/').at(-1);
  if (schema.anyOf) return schema.anyOf.map(typeName).join(' | ');
  if (schema.oneOf) return schema.oneOf.map(typeName).join(' | ');
  if (schema.enum) return schema.enum.map(value => JSON.stringify(value)).join(' | ');
  if (Array.isArray(schema.type)) return schema.type.join(' | ');
  return schema.type ?? 'object';
}
let changed = 0;
let obsolete = 0;
for (const locale of locales) {
  const { dir, url } = localePaths(config.outDir, locale, 'mcp');
  const ru = locale === 'ru';
  for (const profile of profiles) for (const tool of profile.tools) {
    const schema = tool.inputSchema;
    const required = new Set(schema.required ?? []);
    const rows = Object.entries(schema.properties ?? {}).map(([name, field]) =>
      `| \`${name}\` | ${required.has(name) ? (ru ? 'да' : 'yes') : (ru ? 'нет' : 'no')} | ${tableText(typeName(field))} | ${tableText(field.description)} |`).join('\n');
    const block = managed(`## ${ru ? 'Снимок tools/list' : 'Captured tools/list'}\n\n`
      + `${ru ? 'Источник' : 'Source'}: \`${config.version}\`, \`tools/list\`, ${ru ? 'профиль' : 'profile'} \`${profile.name}\`.\n\n`
      + (tool.description ? tool.description + '\n\n' : '')
      + `### ${ru ? 'Входные параметры' : 'Input parameters'}\n\n`
      + `| ${ru ? 'Имя' : 'Name'} | ${ru ? 'Обязателен' : 'Required'} | ${ru ? 'Тип' : 'Type'} | ${ru ? 'Описание' : 'Description'} |\n|---|---|---|---|\n`
      + (rows || `| — | — | — | ${ru ? 'Параметров нет.' : 'No parameters.'} |`) + '\n\n'
      + `<details>\n<summary>JSON Schema</summary>\n\n\`\`\`json\n${JSON.stringify(schema, null, 2)}\n\`\`\`\n\n</details>\n`);
    changed += updatePage(join(dir, tool.name + '.md'), block, generated =>
      `---\ntitle: ${tool.name}\ndescription: ${JSON.stringify(ru ? `${tool.name} — профиль MCP ${profile.name}` : `${tool.name} — ${profile.name} MCP profile`)}\n---\n\n`
      + `## ${ru ? 'Входные параметры' : 'Input parameters'}\n\n`
      + (ru ? 'Таблица и JSON Schema ниже получены через tools/list выбранной сборки. Описания сохранены дословно как метаданные интерфейса, а не независимая проверка поведения. Каталог не подтверждает доступность всех операций на любой раскладке или без требуемого плагина.\n\n'
        : 'The table and JSON Schema below come from tools/list in the selected build. Descriptions are preserved literally as interface metadata, not independently verified behavior. The catalog does not establish availability on every layout or without a required plugin.\n\n')
      + generated + '\n\n'
      + `## ${ru ? 'Примеры' : 'Examples'}\n\n`
      + `- [${ru ? 'Настройка MCP и область workspace' : 'MCP setup and workspace scope'}](${url}/guides/mcp-setup/)\n`
      + `- [${ru ? 'Сценарий Constellation и последовательность handles' : 'Constellation walkthrough and handle sequence'}](${url}/guides/constellation/)\n\n`
      + `## ${ru ? 'См. также' : 'See also'}\n\n`
      + `- [${ru ? 'Справочник MCP' : 'MCP reference'}](${url}/mcp/)\n- [${ru ? 'Справочник Constellation' : 'Constellation reference'}](${url}/reference/constellation/)\n`, config.check) ? 1 : 0;
  }
  const overview = `---\ntitle: ${JSON.stringify(ru ? 'Инструменты MCP' : 'MCP Tools')}\ndescription: ${JSON.stringify(ru ? 'Раздельные каталоги full и constellation-read из живого сервера.' : 'Separate full and constellation-read catalogs from the live server.')}\n---\n\n`
    + `${ru ? 'Снимок интерфейса' : 'Interface snapshot'}: \`${config.version}\`.\n\n`
    + (ru ? '**full сохраняет запись.** Дополнительный профиль constellation-read её не заменяет. Числа ниже — зарегистрированные инструменты, не число проверенных end-to-end сценариев. Некоторые инструменты требуют плагин.\n\n'
      : '**full retains write tools.** The additional constellation-read profile does not replace it. Counts below describe registered tools, not accepted end-to-end scenarios. Some tools require a plugin.\n\n')
    + `- [${ru ? 'Настройка подключения' : 'Connection setup'}](${url}/guides/mcp-setup/)\n`
    + `- [${ru ? 'Практический сценарий Constellation' : 'Constellation how-to'}](${url}/guides/constellation/)\n`
    + `- [${ru ? 'Контракты, ограничения и диагностика' : 'Contracts, limitations and diagnostics'}](${url}/reference/constellation/)\n\n`
    + profiles.map(profile => `## \`${profile.name}\` — ${profile.tools.length} ${ru ? 'инструментов' : 'tools'}\n\n`
      + '```bash\n' + `forgeplan serve --profile ${profile.name}\n` + '```\n\n'
      + `| ${ru ? 'Инструмент' : 'Tool'} | ${ru ? 'Назначение из tools/list' : 'Purpose from tools/list'} |\n|---|---|\n`
      + [...profile.tools].sort((a, b) => a.name.localeCompare(b.name)).map(tool =>
        `| [\`${tool.name}\`](${url}/mcp/${tool.name}/) | ${tableText(tool.description)} |`).join('\n') + '\n').join('\n');
  changed += updateWholePage(join(dir, 'index.md'), overview, config.check) ? 1 : 0;
  if (existsSync(dir)) for (const file of readdirSync(dir)) {
    if (file.endsWith('.md') && file !== 'index.md' && !allNames.has(file.slice(0, -3))) {
      console.error(`[mcp-gen] page without a registered tool (review manually): ${join(dir, file)}`);
      obsolete++;
    }
  }
}
console.log(`[mcp-gen] ${profiles.map(profile => `${profile.name}=${profile.tools.length}`).join(', ')}, 2 locales; ${changed} ${config.check ? 'missing/stale' : 'updated'} pages; ${obsolete} unmatched pages`);
if ((config.check && changed) || obsolete) process.exitCode = 1;
