#!/usr/bin/env node
// Generate Starlight marketplace pages from the ForgePlan marketplace repo.
//
// Source of truth: <MARKETPLACE_DIR>/.claude-plugin/marketplace.json (+ per-plugin
// commands/agents/skills directories). An explicit source directory is required.
// Regeneration updates English pages only; Russian pages need a separate review.
//
// Usage: MARKETPLACE_DIR=/path/to/published-marketplace node scripts/generate-marketplace-docs.mjs
//
// Idempotent: a second run with no source change rewrites identical bytes.

import { readFileSync, writeFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve, join } from 'node:path';
import { createHash } from 'node:crypto';

const __dirname = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = resolve(__dirname, '../src/content/docs/docs/marketplace');
const OVERVIEW = resolve(OUT_DIR, 'overview.md');
const MARKETPLACE_DIR = process.env.MARKETPLACE_DIR;

const MARKER_START = '<!-- plugins:start -->';
const MARKER_END = '<!-- plugins:end -->';
const PROFILES_START = '<!-- profiles:start -->';
const PROFILES_END = '<!-- profiles:end -->';
const SNAPSHOT_START = '<!-- snapshot:start -->';
const SNAPSHOT_END = '<!-- snapshot:end -->';

// Curated corrections to optimistic metadata, grounded in the published scripts.
// Keep these local to documentation; do not rewrite the external plugin metadata.
const PLUGIN_DESCRIPTIONS = {
  'agents-tdd': 'Test-driven development pipeline: an orchestrator coordinates planning, failing tests, independent RED validation and implementation in separate contexts. During an initialized active cycle, its hook restricts recognized writes to classified source/test paths; it is not a filesystem sandbox. Use when an active spec carries concrete scenarios and tests should be the implementation oracle.',
  'agents-bmad': 'BMAD greenfield methodology: an orchestrator coordinates Analyst, PM, Architect, Scrum-Master, Dev and QA in separate contexts with independent reviews. The no-code-before-plan hook applies to recognized source/test writes during an initialized active cycle, with explicit escape paths. Use for a new product or service, not a single feature in an active system or legacy modernization.',
  'agents-canvas': 'CANVAS design-system-to-code methodology: a coordinator takes a Pencil design through Capture, Audit, Norm-check, Vectorize and Assemble using separate generator and verifier contexts. Its write hook checks configured guarded paths and a local tokens_active flag during an initialized active cycle; it does not itself verify RFC activation. Use to build native components in the consuming project’s framework with token fidelity and visual checks.',
  'agents-sparc': 'SPARC for a single feature in an existing active system: Specification, optional Pseudocode, Architecture, Refinement and Completion. Three mandatory independent gates certify Specification, Architecture and Refinement. Pseudocode, when used, is certified at Architecture rather than at a separate blocking gate; MICRO scope skips it. Source dispatch waits for the Architecture RFC to be active, but SPARC supplies no write-time hook.',
  'dev-toolkit': '[DEPRECATED - superseded by fpl-skills] Universal engineering toolkit with multi-expert audit, wave-based sprints, session-context recall, structured reports, reminder hooks and safety hooks. Reminder scripts print suggestions; they do not automatically dispatch the advisor or run an audit. Existing users can retain it; new users should choose fpl-skills.',
};

