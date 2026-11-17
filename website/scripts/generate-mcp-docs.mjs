#!/usr/bin/env node
// Generate MCP input tables and examples from a captured runtime tools/list.
// Usage/return sections outside those generated sections remain curated Markdown.
// Usage: node website/scripts/generate-mcp-docs.mjs --schema-file tools-list.json

import { readFileSync, writeFileSync, mkdirSync, existsSync, readdirSync, unlinkSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = resolve(__dirname, '../src/content/docs/docs/mcp');

const SCHEMA_ANNOTATIONS = new Set([
  '$schema', '$comment', 'title', 'description', 'default', 'examples',
  'deprecated', 'readOnly', 'writeOnly', 'format',
]);
const SCHEMA_KEYWORDS = new Set([
  ...SCHEMA_ANNOTATIONS, '$ref', '$defs', 'definitions', 'type', 'enum', 'const',
  'properties', 'required', 'items', 'anyOf', 'oneOf', 'minimum', 'maximum',
  'minLength', 'maxLength', 'minItems', 'maxItems', 'pattern',
]);

function isReleaseVersion(value) {
  if (typeof value !== 'string' || /[^0-9A-Za-z.+-]/.test(value)) return false;
  const match = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/.exec(value);
  return Boolean(match) && (!match[4] || match[4].split('.').every(part =>
    !/^\d+$/.test(part) || part === '0' || !part.startsWith('0')));
}

function escapeYaml(s) {
  return s.replace(/\\/g, '\\\\').replace(/"/g, '\\"').replace(/\r?\n/g, ' ');
}

function tableCell(text) {
  return String(text).replace(/\r?\n/g, ' ').replace(/\|/g, '\\|');
}

function readCapture(file) {
  const capture = JSON.parse(readFileSync(file, 'utf8'));
  if (!isReleaseVersion(capture?.source?.version)) {
    throw new Error('Expected source.version as SemVer (for example 0.38.0 or 0.39.0-rc.1).');
  }
  if (!Array.isArray(capture.tools) || capture.tools.length === 0) {
    throw new Error('Expected a non-empty tools array; existing docs were not changed.');
  }
  const names = new Set();
  for (const tool of capture.tools) {
    if (typeof tool?.name !== 'string' || tool.name.trim() !== tool.name ||
        !/^forgeplan_[a-z0-9_]+$/.test(tool.name) || names.has(tool.name)) {
      throw new Error(`Invalid or duplicate tool name: ${tool.name}`);
    }
    names.add(tool.name);
    if (typeof tool.description !== 'string' || !tool.description.trim()) {
      throw new Error(`Missing description for ${tool.name}`);
    }
    tool.schema = resolveSchema(tool.inputSchema, tool.inputSchema);
    if (tool.schema.type !== 'object') throw new Error(`${tool.name}: inputSchema must be an object schema.`);
    validateSchema(tool.schema);
  }
  return capture;
}

// Only the schema forms used by the MCP input contract are supported. New forms
// fail before output is touched instead of silently losing constraints.
function resolveSchema(schema, root, refs = []) {
  if (!schema || typeof schema !== 'object' || Array.isArray(schema)) {
    throw new Error('Expected a JSON Schema object.');
  }
  for (const keyword of Object.keys(schema)) {
    if (!SCHEMA_KEYWORDS.has(keyword)) throw new Error(`Unsupported input schema keyword: ${keyword}`);
  }
  let resolved = { ...schema };
  if (schema.$ref !== undefined) {
    const ref = schema.$ref;
    if (typeof ref !== 'string' || !ref.startsWith('#/') || refs.includes(ref)) {
      throw new Error(`Unsupported or cyclic schema reference: ${ref}`);
    }
    if (Object.keys(schema).some(key => key !== '$ref' && !SCHEMA_ANNOTATIONS.has(key))) {
      throw new Error('Only annotation siblings of $ref are supported.');
    }
    const target = ref.slice(2).split('/').reduce((node, part) => {
      const key = part.replace(/~1/g, '/').replace(/~0/g, '~');
      return node && Object.hasOwn(node, key) ? node[key] : undefined;
    }, root);
    const { $ref, ...siblings } = schema;
    resolved = { ...resolveSchema(target, root, [...refs, ref]), ...siblings };
  }
  if (resolved.anyOf && resolved.oneOf) throw new Error('Combining anyOf and oneOf is unsupported.');
  const combiner = resolved.anyOf ? 'anyOf' : resolved.oneOf ? 'oneOf' : null;
  if (combiner && Object.keys(resolved).some(key => key !== combiner && !SCHEMA_ANNOTATIONS.has(key))) {
    throw new Error('Union schemas must contain only branches and annotations.');
  }
  for (const keyword of ['$defs', 'definitions']) {
    if (resolved[keyword] !== undefined) {
      if (!resolved[keyword] || typeof resolved[keyword] !== 'object' || Array.isArray(resolved[keyword])) {
        throw new Error(`${keyword} must be an object.`);
      }
      resolved[keyword] = Object.fromEntries(Object.entries(resolved[keyword])
        .map(([name, definition]) => [name, resolveSchema(definition, root, refs)]));
    }
  }
  if (resolved.properties !== undefined) {
    if (!resolved.properties || typeof resolved.properties !== 'object' || Array.isArray(resolved.properties)) {
      throw new Error('Schema properties must be an object.');
    }
    resolved.properties = Object.fromEntries(Object.entries(resolved.properties)
      .map(([name, field]) => [name, resolveSchema(field, root, refs)]));
  }
  if (resolved.items !== undefined) resolved.items = resolveSchema(resolved.items, root, refs);
  for (const keyword of ['anyOf', 'oneOf']) {
    if (resolved[keyword] !== undefined) {
      if (!Array.isArray(resolved[keyword]) || !resolved[keyword].length) {
        throw new Error(`${keyword} must be a non-empty array.`);
      }
      resolved[keyword] = resolved[keyword].map(branch => resolveSchema(branch, root, refs));
    }
  }
  return resolved;
}

function schemaType(schema) {
  const branches = schema.anyOf || schema.oneOf;
  if (branches) return [...new Set(branches.map(schemaType))].join(' | ');
  const types = Array.isArray(schema.type) ? schema.type : [schema.type];
  const knownTypes = ['string', 'integer', 'number', 'boolean', 'array', 'object', 'null'];
  if (!types.length || types.some(type => !knownTypes.includes(type))) {
    throw new Error(`Unsupported or missing JSON type: ${JSON.stringify(schema.type)}`);
  }
  return types.map(type => type === 'array' ? `array<${schemaType(schema.items)}>` : type).join(' | ');
}

function validateSchema(schema) {
  schemaType(schema);
  if (schema.enum !== undefined && (!Array.isArray(schema.enum) || !schema.enum.length ||
      schema.enum.some(value => value !== null && typeof value === 'object'))) {
    throw new Error('Schema enum must be a non-empty array of primitive values.');
  }
  if (schema.const !== undefined && schema.const !== null && typeof schema.const === 'object') {
    throw new Error('Only primitive const values are supported.');
  }
  if (schema.required !== undefined && (!Array.isArray(schema.required) ||
      new Set(schema.required).size !== schema.required.length ||
      schema.required.some(name => typeof name !== 'string' || !Object.hasOwn(schema.properties || {}, name)))) {
    throw new Error('Every required field must be unique and exist in properties.');
  }
  for (const key of ['minimum', 'maximum']) {
    if (schema[key] !== undefined && !Number.isFinite(schema[key])) throw new Error(`${key} must be finite.`);
  }
  for (const key of ['minLength', 'maxLength', 'minItems', 'maxItems']) {
    if (schema[key] !== undefined && (!Number.isInteger(schema[key]) || schema[key] < 0)) {
      throw new Error(`${key} must be a non-negative integer.`);
    }
  }
  for (const [min, max] of [['minimum', 'maximum'], ['minLength', 'maxLength'], ['minItems', 'maxItems']]) {
    if (schema[min] !== undefined && schema[max] !== undefined && schema[min] > schema[max]) {
      throw new Error(`${min} must not exceed ${max}.`);
    }
  }
  if (schema.pattern !== undefined) {
    if (typeof schema.pattern !== 'string') throw new Error('pattern must be a string.');
    new RegExp(schema.pattern);
  }
  for (const keyword of ['$defs', 'definitions']) {
    for (const definition of Object.values(schema[keyword] || {})) validateSchema(definition);
  }
  for (const field of Object.values(schema.properties || {})) validateSchema(field);
  if (schema.items) validateSchema(schema.items);
  for (const branch of schema.anyOf || schema.oneOf || []) validateSchema(branch);
}

function schemaDetails(schema) {
  const notes = [];
  if (schema.description) notes.push(schema.description);
  if (Object.hasOwn(schema, 'default')) notes.push(`Schema default: \`${JSON.stringify(schema.default)}\`.`);
  if (schema.enum) notes.push(`Enum: ${schema.enum.map(value => `\`${JSON.stringify(value)}\``).join(', ')}.`);
  if (Object.hasOwn(schema, 'const')) notes.push(`Value: \`${JSON.stringify(schema.const)}\`.`);
  for (const key of ['format', 'minimum', 'maximum', 'minLength', 'maxLength', 'minItems', 'maxItems', 'pattern']) {
    if (schema[key] !== undefined) notes.push(`\`${key}\`: \`${schema[key]}\`.`);
  }
  for (const branch of schema.anyOf || schema.oneOf || []) {
    const details = schemaDetails(branch);
    if (details) notes.push(`${schemaType(branch)}: ${details}`);
  }
  return notes.join(' ');
}

