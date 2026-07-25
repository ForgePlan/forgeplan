#!/usr/bin/env node
// forge-understand — MVP loop:
//   scan a whole repo (forgeplan artifacts + freeform docs + code structure)
//   with headless Claude Code, render an interactive "understanding map" via the
//   bundled forge-diagram skill, write the HTML, open it in the browser.
//
// This proves the full loop the web view will later orchestrate. It deliberately
// has no backend and no dependencies beyond Node built-ins + the `claude` CLI.
//
// Usage:
//   node run.mjs                     # scan the Forgeplan repo (default), open result
//   node run.mjs --repo /path/to/x   # scan a different repo
//   node run.mjs --model opus        # force a model alias (default: your session model)
//   node run.mjs --out ./out/x.html  # custom output path
//   node run.mjs --no-open           # don't auto-open the browser
//   node run.mjs --dry-run           # print what would run, call nothing

import { spawn } from "node:child_process";
import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { dirname, join, resolve, isAbsolute } from "node:path";
import { fileURLToPath } from "node:url";
import { platform } from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));

// ---- args ------------------------------------------------------------------
const argv = process.argv.slice(2);
const flag = (name) => argv.includes(name);
const opt = (name, def) => {
  const i = argv.indexOf(name);
  return i !== -1 && argv[i + 1] ? argv[i + 1] : def;
};

const repo = resolve(opt("--repo", resolve(__dirname, "..", "..")));
const model = opt("--model", null); // null => use the caller's default session model
const lang = opt("--lang", null);   // null => match the project's primary docs language
const refineArg = opt("--refine", null); // path to an existing map to re-language / re-layout
const dryRun = flag("--dry-run");
const noOpen = flag("--no-open");
const stamp = new Date().toISOString().replace(/[:.]/g, "-");
const outArg = opt("--out", join(__dirname, "out", `understanding-${stamp}.html`));
const outPath = isAbsolute(outArg) ? outArg : resolve(process.cwd(), outArg);

const refineSource = refineArg
  ? readFileSync(resolve(process.cwd(), refineArg), "utf8")
  : null;
const langNames = { ru: "Russian", en: "English", de: "German", es: "Spanish", fr: "French" };
const languageDirective = lang
  ? `Write ALL human-readable text in ${langNames[lang] ?? lang}. Keep code identifiers, file paths, and technical tokens (forgeplan-core, R_eff, ADR-003, MCP, BGE-M3, fpl, crate names, relation names) verbatim and untranslated.`
  : `Write all human-readable text in the same language as the project's primary documentation; keep code identifiers and technical tokens verbatim.`;

// ---- load the skill (renderer guide) + style reference ---------------------
const skillDir = join(__dirname, "skills", "forge-diagram");
const skillRaw = readFileSync(join(skillDir, "SKILL.md"), "utf8");
const rendererGuide = skillRaw.replace(/^---[\s\S]*?---\s*/, "").trim(); // strip frontmatter
const styleReference = readFileSync(
  join(skillDir, "references", "architecture-example.html"),
  "utf8",
);

// ---- build the headless prompt --------------------------------------------
const writeContract = `OUTPUT CONTRACT (critical): use the Write tool to save the COMPLETE, self-contained HTML document to exactly this absolute path:
  ${outPath}
The document must start at <!DOCTYPE html> and end at </html>. Do NOT print the HTML in your text response — large HTML gets truncated mid-stream over stdout, so writing the file is the reliable path. After writing, reply with a single line: DONE ${outPath}`;

const scanPrompt = `You are building a single-screen, interactive HTML "understanding map" of the repository at:
  ${repo}

GOAL: a returning developer should open the file, click around, and *immediately* grasp how this system is put together — what the major pieces are, how they connect, and the few important flows through it. This is a RECALL tool for a finished system, not a design diagram.

LANGUAGE: ${languageDirective}

STEP 1 — SCAN THE WHOLE REPO (read-only). Use Glob/Grep/Read to gather understanding from all three sources:
  (a) Forgeplan artifacts: .forgeplan/**/*.md — PRD/RFC/ADR/Epic/Spec/Problem and their typed links (refs, informs, based_on, parent/child).
  (b) Freeform docs: README*, CLAUDE.md, docs/**/*.md, and any ADRs/design notes OUTSIDE .forgeplan.
  (c) Code structure: top-level packages/crates, entry points, the main modules, and how they depend on each other.
Reconcile the three. Where docs and code disagree, CODE is the source of truth for what exists; docs explain why. Be efficient — sample representative files, don't read every line of every file.

STEP 2 — SYNTHESISE: zones (group by subsystem / Epic / layer), nodes (components & artifacts), typed edges (data/control flow, dependencies, "informs"), and 3–6 named flows a reader can toggle.

STEP 3 — RENDER following the RENDERER GUIDE below (especially the Layout discipline) and matching the look + interactivity of the STYLE REFERENCE. Give the diagram room to breathe — no overlapping boxes, arrows, or labels.

${writeContract}

===== RENDERER GUIDE =====
${rendererGuide}

===== STYLE REFERENCE (match this look & interaction model) =====
${styleReference}
`;