const COMPONENT_DESCRIPTIONS = {
  'laws-of-ux/agents/ux-reviewer': 'On-demand frontend reviewer applying 30 Laws of UX to HTML/CSS/React/Vue and reporting violations, warnings and improvement examples. The supplied ux-hint.sh hook prints a fixed UX reminder; it does not launch this reviewer or produce code-specific findings.',
  'fpf/agents/fpf-advisor': 'On-demand reasoning advisor for architecture, planning and decisions, recommending /fpf-decompose, /fpf-evaluate or /fpf-reason. Despite the hook-triggered wording in upstream metadata, this plugin registers no hooks and does not automatically launch the advisor.',
  'fpl-skills/agents/dev-advisor': 'Development advisor for audit, testing, security and specialist-agent recommendations when invoked. The supplied reminder/router scripts print context or suggestions; they do not dispatch this advisor or run the suggested review.',
  'dev-toolkit/agents/dev-advisor': 'Development advisor for audit, testing, security and specialist-agent recommendations when invoked. The supplied test/report hint scripts print reminders; they do not dispatch this advisor or run an audit.',
  'forgeplan-workflow/agents/forge-advisor': 'Workflow advisor for routing, evidence, health checks and ADR capture when invoked. The supplied pre-code check only warns about a missing active PRD; it neither launches this advisor nor blocks that write. Separate safety-hook behavior should not be confused with an automatic methodology review.',
  'forgeplan-orchestra/agents/orchestra-advisor': 'Advisor for corresponding Orchestra task updates when invoked. The supplied forge-sync-hint.sh script only prints a suggestion to run /sync after recognized Forgeplan activity; it does not dispatch the advisor or synchronize tasks itself.',
  'slopocop/agents/slop-cop': 'On-demand reviewer of Russian and English prose for AI-writing patterns, with prioritized findings and a cleanliness score; the Russian workflow uses the humanizer-ru scanner. It flags rather than silently rewrites. PostToolUse may suggest /slop-audit or /slop-humanize; it does not automatically launch this reviewer.',
  'agents-sparc/agents/sparc-orchestrator': 'Coordinates one feature in an active codebase using separate phase and verifier contexts. Mandatory independent gates are at Specification, Architecture and Refinement. Optional Pseudocode is absorbed and certified at Architecture, not separately gated; MICRO scope omits it. Coordinates implementation and Completion evidence, but does not write source or activate artifacts. SPARC has no write-time hook: phase ordering binds dispatches, not out-of-band edits.',
  'agents-sparc/skills/sparc': 'Run one feature through Specification, optional Pseudocode, Architecture, Refinement and Completion. Independent gates are mandatory at Specification, Architecture and Refinement; Pseudocode is certified at Architecture and skipped for MICRO scope. No SPARC write-time hook is installed; Refinement may delegate to TDD. Not for a new product or legacy modernization.',
  'agents-tdd/skills/tdd': 'Launch the TDD orchestrator for an active PRD/SPEC with scenarios: plan tests, produce RED, independently validate it, then implement GREEN. The initialized branch-state hook constrains recognized writes to classified targets; frozen-test protection is subject to those same tool/path limits, not filesystem-wide immutability.',
  'agents-bmad/skills/bmad': 'Launch the BMAD greenfield persona pipeline with separate producer/reviewer contexts. Its initialized branch-state hook restricts recognized source/test writes before readiness, with done-state, override, scratch-path and Bash target-extraction exemptions described above.',
  'agents-canvas/skills/canvas': 'Launch the Pencil-to-native-code design-system pipeline with independent design, requirement, token-contract and implementation checks. During an initialized active cycle, its Write/Edit/MultiEdit hook blocks guarded paths while the local tokens_active flag is false. The hook does not certify RFC lifecycle status or intercept Bash/external writes.',
};

const RUNTIME_NOTES = {
  'fpl-skills': '**Operational prerequisites:** `/smith` dispatches `agents-pro:smith`, so the `agents-pro` plugin must be available. `/smith-bootstrap` halts unless all five required plugins are enabled: `fpl-skills`, `agents-pro`, `agents-sparc`, `agents-core` and `forgeplan-workflow`. It recommends `fpl-hsmem` and `fpf` separately. These are workflow prerequisites, not dependencies automatically installed by `/plugin install fpl-skills`. Session, prompt and test/report hints do not automatically dispatch `dev-advisor` or run suggested reviews.',
  'laws-of-ux': 'The supplied frontend hook prints a fixed UX reminder. It does not dispatch `ux-reviewer`, inspect the code for violations or automatically run a review; invoke the reviewer or editorial command separately.',
  'fpf': 'No `hooks/hooks.json` is supplied. The advisor is available when invoked, not automatically launched by the hook described in its upstream metadata.',
  'dev-toolkit': 'Test/report reminder hooks print suggestions; they do not launch `dev-advisor`, an audit or an independent review. The safety hook is a separate mechanism.',
  'forgeplan-workflow': 'The pre-code script warns about a missing active PRD and exits successfully; it does not block that write or launch `forge-advisor`. The separate safety script is not an automatic methodology review. `/forge-audit` requires Evidence emission and claim release under its operative procedure; the older optional-evidence introduction in the source command is stale.',
  'forgeplan-orchestra': 'The supplied sync hint prints a suggestion to run `/sync`; it neither dispatches `orchestra-advisor` nor creates or updates tracker tasks automatically.',
  'slopocop': 'The prose PostToolUse hook only suggests `/slop-audit` or `/slop-humanize` for matching input and skips conventional boilerplate. It does not dispatch `slop-cop`. Session/prompt communication-layer loading is separate from editorial review.',
  'agents-tdd': 'Enforcement applies only to recognized tool calls during an initialized active branch cycle and to targets classified by the configured source/test patterns. Missing state, a non-repository or detached branch, and `phase: done` allow writes. Unrecognized Bash write syntax or an unresolved target is allowed; only the first extracted Bash target is classified. RED permits source stubs carrying `STUB:TDD`; GREEN blocks classified test writes. A SPEC hash check runs only when a frozen hash is present, and an absent SPEC file is not blocked. This is not a filesystem sandbox and does not intercept arbitrary external writes.',
  'agents-bmad': 'The hook covers recognized `Write`, `Edit`, `MultiEdit` and Bash targets classified by configured source/test patterns during an initialized active branch cycle. Missing state, a non-repository or detached branch, `phase: done`, an explicit state/environment override, and `.bmad-scratch/` paths allow writes. Unrecognized Bash writes or unresolved targets are allowed; only the first extracted target is classified. Other file kinds are allowed. Readiness enforcement is not a filesystem sandbox or a block on arbitrary human/external writes.',
  'agents-canvas': 'The hook matches only `Write`, `Edit` and `MultiEdit`, not Bash or external writes. In an initialized active branch cycle, it guards configured paths while the local `tokens_active` flag is false. Missing state, a non-repository or detached branch, `phase: done`, an explicit override, `.canvas-scratch/`, unguarded paths and missing target paths allow writes. `tokens_active: true` unlocks writes; the script does not query the tokens RFC lifecycle. After independent Gate V PASS, the coordinator requests activation from the main-session orchestrator and records the local flag only after that orchestrator activates the tokens RFC. This is not a filesystem sandbox.',
  'agents-sparc': 'The three mandatory independent reviews are Specification, Architecture and Refinement. Optional Pseudocode receives its absorption check at Architecture, with no separate Pseudocode gate; MICRO scope skips it. SPARC has no hook or state-file write barrier. Phase ordering delays source/test dispatch until the Architecture RFC is active; human or out-of-band edits are reviewed later rather than blocked at write time.',
};