function validExample(value, schema) {
  if (schema.anyOf && !schema.anyOf.some(branch => validExample(value, branch))) return false;
  if (schema.oneOf && schema.oneOf.filter(branch => validExample(value, branch)).length !== 1) return false;
  if (schema.enum && !schema.enum.some(item => JSON.stringify(item) === JSON.stringify(value))) return false;
  if (Object.hasOwn(schema, 'const') && JSON.stringify(value) !== JSON.stringify(schema.const)) return false;
  const types = schema.type === undefined ? [] : [].concat(schema.type);
  if (types.length && !types.some(type => {
    if (type === 'null') return value === null;
    if (type === 'array') return Array.isArray(value);
    if (type === 'object') return value !== null && typeof value === 'object' && !Array.isArray(value);
    if (type === 'integer') return Number.isInteger(value);
    return typeof value === type;
  })) return false;
  if (value === null) return true;
  if (typeof value === 'number' && ((schema.minimum !== undefined && value < schema.minimum) ||
      (schema.maximum !== undefined && value > schema.maximum))) return false;
  if (typeof value === 'string') {
    const length = Array.from(value).length;
    if ((schema.minLength !== undefined && length < schema.minLength) ||
        (schema.maxLength !== undefined && length > schema.maxLength) ||
        (schema.pattern !== undefined && !new RegExp(schema.pattern).test(value))) return false;
  }
  if (Array.isArray(value)) {
    return !(schema.minItems !== undefined && value.length < schema.minItems) &&
      !(schema.maxItems !== undefined && value.length > schema.maxItems) &&
      (!schema.items || value.every(item => validExample(item, schema.items)));
  }
  if (typeof value === 'object') {
    // Examples deliberately use declared fields even when additionalProperties
    // is allowed: an ignored field must never look like a supported argument.
    return (schema.required || []).every(name => Object.hasOwn(value, name)) &&
      Object.entries(value).every(([name, item]) => Object.hasOwn(schema.properties || {}, name) &&
        validExample(item, schema.properties[name]));
  }
  return true;
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
    forgeplan_init: { force: false },
    forgeplan_new: { kind: 'prd', title: 'Authentication system' },
    forgeplan_list: { kind: 'prd', status: 'active' },
    forgeplan_update: { id: 'PRD-001', title: 'Authentication and sessions' },
    forgeplan_delete: { id: 'NOTE-001' },
    forgeplan_deprecate: { id: 'PRD-001', reason: 'Feature cancelled; no replacement.' },
    forgeplan_supersede: { id: 'ADR-001', by: 'ADR-002' },
    forgeplan_capture: { decision: 'Use PostgreSQL for transactional storage.', context: 'The service needs multi-row transactions.' },
    forgeplan_link: { source: 'EVID-001', target: 'PRD-001', relation: 'informs' },
    forgeplan_unlink: { source: 'EVID-001', target: 'PRD-001', relation: 'informs' },
    forgeplan_search: { query: 'authentication flow', limit: 5 },
    forgeplan_route: { description: 'Add rate limiting to the API' },
    forgeplan_generate: { kind: 'prd', description: 'OAuth2 login flow' },
    forgeplan_journal: { kind: 'adr', risk: true },
    forgeplan_fpf_search: { query: 'trust calculus', limit: 5 },
    forgeplan_fpf_section: { id: 'B.3' },
    forgeplan_estimate: { id: 'PRD-001', grade: 'senior' },
    forgeplan_guard: { target_session_phase: 'coding' },
    forgeplan_claim: { id: 'RFC-001', agent: 'worker-1', ttl_minutes: 30 },
    forgeplan_release: { id: 'RFC-001', agent: 'worker-1' },
    forgeplan_dispatch: { agents: 2, kind: 'rfc', agent_skills: [['rust'], ['docs']] },
    forgeplan_phase_advance: { id: 'RFC-001', to: 'test', reason: 'Implementation ready for testing.' },
    forgeplan_coverage_business: { domain_model_id: 'DM-001' },
    forgeplan_hypothesis_promote: {
      hypothesis_id: 'HYP-001', new_state: 'parked', rationale: 'Awaiting an independent source.',
    },
    forgeplan_discover_start: { project_name: 'billing-service' },
    forgeplan_discover_complete: { session_id: 'disc-20261023-120000' },
    forgeplan_discover_finding: {
      session_id: 'disc-20261023-120000', phase: 'code', tier: 1,
      kind: 'note', title: 'Billing entry point', body: 'The billing module handles invoice creation.',
      source_files: ['src/billing.rs'],
    },
    forgeplan_ingest: { mapping: 'map.yaml', source: 'docs/components.md', dry_run: true },
    forgeplan_import: {
      data: JSON.stringify({
        version: 1,
        artifacts: [{
          id: 'NOTE-001', kind: 'note', status: 'draft', title: 'Storage decision',
          body: '## Decision\nUse PostgreSQL for transactional storage.',
        }],
        relations: [],
      }),
    },
    forgeplan_playbook_show: { target: 'brownfield-code' },
    forgeplan_playbook_validate: { file: '.forgeplan/playbooks/brownfield-code.yaml' },
    forgeplan_playbook_run: { target: 'brownfield-code', dry_run: true },
    forgeplan_plugins_info: { name: 'forgeplan' },
    forgeplan_interview_packet_draft: { domain: 'billing', seed_ids: ['HYP-001'] },
    forgeplan_interview_packet_ingest: { transcript_path: 'interviews/billing.md' },
  };
  const example = examples[tool.name] || Object.fromEntries((tool.schema.required || []).map(name => {
    if (name === 'id') return [name, 'PRD-001'];
    throw new Error(`${tool.name}: add an example for required field ${name}.`);
  }));
  if (!validExample(example, tool.schema)) throw new Error(`${tool.name}: example does not match inputSchema.`);
  return JSON.stringify(example, null, 2);
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