const refinePrompt = `You are REVISING an existing self-contained HTML "understanding map". Do NOT scan any repository — work only from the HTML provided at the end. Preserve all captured understanding, structure, interactivity (flow chips, clickable nodes, animations), and dark mode. Change ONLY these two things:

1. LANGUAGE: ${languageDirective}

2. LAYOUT: the current version is too cramped — boxes, arrows, and labels overlap. Re-lay-it-out for breathing room per the Layout discipline in the guide below: a generously sized viewBox, columnar/layered placement, >=40px vertical / >=60px horizontal gaps between boxes, no overlapping text, edges drawn behind opaque-masked node backgrounds and routed AROUND boxes (never through them), spread edge entry/exit points so lines don't bundle, legend outside every zone. Favour clarity over completeness.

Keep the same nodes, zones, edges, flows, and behaviour — only the language and the visual layout change.

${writeContract}

===== RENDERER GUIDE (layout & language rules) =====
${rendererGuide}

===== EXISTING MAP TO REVISE =====
${refineSource}
`;

const prompt = refineSource ? refinePrompt : scanPrompt;

// ---- assemble the claude invocation ---------------------------------------
// The agent reads the repo and writes exactly one file (the output HTML). We
// read that file back — far more reliable than capturing a ~40KB HTML over
// stdout, which truncates at the token limit. NOTE: this gives the agent the
// Write tool, so it is NOT a hard read-only sandbox. For untrusted repos run it
// against a throwaway checkout, or scope writes via a settings allowlist
// (`Write(<outPath>)`) — see README.
const args = ["-p", prompt, "--add-dir", repo, "--output-format", "text",
  "--permission-mode", "acceptEdits"];
if (model) args.push("--model", model);
// keep the tool allowlist last so the variadic doesn't swallow other flags
args.push("--allowedTools", "Read", "Glob", "Grep", "Write");

mkdirSync(dirname(outPath), { recursive: true });

if (dryRun) {
  console.log("forge-understand — DRY RUN");
  console.log("  mode:        ", refineSource ? `refine (${refineArg})` : "scan");
  console.log("  repo:        ", repo);
  console.log("  lang:        ", lang ?? "(match project docs)");
  console.log("  model:       ", model ?? "(session default)");
  console.log("  out:         ", outPath);
  console.log("  prompt chars:", prompt.length, `(~${Math.round(prompt.length / 4)} tokens)`);
  console.log("  command:     ", "claude", "-p", "<prompt>", "--add-dir", repo,
    "--output-format", "text", "--permission-mode", "acceptEdits",
    ...(model ? ["--model", model] : []), "--allowedTools", "Read", "Glob", "Grep", "Write");
  console.log("\n(no call made; rerun without --dry-run to execute)");
  process.exit(0);
}

// ---- run -------------------------------------------------------------------
console.log(refineSource
  ? `forge-understand: revising ${refineArg} (language + layout) …`
  : `forge-understand: scanning ${repo} …`);
console.log(refineSource
  ? "(no repo scan — just re-rendering the existing map)\n"
  : "(this runs a full agentic read-only scan; expect minutes + token cost)\n");

const child = spawn("claude", args, { cwd: repo, stdio: ["ignore", "pipe", "inherit"] });

let stdout = "";
child.stdout.on("data", (d) => (stdout += d.toString()));

child.on("error", (err) => {
  console.error("Failed to launch `claude`. Is the CLI installed and on PATH?");
  console.error(err.message);
  process.exit(1);
});

child.on("close", (code) => {
  if (code !== 0) {
    console.error(`\nclaude exited with code ${code}.`);
    process.exit(code ?? 1);
  }
  // Primary: the agent wrote the HTML to outPath. Read it back.
  let html = null;
  if (existsSync(outPath)) {
    const onDisk = readFileSync(outPath, "utf8");
    if (/<\/html>/i.test(onDisk)) html = onDisk;
  }
  // Fallback: maybe the agent printed HTML to stdout instead of writing a file.
  if (!html) {
    const fromStdout = extractHtml(stdout);
    if (fromStdout) {
      html = fromStdout;
      writeFileSync(outPath, html, "utf8");
    }
  }
  if (!html) {
    console.error("\nNo complete HTML document produced. Raw stdout saved for inspection:");
    writeFileSync(outPath + ".raw.txt", stdout, "utf8");
    console.error("  " + outPath + ".raw.txt");
    process.exit(2);
  }
  console.log(`\n✓ map ready: ${outPath} (${html.length} bytes)`);
  if (!noOpen) openInBrowser(outPath);
  else console.log("  open it manually to view the map.");
});

// ---- helpers ---------------------------------------------------------------
function extractHtml(text) {
  if (!text) return null;
  // unwrap a ```html ... ``` fence if the model added one
  const fence = text.match(/```(?:html)?\s*([\s\S]*?)```/i);
  const body = fence ? fence[1] : text;
  const start = body.search(/<!DOCTYPE html>|<html[\s>]/i);
  if (start === -1) return null;
  const endIdx = body.toLowerCase().lastIndexOf("</html>");
  return endIdx === -1 ? body.slice(start) : body.slice(start, endIdx + 7);
}

function openInBrowser(file) {
  const cmd = platform() === "darwin" ? "open" : platform() === "win32" ? "start" : "xdg-open";
  spawn(cmd, [file], { stdio: "ignore", detached: true, shell: platform() === "win32" }).unref();
  console.log("  opening in your browser…");
}