function fail(msg) {
  console.error(`[mkt-gen] ${msg}`);
  process.exit(1);
}

function unquote(s) {
  const t = s.trim();
  if (t.startsWith('"') && t.endsWith('"')) {
    return t.slice(1, -1).replace(/\\"/g, '"');
  }
  if (t.startsWith("'") && t.endsWith("'")) {
    return t.slice(1, -1).replace(/''/g, "'");
  }
  return t;
}

// Minimal frontmatter reader for descriptions and skill invocation flags.
function parseFrontmatter(text) {
  const m = text.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  if (!m) return {};
  const lines = m[1].split('\n');
  const out = {};
  for (let i = 0; i < lines.length; i++) {
    const nameMatch = lines[i].match(/^name:\s*(.*)$/);
    const flagMatch = lines[i].match(/^(user-invocable|disable-model-invocation):\s*(true|false)\s*$/);
    if (flagMatch) {
      out[flagMatch[1]] = flagMatch[2] === 'true';
      continue;
    }
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

// Keep the English preamble as well as the EN half: safety limits can precede EN:.
// Translate the shared Russian release note without changing upstream metadata.
function englishOnly(desc) {
  if (!desc) return '';
  const translated = desc.replace(
    /Каждый агент пакета получил секцию `## Model tier`:[\s\S]*$/,
    'Each agent now has a `## Model tier` section explaining the tier its work needs, set by the hardest stage rather than the most frequent one. The frontmatter `model:` value is a Claude Code binding, not a requirement: OMP, OpenCode, Codex and Gemini do not use those model names. Configure a model at the required tier; if you must miss, miss upward. The tier ladder is in `docs/GUIDE-AI-SDLC-PDLC-RU.md` section 6.2 and is not repeated here.'
  );
  return translated.split(/\s+RU:/)[0].split(/\s+Triggers:/)[0].replace(/(^|\s+)EN:\s*/g, '$1').trim();
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

function listDirFiles(dir, ext = '.md', recursive = false) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { withFileTypes: true })
    .flatMap((entry) => {
      if (recursive && entry.isDirectory()) {
        return listDirFiles(join(dir, entry.name), ext, true).map((name) => `${entry.name}/${name}`);
      }
      return entry.isFile() && entry.name.endsWith(ext) ? [entry.name.slice(0, -ext.length)] : [];
    })
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
  if (!req || Object.keys(req).length === 0) return 'No additional requirements declared in the catalog';
  const structured = ['cli', 'mcp', 'plugins'].some((k) => Array.isArray(req[k]));
  if (structured) {
    const parts = [];
    for (const [label, key] of [['CLI', 'cli'], ['MCP', 'mcp'], ['Plugins', 'plugins']]) {
      const items = normEntries(req[key]).map((e) => {
        let s = e.name + (e.version ? ` ${e.version}` : '');
        if (e.optional) s += ' (optional)';
        if (e.reason) s += ` (${e.reason})`;
        return s;
      });
      if (items.length) parts.push(`${label}: ${items.join(', ')}`);
    }
    return parts.length ? parts.join('; ') : 'No additional requirements declared in the catalog';
  }
  const parts = Object.entries(req).map(([k, v]) =>
    typeof v === 'string' ? v : `${k}: ${JSON.stringify(v)}`
  );
  return parts.join(' ') || 'No additional requirements declared in the catalog';
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
  const fullDesc = englishOnly(PLUGIN_DESCRIPTIONS[plugin.name] || plugin.description || '').replace(/\s+/g, ' ').trim();
  const shortDesc = firstSentence(fullDesc);
  const pdir = join(root, plugin.source || `./plugins/${plugin.name}`);
  const commands = listDirFiles(join(pdir, 'commands'));
  const agents = listDirFiles(join(pdir, 'agents'), '.md', true);
  const skills = listSkillDirs(join(pdir, 'skills'));

  let body = `---\ntitle: ${plugin.name}\ndescription: "${yamlEscape(shortDesc)}"\n---\n\n`;
  body += `${PLUGIN_DESCRIPTIONS[plugin.name] ? '**Runtime summary:** ' : '**Catalog description:** '}${fullDesc}\n\n`;
  body += `**Cost**: ${plugin.cost || 'n/a'} | **Stability**: ${plugin.stability || 'n/a'} | **Version**: ${plugin.version || 'n/a'} | **Category**: ${plugin.category || 'n/a'}\n\n`;
  body += `**Targets**: ${(plugin.targets || []).join(', ') || 'n/a'}\n\n`;
  body += `**Requires**: ${renderRequires(plugin.requires)}\n\n`;
  body += `**Packaged surface**: ${commands.length} command files, ${agents.length} agent files, ${skills.length} skills\n\n`;
  if (plugin.name === 'forgeplan-brownfield-pack') {
    body += `**Core 0.38.0 limitation:** \`forgeplan_interview_packet_draft\` and \`forgeplan_interview_packet_ingest\` are stub handlers: after workspace/path checks they return \`status: not_implemented\`. Installing this pack does not enable their implementation. Use the \`interview-packager\` skill and a manual interview workflow instead.\n\n`;
  }
  if (plugin.deprecated) {
    body += `**Superseded by**: ${plugin.supersededBy || 'n/a'}\n\n`;
  }
  if (RUNTIME_NOTES[plugin.name]) {
    body += `## Runtime boundaries\n\n${RUNTIME_NOTES[plugin.name]}\n\n`;
  }

  if (commands.length) {
    body += `## Commands\n\nDescriptions below are packaged metadata, with runtime qualifications noted on this page where applicable.\n\n| Command | What it does |\n|---|---|\n`;
    for (const c of commands) {
      const desc = englishOnly(readDescription(join(pdir, 'commands', `${c}.md`)));
      body += `| \`/${c}\` | ${cell(desc)} |\n`;
    }
    body += `\n`;
  }

  if (agents.length) {
    body += `## Agents\n\n`;
    body += `Claude Code scoped names are shown below. Its [agent loader](https://code.claude.com/docs/en/plugins/components#agents) scans \`agents/\` recursively, keeps subfolder segments and applies frontmatter \`name\` to the final segment. Other runtimes may use different names. Descriptions are packaged metadata except for curated corrections to documented runtime behavior; runtime boundaries, where provided, take precedence over optimistic metadata wording.\n\n`;
    body += `| Agent | Source path | What it does |\n|---|---|---|\n`;
    for (const a of agents) {
      const meta = parseFrontmatter(readFileSync(join(pdir, 'agents', `${a}.md`), 'utf8'));
      const parts = a.split('/');
      parts[parts.length - 1] = meta.name || parts[parts.length - 1];
      const scopedName = [plugin.name, ...parts].join(':');
      const desc = COMPONENT_DESCRIPTIONS[`${plugin.name}/agents/${a}`] || englishOnly(meta.description || '');
      body += `| \`${scopedName}\` | \`agents/${a}.md\` | ${cell(desc)} |\n`;
    }
    body += `\n`;
  }

  if (skills.length) {
    body += `## Skills\n\n`;
    body += `Skills are listed separately from command files. In Claude Code, user-invocable skills can also be slash entry points; invocation flags below are taken from their frontmatter. Descriptions are packaged metadata with curated runtime corrections where needed; they are not independent guarantees of enforcement.\n\n`;
    for (const s of skills) {
      const meta = parseFrontmatter(readFileSync(join(pdir, 'skills', s, 'SKILL.md'), 'utf8'));
      // This skill's source description is Russian-only; keep the English page readable.
      const desc = plugin.name === 'slopocop' && s === 'humanizer-ru'
        ? 'Humanize Russian-language text: remove AI-generation patterns, bureaucratic wording, padding and excessive formality, or rewrite text that sounds robotic or artificial. Use whenever the user asks for these edits, including requests such as "rewrite" or "make it better" on supplied Russian prose. Russian only; use the English humanizer for English. Not for translation, writing from scratch, grammar-only edits or code.'
        : COMPONENT_DESCRIPTIONS[`${plugin.name}/skills/${s}`] || englishOnly(meta.description || '');
      const flags = [];
      if (meta['user-invocable'] === false) flags.push('`user-invocable: false`');
      if (meta['disable-model-invocation'] === true) flags.push('`disable-model-invocation: true` (explicit invocation only)');
      body += `- \`${s}\` - ${cell(desc)}${flags.length ? ` **Invocation**: ${flags.join('; ')}.` : ''}\n`;
    }
    body += `\n`;
  }
  if (plugin.name === 'fpl-skills') {
    body += `The bundled \`forgeplan-cookbook\` description refers to forgeplan 0.32.1, not the current core release. Use the [CLI reference](/docs/cli/) and [MCP reference](/docs/mcp/) for current commands, tools and parameters.\n\n`;
  }

  const hooksPath = join(pdir, 'hooks/hooks.json');
  if (existsSync(hooksPath)) {
    const events = Object.keys(JSON.parse(readFileSync(hooksPath, 'utf8')).hooks || {}).sort();
    body += `## Hooks\n\n`;
    body += `Registered events in \`hooks/hooks.json\` (${events.length}): ${events.map((event) => `\`${event}\``).join(', ')}.\n\n`;
    body += `Registration does not guarantee activation in every client. Client support, setup and defaults-off settings still apply.\n\n`;
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
    const purpose = cell(firstSentence(englishOnly(PLUGIN_DESCRIPTIONS[p.name] || p.description || '').replace(/^\[DEPRECATED[^\]]*\]\s*/, '')));
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

function snapshotBlock(manifest, manifestText, root) {
  let commands = 0;
  let agents = 0;
  let skills = 0;
  for (const plugin of manifest.plugins) {
    const pdir = join(root, plugin.source || `./plugins/${plugin.name}`);
    commands += listDirFiles(join(pdir, 'commands')).length;
    agents += listDirFiles(join(pdir, 'agents'), '.md', true).length;
    skills += listSkillDirs(join(pdir, 'skills')).length;
  }
  const fingerprint = createHash('sha256').update(manifestText).digest('hex');
  return `${SNAPSHOT_START}\nCatalog version: **${manifest.metadata?.version || 'not declared'}**. Generated inventory: **${manifest.plugins.length} plugins, ${commands} command files, ${agents} agent files and ${skills} skills**. Agent files are counted recursively.\n\nCatalog fingerprint (SHA-256 of the source manifest bytes): \`${fingerprint}\`. This identifies the catalog, not every component file; counts and pages are generated from component files in the same explicit source directory.\n${SNAPSHOT_END}`;
}

function main() {
  if (!MARKETPLACE_DIR) fail('set MARKETPLACE_DIR explicitly to the marketplace source to publish; no local default is used');
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

  const manifestText = readFileSync(manifestPath, 'utf8');
  const manifest = JSON.parse(manifestText);
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
  overview = replaceBlock(overview, SNAPSHOT_START, SNAPSHOT_END, snapshotBlock(manifest, manifestText, MARKETPLACE_DIR), 'snapshot');
  overview = replaceBlock(overview, MARKER_START, MARKER_END, pluginsTable(plugins), 'plugins');
  if (overview.includes(PROFILES_START)) {
    overview = replaceBlock(overview, PROFILES_START, PROFILES_END, profilesTable(manifest.profiles), 'profiles');
  }
  writeFileSync(OVERVIEW, stripEmDashes(overview));

  console.log(`[mkt-gen] wrote ${wrote} plugin pages + overview table to ${OUT_DIR}`);
}

main();