function inputSection(tool, version) {
  let body = '## Input parameters\n\n';
  const fields = Object.entries(tool.schema.properties || {});
  if (!fields.length) {
    body += '_No declared input fields. Call this tool with an empty object `{}`._\n\n';
  } else {
    body += '| Name | JSON type | Required | Description / constraints |\n|---|---|---|---|\n';
    for (const [name, schema] of fields) {
      const required = (tool.schema.required || []).includes(name) ? 'yes' : 'no';
      body += `| \`${name}\` | \`${tableCell(schemaType(schema))}\` | ${required} | ${tableCell(schemaDetails(schema)) || '-'} |\n`;
    }
    body += '\nRequired means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.\n\n';
  }
  body += `_Schema source: MCP \`tools/list\`, Forgeplan ${version}, \`inputSchema\`._\n\n`;
  return body;
}

function replaceSection(body, heading, replacement) {
  const pattern = new RegExp(`^## ${heading}\\r?\\n[\\s\\S]*?(?=^## |$(?![\\s\\S]))`, 'm');
  if (pattern.test(body)) return body.replace(pattern, () => replacement);
  const firstHeading = body.search(/^## /m);
  if (firstHeading < 0) return `${body.trimEnd()}\n\n${replacement}`;
  return body.slice(0, firstHeading) + replacement + body.slice(firstHeading);
}

function pageFor(tool, version, existing) {
  const frontmatter = existing.match(/^---\r?\n[\s\S]*?\r?\n---(?:\r?\n|$)/)?.[0];
  // Authored SEO descriptions and other frontmatter keys are curated too.
  // Runtime metadata is only a fallback for a new page and needs source review.
  const header = frontmatter ||
    `---\ntitle: ${tool.name}\ndescription: "${escapeYaml(tool.description)}"\n---\n\n`;
  let body = frontmatter ? existing.slice(frontmatter.length) : existing ||
    `${stripEmDashes(tool.description)}\n\n**Category**: ${categoryFor(tool.name)}\n\n` +
    '## Usage from an AI agent\n\nConnect your MCP client to `forgeplan serve` over stdio.\n\n' +
    '## See also\n\n- [MCP overview](/docs/mcp/)\n- [`forgeplan serve`](/docs/cli/serve/)\n';
  body = replaceSection(body, 'Input parameters', stripEmDashes(inputSection(tool, version)));
  const example = '## Example invocation\n\n' +
    'Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.\n\n' +
    `\`\`\`json\n${exampleFor(tool)}\n\`\`\`\n\n`;
  body = replaceSection(body, 'Example invocation', stripEmDashes(example));
  return header + body;
}

function overviewFor(tools, version) {
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
  body += `## Contract source\n\nThis reference uses the **Forgeplan ${version}** runtime \`tools/list\` response, not Rust handler signatures. Each tool's \`inputSchema\` supplies the JSON keys, types and required fields. Fields such as \`workspace\` are optional where listed and are not accepted by every tool. Start the server in the intended project directory when a tool has no workspace argument.\n\n`;
  body += 'Tool descriptions below are runtime metadata, not proof of execution. In 0.38.0 both `forgeplan_interview_packet_*` handlers remain `not_implemented` even after installing the marketplace pack. `forgeplan_discover_complete` returns a session summary and asks the caller to run health separately. See the curated tool pages for these implementation limits.\n\n';
  body += 'Successful tool payloads are usually JSON encoded in MCP text content, not a universal `ok: true` envelope. Handler refusals use MCP error results (`isError: true`) with text; invalid parameters can instead be JSON-RPC errors. Workspace trace fields are returned only by handlers that add them.\n\n';
  body += '`_next_action` and other hints are advisory text, not commands to execute automatically or consent to destructive operations. Some 0.38.0 hints mix CLI syntax with MCP names or mention unregistered tools. Use the registered tool name and JSON arguments in this reference, or check the release CLI help; do not automatically force a transition, overwrite data, or delete a conflicting artifact because a hint suggests it.\n\n';
  body += '### Capture tools/list without API keys\n\nListing tool schemas does not invoke an LLM or require provider credentials. With the release binary on PATH, this Python standard-library example starts a server in a temporary directory, performs MCP initialization, captures the tool list and stops the child process. It does not call any workspace or generation tool.\n\n';
  body += `\`\`\`python
import json
import subprocess
import tempfile
from pathlib import Path

output = Path("tools-list.json").resolve()
with tempfile.TemporaryDirectory() as directory:
    server = subprocess.Popen(
        ["forgeplan", "serve"], cwd=directory,
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
    )
    def send(message):
        server.stdin.write(json.dumps(message) + "\\n")
        server.stdin.flush()

    def receive(request_id):
        for line in server.stdout:
            message = json.loads(line)
            if message.get("id") == request_id:
                if "error" in message:
                    raise RuntimeError(message["error"])
                return message["result"]
        raise RuntimeError("MCP server closed before replying")

    try:
        send({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2024-11-05", "capabilities": {},
            "clientInfo": {"name": "docs-snapshot", "version": "1.0"},
        }})
        initialized = receive(1)
        send({"jsonrpc": "2.0", "method": "notifications/initialized"})
        tools = []
        cursor = None
        request_id = 2
        while True:
            send({"jsonrpc": "2.0", "id": request_id, "method": "tools/list",
                  "params": {} if cursor is None else {"cursor": cursor}})
            listing = receive(request_id)
            request_id += 1
            tools.extend(listing["tools"])
            cursor = listing.get("nextCursor")
            if not cursor:
                break
        capture = {"source": {"version": initialized["serverInfo"]["version"],
                              "initialize": initialized}, "tools": tools}
        output.write_text(json.dumps(capture, indent=2) + "\\n", encoding="utf-8")
    finally:
        server.terminate()
        server.wait()
\`\`\`

Regenerate the English contract sections from the repository root:

\`\`\`bash
node website/scripts/generate-mcp-docs.mjs --schema-file tools-list.json
\`\`\`

The input is \`{ "source": { "version": "${version}" }, "tools": [...] }\`. The version must be SemVer, including optional prerelease/build identifiers (for example \`0.39.0-rc.1\`). Invalid/empty captures and unsupported schema keywords are rejected before any output changes. The generator supports the captured input-schema subset, not arbitrary JSON Schema: unknown assertions (including \`minProperties\`, \`dependentRequired\` and \`exclusiveMinimum\`) fail closed; \`format\` is an annotation, not an extra validation rule.

By default, a capture that omits an existing tool page is rejected as potentially incomplete. Only for a deliberately removed tool, use \`--schema-file tools-list.json --prune\` to permit deletion of omitted English pages. An empty capture is rejected even with \`--prune\`. Unchanged pages are not rewritten.

Input tables and primary invocation examples are regenerated; authored frontmatter, curated usage, return descriptions and see-also links are retained. New-page titles/descriptions fall back to runtime metadata and need source review. Review the curated sections and the Russian translation when updating the capture. Generation is not verification of tool behavior.

## Tool catalogue

`;
  for (const g of order) {
    const items = groups[g];
    if (!items || !items.length) continue;
    body += `### ${g}\n\n`;
    body += `| Tool | Description |\n|---|---|\n`;
    items.sort((a, b) => a.name.localeCompare(b.name));
    for (const t of items) {
      body += `| [\`${t.name}\`](/docs/mcp/${t.name}/) | ${tableCell(t.description)} |\n`;
    }
    body += `\n`;
  }
  return stripEmDashes(body);
}

function main() {
  const args = process.argv.slice(2);
  const usage = 'Usage: node website/scripts/generate-mcp-docs.mjs --schema-file tools-list.json [--prune]';
  if (args.length === 1 && args[0] === '--help') {
    console.log(usage);
    return;
  }
  if ((args.length !== 2 && args.length !== 3) || args[0] !== '--schema-file' ||
      !args[1] || args[1].startsWith('--') || (args.length === 3 && args[2] !== '--prune')) {
    throw new Error(usage);
  }
  const { tools, source } = readCapture(resolve(args[1]));
  const prune = args[2] === '--prune';
  const names = new Set(tools.map(tool => `${tool.name}.md`));
  const existingFiles = existsSync(OUT_DIR) ? readdirSync(OUT_DIR) : [];
  const omitted = existingFiles.filter(filename => /^forgeplan_[a-z0-9_]+\.md$/.test(filename) && !names.has(filename));
  if (omitted.length && !prune) {
    throw new Error(`Capture omits existing tool pages: ${omitted.join(', ')}. Capture the complete tools/list; use --prune only for intentional removals.`);
  }
  // Parse, resolve and render every schema/example before overwriting or removing
  // any page. A broken capture must not erase a previously useful reference.
  const pages = new Map();
  for (const tool of tools) {
    const filename = `${tool.name}.md`;
    const path = resolve(OUT_DIR, filename);
    const existing = existsSync(path) ? readFileSync(path, 'utf8') : '';
    pages.set(filename, { existing, content: pageFor(tool, source.version, existing) });
  }
  const indexPath = resolve(OUT_DIR, 'index.md');
  pages.set('index.md', {
    existing: existsSync(indexPath) ? readFileSync(indexPath, 'utf8') : '',
    content: overviewFor(tools, source.version),
  });
  mkdirSync(OUT_DIR, { recursive: true });
  let written = 0;
  for (const [filename, { existing, content }] of pages) {
    if (existing !== content) {
      writeFileSync(resolve(OUT_DIR, filename), content);
      written += 1;
    }
  }
  for (const filename of omitted) unlinkSync(resolve(OUT_DIR, filename));
  console.log(`[mcp-gen] rendered ${pages.size} files, wrote ${written}, removed ${omitted.length} from Forgeplan ${source.version} tools/list to ${OUT_DIR}`);
}

try {
  main();
} catch (error) {
  console.error(`[mcp-gen] ${error.message}`);
  process.exitCode = 1;
}
